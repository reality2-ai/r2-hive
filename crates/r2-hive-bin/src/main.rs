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
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

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

    /// Override the local management-socket path (R2-HIVE §5.1).
    /// Default: ${XDG_RUNTIME_DIR}/r2-hive.sock on Linux, ${TMPDIR}/r2-hive.sock on macOS.
    #[arg(long)]
    mgmt_socket: Option<PathBuf>,

    /// Override the master-secret store path (R2-HIVE §3.1).
    /// Default: $XDG_STATE_HOME/r2/master.key. Only honoured when the
    /// resolved identity backend is `file`.
    #[arg(long)]
    identity_store: Option<PathBuf>,

    /// Identity backend selection (R2-HIVE §3.1, R2-HIVE §3.2).
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
    /// drives the R2-USB v2 protocol against any it finds. Disable
    /// for headless servers / containers / development rigs that
    /// don't want the noise.
    #[arg(long)]
    no_usb: bool,

    /// Directory the USB watcher scans for serial devices. Defaults
    /// to `/dev`. Override for testing or custom udev layouts.
    #[arg(long, default_value = "/dev")]
    usb_dir: PathBuf,

    /// **DEV/TEST ONLY.** Auto-confirm any SAS prompt from a freshly
    /// attached USB peripheral. Equivalent to a UI operator clicking
    /// "yes, the codes match" with no human in the loop. Production
    /// deployments MUST NOT set this; it defeats the §6.4.4 SAS
    /// verification.
    #[arg(long)]
    usb_auto_confirm_unsafe: bool,

    /// **DEV/TEST ONLY.** Bypass the default-deny USB device filter
    /// and try to talk R2-USB v2 to every CDC-ACM device that
    /// appears. Production deployments leave this off; the right
    /// path is `--usb-vid-pid VID:PID` (or `r2hive usb prepare` per
    /// Phase USB-4) for known peripherals.
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

/// Parse a `vid:pid` argument as two lowercase-hex u16s.
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

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<HiveState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| compat::handshake::handle_connection(socket, state))
}

async fn health() -> Response {
    ([(header::CONTENT_TYPE, "application/json")],
     r#"{"status":"ok","class":"ai.reality2.wayfinder"}"#).into_response()
}

async fn routes_json(State(state): State<Arc<HiveState>>) -> Response {
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

async fn stats_json(State(state): State<Arc<HiveState>>) -> Response {
    let peers = state.ws_transport.peers().peer_count().await;
    let frames = state.frames_routed.load(Ordering::Relaxed);
    let connections_total = state.connections_total.load(Ordering::Relaxed);
    let uptime_secs = state.started_at.elapsed().as_secs();

    let json = format!(
        r#"{{"connections":{},"frames_routed":{},"connections_total":{},"uptime_secs":{}}}"#,
        peers, frames, connections_total, uptime_secs
    );

    ([(header::CONTENT_TYPE, "application/json")], json).into_response()
}

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

    let self_hive_id = fnv1a_addr(&args.name);
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
            #[cfg(feature = "ble")]
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
            #[cfg(not(feature = "ble"))]
            {
                log::warn!(
                    "  --auto: BLE present on host but binary built without `--features ble`"
                );
            }
        }
        if !args.lora && report.should_run_lora() {
            #[cfg(feature = "lora")]
            {
                args.lora = true;
                log::info!(
                    "  --auto: enabling LoRa (socket {} reachable)",
                    report.lora.socket_path.display()
                );
            }
            #[cfg(not(feature = "lora"))]
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

    log::info!("r2-hive listening on {}", addr);
    log::info!("  Dashboard: http://{}:{}/", args.bind, args.port);
    log::info!("  WebSocket: ws://{}:{}/r2", args.bind, args.port);
    log::info!("  Buffer: {} frames/group", args.buffer_size);
    log::info!("  Max connections: {}", args.max_connections);

    let mut active_plugins = vec!["word-codes", "dashboard"];

    // LAN discovery: UDP beacon
    if args.lan {
        match start_lan_discovery(&args, &state, self_hive_id).await {
            Ok(()) => active_plugins.push("lan-discovery"),
            Err(e) => log::error!("LAN discovery failed to start: {}", e),
        }
    }

    // BLE transport
    #[cfg(feature = "ble")]
    if args.ble {
        match start_ble(&args, &state, self_hive_id).await {
            Ok(()) => active_plugins.push("ble"),
            Err(e) => log::error!("BLE failed to start: {}", e),
        }
    }

    // LoRa transport via arduino-router IPC
    #[cfg(feature = "lora")]
    if args.lora {
        match start_lora(&args, &state, self_hive_id).await {
            Ok(()) => active_plugins.push("lora"),
            Err(e) => log::error!("LoRa failed to start: {}", e),
        }
    }

    log::info!("  Plugins: {}", active_plugins.join(", "));

    // Spawn route engine maintenance (decay neighbours/paths every 30s)
    tokio::spawn(router::maintenance_loop(state.clone()));

    // Phase USB-3c: USB-attached peripheral watcher (R2-USB v2).
    // Linux-only; behind a runtime opt-out so headless servers can
    // skip it.
    #[cfg(target_os = "linux")]
    if !args.no_usb {
        let usb_handle = spawn_usb_watcher(
            args.usb_dir.clone(),
            args.usb_auto_confirm_unsafe,
            args.usb_allow_any,
            args.usb_vid_pid.clone(),
            state.clone(),
        );
        state.set_usb_handle(usb_handle);
        if args.usb_allow_any {
            log::warn!(
                "  USB watcher: --usb-allow-any is set — every CDC-ACM device \
                 will be probed for R2-USB v2. DEVELOPMENT USE ONLY."
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

    // Spawn the local management API (R2-HIVE §§3, 5, 6.3) unless opted out.
    let _mgmt_handle = if args.no_mgmt {
        log::info!("  Management API: disabled (--no-mgmt)");
        None
    } else {
        let mgmt_socket_path = args.mgmt_socket.unwrap_or_else(mgmt::default_socket_path);
        let store_path = args.identity_store.unwrap_or_else(FileStore::default_path);
        // Resolve the identity backend per the operator flag
        // (R2-HIVE §3.2, Phase 4c).
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
        let existed = match store.backend() {
            mgmt::identity::StoreBackend::File => store_path.exists(),
            // Keyring "exists" probe would require an actual read; defer
            // honest reporting to Phase 4c follow-up. For now, report
            // false so the daemon log says "generated" or "loaded
            // existing" based on what `load_or_create` actually did
            // (created_this_start tracks the truth).
            _ => false,
        };
        match DaemonState::with_identity_store(store.as_ref()) {
            Ok(daemon_state) => {
                // Attach the HiveState so the r2.api.* primitive surface
                // (R2-HOST-API §3) can reach the wire / route / transport
                // layer. Done after with_identity so existing tests that
                // construct DaemonState in isolation still pass.
                daemon_state.attach_hive_state(state.clone());
                let _ = existed; // remains accurate for File backend; keyring uses the daemon_state flag
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
                // secret (R2-PLUGIN §13.5). Without this, web plugins
                // serve in dev-mode and stamp X-R2-Web-Auth: dev-mode on
                // every response.
                if let Some(key) = daemon_state.derive_web_auth_key() {
                    let auth = std::sync::Arc::new(r2_hive::web_auth::WebAuth::new(key));
                    state.set_web_auth(auth);
                    log::info!("  Web auth: enabled (cookie-bound to master secret)");
                } else {
                    log::warn!("  Web auth: dev-mode (no identity); web plugins are unauthenticated");
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
        .merge(plugins::dashboard::routes())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    if let Some(daemon_state) = ws_mgmt_state {
        // /r2/mgmt is the loopback parallel transport for R2-HOST-API §2.2.
        // It carries the same R2-WIRE extended frames as the UDS but as
        // binary WebSocket messages (no length prefix; WS provides framing).
        app = app.route("/r2/mgmt", get(mgmt::ws::handler))
                 .layer(axum::Extension(daemon_state));
        log::info!("  Management WS: ws://{}:{}/r2/mgmt", args.bind, args.port);
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Phase 3e: tell systemd we're ready and start the watchdog ping.
    // No-ops on non-systemd builds / non-systemd parents.
    r2_hive::systemd::notify_ready();
    r2_hive::systemd::spawn_watchdog();

    axum::serve(listener, app).await.unwrap();
}

async fn start_lan_discovery(args: &Args, state: &Arc<HiveState>, self_hive_id: u32) -> Result<(), String> {
    use r2_discovery::discovery::udp_beacon::UdpBeacon;
    use r2_discovery::bindings::udp_lan::UdpLanTransport;
    use r2_discovery::{AsyncTransport, hive_id_from_rbid, rbid_for_hive_id};

    // Start UDP transport on the R2-WIRE port for frame exchange
    let udp_addr = format!("{}:{}", args.bind, 21042);
    let udp = UdpLanTransport::bind(&udp_addr).await?;
    log::info!("  UDP LAN: {}", udp_addr);

    // Register UDP transport with wayfinder state
    state.set_udp_transport(udp.clone()).await;

    // Spawn UDP frame receive loop - route inbound frames
    let state_rx = state.clone();
    let udp_rx = udp.clone();
    tokio::spawn(async move {
        while let Some(frame) = udp_rx.recv().await {
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
            let hive_ids = state_rx.ws_transport.peers().hive_ids().await;
            for hive_id in hive_ids {
                let _ = state_rx.ws_transport.peers().send(hive_id, data).await;
            }
        }
    });

    // Start UDP beacon for discovery (broadcast on port 21044).
    // The rbid encodes the canonical hive_id in the high 4 bytes so peers
    // arriving on UDP get the same hive_id as peers arriving on BLE.
    let rotating = random_rbid();
    let rbid = rbid_for_hive_id(self_hive_id, [rotating[0], rotating[1], rotating[2], rotating[3]]);
    let beacon = UdpBeacon::new(
        0x00000000, // class hash - generic for now
        &rbid,
        21042,      // advertise our UDP R2-WIRE port
        &[],        // bloom
        0,          // bloom_k
    );

    let (beacon_tx, mut beacon_rx) = tokio::sync::mpsc::channel(64);
    beacon.start(beacon_tx).await?;

    // Handle discovered peers
    let state2 = state.clone();
    tokio::spawn(async move {
        while let Some(discovered) = beacon_rx.recv().await {
            let addr_str = String::from_utf8_lossy(&discovered.address).to_string();
            log::info!(
                "LAN peer discovered: class=0x{:08X} at {}",
                discovered.class_hash, addr_str
            );
            if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                // Use the canonical hive_id from the beacon's rbid, NOT a hash
                // of the address — so the same physical hive gets the same id
                // regardless of which transport discovered it.
                let hive_id = hive_id_from_rbid(&discovered.rbid);
                if hive_id == self_hive_id {
                    continue; // skip our own beacon (UdpBeacon dedup misses on different ports)
                }
                if let Some(udp) = state2.udp_transport.read().await.as_ref() {
                    udp.add_peer(hive_id, addr).await;
                    log::info!("LAN peer registered: hive_id=0x{:08X} addr={}", hive_id, addr);
                }
                // Ingest a beacon-discovery observation so the route engine
                // knows this peer exists. Without this, the engine only learns
                // about peers from actual frame traffic, and the bootstrap
                // FLOOD reaches no one because the neighbour table is empty.
                // Quality is lower than for actual delivery — beacons confirm
                // presence, not delivery success.
                use r2_route::neighbour::{MobilityClass, Observation};
                use r2_route::transport::{QualitySample, Transport};
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as u32).unwrap_or(0);
                let obs = Observation {
                    hive_id,
                    transport: Transport::Internet,
                    timestamp: now_secs,
                    quality: QualitySample::Direct(0.6),
                    rssi: None,
                    mcu_origin: false,
                    mobility: MobilityClass::Infrastructure,
                };
                state2.route_engine.lock().await.ingest_observation(obs);
            }
        }
    });

    Ok(())
}

#[cfg(feature = "ble")]
async fn start_ble(args: &Args, state: &Arc<HiveState>, self_hive_id: u32) -> Result<(), String> {
    use r2_discovery::bindings::ble::BleTransport;
    use r2_discovery::discovery::ble_beacon::BleBeaconScanner;
    use r2_discovery::{AsyncTransport, BeaconScanner, hive_id_from_rbid, rbid_for_hive_id};

    // Create BLE transport (starts scheduler + L2CAP listener)
    let (ble, disco_rx) = BleTransport::new(args.name.clone())
        .await
        .map_err(|e| format!("BLE init failed: {}", e))?;

    // Start BLE beacon advertising. The rbid carries the canonical hive_id
    // in its high 4 bytes so peers see the same id as on UDP.
    {
        use r2_discovery::discovery::ble_beacon::BleBeaconAdvertiser;
        use r2_discovery::BeaconAdvertiser;
        let rotating = random_rbid();
        let rbid = rbid_for_hive_id(self_hive_id, [rotating[0], rotating[1], rotating[2], rotating[3]]);
        let advertiser = BleBeaconAdvertiser::new(ble.sched().clone(), rbid);
        advertiser.start(0x00000000, &[], 0).await;
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
        while let Some(frame) = ble_rx.recv().await {
            state_rx.frames_routed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            log::info!("BLE inbound: {} bytes from hive 0x{:08X}", frame.data.len(), frame.source_hive);

            let _ = router::route_frame(
                &state_rx, frame.source_hive,
                r2_route::transport::Transport::Ble,
                &frame.data,
            ).await;

            // Also forward to WebSocket peers (compat layer for browser clients).
            let hive_ids = state_rx.ws_transport.peers().hive_ids().await;
            for hive_id in hive_ids {
                let _ = state_rx.ws_transport.peers().send(hive_id, &frame.data).await;
            }
        }
    });

    // Spawn BLE beacon discovery handler. Use canonical hive_id from beacon
    // rbid so this peer matches its UDP-discovered counterpart.
    let scanner = BleBeaconScanner::new(disco_rx);
    let state2 = state.clone();
    let ble2 = ble.clone();
    tokio::spawn(async move {
        while let Some(beacon) = scanner.next_beacon().await {
            if beacon.address.len() >= 6 {
                let addr = bluer::Address([
                    beacon.address[0], beacon.address[1], beacon.address[2],
                    beacon.address[3], beacon.address[4], beacon.address[5],
                ]);
                let hive_id = hive_id_from_rbid(&beacon.rbid);
                if hive_id == self_hive_id {
                    continue; // skip our own beacon if it loops back
                }
                ble2.register_peer(hive_id, addr).await;
                log::info!("BLE peer discovered: hive=0x{:08X} addr={}", hive_id, addr);

                // Ingest a beacon-discovery observation so the route engine
                // knows this BLE peer exists. Same rationale as the UDP
                // beacon handler in start_lan_discovery.
                use r2_route::neighbour::{MobilityClass, Observation};
                use r2_route::transport::{QualitySample, Transport};
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as u32).unwrap_or(0);
                let obs = Observation {
                    hive_id,
                    transport: Transport::Ble,
                    timestamp: now_secs,
                    quality: QualitySample::Direct(0.6),
                    rssi: None,
                    mcu_origin: false,
                    mobility: MobilityClass::Mobile,
                };
                state2.route_engine.lock().await.ingest_observation(obs);
            }
        }
    });

    Ok(())
}

#[cfg(feature = "lora")]
async fn start_lora(args: &Args, state: &Arc<HiveState>, _self_hive_id: u32) -> Result<(), String> {
    use r2_discovery::bindings::lora::LoraTransport;
    use r2_discovery::AsyncTransport;

    // Connect to the local arduino-router IPC socket and verify the radio
    // is reachable. Construction will fail with a useful message if the
    // socket isn't there or the radio isn't responding.
    let lora = LoraTransport::with_socket(&args.lora_socket).await?;
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
        while let Some(frame) = lora_rx.recv().await {
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
            let hive_ids = state_rx.ws_transport.peers().hive_ids().await;
            for hive_id in hive_ids {
                let _ = state_rx.ws_transport.peers().send(hive_id, ws_data).await;
            }
        }
        log::info!("LoRa receive loop exited");
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
/// review, which defeats §6.4.4 SAS verification. Production
/// deployments leave it false; the eventual Cosmic / KDE applet will
/// drive `SessionControl::UserConfirms` via the management socket.
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

#[cfg(target_os = "linux")]
fn hex_short(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect::<String>() + "..."
}

fn random_rbid() -> [u8; 8] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let mut rbid = [0u8; 8];
    for i in 0..8 {
        rbid[i] = ((t >> (i * 8)) & 0xFF) as u8;
    }
    rbid
}

fn fnv1a_addr(s: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &byte in s.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}
