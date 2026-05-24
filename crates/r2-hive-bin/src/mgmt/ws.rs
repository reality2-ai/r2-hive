//! WebSocket parallel transport for the R2-HIVE / R2-HOST-API local
//! management surface.
//!
//! See R2-HOST-API §2.2. The endpoint is mounted at `/r2/mgmt`. Each binary
//! WebSocket message carries exactly one R2-WIRE extended frame (no length
//! prefix — WebSocket provides framing). The dispatcher is the same one
//! that handles UDS connections; the difference is purely the framing
//! envelope.
//!
//! Per-connection model matches the UDS handler in socket.rs: each
//! connection has its own subscription registry. Notifications and
//! responses are interleaved on the same socket via tokio::select on the
//! outbound mpsc channel and the WS receive stream.
//!
//! Text WebSocket messages are a protocol violation per R2-HOST-API §2.2
//! and cause the handler to close the connection.
//!
//! v0.1 access control: the route is bound to loopback by the axum
//! `bind` address and trusts the loopback origin. Browser device pairing
//! (R2-WEB §1.1) is a v0.2 deliverable.

use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::Extension;
use axum::response::IntoResponse;
use tokio::sync::{mpsc, Mutex};

use super::api::handle_frame_with_subs;
use super::state::DaemonState;
use super::subscriptions::SubscriptionRegistry;

/// Outbound mpsc channel capacity per connection. Per R2-HOST-API §4.3.
const OUTBOUND_QUEUE_CAPACITY: usize = 1024;

/// Axum route handler. Mount with:
/// ```ignore
/// .route("/r2/mgmt", axum::routing::get(crate::mgmt::ws::handler))
/// .layer(Extension(daemon_state))
/// ```
/// `DaemonState` is read via `Extension` rather than `State` so this route
/// composes with the existing `Arc<HiveState>` state without forcing a
/// router-wide state-type refactor.
pub async fn handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<DaemonState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: DaemonState) {
    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(OUTBOUND_QUEUE_CAPACITY);

    let (subscriber_id, subs) = if let Some(hive) = state.hive_state() {
        let (id, subs) = hive.register_subscriber(out_tx.clone()).await;
        (Some(id), subs)
    } else {
        (None, Arc::new(Mutex::new(SubscriptionRegistry::new())))
    };

    loop {
        tokio::select! {
            // Outbound: response or unsolicited notification queued by
            // a handler or by HiveState::deliver_inbound.
            outbound = out_rx.recv() => {
                let frame = match outbound {
                    Some(f) => f,
                    None => break, // sender dropped
                };
                if socket.send(Message::Binary(frame.into())).await.is_err() {
                    break;
                }
            }
            // Inbound: request from the WS peer.
            inbound = socket.recv() => {
                let msg = match inbound {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        log::warn!("/r2/mgmt: recv error: {e}");
                        break;
                    }
                    None => break, // peer closed
                };
                match msg {
                    Message::Binary(bytes) => {
                        let response = handle_frame_with_subs(&bytes, &state, &subs).await;
                        if out_tx.send(response).await.is_err() {
                            break;
                        }
                    }
                    Message::Text(_) => {
                        let _ = socket
                            .send(Message::Close(Some(CloseFrame {
                                code: 1003,
                                reason: "/r2/mgmt expects binary frames".into(),
                            })))
                            .await;
                        break;
                    }
                    Message::Ping(_) | Message::Pong(_) => {}
                    Message::Close(_) => break,
                }
            }
        }
    }

    drop(out_tx);

    if let (Some(hive), Some(id)) = (state.hive_state(), subscriber_id) {
        hive.unregister_subscriber(id).await;
    }
}
