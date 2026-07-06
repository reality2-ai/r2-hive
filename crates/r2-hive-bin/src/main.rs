//! # r2-hive daemon entry point — bring-up, wiring, and the HTTP/WS front door
//!
//! ## Why this file exists
//!
//! Everything else in this crate is a subsystem waiting to be composed: the
//! shared state hub (`hive.rs`), the inbound router (`router.rs`), the
//! transports, the management API, the web-plugin surface, the USB watcher.
//! Something has to decide, in one auditable place, *which* of those run on
//! this particular host, in *what order*, with *what configuration* — and
//! refuse unsafe combinations (public bind without opt-in, keyring backend
//! on a build without the feature). This file is that place: CLI parsing,
//! config-file layering, transport auto-detection, subsystem bring-up, and
//! the axum HTTP/WebSocket listener that is the daemon's front door.
//!
//! Bring-up order matters and is deliberate: state hub → ensemble sink →
//! optional transports (LAN/BLE/LoRa) → route-engine maintenance → USB
//! watcher → management API (identity custody first — web-auth derives from
//! the master secret) → axum router LAST, so the `/r2/mgmt` route can see
//! whether mgmt actually started.
//!
//! ## How it interlinks (grep-verified)
//!
//! - Builds the `Arc<HiveState>` (`hive.rs`) that every task shares.
//! - Each transport bring-up (`start_lan_discovery` / `start_ble` /
//!   `start_lora`) spawns a receive loop that feeds every inbound frame to
//!   `router::route_frame` — the daemon's single routing decision point —
//!   and mirrors bytes to legacy WebSocket peers during the compat era.
//! - `/r2` upgrades into `compat/handshake.rs` (legacy browser protocol);
//!   `/r2/mgmt` (loopback only) into `mgmt/ws.rs`; `/health`, `/stats`,
//!   `/routes` are the ops surface; `/ensemble/*`, `/plugin/*` and
//!   `/r2/web/provision` are the R2-PLUGIN §13 web surface (`web.rs`).
//! - `spawn_usb_watcher` wires `usb_hotplug.rs` events into the same
//!   router, treating a paired dongle's radio as a local transport.
//! - `apply_config_layer` merges `config.rs` (TOML) under explicit CLI
//!   flags; `autoconfig.rs` supplies the `--auto` detection report;
//!   `systemd.rs` handles readiness + watchdog.
//!
//! ## Canon (r2-specifications)
//!
//! - R2-HOST-API §2.2 (loopback mgmt WebSocket transport) —
//!   `r2-specifications/specs/r2-core/R2-HOST-API.md`.
//! - R2-PLUGIN §13, §13.5 (web-plugin assets, browser provisioning) —
//!   `r2-specifications/specs/r2-core/R2-PLUGIN.md`.
//! - R2-WIRE §4.3.5 (compact↔extended transcode at the LoRa boundary) —
//!   `r2-specifications/specs/r2-core/R2-WIRE.md`.
//! - R2-USB (peripheral protocol; Appendix A transport kinds) —
//!   `r2-specifications/specs/r2-core/R2-USB.md`.
//! - R2-PROVISION §5.3.4 (SAS verification — why `--usb-auto-confirm-unsafe`
//!   is dev-only) — `r2-specifications/specs/r2-core/R2-PROVISION.md`.
//! - R2-DISCOVERY §3.3 / R2-BEACON §6.1 (rotating RBID advertisement) —
//!   `r2-specifications/specs/r2-core/R2-DISCOVERY.md`,
//!   `r2-specifications/specs/r2-core/R2-BEACON.md`.
//! - R2-FNV (self hive_id derivation from `--name`) —
//!   `r2-specifications/specs/r2-core/R2-FNV.md`.
//!
//! **Citation note (specs-ruled):** no R2-HIVE spec exists (implementation
//! repo name). Former "R2-HIVE §…" cites here are re-anchored to the real
//! canon — socket contract R2-TG-TOOL §5.1 (incl. the normative
//! `r2tgd.sock` filename, specs fa94443), identity custody R2-TG-TOOL §3 +
//! R2-WIRE §6.2.1, single-active-TG R2-TRUST §13.2 — with genuinely
//! daemon-local choices (store path, backend selection) marked as such.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::extract::{State, WebSocketUpgrade};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use clap::{CommandFactory, FromArgMatches, Parser};
use tower_http::cors::CorsLayer;

use r2_hive::{compat, hive, mgmt, plugins, router};
use hive::HiveState;
use mgmt::identity::FileStore;
use mgmt::state::DaemonState;

#[derive(Parser)]
#[command(name = "r2-hive", about = "R2 Hive — multi-transport mesh participant")]
struct Args {
    /// Port to listen on (WebSocket + HTTP).
    #[arg(long, default_value = "21042")]
    port: u16,

    /// Bind address.
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Permit binding the HTTP/WebSocket listener to a non-loopback address.
    /// The management WebSocket remains disabled on non-loopback listeners;
    /// use the Unix-domain management socket for local control.
    #[arg(long)]
    allow_public_bind: bool,

    /// Event buffer size per trust group.
    #[arg(long, default_value = "1000")]
    buffer_size: usize,

    /// Maximum total connections.
    #[arg(long, default_value = "10000")]
    max_connections: usize,

    /// Enable LAN discovery via mDNS and UDP transport.
    #[arg(long)]
    lan: bool,

    /// Enable BLE transport (beacon discovery + L2CAP connections).
    #[arg(long)]
    ble: bool,

    /// Enable LoRa transport via the local arduino-router IPC socket.
    /// Requires the host to be running arduino-router with an attached
    /// microcontroller flashed with the r2_lora_service Arduino sketch.
    #[arg(long)]
    lora: bool,

    /// Path to the arduino-router Unix socket. Only used with --lora.
    #[arg(long, default_value = "/var/run/arduino-router.sock")]
    lora_socket: String,

    /// Device name for mDNS/BLE advertisement.
    #[arg(long, default_value = "r2-hive")]
    name: String,

    /// Override the local management-socket path. Socket contract per
    /// R2-TG-TOOL §5.1 (v0.3): per-user path, mode 0600, same-UID, AND the
    /// `r2tgd.sock` filename — the well-known name is normative (specs
    /// ruling fa94443; renamed from r2-hive.sock).
    /// Default: ${XDG_RUNTIME_DIR}/r2tgd.sock on Linux, ${TMPDIR}/r2tgd.sock on macOS.
    #[arg(long)]
    mgmt_socket: Option<PathBuf>,

    /// Override the master-secret store path. Custody boundary per
    /// R2-TG-TOOL §3 + R2-WIRE §6.2.1; the concrete path is daemon-local
    /// (layout mirrors R2-TG-TOOL §9, informative).
    /// Default: $XDG_STATE_HOME/r2/master.key. Only honoured when the
    /// resolved identity backend is `file`.
    #[arg(long)]
    identity_store: Option<PathBuf>,

    /// Identity backend selection (daemon-local choice; the custody
    /// boundary it serves is R2-TG-TOOL §3 + R2-WIRE §6.2.1).
    /// `auto` picks the platform keyring when the `keyring` cargo
    /// feature is built in and reachable, falling back to the file
    /// store; `file` forces the file store; `keyring` forces the
    /// platform keyring (errors out if the build doesn't include the
    /// `keyring` feature). Default: `auto`.
    #[arg(long, default_value = "auto")]
    identity_backend: String,

    /// Disable the local management API (start only the mesh-side stack).
    #[arg(long)]
    no_mgmt: bool,

    /// **DEV BUILDS ONLY** (R2-BUILDMODE §5.1): serve web-plugin assets
    /// without browser auth when the web-auth registry is unavailable. The
    /// flag does not exist in a prod binary — structural absence, not
    /// default-off.
    #[cfg(feature = "dev")]
    #[arg(long)]
    web_dev_mode: bool,

    /// Run transport auto-detection at startup. When set, --lan, --ble,
    /// and --lora that are *not* explicitly passed are filled in from
    /// the detection report. Explicit flags always win. The detection
    /// report is logged either way (as a diagnostic) at INFO level.
    #[arg(long)]
    auto: bool,

    /// Path to a TOML config file (Phase 4b). Default:
    /// `$XDG_CONFIG_HOME/r2/hive.toml` or `~/.config/r2/hive.toml`.
    /// File values are overridden by any explicit CLI flag.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Disable the USB-attached peripheral watcher. By default
    /// r2-hive scans `/dev` for `ttyACM*`/`ttyUSB*` devices and
    /// drives the R2-USB v0.1 protocol against any it finds. Disable
    /// for headless servers / containers / development rigs that
    /// don't want the noise.
    #[arg(long)]
    no_usb: bool,

    /// Directory the USB watcher scans for serial devices. Defaults
    /// to `/dev`. Override for testing or custom udev layouts.
    #[arg(long, default_value = "/dev")]
    usb_dir: PathBuf,

    /// **DEV BUILDS ONLY** (R2-BUILDMODE §5.1): auto-confirm any SAS prompt
    /// from a freshly attached USB peripheral — defeats R2-PROVISION §5.3.4.
    /// Does not exist in a prod binary.
    #[cfg(feature = "dev")]
    #[arg(long)]
    usb_auto_confirm_unsafe: bool,

    /// **DEV BUILDS ONLY** (R2-BUILDMODE §5.1): bypass the default-deny USB
    /// device filter. Does not exist in a prod binary; the prod path is
    /// `--usb-vid-pid VID:PID` / `r2hive usb prepare`.
    #[cfg(feature = "dev")]
    #[arg(long)]
    usb_allow_any: bool,

    /// Permit a USB peripheral by `(idVendor, idProduct)` pair (each
    /// in lowercase hex, separated by `:`, e.g. `--usb-vid-pid
    /// 303a:1001`). Repeatable. The watcher only opens devices that
    /// match one of the listed pairs (plus operator-explicit
    /// allowlist, plus `--usb-allow-any` when set). Default empty —
    /// no device is permitted until R2 has a canonical assigned VID
    /// or operators configure their hardware here.
    #[arg(long, value_parser = parse_vid_pid)]
    usb_vid_pid: Vec<(u16, u16)>,
}

/// Parse a `vid:pid` argument as two hex u16s (`0x` prefixes tolerated).
///
/// **Used-by:** clap, as the `value_parser` for `--usb-vid-pid`.
fn parse_vid_pid(s: &str) -> Result<(u16, u16), String> {
    let (v, p) = s
        .split_once(':')
        .ok_or_else(|| format!("expected VID:PID, got {s:?}"))?;
    let vid = u16::from_str_radix(v.trim_start_matches("0x"), 16)
        .map_err(|e| format!("bad VID {v:?}: {e}"))?;
    let pid = u16::from_str_radix(p.trim_start_matches("0x"), 16)
        .map_err(|e| format!("bad PID {p:?}: {e}"))?;
    Ok((vid, pid))
}

/// Upgrade `/r2` into the legacy browser-WebSocket protocol
/// (`compat/handshake.rs` owns the connection from here).
///
/// **Used-by:** the axum route table in [`main`].
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<HiveState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| compat::handshake::handle_connection(socket, state))
}

/// `/health` liveness probe: static JSON, no auth, no state — safe on any
/// listener because it reveals nothing about the mesh.
///
/// **Used-by:** the axum route table in [`main`]; ops probes / systemd checks.
async fn health() -> Response {
    ([(header::CONTENT_TYPE, "application/json")],
     r#"{"status":"ok","class":"ai.reality2.wayfinder"}"#).into_response()
}

/// `/routes` topology dump: the route engine's neighbour + path tables as
/// JSON (per-transport link quality, confidence, ages) for dashboards and
/// bench instrumentation.
///
/// **Dependencies:** the `route_engine` lock (read snapshot);
/// `mgmt::ws::authorize_upgrade` for the auth gate.
///
/// **Used-by:** the axum route table in [`main`]; consumed by the composer
/// dashboard and bench scripts.
async fn routes_json(
    State(state): State<Arc<HiveState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    // R2 audit P0: /routes exposed the neighbour/path topology graph UNAUTHENTICATED while publicly
    // proxied. Gate it behind the same auth as mgmt (same-origin + web-auth cookie), fail-closed.
    if let Err(resp) = mgmt::ws::authorize_upgrade(&state, &headers) {
        return resp;
    }
    use r2_route::transport::Transport;
    let engine = state.route_engine.lock().await;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    let mut neighbours = String::from("[");
    let mut first = true;
    for entry in engine.neighbours().iter() {
        if !first { neighbours.push(','); }
        first = false;
        let age = now_secs.saturating_sub(entry.last_seen);
        // Emit per-transport quality for the four known transports.
        let q_ble  = entry.link_quality[Transport::Ble.index()];
        let q_wifi = entry.link_quality[Transport::Wifi.index()];
        let q_lora = entry.link_quality[Transport::Lora.index()];
        let q_inet = entry.link_quality[Transport::Internet.index()];
        neighbours.push_str(&format!(
            r#"{{"hive_id":"0x{:08X}","confidence":{:.3},"age_secs":{},"samples":{},"mobility":"{:?}","mcu_only":{},"quality":{{"ble":{:.3},"wifi":{:.3},"lora":{:.3},"internet":{:.3}}}}}"#,
            entry.hive_id, entry.confidence, age, entry.sample_count,
            entry.mobility, entry.mcu_only, q_ble, q_wifi, q_lora, q_inet
        ));
    }
    neighbours.push(']');

    let mut paths = String::from("[");
    let mut first = true;
    for entry in engine.paths().iter() {
        if !first { paths.push(','); }
        first = false;
        let age = now_secs.saturating_sub(entry.last_updated);
        paths.push_str(&format!(
            r#"{{"destination":"0x{:08X}","next_hop":"0x{:08X}","confidence":{:.3},"age_secs":{},"samples":{}}}"#,
            entry.destination, entry.next_hop, entry.confidence, age, entry.sample_count
        ));
    }
    paths.push(']');

    let json = format!(
        r#"{{"self":"0x{:08X}","now":{},"neighbours":{},"paths":{}}}"#,
        state.self_hive_id, now_secs, neighbours, paths
    );
    ([(header::CONTENT_TYPE, "application/json")], json).into_response()
}

/// `/stats` counters: connection/frame totals and uptime as JSON.
///
/// **Dependencies:** `ws_transport` peer map + the atomic counters on
/// [`HiveState`]; `mgmt::ws::authorize_upgrade` for the auth gate.
///
/// **Used-by:** the axum route table in [`main`]; dashboards/ops.
async fn stats_json(
    State(state): State<Arc<HiveState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    // R2 audit P0: /stats exposed topology/neighbour stats UNAUTHENTICATED while publicly proxied.
    // Gate it behind the same auth as mgmt (same-origin + web-auth cookie), fail-closed.
    if let Err(resp) = mgmt::ws::authorize_upgrade(&state, &headers) {
        return resp;
    }
    use r2_discovery::PeerMap;
    let peers = state.ws_transport.peers().peer_count();
    let frames = state.frames_routed.load(Ordering::Relaxed);
    let connections_total = state.connections_total.load(Ordering::Relaxed);
    let uptime_secs = state.started_at.elapsed().as_secs();

    let json = format!(
        r#"{{"connections":{},"frames_routed":{},"connections_total":{},"uptime_secs":{}}}"#,
        peers, frames, connections_total, uptime_secs
    );

    ([(header::CONTENT_TYPE, "application/json")], json).into_response()
}

/// Daemon entry point. Composes the subsystems in dependency order (see the
/// file head for why the order is what it is) and then serves axum forever.
///
/// **Dependencies:** everything in the interlink map above; refuses to start
/// on a non-loopback bind without `--allow-public-bind`, and exits on an
/// unknown/unbuilt identity backend rather than falling back silently.
///
/// **Used-by:** the binary target (`cargo run -p r2-hive`); systemd units.
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Parse CLI with metadata so we can distinguish "user passed this"
    // from "compiled-in default" — needed to layer the config file
    // *under* CLI overrides without losing explicit user choices.
    let cli_matches = Args::command().get_matches();
    let mut args = Args::from_arg_matches(&cli_matches).expect("clap derive");

    // Load layered config (file values, then CLI overrides). Phase 4b.
    apply_config_layer(&mut args, &cli_matches);

    // Self hive_id = raw FNV-1a-32 of --name (R2-FNV; no canonicalisation —
    // the operator's exact name string is the identity input).
    let self_hive_id = r2_fnv::fnv1a_32(args.name.as_bytes());
    log::info!("self hive_id: 0x{:08X} (from name '{}')", self_hive_id, args.name);
    let state = Arc::new(HiveState::new(self_hive_id, args.buffer_size, args.max_connections));

    // Phase 4a: transport auto-detection. Always probe (cheap; logged
    // for ops). When --auto is set, fold the detection report into the
    // transport flags so a flagless `r2-hive --auto` Just Works on the
    // four supported deployment profiles.
    let report = r2_hive::autoconfig::detect_profile(std::path::Path::new(&args.lora_socket));
    log::info!("Transport profile:");
    for line in report.summary_lines() {
        log::info!("  {line}");
    }
    if args.auto {
        if !args.lan && report.should_run_lan() {
            args.lan = true;
            log::info!("  --auto: enabling LAN (networking present)");
        }
        if !args.ble && report.should_run_ble() {
            #[cfg(feature = "transport-ble")]
            {
                args.ble = true;
                if report.ble.qca {
                    log::warn!(
                        "  --auto: enabling BLE on QCA driver — see TEST-RIG.md known-issues; \
                         route engine should de-prioritise BLE on this host"
                    );
                } else {
                    log::info!("  --auto: enabling BLE (hci0 present, driver {})", report.ble.driver);
                }
            }
            #[cfg(not(feature = "transport-ble"))]
            {
                log::warn!(
                    "  --auto: BLE present on host but binary built without `--features ble`"
                );
            }
        }
        if !args.lora && report.should_run_lora() {
            #[cfg(feature = "transport-lora")]
            {
                args.lora = true;
                log::info!(
                    "  --auto: enabling LoRa (socket {} reachable)",
                    report.lora.socket_path.display()
                );
            }
            #[cfg(not(feature = "transport-lora"))]
            {
                log::warn!(
                    "  --auto: LoRa socket present but binary built without `--features lora`"
                );
            }
        }
    }

    // Wire the ensemble registry's OutboundSink to the hive's transport
    // layer so events emitted by sentant `Action::Send` are framed and
    // routed back through `send_to_hive` / `broadcast_to_tg`.
    state.ensembles.set_sink(std::sync::Arc::new(
        r2_hive::mgmt::ensemble::HiveOutboundSink {
            hive: state.clone(),
        },
    ));

    // Slot for the DaemonState reference that the /r2/mgmt WebSocket route
    // needs. Populated by the mgmt-bringup block below (unless --no-mgmt is
    // set). Kept outside the if/else so the router can reference it whether
    // mgmt is on or off.
    let mut ws_mgmt_state: Option<DaemonState> = None;

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .expect("invalid bind address");
    if !addr.ip().is_loopback() && !args.allow_public_bind {
        log::error!(
            "refusing non-loopback bind {} without --allow-public-bind; \
             default is loopback to keep local control surfaces private",
            addr
        );
        return;
    }

    log::info!("r2-hive listening on {}", addr);
    log::info!("  Dashboard: http://{}:{}/", args.bind, args.port);
    log::info!("  WebSocket: ws://{}:{}/r2", args.bind, args.port);
    log::info!("  Buffer: {} frames/group", args.buffer_size);
    log::info!("  Max connections: {}", args.max_connections);
    #[cfg(feature = "dev")]
    if args.web_dev_mode {
        state.set_web_dev_mode(true);
        log::warn!(
            "  Web auth: --web-dev-mode is set — web-plugin assets may be served without auth. DEVELOPMENT USE ONLY."
        );
    }

    // `mut` is used by the transport-gated `active_plugins.push(...)` calls below; when all such
    // transports are composed out, no push remains, so allow the otherwise-unused `mut`.
    #[allow(unused_mut)]
    let mut active_plugins = vec!["word-codes", "dashboard"];

    // LAN discovery: UDP beacon
    #[cfg(feature = "transport-udp")]
    if args.lan {
        match start_lan_discovery(&args, &state, self_hive_id).await {
            Ok(()) => active_plugins.push("lan-discovery"),
            Err(e) => log::error!("LAN discovery failed to start: {}", e),
        }
    }

    // BLE transport
    #[cfg(feature = "transport-ble")]
    if args.ble {
        match start_ble(&args, &state, self_hive_id).await {
            Ok(()) => active_plugins.push("ble"),
            Err(e) => log::error!("BLE failed to start: {}", e),
        }
    }

    // LoRa transport via arduino-router IPC
    #[cfg(feature = "transport-lora")]
    if args.lora {
        match start_lora(&args, &state, self_hive_id).await {
            Ok(()) => active_plugins.push("lora"),
            Err(e) => log::error!("LoRa failed to start: {}", e),
        }
    }

    log::info!("  Plugins: {}", active_plugins.join(", "));

    // Spawn route engine maintenance (decay neighbours/paths every 30s)
    tokio::spawn(router::maintenance_loop(state.clone()));

    // Phase USB-3c: USB-attached peripheral watcher (R2-USB v0.1).
    // Linux-only; behind a runtime opt-out so headless servers can
    // skip it.
    #[cfg(target_os = "linux")]
    if !args.no_usb {
        // R2-BUILDMODE §5.1: in a prod build the two USB bypasses are
        // compile-time false — the flags don't exist, so these fold away.
        #[cfg(feature = "dev")]
        let (usb_auto_confirm, usb_allow_any) =
            (args.usb_auto_confirm_unsafe, args.usb_allow_any);
        #[cfg(not(feature = "dev"))]
        let (usb_auto_confirm, usb_allow_any) = (false, false);
        let usb_handle = spawn_usb_watcher(
            args.usb_dir.clone(),
            usb_auto_confirm,
            usb_allow_any,
            args.usb_vid_pid.clone(),
            state.clone(),
        );
        state.set_usb_handle(usb_handle);
        if usb_allow_any {
            log::warn!(
                "  USB watcher: --usb-allow-any is set — every CDC-ACM device \
                 will be probed for R2-USB v0.1. DEVELOPMENT USE ONLY."
            );
        } else if args.usb_vid_pid.is_empty() {
            log::info!(
                "  USB watcher: enabled (scan dir {}); no VID/PIDs configured — \
                 watcher will skip every device until --usb-vid-pid is set or a \
                 device is added via `r2hive usb prepare` (Phase USB-4).",
                args.usb_dir.display()
            );
        } else {
            log::info!(
                "  USB watcher: enabled (scan dir {}); permitted VID:PID pairs: {}",
                args.usb_dir.display(),
                args.usb_vid_pid
                    .iter()
                    .map(|(v, p)| format!("{v:04x}:{p:04x}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    // Spawn the local management API unless opted out (socket discipline
    // R2-TG-TOOL §5; UDS binding R2-HOST-API §2.2; identity custody
    // R2-TG-TOOL §3; SAS pairing gate R2-PROVISION §5.3.4).
    let _mgmt_handle = if args.no_mgmt {
        log::info!("  Management API: disabled (--no-mgmt)");
        None
    } else {
        let mgmt_socket_path = args.mgmt_socket.unwrap_or_else(mgmt::default_socket_path);
        let store_path = args.identity_store.unwrap_or_else(FileStore::default_path);
        // Resolve the identity backend per the operator flag
        // (daemon-local backend policy, Phase 4c; custody boundary
        // R2-TG-TOOL §3 + R2-WIRE §6.2.1).
        let store: Box<dyn mgmt::identity::IdentityStore> = match args.identity_backend.as_str() {
            "file" => Box::new(FileStore::new(store_path.clone())),
            "auto" => mgmt::identity::auto_store(store_path.clone()),
            "keyring" | "libsecret" | "keychain" | "wincred" => {
                #[cfg(feature = "keyring")]
                {
                    Box::new(mgmt::identity::KeyringStore::new())
                }
                #[cfg(not(feature = "keyring"))]
                {
                    log::error!(
                        "--identity-backend={}: this build of r2-hive does not include the `keyring` cargo feature; rebuild with `--features keyring` or use `--identity-backend file`",
                        args.identity_backend
                    );
                    return;
                }
            }
            other => {
                log::error!(
                    "--identity-backend={}: unknown value (expected auto|file|keyring)",
                    other
                );
                return;
            }
        };
        match DaemonState::with_identity_store(store.as_ref()) {
            Ok(daemon_state) => {
                // Attach the HiveState so the r2.api.* primitive surface
                // (R2-HOST-API §3) can reach the wire / route / transport
                // layer. Done after with_identity so existing tests that
                // construct DaemonState in isolation still pass.
                daemon_state.attach_hive_state(state.clone());
                log::info!(
                    "  Management API: {} (identity {} via {} — fingerprint {})",
                    mgmt_socket_path.display(),
                    if daemon_state.identity_created_this_start() {
                        "generated fresh"
                    } else {
                        "loaded existing"
                    },
                    daemon_state.identity_backend(),
                    daemon_state.identity_fingerprint()
                );
                // Note for the WS route: the loopback /r2/mgmt WebSocket
                // surface (R2-HOST-API §2.2) is mounted onto the existing
                // axum router below, sharing this DaemonState via Extension.
                // Install browser-auth registry derived from the master
                // secret (R2-PLUGIN §13.5). Without this, web-plugin
                // assets fail closed unless --web-dev-mode was explicitly
                // requested.
                if let Some(key) = daemon_state.derive_web_auth_key() {
                    let auth = std::sync::Arc::new(r2_hive::web_auth::WebAuth::new(key));
                    state.set_web_auth(auth);
                    log::info!("  Web auth: enabled (cookie-bound to master secret)");
                } else {
                    log::warn!(
                        "  Web auth: unavailable (no identity); web-plugin assets fail closed unless --web-dev-mode is set"
                    );
                }
                ws_mgmt_state = Some(daemon_state.clone());
                match mgmt::socket::spawn(mgmt_socket_path, daemon_state).await {
                    Ok(h) => Some(h),
                    Err(e) => {
                        log::error!("management socket failed to start: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::error!("identity custody failed: {} — management API not started", e);
                None
            }
        }
    };

    // Build the axum router. Done here (rather than earlier) so the
    // /r2/mgmt route can pick up `ws_mgmt_state` populated by the mgmt
    // bringup above. The mgmt WS surface is only mounted if the daemon
    // started its mgmt subsystem; --no-mgmt also disables /r2/mgmt.
    let mut app = Router::new()
        .route("/r2", get(ws_handler))
        .route("/health", get(health))
        .route("/stats", get(stats_json))
        .route("/routes", get(routes_json))
        // R2-PLUGIN §13: web-plugin static asset surface. Single
        // catch-all per top-level segment; the registry resolves the
        // exact mount per request, so mount/unmount needs no router
        // rebuild (§13.4 atomic mount/unmount).
        .route("/ensemble/{*rest}", get(r2_hive::web::serve_web_plugin))
        .route("/plugin/{*rest}", get(r2_hive::web::serve_web_plugin))
        // R2-PLUGIN §13.5 — browser provisioning endpoint.
        .route(
            "/r2/web/provision",
            get(r2_hive::web::web_provision_get).post(r2_hive::web::web_provision_post),
        )
        .merge(plugins::word_codes::routes())
        .merge(plugins::dashboard::routes());

    if let Some(daemon_state) = ws_mgmt_state {
        // /r2/mgmt is the loopback parallel transport for R2-HOST-API §2.2.
        // It carries the same R2-WIRE extended frames as the UDS but as
        // binary WebSocket messages (no length prefix; WS provides framing).
        if addr.ip().is_loopback() {
            app = app
                .route("/r2/mgmt", get(mgmt::ws::handler))
                .layer(axum::Extension(daemon_state));
            log::info!("  Management WS: ws://{}:{}/r2/mgmt (auth required)", args.bind, args.port);
        } else {
            log::warn!(
                "  Management WS: disabled on non-loopback listener {}; use the management Unix socket",
                addr
            );
        }
    }

    let app = app.layer(CorsLayer::permissive()).with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Phase 3e: tell systemd we're ready and start the watchdog ping.
    // No-ops on non-systemd builds / non-systemd parents.
    r2_hive::systemd::notify_ready();
    r2_hive::systemd::spawn_watchdog();

    axum::serve(listener, app).await.unwrap();
}

/// Bring up the UDP LAN transport + beacon and spawn its receive loop.
///
/// **Purpose:** binds the R2-WIRE UDP port, registers the transport on
/// [`HiveState`] so egress can use it, then loops every inbound datagram
/// into `router::route_frame` (plus the legacy word-code `WC:` sideband and
/// a compat mirror to WebSocket peers). Recv errors back off and continue —
/// a transient ICMP error must never kill the loop.
///
/// **Dependencies:** `r2_discovery` UDP binding + beacon, `router.rs`,
/// `plugins/word_codes.rs`.
///
/// **Used-by:** [`main`] when `--lan` (or `--auto` detection) enables LAN.
#[cfg(feature = "transport-udp")]
async fn start_lan_discovery(args: &Args, state: &Arc<HiveState>, _self_hive_id: u32) -> Result<(), String> {
    use r2_discovery::discovery::udp_beacon::UdpBeacon;
    use r2_discovery::bindings::udp_lan::UdpLanTransport;
    use r2_discovery::{AsyncTransport, BeaconAdvertiser, PeerMap, Rbid};

    // Start UDP transport on the R2-WIRE port for frame exchange
    let udp_addr = format!("{}:{}", args.bind, 21042);
    let udp = Arc::new(
        UdpLanTransport::bind(&udp_addr)
            .await
            .map_err(|e| format!("UDP bind failed: {:?}", e))?,
    );
    log::info!("  UDP LAN: {}", udp_addr);

    // Register UDP transport with wayfinder state
    state.set_udp_transport(udp.clone()).await;

    // Spawn UDP frame receive loop - route inbound frames
    let state_rx = state.clone();
    let udp_rx = udp.clone();
    tokio::spawn(async move {
        loop {
            // A transient recv error (e.g. ECONNREFUSED surfaced from a prior
            // send's ICMP port-unreachable) must NOT permanently kill the
            // inbound loop — log, back off briefly, and keep serving.
            let frame = match udp_rx.recv().await {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("UDP LAN recv error: {:?}; continuing", e);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let data = &frame.data;

            // Check for word code broadcast (proximity-limited join support)
            if data.len() > 3 && &data[..3] == b"WC:" {
                if let Ok(msg) = std::str::from_utf8(data) {
                    let parts: Vec<&str> = msg[3..].splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let (words, tg_hash, join_code) = (parts[0], parts[1], parts[2]);
                        state_rx.word_codes.register(
                            words.to_string(), tg_hash.to_string(), join_code.to_string()
                        ).await;
                        log::info!("word code received from LAN peer: {} -> tg:{}", words, &tg_hash[..8.min(tg_hash.len())]);
                    }
                }
                continue;
            }

            // Regular R2-WIRE frame: feed the route engine and let it forward
            // through whatever transport mix it sees fit. The router is
            // trust-agnostic — UDP inbound has no TG context to enrich with.
            state_rx.frames_routed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            log::info!("UDP inbound: {} bytes from hive 0x{:08X}", data.len(), frame.source_hive);

            let _ = router::route_frame(
                &state_rx, frame.source_hive,
                r2_route::transport::Transport::Internet,
                data,
            ).await;

            // Also forward to any WebSocket peers (compat layer for browser
            // clients that haven't been migrated to the route engine yet).
            let hive_ids = state_rx.ws_transport.peers().hive_ids();
            for hive_id in hive_ids {
                let _ = state_rx.ws_transport.send(hive_id, data).await;
            }
        }
    });

    // Advertise this hive over the UDP beacon (R2-BEACON over R2-WIFI).
    // The advertised RBID is the device's own rotating id (R2-DISCOVERY §3.3 /
    // R2-BEACON §6.1) — NOT a hive_id (it must not be derivable). TODO: derive it
    // via PeerRegistry::own_rbid(epoch) from the trust-layer session_key
    // (R2-PROVISION/R2-TRUST) once that is wired; for now a rotating placeholder
    // (beacon emit is a scaffold returning Unsupported).
    let beacon = UdpBeacon::new(
        0x00000000,          // class hash - generic for now
        Rbid(random_rbid()),
        21042,               // advertise our UDP R2-WIRE port
        &[],                 // bloom
        0,                   // bloom_k
    );
    let _ = beacon.start(&[], 0).await; // scaffold: non-fatal until R2-WIFI emit lands

    // TODO(r2-discovery): UDP beacon SCANNING + peer registration are not in the
    // ratified R2-DISCOVERY v0.1 API — UdpBeacon is advertiser-only, UdpLanTransport
    // has no add_peer, and rbid->hive_id requires a PeerRegistry. The prior
    // discovered-peer handler (resolve RBID -> register peer -> ingest a route
    // observation) is retired until that surface lands; see
    // docs/r2-discovery-consumer-requirements.md §8. (Recoverable from git history.)

    Ok(())
}

/// Bring up the BLE transport (L2CAP CoC + beacon advertise/scan) and spawn
/// its receive + discovery loops.
///
/// **Purpose:** same shape as [`start_lan_discovery`] — register transport,
/// feed inbound frames to `router::route_frame`, mirror to WS peers — plus a
/// discovery loop that registers scanned peers and ingests a route-engine
/// observation per beacon so BLE neighbours become routable.
///
/// **Dependencies:** `r2_discovery` BLE binding/scanner/advertiser,
/// `router.rs`, the route engine lock.
///
/// **Used-by:** [`main`] when `--ble` (or `--auto` detection) enables BLE.
#[cfg(feature = "transport-ble")]
async fn start_ble(args: &Args, state: &Arc<HiveState>, self_hive_id: u32) -> Result<(), String> {
    use r2_discovery::bindings::ble::BleTransport;
    use r2_discovery::discovery::ble_beacon::BleBeaconScanner;
    use r2_discovery::{provisional_hive_id, AsyncTransport, BeaconScanner, PeerMap, Rbid};

    // Create BLE transport (starts scheduler + L2CAP listener)
    let (ble, disco_rx) = BleTransport::new(args.name.clone())
        .await
        .map_err(|e| format!("BLE init failed: {:?}", e))?;

    // Start BLE beacon advertising with this device's own rotating RBID
    // (R2-DISCOVERY §3.3 / R2-BEACON §6.1 — NOT a hive_id). TODO: derive via
    // PeerRegistry::own_rbid(epoch) from the trust-layer session_key once wired.
    {
        use r2_discovery::discovery::ble_beacon::BleBeaconAdvertiser;
        use r2_discovery::BeaconAdvertiser;
        let rbid = Rbid(random_rbid());
        let advertiser = BleBeaconAdvertiser::new(ble.sched().clone(), rbid);
        let _ = advertiser.start(&[], 0).await; // scaffold: non-fatal until R2-BLE emit lands
    }

    log::info!("  BLE: adapter ready, advertising + scanning");

    // Register with state
    state.set_ble_transport(ble.clone()).await;

    // Spawn BLE frame receive loop. Feed inbound frames through the trust-
    // agnostic router so the route engine sees BLE observations and the
    // forwarding decision uses the multi-transport fallback chain via
    // state.send_to_hive(). BLE inbound has no TG context for enrichment.
    let state_rx = state.clone();
    let ble_rx = ble.clone();
    tokio::spawn(async move {
        loop {
            // Transient recv errors must not kill the inbound loop (see UDP).
            let frame = match ble_rx.recv().await {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("BLE recv error: {:?}; continuing", e);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            state_rx.frames_routed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            log::info!("BLE inbound: {} bytes from hive 0x{:08X}", frame.data.len(), frame.source_hive);

            let _ = router::route_frame(
                &state_rx, frame.source_hive,
                r2_route::transport::Transport::Ble,
                &frame.data,
            ).await;

            // Also forward to WebSocket peers (compat layer for browser clients).
            let hive_ids = state_rx.ws_transport.peers().hive_ids();
            for hive_id in hive_ids {
                let _ = state_rx.ws_transport.send(hive_id, &frame.data).await;
            }
        }
    });

    // Spawn BLE beacon discovery handler. Use canonical hive_id from beacon
    // rbid so this peer matches its UDP-discovered counterpart.
    let scanner = BleBeaconScanner::new(disco_rx);
    let state2 = state.clone();
    let ble2 = ble.clone();
    tokio::spawn(async move {
        while let Ok(obs) = scanner.next_beacon().await {
            // Resolve the observed RBID to a known peer (R2-DISCOVERY §3.3). With no
            // PeerRegistry wired yet, fall back to a provisional id from the transport
            // address; a trusted-peer resolver supplies the canonical id once the
            // trust layer (R2-PROVISION/R2-TRUST) feeds session_keys in.
            let hive_id = provisional_hive_id(&obs.transport_address);
            if hive_id == self_hive_id {
                continue; // skip our own beacon if it loops back
            }
            // Register the discovered peer's transport address (R2-TRANSPORT §2.1.3
            // add_peer upcall) so outbound BLE sends can resolve it.
            ble2.peers().add_peer(hive_id, obs.transport_address.0.clone(), obs.link);
            log::info!("BLE peer discovered: hive=0x{:08X} addr={}", hive_id, obs.transport_address.0);

            // Ingest a beacon-discovery observation so the route engine knows this
            // BLE peer exists. Same rationale as the UDP beacon handler.
            use r2_route::neighbour::{MobilityClass, Observation};
            use r2_route::transport::{QualitySample, Transport};
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as u32).unwrap_or(0);
            let route_obs = Observation {
                hive_id,
                transport: Transport::Ble,
                timestamp: now_secs,
                quality: QualitySample::Direct(0.6),
                rssi: None,
                mcu_origin: false,
                mobility: MobilityClass::Mobile,
                // v0.7: this IS a scanning surface, but BeaconObservation does
                // not yet surface build_class — None until core adds the byte
                // (flagged; then: Some(BuildMode::from_wire(obs.build_class))).
                build_mode: None,
            };
            state2.route_engine.lock().await.ingest_observation(route_obs);
        }
    });

    Ok(())
}

/// Bring up the LoRa transport (via the arduino-router IPC socket) and
/// spawn its receive loop.
///
/// **Purpose:** LoRa carries COMPACT R2-WIRE frames on the air; this loop
/// transcodes compact→extended at the transport boundary (R2-WIRE §4.3.5)
/// before handing frames to `router::route_frame`, mirroring the extended
/// bytes to WS peers so browser decoders see one consistent format.
///
/// **Dependencies:** `r2_discovery` LoRa binding, the r2-wire transcoder,
/// `router.rs`; requires arduino-router listening on `--lora-socket`.
///
/// **Used-by:** [`main`] when `--lora` (or `--auto` detection) enables LoRa.
#[cfg(feature = "transport-lora")]
async fn start_lora(args: &Args, state: &Arc<HiveState>, _self_hive_id: u32) -> Result<(), String> {
    use r2_discovery::bindings::lora::LoraTransport;
    use r2_discovery::{AsyncTransport, PeerMap};

    // Connect to the local arduino-router IPC socket and verify the radio
    // is reachable. Construction will fail with a useful message if the
    // socket isn't there or the radio isn't responding.
    let lora = Arc::new(
        LoraTransport::with_socket(&args.lora_socket)
            .await
            .map_err(|e| format!("LoRa connect failed: {:?}", e))?,
    );
    log::info!("  LoRa: arduino-router socket {} (transport ready)", args.lora_socket);

    // Register with state so the route engine can send outbound frames
    // over LoRa via send_to_hive_via (with extended→compact transcoding
    // at the transport boundary).
    state.set_lora_transport(lora.clone()).await;

    // Spawn the LoRa receive loop. Each frame received via the IPC poll
    // gets handed to the route engine the same way UDP/BLE frames are.
    // The route engine then decides whether to forward via WiFi/UDP/BLE/WS
    // based on the frame's target and the engine's neighbour table.
    let state_rx = state.clone();
    let lora_rx = lora.clone();
    tokio::spawn(async move {
        log::info!("LoRa receive loop started");
        loop {
            // Transient recv errors must not kill the inbound loop (see UDP).
            let frame = match lora_rx.recv().await {
                Ok(f) => f,
                Err(e) => {
                    log::warn!("LoRa recv error: {:?}; continuing", e);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let data = &frame.data;
            log::info!(
                "LoRa inbound: {} bytes from hive 0x{:08X} (RSSI signalled by IPC)",
                data.len(),
                frame.source_hive
            );
            state_rx
                .frames_routed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // LoRa carries compact-format R2-WIRE frames (12-byte header).
            // The route engine works in extended format (22-byte header).
            // Transcode at the transport boundary per R2-WIRE §4.3.5 before
            // handing to route_frame. If transcoding fails (frame is too
            // short or malformed), fall back to passing raw bytes to WS
            // peers who can decide for themselves.
            let extended = {
                let mut buf = vec![0u8; data.len() + 64]; // headroom for extended header expansion
                match r2_wire::transcode::transcode_compact_to_extended(data, &mut buf) {
                    Ok(n) => {
                        log::info!("LoRa: transcoded compact ({} bytes) → extended ({} bytes)", data.len(), n);
                        Some(buf[..n].to_vec())
                    }
                    Err(e) => {
                        log::warn!("LoRa: compact→extended transcode failed: {:?} — skipping route engine", e);
                        None
                    }
                }
            };

            // Feed the EXTENDED frame to the route engine so it can parse
            // the header and make a forwarding decision.
            if let Some(ref ext) = extended {
                let _ = router::route_frame(
                    &state_rx,
                    frame.source_hive,
                    r2_route::transport::Transport::Lora,
                    ext,
                )
                .await;
            }

            // Also forward to any WebSocket peers (browser clients still
            // attached via /r2). Use the EXTENDED bytes so browser-side
            // decoders see a consistent format regardless of inbound transport.
            let ws_data = extended.as_deref().unwrap_or(data);
            let hive_ids = state_rx.ws_transport.peers().hive_ids();
            for hive_id in hive_ids {
                let _ = state_rx.ws_transport.send(hive_id, ws_data).await;
            }
        }
    });

    Ok(())
}

/// Layer config-file defaults under CLI flags (Phase 4b).
///
/// For each setting, the precedence is:
/// 1. CLI flag, if explicitly passed by the user.
/// 2. `[section].field` from the loaded config file.
/// 3. Compiled-in default (already in `args` from clap parse).
///
/// Booleans use OR semantics — a `true` in either CLI or config wins.
/// To force-off a transport that the config enables, edit the config
/// (we don't ship `--no-foo` flags yet).
///
/// **Dependencies:** `config.rs` (`HiveConfig`); clap's `ValueSource` to
/// distinguish "user passed this" from "compiled-in default". Exits(2) on
/// an unreadable/invalid config file rather than running misconfigured.
///
/// **Used-by:** [`main`], immediately after CLI parse.
fn apply_config_layer(args: &mut Args, matches: &clap::ArgMatches) {
    use clap::parser::ValueSource;

    let path = args
        .config
        .clone()
        .or_else(r2_hive::config::HiveConfig::default_path);
    let cfg = match path.as_ref() {
        Some(p) => match r2_hive::config::HiveConfig::load_optional(p) {
            Ok(c) => {
                if p.exists() {
                    log::info!("config: loaded {}", p.display());
                }
                c
            }
            Err(e) => {
                log::error!("{e}");
                std::process::exit(2);
            }
        },
        None => r2_hive::config::HiveConfig::default(),
    };

    // explicitly_set: true iff the operator passed this flag on the CLI.
    let explicitly_set = |id: &str| -> bool {
        matches
            .value_source(id)
            .map(|s| s == ValueSource::CommandLine)
            .unwrap_or(false)
    };

    if !explicitly_set("name") {
        args.name = cfg.daemon.name.clone();
    }
    if !explicitly_set("bind") {
        args.bind = cfg.daemon.bind.clone();
    }
    if !explicitly_set("port") {
        args.port = cfg.daemon.port;
    }
    if !explicitly_set("buffer_size") {
        args.buffer_size = cfg.daemon.buffer_size;
    }
    if !explicitly_set("max_connections") {
        args.max_connections = cfg.daemon.max_connections;
    }

    // Transports: OR semantics for booleans so config-enabled
    // transports stay on even when the operator omits the flag.
    args.auto = args.auto || cfg.transports.auto;
    args.lan = args.lan || cfg.transports.lan;
    args.ble = args.ble || cfg.transports.ble;
    args.lora = args.lora || cfg.transports.lora;
    if !explicitly_set("lora_socket") {
        args.lora_socket = cfg
            .transports
            .lora_socket
            .to_string_lossy()
            .into_owned();
    }

    if !explicitly_set("identity_backend") {
        args.identity_backend = cfg.identity.backend.clone();
    }
    if !explicitly_set("identity_store") && cfg.identity.store.is_some() {
        args.identity_store = cfg.identity.store.clone();
    }

    // [management].enabled = false ⇒ same as --no-mgmt.
    if !explicitly_set("no_mgmt") && !cfg.management.enabled {
        args.no_mgmt = true;
    }
    if !explicitly_set("mgmt_socket") && cfg.management.socket.is_some() {
        args.mgmt_socket = cfg.management.socket.clone();
    }
}

/// Phase USB-3c: spawn the hot-plug watcher and a log-and-react
/// consumer that surfaces session events to the operator.
///
/// `auto_confirm_unsafe` is a dev-only escape hatch — when true,
/// every `PairingPrompt` is immediately confirmed without operator
/// review, which defeats R2-PROVISION §5.3.4 (SAS verification). Production
/// deployments leave it false; the eventual Cosmic / KDE applet will
/// drive `SessionControl::UserConfirms` via the management socket.
///
/// **Dependencies:** `usb_hotplug.rs` (watcher + handle), `usb_serial.rs`
/// (session control), [`handle_session_event`] for per-event handling.
///
/// **Used-by:** [`main`] unless `--no-usb`; the returned handle is stashed
/// on [`HiveState`] for the `r2.mgmt.usb.*` handlers.
#[cfg(target_os = "linux")]
fn spawn_usb_watcher(
    scan_dir: PathBuf,
    auto_confirm_unsafe: bool,
    allow_any: bool,
    vid_pid_allowlist: Vec<(u16, u16)>,
    state: Arc<HiveState>,
) -> r2_hive::usb_hotplug::UsbBringupHandle {
    use r2_hive::usb_hotplug::{watcher_with_default_store, HotPlugEvent, UsbFilter};
    use r2_hive::usb_serial::SessionControl;

    if auto_confirm_unsafe {
        log::warn!(
            "  USB watcher: --usb-auto-confirm-unsafe is set — SAS \
             verification is skipped. DEVELOPMENT USE ONLY."
        );
    }

    let filter = UsbFilter {
        vid_pid_allowlist,
        allow_any,
        explicit_paths: Default::default(),
    };
    let (watcher, mut rx) = watcher_with_default_store(scan_dir);
    let watcher = watcher.with_filter(filter);

    // Capture the bringup handle BEFORE moving the watcher into its
    // run task. The mgmt-event handlers (and eventually applets) use
    // this handle to drive `r2.mgmt.usb.{prepare,confirm,abort,unpair,list}`.
    let handle = watcher.handle();
    // TODO Phase USB-4b: stash this on HiveState so mgmt handlers can
    // reach it. For now just log that it exists; the consumer task
    // below uses the handle directly to surface events.
    log::debug!(
        "USB watcher handle ready ({} initial mounts)",
        handle.status().len()
    );
    // The watcher needs to live in a task so we can also drive control
    // messages back to it. Instead of moving the watcher into the
    // run() task and losing the `send_control` handle, wrap it in an
    // Arc<Mutex<...>> ... but that complicates the run loop. Simpler
    // for v0.1: route control via a side channel that loops back.
    // The applet path (Phase 3f/3g) can build the Arc<Mutex> wrapper
    // when the management surface for it lands.
    let (ctrl_loopback_tx, mut ctrl_loopback_rx) =
        tokio::sync::mpsc::channel::<(std::path::PathBuf, SessionControl)>(16);

    // Watcher task — consumes scan ticks; spawns per-device sessions.
    tokio::spawn(watcher.run());

    // Consumer task — logs every event; drives auto-confirm if set;
    // feeds inbound R2-WIRE frames into the route engine via
    // `router::route_frame` (Phase USB-5).
    let state_for_consumer = state.clone();
    let handle_for_consumer = handle.clone();
    tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                HotPlugEvent::DeviceAttached { path } => {
                    log::info!("USB peripheral attached: {}", path.display());
                }
                HotPlugEvent::DeviceDetached { path } => {
                    log::info!("USB peripheral detached: {}", path.display());
                }
                HotPlugEvent::Error { path, message } => {
                    log::warn!(
                        "USB peripheral {} error: {message}",
                        path.display()
                    );
                }
                HotPlugEvent::Session { path, event } => {
                    handle_session_event(
                        &path,
                        event,
                        auto_confirm_unsafe,
                        &ctrl_loopback_tx,
                        &state_for_consumer,
                        &handle_for_consumer,
                    )
                    .await;
                }
            }
        }
    });

    // Loopback drain — currently only used for diagnostic logging.
    // The Phase 3f/3g applet will replace this with a real route from
    // r2.mgmt.pair.* events back to the watcher's control channels.
    tokio::spawn(async move {
        while let Some((path, ctrl)) = ctrl_loopback_rx.recv().await {
            log::debug!(
                "USB control for {}: {:?} (loopback only — no watcher hookup yet)",
                path.display(),
                ctrl
            );
        }
    });

    handle
}

/// React to one USB session event: log the protocol milestones (SYNC/CAPS/
/// pairing), drive the dev-only auto-confirm, and — the important arm —
/// feed each `WireFrame` into `router::route_frame` as if it arrived on a
/// directly-attached radio of the dongle's advertised transport kind.
///
/// **Dependencies:** [`kind_for_local_id_via_handle`] (CAPS → route-engine
/// transport mapping), `router.rs`, the control loopback channel.
///
/// **Used-by:** the consumer task inside [`spawn_usb_watcher`] only.
#[cfg(target_os = "linux")]
async fn handle_session_event(
    path: &std::path::Path,
    event: r2_hive::usb::UsbEvent,
    auto_confirm_unsafe: bool,
    ctrl_tx: &tokio::sync::mpsc::Sender<(
        std::path::PathBuf,
        r2_hive::usb_serial::SessionControl,
    )>,
    state: &Arc<HiveState>,
    handle: &r2_hive::usb_hotplug::UsbBringupHandle,
) {
    use r2_hive::usb::UsbEvent;
    use r2_hive::usb_serial::SessionControl;
    match event {
        UsbEvent::SyncNegotiated { version, flags } => {
            log::info!(
                "USB {}: SYNC negotiated (v{version}, flags=0x{flags:02X})",
                path.display()
            );
        }
        UsbEvent::Caps(caps) => {
            log::info!(
                "USB {}: CAPS — hive_id_bytes={}, fw=\"{}\" v{}, transports:",
                path.display(),
                hex_short(&caps.hive_id_bytes),
                caps.firmware_id,
                caps.firmware_version,
            );
            for t in &caps.transports {
                log::info!(
                    "  • local_id={}, kind={:?}, region={:?}",
                    t.local_id, t.kind, t.region
                );
            }
        }
        UsbEvent::PairingPrompt {
            hive_id_bytes,
            firmware_id,
            sas_code,
        } => {
            log::warn!(
                "USB {}: PAIRING PROMPT — peer {} (\"{}\"), SAS code = {:06}",
                path.display(),
                hex_short(&hive_id_bytes),
                firmware_id,
                sas_code,
            );
            if auto_confirm_unsafe {
                log::warn!("  AUTO-CONFIRMING (--usb-auto-confirm-unsafe)");
                let _ = ctrl_tx
                    .send((path.to_path_buf(), SessionControl::UserConfirms))
                    .await;
            } else {
                log::info!(
                    "  Verify the code matches what the peripheral displays, then \
                     confirm via `r2hive usb confirm {}` (TODO Phase 3f).",
                    path.display()
                );
            }
        }
        UsbEvent::Paired {
            hive_id_bytes,
            reconnect,
        } => {
            log::info!(
                "USB {}: PAIRED ({}, hive_id_bytes={})",
                path.display(),
                if reconnect { "reconnect" } else { "first-attach" },
                hex_short(&hive_id_bytes)
            );
        }
        UsbEvent::PairingFailed { reason } => {
            log::warn!("USB {}: pairing failed — {reason}", path.display());
        }
        UsbEvent::WireFrame { local_id, bytes } => {
            log::debug!(
                "USB {}: WireFrame on local_id={} ({} bytes)",
                path.display(),
                local_id,
                bytes.len()
            );
            // Phase USB-5: look up which transport-kind this
            // local_id represents in the device's CAPS, map to a
            // route-engine Transport variant, and feed the frame
            // into router::route_frame as if it had arrived on a
            // directly-attached radio of that kind.
            if let Some(transport) =
                kind_for_local_id_via_handle(handle, path, local_id)
            {
                let _ = r2_hive::router::route_frame(state, 0, transport, &bytes).await;
            } else {
                log::debug!(
                    "USB {}: local_id={} has no routable kind; frame dropped",
                    path.display(),
                    local_id
                );
            }
        }
        UsbEvent::Control { msg_type, body } => {
            log::debug!(
                "USB {}: control msg_type={msg_type} ({} body bytes)",
                path.display(),
                body.len()
            );
        }
        UsbEvent::Error(e) => {
            log::warn!("USB {}: protocol error — {e}", path.display());
        }
    }
}

/// Look up the route-engine `Transport` variant associated with this
/// dongle's `local_id`, by consulting the watcher's status snapshot
/// for the device's CAPS-advertised transports. Returns `None` when
/// the device hasn't published CAPS yet, when the kind isn't known
/// to R2-USB Appendix A, or when the route engine doesn't represent
/// it (e.g. ZigBee, Thread).
///
/// **Used-by:** [`handle_session_event`] (WireFrame arm) only; the inverse
/// mapping lives in `hive.rs::transport_to_caps_kind` — keep the two tables
/// in sync with R2-USB Appendix A.
#[cfg(target_os = "linux")]
fn kind_for_local_id_via_handle(
    handle: &r2_hive::usb_hotplug::UsbBringupHandle,
    path: &std::path::Path,
    local_id: u8,
) -> Option<r2_route::transport::Transport> {
    use r2_hive::usb::TransportKind;
    use r2_route::transport::Transport;
    let snap = handle.status().into_iter().find(|s| s.path == path)?;
    let transports = snap.advertised_transports.as_ref()?;
    let descriptor = transports.iter().find(|t| t.local_id == local_id)?;
    // R2-USB Appendix A enumeration → route-engine Transport.
    match &descriptor.kind {
        TransportKind::Enumerated(1) => Some(Transport::Lora),
        TransportKind::Enumerated(2) => Some(Transport::Ble),
        TransportKind::Enumerated(3) => Some(Transport::Wifi),
        TransportKind::Enumerated(4) => Some(Transport::Internet),
        // 5..=8 = zigbee / 802154 / nrf24 / thread — not modelled
        // by R2-ROUTE today. 9..=99 reserved. 100+ experimental.
        _ => None,
    }
}

/// First 4 bytes as hex + "..." — log-line abbreviation for peer hive-id
/// byte strings (full ids are noise at INFO level).
///
/// **Used-by:** [`handle_session_event`] log lines only.
#[cfg(target_os = "linux")]
fn hex_short(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect::<String>() + "..."
}

/// Time-derived placeholder RBID for beacon advertisement. NOT the real
/// R2-DISCOVERY §3.3 rotating id — that must derive from the trust-layer
/// session key via `PeerRegistry::own_rbid(epoch)` once wired (the TODOs at
/// both call sites track this). Placeholder is acceptable only because
/// beacon emit is still a scaffold returning Unsupported.
///
/// **Used-by:** [`start_lan_discovery`] and [`start_ble`]; unused when those
/// transports are composed out (e.g. `--no-default-features`), hence the
/// `allow(dead_code)` rather than threading `cfg(any(...))` through.
#[allow(dead_code)]
fn random_rbid() -> [u8; 8] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let mut rbid = [0u8; 8];
    for i in 0..8 {
        rbid[i] = ((t >> (i * 8)) & 0xFF) as u8;
    }
    rbid
}

