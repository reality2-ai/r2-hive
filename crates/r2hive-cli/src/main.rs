//! r2hive — reference CLI for the r2-hive local management API.
//!
//! Phase 0 surface: `daemon status`, `identity status`.
//! Phase 1 surface (R2-HOST-API §3): `peers list/query`, `event send/subscribe`,
//! `tg current`, `cap query`. See R2-HIVE §10 for the broader command surface
//! (ensemble/sentant/plugin/transport are pending Phase 2+).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rand::RngCore;
use tokio::net::UnixStream;

use r2_hive::default_socket_path;
use r2_hive::mgmt::api::{
    build_cap_query_request, build_event_send_request, build_event_subscribe_request,
    build_identity_status_request, build_peer_list_request, build_peer_query_request,
    build_status_request, build_tg_current_request, build_web_provision_request,
    parse_identity_status_response, parse_status_response,
};
use r2_hive::mgmt::ensemble::{
    build_info_request, build_list_request, build_load_request, build_load_request_from_path,
    build_reset_request, build_stop_request,
};
#[cfg(target_os = "linux")]
use r2_hive::mgmt::usb::{
    build_abort_request, build_confirm_request, build_list_request as build_usb_list_request,
    build_prepare_request, build_unpair_request,
};
use r2_hive::mgmt::framing::{read_frame, write_frame};
use r2_hive::mgmt::primitive::{
    parse_cap_query_response, parse_event_subscribe_response, parse_peer_list_response,
    parse_peer_query_response, parse_tg_current_response,
};

#[derive(Debug, Parser)]
#[command(name = "r2hive", version, about = "R2 hive management CLI")]
struct Cli {
    /// Override the default management-socket path.
    #[arg(long, global = true)]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Daemon-level operations.
    Daemon {
        #[command(subcommand)]
        op: DaemonOp,
    },
    /// Identity (master secret) operations.
    Identity {
        #[command(subcommand)]
        op: IdentityOp,
    },
    /// Trust-group context queries (R2-HOST-API §3.2 tg.*).
    Tg {
        #[command(subcommand)]
        op: TgOp,
    },
    /// Peer queries (R2-HOST-API §3.2 peer.*).
    Peers {
        #[command(subcommand)]
        op: PeersOp,
    },
    /// Capability advertisement queries (R2-HOST-API §3.2 cap.query).
    Cap {
        #[command(subcommand)]
        op: CapOp,
    },
    /// Application event traffic (R2-HOST-API §3.2 event.*).
    Event {
        #[command(subcommand)]
        op: EventOp,
    },
    /// Ensemble lifecycle (R2-HIVE §5.3 ensemble.*).
    Ensemble {
        #[command(subcommand)]
        op: EnsembleOp,
    },
    /// Web plugin operator commands (R2-PLUGIN §13).
    Web {
        #[command(subcommand)]
        op: WebOp,
    },
    /// USB peripheral operator commands (R2-USB §3 + R2-HIVE §6.4).
    /// Linux only.
    Usb {
        #[command(subcommand)]
        op: UsbOp,
    },
}

#[derive(Debug, Subcommand)]
enum WebOp {
    /// Mint a single-use word code for browser provisioning. Print it
    /// to stdout. The code expires after 1 hour and can only be
    /// redeemed once.
    Provision,
}

#[derive(Debug, Subcommand)]
enum UsbOp {
    /// List USB peripherals the watcher is tracking, with their
    /// session state, device_id (if known), and any pending SAS
    /// prompt awaiting confirmation.
    List,
    /// Add a `/dev/ttyACM*` (or `ttyUSB*`) path to the watcher's
    /// explicit allowlist. The watcher will pick it up on the next
    /// poll and start the R2-USB v2 protocol against it. Use this
    /// for boards whose USB VID/PID isn't in r2-hive's compiled-in
    /// list.
    Prepare {
        /// Filesystem path to the device, e.g. `/dev/ttyACM0`.
        path: String,
    },
    /// Confirm a pending SAS code on a paired peripheral (R2-HIVE §6.4.4).
    /// Use after running `r2hive usb list`, comparing the displayed
    /// 6-digit code with what the peripheral renders, and verifying
    /// they match.
    Confirm {
        /// Filesystem path of the peripheral whose SAS prompt is open.
        path: String,
    },
    /// Reject a pending SAS code (mismatch, change of mind, or just
    /// stop the pairing).
    Abort {
        /// Filesystem path of the peripheral whose SAS prompt is open.
        path: String,
    },
    /// Forget the link key for a previously-paired peripheral.
    /// Subsequent attaches with this `device_id` trigger fresh
    /// §6.4.3 first-attach pairing. Provide the device_id as
    /// 32-character lowercase hex.
    Unpair {
        /// 16-byte device_id, lowercase hex (32 chars).
        device_id_hex: String,
    },
}

#[derive(Debug, Subcommand)]
enum EnsembleOp {
    /// Load an ensemble score (YAML by default; --json or --toml to change dialect).
    Load {
        /// Path to the score file. Use `-` to read from stdin.
        path: String,
        /// Treat input as JSON instead of YAML.
        #[arg(long, conflicts_with_all = ["yaml", "toml"])]
        json: bool,
        /// Treat input as YAML (default; explicit flag).
        #[arg(long, conflicts_with_all = ["json", "toml"])]
        yaml: bool,
        /// Treat input as TOML.
        #[arg(long, conflicts_with_all = ["json", "yaml"])]
        toml: bool,
    },
    /// List loaded ensembles.
    List,
    /// Detailed info on one loaded ensemble.
    Info {
        /// Ensemble id (the score's `name` field).
        id: String,
    },
    /// Stop and unload an ensemble.
    Stop {
        /// Ensemble id.
        id: String,
    },
    /// Reset a Failed ensemble back to Healthy (clears restart ledger).
    Reset {
        /// Ensemble id.
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonOp {
    /// Show daemon status (version, build, uptime).
    Status,
}

#[derive(Debug, Subcommand)]
enum IdentityOp {
    /// Show master-secret presence, fingerprint, and store backend.
    Status,
}

#[derive(Debug, Subcommand)]
enum TgOp {
    /// Show the daemon's currently-attached trust group, if any.
    Current,
}

#[derive(Debug, Subcommand)]
enum PeersOp {
    /// List hive_ids the daemon is observing in the active TG.
    List,
    /// Detailed status for one peer.
    Query {
        /// Hive ID in 0xHEX or decimal.
        hive_id: String,
    },
}

#[derive(Debug, Subcommand)]
enum CapOp {
    /// Capability set for the local daemon (or a specific hive with --target).
    Query {
        /// Optional target hive_id.
        #[arg(long)]
        target: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum EventOp {
    /// Send a single event into the active TG / mesh.
    Send {
        /// Event class string (will be FNV-hashed).
        event_class: String,
        /// Target a specific hive_id (omit for broadcast).
        #[arg(long)]
        target: Option<String>,
        /// Restrict delivery to a specific class on the receiver side.
        #[arg(long)]
        target_class: Option<String>,
        /// Inner payload as hex bytes (default: empty).
        #[arg(long, default_value = "")]
        payload_hex: String,
    },
    /// Subscribe to a class of events and stream deliveries to stdout
    /// until Ctrl+C.
    Subscribe {
        /// Event class to match (exact). Use `--any` for a broadcast subscription.
        event_class: Option<String>,
        /// Subscribe to every event regardless of class.
        #[arg(long, conflicts_with = "event_class")]
        any: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);

    let result = match cli.cmd {
        Commands::Daemon { op: DaemonOp::Status } => run_daemon_status(&socket_path).await,
        Commands::Identity { op: IdentityOp::Status } => run_identity_status(&socket_path).await,
        Commands::Tg { op: TgOp::Current } => run_tg_current(&socket_path).await,
        Commands::Peers { op: PeersOp::List } => run_peers_list(&socket_path).await,
        Commands::Peers { op: PeersOp::Query { hive_id } } => {
            run_peers_query(&socket_path, &hive_id).await
        }
        Commands::Cap { op: CapOp::Query { target } } => {
            run_cap_query(&socket_path, target.as_deref()).await
        }
        Commands::Event { op: EventOp::Send {
            event_class, target, target_class, payload_hex,
        } } => {
            run_event_send(
                &socket_path,
                &event_class,
                target.as_deref(),
                target_class.as_deref(),
                &payload_hex,
            ).await
        }
        Commands::Event { op: EventOp::Subscribe { event_class, any } } => {
            run_event_subscribe(&socket_path, event_class.as_deref(), any).await
        }
        Commands::Ensemble { op } => run_ensemble(&socket_path, op).await,
        Commands::Web { op: WebOp::Provision } => run_web_provision(&socket_path).await,
        #[cfg(target_os = "linux")]
        Commands::Usb { op } => run_usb(&socket_path, op).await,
        #[cfg(not(target_os = "linux"))]
        Commands::Usb { .. } => Err("usb subcommands are Linux-only".to_string()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("r2hive: {e}");
            ExitCode::from(1)
        }
    }
}

// ───────────── Phase 0 commands ─────────────

async fn run_daemon_status(socket_path: &PathBuf) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();

    let correlation_id = rand::thread_rng().next_u64();
    let request = build_status_request(correlation_id);
    write_frame(&mut writer, &request).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;

    let parsed = parse_status_response(&response).map_err(|e| format!("parse: {e}"))?;
    if parsed.correlation_id != correlation_id {
        return Err(format!(
            "correlation_id mismatch: sent {correlation_id}, got {}",
            parsed.correlation_id
        ));
    }

    println!("r2-hive status");
    println!("  version:    {}", parsed.version);
    println!("  build:      {}", parsed.build_hash);
    println!("  uptime:     {}s", parsed.uptime_seconds);
    println!("  socket:     {}", socket_path.display());
    Ok(())
}

async fn run_identity_status(socket_path: &PathBuf) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();

    let correlation_id = rand::thread_rng().next_u64();
    let request = build_identity_status_request(correlation_id);
    write_frame(&mut writer, &request).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;

    let parsed = parse_identity_status_response(&response).map_err(|e| format!("parse: {e}"))?;
    if parsed.correlation_id != correlation_id {
        return Err(format!(
            "correlation_id mismatch: sent {correlation_id}, got {}",
            parsed.correlation_id
        ));
    }

    println!("identity status");
    println!("  present:            {}", if parsed.present { "yes" } else { "no" });
    println!("  fingerprint:        {}", parsed.fingerprint);
    println!("  backend:            {}", parsed.backend);
    println!("  store path:         {}", parsed.path);
    println!("  created this start: {}", if parsed.created_this_start { "yes" } else { "no" });
    Ok(())
}

// ───────────── Phase 1 (R2-HOST-API) commands ─────────────

async fn run_tg_current(socket_path: &PathBuf) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build_tg_current_request(cid)).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;
    let frame = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    let (_rcid, attached) = parse_tg_current_response(frame.payload).map_err(|e| format!("parse: {e}"))?;
    println!("trust group");
    match attached {
        None => println!("  status:  no TG attached (detached)"),
        Some((tg_id, role, hive_id)) => {
            println!("  status:  attached");
            println!("  tg_id:   0x{}", hex_short(&tg_id));
            println!("  role:    {}", role_name(role));
            println!("  hive_id: 0x{:08X}", hive_id);
        }
    }
    Ok(())
}

async fn run_peers_list(socket_path: &PathBuf) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build_peer_list_request(cid)).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;
    let frame = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    let (_rcid, peers) = parse_peer_list_response(frame.payload).map_err(|e| format!("parse: {e}"))?;
    if peers.is_empty() {
        println!("(no peers)");
    } else {
        println!("peers ({} total)", peers.len());
        for p in peers {
            println!("  0x{:08X}", p as u32);
        }
    }
    Ok(())
}

async fn run_peers_query(socket_path: &PathBuf, hive_id_str: &str) -> Result<(), String> {
    let hive_id = parse_hive_id(hive_id_str)?;
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build_peer_query_request(cid, hive_id)).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;
    let frame = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    let (_rcid, hid, status, last_seen, transports) =
        parse_peer_query_response(frame.payload).map_err(|e| format!("parse: {e}"))?;
    println!("peer 0x{:08X}", hid as u32);
    println!("  status:    {}", peer_status_name(status));
    if let Some(ms) = last_seen {
        println!("  last seen: {}ms (epoch)", ms);
    }
    if !transports.is_empty() {
        println!("  reachable via: {}", transports.join(", "));
    }
    Ok(())
}

async fn run_cap_query(socket_path: &PathBuf, target: Option<&str>) -> Result<(), String> {
    let target = match target {
        Some(s) => Some(parse_hive_id(s)?),
        None => None,
    };
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build_cap_query_request(cid, target)).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;
    let frame = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    let (_rcid, bloom, hashes, classes) =
        parse_cap_query_response(frame.payload).map_err(|e| format!("parse: {e}"))?;
    println!("capabilities");
    if let Some(t) = target {
        println!("  target hive: 0x{:08X}", t as u32);
    }
    println!("  bloom: {} bytes", bloom.len());
    if !hashes.is_empty() {
        println!("  event hashes:");
        for h in &hashes {
            println!("    0x{:08X}", h);
        }
    }
    if !classes.is_empty() {
        println!("  event classes:");
        for c in &classes {
            println!("    {}", c);
        }
    }
    if hashes.is_empty() && classes.is_empty() && bloom.is_empty() {
        println!("  (empty — no advertisements)");
    }
    Ok(())
}

async fn run_event_send(
    socket_path: &PathBuf,
    event_class: &str,
    target: Option<&str>,
    target_class: Option<&str>,
    payload_hex: &str,
) -> Result<(), String> {
    let target_hive = match target {
        Some(s) => Some(parse_hive_id(s)?),
        None => None,
    };
    let payload_bytes = hex_decode(payload_hex)?;
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    let req = build_event_send_request(cid, event_class, &payload_bytes, target_hive, target_class);
    write_frame(&mut writer, &req).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;
    let frame = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    // Two valid response shapes: success (event.send response) or error.
    let send_hash = r2_fnv::r2_hash("r2.api.event.send").map_err(|_| "hash".to_string())?;
    let err_hash = r2_fnv::r2_hash("r2.mgmt.event.error").map_err(|_| "hash".to_string())?;
    if frame.header.event_hash == send_hash {
        let (_rcid, msg_id) = r2_hive::mgmt::primitive::parse_event_send_response(frame.payload)
            .map_err(|e| format!("parse: {e}"))?;
        println!("sent  event_class={}  msg_id={}", event_class, msg_id);
        Ok(())
    } else if frame.header.event_hash == err_hash {
        let detail = decode_error_payload(frame.payload);
        Err(format!("daemon rejected: {detail}"))
    } else {
        Err(format!(
            "unexpected response event_hash 0x{:08X}",
            frame.header.event_hash
        ))
    }
}

async fn run_event_subscribe(
    socket_path: &PathBuf,
    event_class: Option<&str>,
    any: bool,
) -> Result<(), String> {
    let class = if any { None } else { event_class };
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    let req = build_event_subscribe_request(cid, class, None, None);
    write_frame(&mut writer, &req).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection without responding".to_string())?;
    let frame = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    let (_rcid, sub_id) = parse_event_subscribe_response(frame.payload).map_err(|e| format!("parse: {e}"))?;
    eprintln!(
        "subscribed  sub_id={}  filter={}  (Ctrl+C to stop)",
        sub_id,
        match class { Some(c) => format!("event_class={c}"), None => "broadcast".to_string() }
    );

    let delivery_hash = r2_fnv::r2_hash("r2.api.event.delivery").map_err(|_| "hash".to_string())?;
    let err_hash = r2_fnv::r2_hash("r2.mgmt.event.error").map_err(|_| "hash".to_string())?;
    loop {
        let frame = match read_frame(&mut reader).await.map_err(|e| format!("read: {e}"))? {
            Some(f) => f,
            None => {
                eprintln!("daemon closed the connection");
                return Ok(());
            }
        };
        let parsed = match r2_wire::decode_extended(&frame) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("decode error: {e:?}");
                continue;
            }
        };
        if parsed.header.event_hash == delivery_hash {
            print_delivery(parsed.payload);
        } else if parsed.header.event_hash == err_hash {
            let detail = decode_error_payload(parsed.payload);
            eprintln!("[error] {detail}");
        } else {
            eprintln!("[unexpected event_hash 0x{:08X}]", parsed.header.event_hash);
        }
    }
}

// ───────────── Phase 2 ensemble commands ─────────────

async fn run_ensemble(socket_path: &PathBuf, op: EnsembleOp) -> Result<(), String> {
    match op {
        EnsembleOp::Load { path, json, toml, yaml: _ } => {
            run_ensemble_load(socket_path, &path, json, toml).await
        }
        EnsembleOp::List => run_ensemble_list(socket_path).await,
        EnsembleOp::Info { id } => run_ensemble_info(socket_path, &id).await,
        EnsembleOp::Stop { id } => run_ensemble_stop(socket_path, &id).await,
        EnsembleOp::Reset { id } => run_ensemble_reset(socket_path, &id).await,
    }
}

async fn run_ensemble_load(
    socket_path: &PathBuf,
    path: &str,
    json: bool,
    toml: bool,
) -> Result<(), String> {
    let dialect = if json { "json" } else if toml { "toml" } else { "yaml" };

    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    // Path-based loads (the common case) let the daemon resolve
    // web-plugin bundles (R2-PLUGIN §13.2). Stdin loads keep the
    // source-bytes form; web plugins won't auto-mount in that case.
    let request = if path == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("stdin: {e}"))?;
        build_load_request(cid, dialect, &s)
    } else {
        let abs = std::fs::canonicalize(path).map_err(|e| format!("resolve {path}: {e}"))?;
        let abs_str = abs.to_string_lossy().to_string();
        build_load_request_from_path(cid, dialect, &abs_str)
    };
    write_frame(&mut writer, &request).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    let class_hash = parsed.header.event_hash;
    if class_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        let code = read_str_field(parsed.payload, 1);
        return Err(format!("daemon error: {}", code.unwrap_or_else(|| "unknown".into())));
    }
    let id = read_str_field(parsed.payload, 1).unwrap_or_default();
    let count = read_uint_field(parsed.payload, 3).unwrap_or(0);
    let hash = read_uint_field(parsed.payload, 4).unwrap_or(0);
    println!("loaded ensemble '{id}': {count} sentants, score_hash 0x{:08X}", hash as u32);
    Ok(())
}

async fn run_ensemble_list(socket_path: &PathBuf) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    let request = build_list_request(cid);
    write_frame(&mut writer, &request).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    let entries = parse_list_response(parsed.payload);
    if entries.is_empty() {
        println!("(no ensembles loaded)");
    } else {
        for (id, status, count) in entries {
            println!("{id}\t{}\t{count} sentants", status_label(status));
        }
    }
    Ok(())
}

async fn run_ensemble_info(socket_path: &PathBuf, id: &str) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    let request = build_info_request(cid, id);
    write_frame(&mut writer, &request).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    let id = read_str_field(parsed.payload, 1).unwrap_or_default();
    let status = read_uint_field(parsed.payload, 2).unwrap_or(0);
    let count = read_uint_field(parsed.payload, 3).unwrap_or(0);
    let hash = read_uint_field(parsed.payload, 4).unwrap_or(0);
    println!("id:          {id}");
    println!("status:      {}", status_label(status));
    println!("sentants:    {count}");
    println!("score_hash:  0x{:08X}", hash as u32);
    Ok(())
}

async fn run_ensemble_stop(socket_path: &PathBuf, id: &str) -> Result<(), String> {
    ensemble_id_op(socket_path, id, build_stop_request, "stopped").await
}

async fn run_ensemble_reset(socket_path: &PathBuf, id: &str) -> Result<(), String> {
    ensemble_id_op(socket_path, id, build_reset_request, "reset").await
}

async fn ensemble_id_op(
    socket_path: &PathBuf,
    id: &str,
    build: fn(u64, &str) -> Vec<u8>,
    label: &str,
) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    let request = build(cid, id);
    write_frame(&mut writer, &request).await.map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    println!("{label} ensemble '{id}'");
    Ok(())
}

async fn run_web_provision(socket_path: &PathBuf) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    let request = build_web_provision_request(cid);
    write_frame(&mut writer, &request)
        .await
        .map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    let words =
        read_str_field(parsed.payload, 1).ok_or_else(|| "missing word code in response".to_string())?;
    println!("Word code (single-use, 1h TTL): {words}");
    println!("Open http://<hive-host>/r2/web/provision in a browser and paste the code.");
    Ok(())
}

#[cfg(target_os = "linux")]
async fn run_usb(socket_path: &PathBuf, op: UsbOp) -> Result<(), String> {
    match op {
        UsbOp::List => run_usb_list(socket_path).await,
        UsbOp::Prepare { path } => {
            run_usb_path_op(socket_path, &path, build_prepare_request, "prepared").await
        }
        UsbOp::Confirm { path } => {
            run_usb_bool_op(socket_path, &path, build_confirm_request, "confirm").await
        }
        UsbOp::Abort { path } => {
            run_usb_bool_op(socket_path, &path, build_abort_request, "abort").await
        }
        UsbOp::Unpair { device_id_hex } => run_usb_unpair(socket_path, &device_id_hex).await,
    }
}

#[cfg(target_os = "linux")]
async fn run_usb_list(socket_path: &PathBuf) -> Result<(), String> {
    use r2_cbor::{Decoder, Item};
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build_usb_list_request(cid))
        .await
        .map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }

    // Walk the array under key 1.
    let mut dec = Decoder::new(parsed.payload);
    let entries = match dec.next().map_err(|_| "decode root".to_string())? {
        Item::Map(n) => n,
        _ => return Err("response not a map".into()),
    };
    let mut printed = 0;
    for _ in 0..entries {
        let key = dec.next().map_err(|_| "key".to_string())?;
        let val = dec.next().map_err(|_| "val".to_string())?;
        if let Item::UInt(1) = key {
            if let Item::Array(n) = val {
                for _ in 0..n {
                    print_usb_device(&mut dec)?;
                    printed += 1;
                }
            }
        }
    }
    if printed == 0 {
        println!("(no USB peripherals tracked — plug a dongle in, or run `r2hive usb prepare <path>`)");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn print_usb_device(dec: &mut r2_cbor::Decoder<'_>) -> Result<(), String> {
    use r2_cbor::Item;
    let entries = match dec.next().map_err(|_| "device decode".to_string())? {
        Item::Map(n) => n,
        _ => return Err("device not a map".into()),
    };
    let mut path = String::new();
    let mut state: Option<u64> = None;
    let mut device_id: Option<String> = None;
    let mut firmware_id: Option<String> = None;
    let mut pending_sas: Option<u64> = None;
    let mut last_error: Option<String> = None;
    let mut vid: Option<u64> = None;
    let mut pid: Option<u64> = None;
    let mut manufacturer: Option<String> = None;
    let mut product: Option<String> = None;
    for _ in 0..entries {
        let k = dec.next().map_err(|_| "field key".to_string())?;
        let v = dec.next().map_err(|_| "field val".to_string())?;
        let kk = match k {
            Item::UInt(n) => n,
            _ => continue,
        };
        match (kk, v) {
            (1, Item::Text(s)) => path = String::from_utf8_lossy(s).to_string(),
            (2, Item::UInt(n)) => state = Some(n),
            (3, Item::Bytes(b)) => {
                device_id = Some(b.iter().map(|x| format!("{x:02x}")).collect())
            }
            (4, Item::Text(s)) => firmware_id = Some(String::from_utf8_lossy(s).to_string()),
            (5, Item::UInt(n)) => pending_sas = Some(n),
            (6, Item::Text(s)) => last_error = Some(String::from_utf8_lossy(s).to_string()),
            (7, Item::UInt(n)) => vid = Some(n),
            (8, Item::UInt(n)) => pid = Some(n),
            (9, Item::Text(s)) => manufacturer = Some(String::from_utf8_lossy(s).to_string()),
            (10, Item::Text(s)) => product = Some(String::from_utf8_lossy(s).to_string()),
            _ => {}
        }
    }
    println!("─ {path}");
    if let Some(s) = state {
        println!("    state         : {} ({})", session_state_label(s), s);
    }
    if let (Some(v), Some(p)) = (vid, pid) {
        let mut line = format!("    usb           : {v:04x}:{p:04x}");
        if let Some(m) = &manufacturer {
            line.push_str(&format!(" {m}"));
        }
        if let Some(p) = &product {
            line.push_str(&format!(" / {p}"));
        }
        println!("{line}");
    }
    if let Some(id) = &device_id {
        println!("    device_id     : {id}");
    }
    if let Some(fw) = &firmware_id {
        println!("    firmware_id   : {fw}");
    }
    if let Some(sas) = pending_sas {
        println!("    PENDING SAS   : {sas:06}  ← run `r2hive usb confirm {path}` to accept");
    }
    if let Some(err) = &last_error {
        println!("    last error    : {err}");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn session_state_label(s: u64) -> &'static str {
    match s {
        0 => "Initial",
        1 => "SyncSent",
        2 => "AwaitingCaps",
        3 => "Reconnecting",
        4 => "PairingHelloSent",
        5 => "PairingCommitReceived",
        6 => "PairingAwaitingUser",
        7 => "PairingConfirmSent",
        8 => "Active",
        9 => "Closed",
        _ => "Unknown",
    }
}

#[cfg(target_os = "linux")]
async fn run_usb_path_op(
    socket_path: &PathBuf,
    path: &str,
    build: fn(u64, &str) -> Vec<u8>,
    label: &str,
) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build(cid, path))
        .await
        .map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    println!("{label} {path}");
    Ok(())
}

#[cfg(target_os = "linux")]
async fn run_usb_bool_op(
    socket_path: &PathBuf,
    path: &str,
    build: fn(u64, &str) -> Vec<u8>,
    label: &str,
) -> Result<(), String> {
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build(cid, path))
        .await
        .map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    let accepted = read_bool_field(parsed.payload, 1).unwrap_or(false);
    if accepted {
        println!("{label}: {path}");
    } else {
        return Err(format!(
            "{label} ignored — no session running for {path} (run `r2hive usb list`?)"
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
async fn run_usb_unpair(socket_path: &PathBuf, device_id_hex: &str) -> Result<(), String> {
    let bytes = decode_device_id(device_id_hex)?;
    let mut stream = connect(socket_path).await?;
    let (mut reader, mut writer) = stream.split();
    let cid = rand::thread_rng().next_u64();
    write_frame(&mut writer, &build_unpair_request(cid, &bytes))
        .await
        .map_err(|e| format!("write: {e}"))?;
    let response = read_frame(&mut reader)
        .await
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "daemon closed connection before responding".to_string())?;
    let parsed = r2_wire::decode_extended(&response).map_err(|e| format!("decode: {e:?}"))?;
    if parsed.header.event_hash == r2_fnv::r2_hash("r2.mgmt.event.error").unwrap() {
        return Err(format!(
            "daemon error: {}",
            read_str_field(parsed.payload, 1).unwrap_or_else(|| "unknown".into())
        ));
    }
    println!("unpaired {device_id_hex}");
    Ok(())
}

#[cfg(target_os = "linux")]
fn decode_device_id(s: &str) -> Result<[u8; 16], String> {
    let s = s.trim();
    if s.len() != 32 {
        return Err(format!(
            "device_id must be 32 hex chars (16 bytes), got {} chars",
            s.len()
        ));
    }
    let mut out = [0u8; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("bad hex at offset {i}: {e}"))?;
    }
    Ok(out)
}

#[cfg(target_os = "linux")]
fn read_bool_field(payload: &[u8], target: u64) -> Option<bool> {
    use r2_cbor::{Decoder, Item};
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(kk) = k {
            if kk == target {
                if let Item::Bool(b) = v {
                    return Some(b);
                }
            }
        }
    }
    None
}

fn status_label(s: u64) -> &'static str {
    match s {
        0 => "Healthy",
        1 => "Degraded",
        2 => "Failed",
        _ => "Unknown",
    }
}

fn read_uint_field(payload: &[u8], target: u64) -> Option<u64> {
    use r2_cbor::{Decoder, Item};
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(k) = key {
            if k == target {
                if let Item::UInt(n) = val {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn read_str_field(payload: &[u8], target: u64) -> Option<String> {
    use r2_cbor::{Decoder, Item};
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(k) = key {
            if k == target {
                if let Item::Text(s) = val {
                    return std::str::from_utf8(s).ok().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

fn parse_list_response(payload: &[u8]) -> Vec<(String, u64, u64)> {
    use r2_cbor::{Decoder, Item};
    let mut out = Vec::new();
    let mut dec = Decoder::new(payload);
    let entries = match dec.next() {
        Ok(Item::Map(n)) => n,
        _ => return out,
    };
    for _ in 0..entries {
        let key = match dec.next() { Ok(k) => k, _ => return out };
        match key {
            Item::UInt(1) => {
                let arr_len = match dec.next() {
                    Ok(Item::Array(n)) => n,
                    _ => return out,
                };
                for _ in 0..arr_len {
                    let map_len = match dec.next() {
                        Ok(Item::Map(n)) => n,
                        _ => return out,
                    };
                    let mut id = String::new();
                    let mut status = 0u64;
                    let mut count = 0u64;
                    for _ in 0..map_len {
                        let k = match dec.next() { Ok(k) => k, _ => return out };
                        let v = match dec.next() { Ok(v) => v, _ => return out };
                        match (k, v) {
                            (Item::UInt(1), Item::Text(s)) => {
                                id = std::str::from_utf8(s).unwrap_or("").to_string();
                            }
                            (Item::UInt(2), Item::UInt(n)) => status = n,
                            (Item::UInt(3), Item::UInt(n)) => count = n,
                            _ => {}
                        }
                    }
                    out.push((id, status, count));
                }
            }
            _ => {
                // skip unknown key/value
                let _ = dec.next();
            }
        }
    }
    out
}

// ───────────── Helpers ─────────────

async fn connect(socket_path: &PathBuf) -> Result<UnixStream, String> {
    UnixStream::connect(socket_path).await.map_err(|e| {
        format!(
            "cannot connect to daemon socket at {}: {e}",
            socket_path.display()
        )
    })
}

fn parse_hive_id(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).map_err(|e| format!("invalid hive_id '{s}': {e}"))
    } else {
        s.parse::<u64>().map_err(|e| format!("invalid hive_id '{s}': {e}"))
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.is_empty() {
        return Ok(Vec::new());
    }
    if s.len() % 2 != 0 {
        return Err("hex string has odd length".into());
    }
    (0..s.len()).step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("bad hex: {e}")))
        .collect()
}

fn hex_short(b: &[u8]) -> String {
    // First 8 bytes of hex with ellipsis if longer.
    let n = b.len().min(8);
    let mut s = String::with_capacity(n * 2 + 4);
    for byte in &b[..n] {
        s.push_str(&format!("{:02X}", byte));
    }
    if b.len() > n {
        s.push_str("...");
    }
    s
}

fn role_name(role: u8) -> &'static str {
    match role {
        0 => "not a member",
        1 => "member",
        2 => "key_holder",
        _ => "unknown",
    }
}

fn peer_status_name(status: u64) -> &'static str {
    match status {
        0 => "unknown",
        1 => "self",
        2 => "neighbour",
        3 => "entangled",
        4 => "relayed-only",
        _ => "?",
    }
}

fn print_delivery(payload: &[u8]) {
    use r2_cbor::{Decoder, Item};
    let mut dec = Decoder::new(payload);
    let entries = match dec.next() {
        Ok(Item::Map(n)) => n,
        _ => {
            eprintln!("[delivery: bad payload]");
            return;
        }
    };
    let mut sub_id = 0u64;
    let mut event_class = String::new();
    let mut from_hive = 0u64;
    let mut msg_id = 0u64;
    let mut inner_payload: Vec<u8> = Vec::new();
    for _ in 0..entries {
        let key = dec.next();
        let val = dec.next();
        match (key, val) {
            (Ok(Item::UInt(1)), Ok(Item::UInt(n))) => sub_id = n,
            (Ok(Item::UInt(2)), Ok(Item::Text(b))) => {
                event_class = String::from_utf8_lossy(b).into_owned();
            }
            (Ok(Item::UInt(4)), Ok(Item::Bytes(b))) => inner_payload = b.to_vec(),
            (Ok(Item::UInt(5)), Ok(Item::UInt(n))) => from_hive = n,
            (Ok(Item::UInt(7)), Ok(Item::UInt(n))) => msg_id = n,
            _ => {}
        }
    }
    let payload_hex: String = inner_payload.iter().take(64).map(|b| format!("{:02X}", b)).collect();
    let suffix = if inner_payload.len() > 64 { "..." } else { "" };
    println!(
        "delivery sub_id={} from=0x{:08X} msg_id={} class={} payload={}{}",
        sub_id, from_hive as u32, msg_id, event_class, payload_hex, suffix
    );
}

fn decode_error_payload(payload: &[u8]) -> String {
    use r2_cbor::{Decoder, Item};
    let mut dec = Decoder::new(payload);
    let entries = match dec.next() {
        Ok(Item::Map(n)) => n,
        _ => return "(unparseable error payload)".into(),
    };
    let mut code = String::new();
    let mut detail = String::new();
    for _ in 0..entries {
        let key = dec.next();
        let val = dec.next();
        match (key, val) {
            (Ok(Item::UInt(1)), Ok(Item::Text(b))) => code = String::from_utf8_lossy(b).into_owned(),
            (Ok(Item::UInt(2)), Ok(Item::Text(b))) => detail = String::from_utf8_lossy(b).into_owned(),
            _ => {}
        }
    }
    if detail.is_empty() {
        code
    } else {
        format!("{code} ({detail})")
    }
}
