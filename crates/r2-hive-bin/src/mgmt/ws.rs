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
//! Access control: the route is mounted only on loopback listeners and
//! every upgrade must carry a valid active R2 web-auth session cookie.
//! Browser-originated upgrades are additionally same-origin checked.

use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::Extension;
use axum::response::{IntoResponse, Response};
use tokio::sync::{mpsc, Mutex};

use super::api::handle_frame_with_subs;
use super::state::DaemonState;
use super::subscriptions::SubscriptionRegistry;
use crate::hive::HiveState;

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
    State(hive): State<Arc<HiveState>>,
    headers: HeaderMap,
    Extension(state): Extension<DaemonState>,
) -> Response {
    match authorize_upgrade(&hive, &headers) {
        Ok(()) => ws.on_upgrade(move |socket| handle_socket(socket, state)).into_response(),
        Err(resp) => resp,
    }
}

/// Reused by the `/routes` + `/stats` topology-read endpoints (R2 audit P0): those exposed the
/// neighbour/path graph unauthenticated while publicly proxied. Gating them behind this same
/// same-origin + web-auth-cookie check closes the leak with the mgmt-equivalent posture.
pub fn authorize_upgrade(hive: &HiveState, headers: &HeaderMap) -> Result<(), Response> {
    if !same_origin_or_non_browser(headers) {
        return Err((StatusCode::FORBIDDEN, "origin rejected").into_response());
    }

    let Some(auth) = hive.web_auth() else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "web auth not configured").into_response());
    };
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    match auth.verify_cookie_header(cookie_header) {
        Ok(_device_id) => Ok(()),
        Err(_) => Err((StatusCode::UNAUTHORIZED, "authentication required").into_response()),
    }
}

fn same_origin_or_non_browser(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    origin_authority(origin)
        .map(|authority| authority.eq_ignore_ascii_case(host))
        .unwrap_or(false)
}

fn origin_authority(origin: &str) -> Option<&str> {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    Some(&rest[..end])
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use crate::web_auth::WebAuth;

    fn headers(cookie: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::HOST, HeaderValue::from_static("127.0.0.1:21042"));
        h.insert(header::ORIGIN, HeaderValue::from_static("http://127.0.0.1:21042"));
        if let Some(cookie) = cookie {
            h.insert(header::COOKIE, HeaderValue::from_str(cookie).unwrap());
        }
        h
    }

    #[test]
    fn authorize_upgrade_requires_auth_registry() {
        let hive = HiveState::new(0x1, 64, 4);
        let err = authorize_upgrade(&hive, &headers(None)).unwrap_err();
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn authorize_upgrade_rejects_missing_cookie() {
        let hive = HiveState::new(0x1, 64, 4);
        hive.set_web_auth(Arc::new(WebAuth::new([0x11; 32])));
        let err = authorize_upgrade(&hive, &headers(None)).unwrap_err();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn authorize_upgrade_accepts_active_cookie_and_rejects_revoked_cookie() {
        let hive = HiveState::new(0x1, 64, 4);
        let auth = Arc::new(WebAuth::new([0x22; 32]));
        let code = auth.mint_provision_code_with_ttl(60);
        let (cred, set_cookie) = auth.redeem_provision_code(&code).unwrap();
        let cookie = set_cookie.split(';').next().unwrap().to_string();
        hive.set_web_auth(auth.clone());

        assert!(authorize_upgrade(&hive, &headers(Some(&cookie))).is_ok());
        auth.revoke_device(&cred.device_id);
        let err = authorize_upgrade(&hive, &headers(Some(&cookie))).unwrap_err();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn authorize_upgrade_rejects_cross_origin_browser() {
        let hive = HiveState::new(0x1, 64, 4);
        hive.set_web_auth(Arc::new(WebAuth::new([0x33; 32])));
        let mut h = headers(None);
        h.insert(header::ORIGIN, HeaderValue::from_static("https://evil.example"));
        let err = authorize_upgrade(&hive, &h).unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }
}
