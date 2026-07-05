//! Unix-domain socket server: accepts connections, reads framed requests,
//! dispatches them through the API, writes framed responses.
//!
//! Per-connection model (R2-HOST-API §4 / §8): each connection owns a
//! subscription registry and an outbound mpsc channel. The reader awaits
//! requests and pushes responses to the channel; a writer task drains the
//! channel and writes to the socket. `HiveState::deliver_inbound` pushes
//! unsolicited `r2.api.event.delivery` notifications onto the same channel.
//!
//! ## Interlinks + canon
//!
//! Spawned from `main.rs` mgmt bring-up (socket path resolved there;
//! discipline per R2-TG-TOOL §5: per-user dir, 0600, same-UID). Each
//! connection: `framing.rs` decode → `api.rs::dispatch` → framed response;
//! subscriber registration via `HiveState::register_subscriber` (torn down
//! on close). Canon: R2-HOST-API §2.2 (UDS binding), §4/§8 (connection
//! model) — `r2-specifications/specs/r2-core/R2-HOST-API.md`;
//! R2-TG-TOOL §5 — `r2-specifications/specs/r2-core/R2-TG-TOOL.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot, Mutex};

use super::api::handle_frame_with_subs;
use super::framing::{read_frame, write_frame};
use super::state::DaemonState;
use super::subscriptions::SubscriptionRegistry;

/// Outbound mpsc channel capacity per connection. Per R2-HOST-API §4.3
/// the implementation default is 1024 pending deliveries.
const OUTBOUND_QUEUE_CAPACITY: usize = 1024;

/// Handle returned by [`spawn`] so callers (the binary, integration tests)
/// can observe the listener's bound path and request a clean shutdown.
pub struct ServerHandle {
    pub socket_path: PathBuf,
    pub shutdown: oneshot::Sender<()>,
    pub join: tokio::task::JoinHandle<()>,
}

/// Spawn the listener on `socket_path`, returning a handle.
///
/// If the path already exists, it is removed first (standard Unix socket
/// hygiene). Caller is responsible for any staler conflicts from a concurrent
/// daemon — v0.1 expects at most one daemon per user session.
pub async fn spawn(socket_path: PathBuf, state: DaemonState) -> std::io::Result<ServerHandle> {
    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
    }
    if let Some(parent) = socket_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).ok();
        }
    }

    let listener = UnixListener::bind(&socket_path)?;
    apply_socket_permissions(&socket_path)?;

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let bound_path = socket_path.clone();
    let state_for_task = state.clone();

    let join = tokio::spawn(async move {
        log::info!("r2-hive mgmt listening on {}", bound_path.display());
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    log::info!("shutdown requested; stopping listener");
                    break;
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, _addr)) => {
                            let st = state_for_task.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, st).await {
                                    log::warn!("connection ended with error: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("accept() failed: {e}");
                        }
                    }
                }
            }
        }
        // Remove the socket file on shutdown (best effort).
        let _ = fs::remove_file(&bound_path);
    });

    Ok(ServerHandle {
        socket_path,
        shutdown: shutdown_tx,
        join,
    })
}

/// Per-connection handler. Sets up the per-connection subscription
/// registry and outbound mpsc channel, registers with HiveState (if
/// available), spawns a writer task, and runs the read loop. Tears down
/// cleanly on EOF or error.
async fn handle_connection(stream: UnixStream, state: DaemonState) -> std::io::Result<()> {
    let (mut reader, mut writer) = stream.into_split();

    // Outbound channel: requests' responses + unsolicited notifications
    // both flow through here.
    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(OUTBOUND_QUEUE_CAPACITY);

    // Register this connection with HiveState (if attached). The returned
    // Arc<Mutex<SubscriptionRegistry>> is the per-connection store; the
    // subscribe/unsubscribe handlers mutate it directly.
    let (subscriber_id, subs) = if let Some(hive) = state.hive_state() {
        let (id, subs) = hive.register_subscriber(out_tx.clone()).await;
        (Some(id), subs)
    } else {
        // No HiveState: still create a local registry so the dispatcher
        // can accept subscribe/unsubscribe and reply, but no deliveries
        // will ever fire.
        (None, Arc::new(Mutex::new(SubscriptionRegistry::new())))
    };

    // Writer task: drain the outbound channel and write to the socket.
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if write_frame(&mut writer, &frame).await.is_err() {
                break;
            }
            if writer.flush().await.is_err() {
                break;
            }
        }
    });

    // Read loop: receive a request, dispatch, push response onto the
    // outbound channel. Returns Ok on clean EOF, Err on socket error.
    let read_result: std::io::Result<()> = loop {
        match read_frame(&mut reader).await {
            Err(e) => break Err(e),
            Ok(None) => break Ok(()), // peer closed cleanly
            Ok(Some(frame)) => {
                let response = handle_frame_with_subs(&frame, &state, &subs).await;
                if out_tx.send(response).await.is_err() {
                    // Writer task has already exited.
                    break Ok(());
                }
            }
        }
    };

    // Tear down: drop the sender so the writer task drains and exits, and
    // unregister from HiveState.
    drop(out_tx);
    let _ = writer_task.await;
    if let (Some(hive), Some(id)) = (state.hive_state(), subscriber_id) {
        hive.unregister_subscriber(id).await;
    }

    read_result
}

#[cfg(unix)]
fn apply_socket_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perm = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perm)
}

#[cfg(not(unix))]
fn apply_socket_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}
