//! WebSocket connection handler — AUTH-FREE §3.2 relay subscribe.
//!
//! The below-TG relay compatibility layer. It:
//! 1. Accepts a version-3 SUBSCRIBE (trust_group + timestamp; NO device identity)
//! 2. Registers the connection with the WebSocket transport's PeerMap under an
//!    EPHEMERAL per-connection handle (no stable device id is recorded)
//! 3. Routes binary frames through the PeerMap (broadcast to trust group)
//! 4. Handles Ping/Pong and Catchup signaling
//!
//! Trust is END-TO-END (TG-HMAC + §7.5.4 deliver-gate at member devices), never
//! device-to-relay: the relay holds no TG secret, so it authenticates nothing.
//! The former Ed25519 device-first handshake (v0.2–v0.10) was REMOVED by Roy ruling
//! (2026-07-07) — it violated the R2-WIRE §6.2.2 `device_id`-off-air MUST and gated
//! nothing. In future phases step 3 is replaced by route-engine-driven forwarding.
//!
//! ## Interlinks + canon
//!
//! Entered from `main.rs`'s `/r2` route (`ws_handler` upgrade). Uses the
//! TG-compat surface on `HiveState` (register/broadcast/buffer/catchup —
//! see the banner in `hive.rs`) and hands binary frames to
//! `router::route_frame`, consuming the outcome (`NotR2Wire` → legacy
//! broadcast; `Flooded` → `flood_tg_peers_not_in` enrichment). Canon:
//! R2-TRANSPORT-RELAY §3.2 v0.11 (auth-free subscribe; structural vectors in
//! `r2-specifications/testing/test-vectors/r2-transport-relay-vectors.json`) —
//! `r2-specifications/specs/r2-core/R2-TRANSPORT-RELAY.md`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use r2_discovery::{LinkQuality, OutboundRx, PeerMap, RelayConn};

use super::protocol::*;
use crate::hive::{hex_encode, HiveState};

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(90);
const TIMESTAMP_WINDOW: u64 = 60;

/// Ephemeral per-connection handle source. The auth-free relay records NO stable device
/// identity (R2-TRANSPORT-RELAY §3.2 v0.11), so the PeerMap key is just a process-unique
/// connection handle: unique-among-live-connections is all the routing/"don't echo to
/// sender" logic needs. Wrap after 2^32 connections is a non-issue (no 4B live at once).
static NEXT_CONN_HANDLE: AtomicU32 = AtomicU32::new(1);

/// Allocate the next ephemeral connection handle (never 0 — 0 stays reserved as "none").
fn next_conn_handle() -> u32 {
    let h = NEXT_CONN_HANDLE.fetch_add(1, Ordering::Relaxed);
    if h == 0 {
        NEXT_CONN_HANDLE.fetch_add(1, Ordering::Relaxed)
    } else {
        h
    }
}

/// Handle a single WebSocket connection through its full lifecycle.
pub async fn handle_connection(mut socket: WebSocket, state: Arc<HiveState>) {
    // Phase 1: Handshake
    let (tg_hash, hive_id) = match subscribe(&mut socket, &state).await {
        Some(result) => result,
        None => return,
    };

    log::info!(
        "connection joined tg:{} (conn=0x{:08X})",
        hex_encode(&tg_hash),
        hive_id
    );

    // Phase 2: Register this connected relay peer (R2-DISCOVERY §4.4.1).
    // `connect` records the peer and returns its outbound frame receiver, drained
    // in the writer branch below. RelayConn identifies this WS connection.
    let quality = LinkQuality {
        latency_ms: 5,
        ..Default::default()
    };
    let mut outbound_rx =
        state.ws_transport.peers().connect(hive_id, RelayConn(hive_id as u64), quality);

    // Also register in the trust group compat map for broadcast routing
    state.register_tg_peer(tg_hash, hive_id).await;

    let mut last_activity = Instant::now();

    // Keepalive: proactively send a WebSocket Ping every 25s. The client
    // (browser) auto-replies with Pong, which arrives on socket.recv()
    // and refreshes last_activity below — so an idle connection (e.g. a
    // key holder waiting to be reached by a joiner) stays up instead of
    // being dropped at HEARTBEAT_TIMEOUT or by a proxy/NAT idle close. A
    // genuinely dead peer stops ponging, so the heartbeat still reaps it.
    // This keeps a hive present in its trust-group bucket while
    // connected; it does not extend any join-code validity.
    let mut keepalive = tokio::time::interval(Duration::from_secs(25));
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    keepalive.tick().await; // consume the immediate first tick

    // Phase 3: Frame routing loop
    loop {
        tokio::select! {
            // Receive from WebSocket (client → wayfinder)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        last_activity = Instant::now();
                        state.frames_routed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        // Push into transport's inbound channel
                        state.ws_transport.peers().push_inbound(hive_id, data.to_vec(), quality);

                        // Buffer for catchup
                        state.buffer_frame(&tg_hash, data.to_vec()).await;

                        // Route engine: parse header, forward based on routing decision.
                        // The router is trust-agnostic; we add intra-TG enrichment
                        // (flood to known TG members the engine hasn't yet observed)
                        // and the legacy 0xFF join broadcast based on RouteOutcome.
                        // The router calls state.send_to_hive() which uses the
                        // multi-transport fallback chain (WS → UDP → BLE) — no
                        // explicit per-transport broadcast needed here.
                        use crate::router::RouteOutcome;
                        match crate::router::route_frame(
                            &state, hive_id,
                            r2_route::transport::Transport::Internet,
                            &data,
                        ).await {
                            RouteOutcome::NotR2Wire => {
                                // Legacy 0xFF join protocol — fall back to TG broadcast.
                                state.broadcast_to_tg(&tg_hash, hive_id, &data).await;
                            }
                            RouteOutcome::Flooded(hops) => {
                                // Intra-TG enrichment: also flood to TG members the
                                // engine doesn't yet know about (freshly connected,
                                // no observation ingested yet).
                                state.flood_tg_peers_not_in(&tg_hash, hive_id, &hops, &data).await;
                            }
                            _ => {} // Drop / DeliverOnly / Directed: nothing extra
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        last_activity = Instant::now();
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Ping) => {
                                let pong = serde_json::to_string(&ServerMessage::Pong).unwrap();
                                if socket.send(Message::Text(pong.into())).await.is_err() {
                                    break;
                                }
                            }
                            Ok(ClientMessage::Catchup { since }) => {
                                let frames = state.catchup_frames(&tg_hash, since).await;
                                for frame_data in frames {
                                    if socket.send(Message::Binary(frame_data.into())).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    // Ping / Pong (including replies to our keepalive) — liveness.
                    _ => { last_activity = Instant::now(); }
                }
            }

            // Send to WebSocket (wayfinder → client): drain this peer's outbound
            // queue (R2-DISCOVERY §4.4 OutboundRx) and forward over the socket.
            frame = outbound_rx.next() => {
                match frame {
                    Some(data) => {
                        if data.is_empty() { continue; }
                        if socket.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // Keepalive ping — see `keepalive` above. Transient-network
            // link keepalive, here over wss.
            _ = keepalive.tick() => {
                if socket.send(Message::Ping(Vec::<u8>::new().into())).await.is_err() {
                    break;
                }
            }

            // Heartbeat timeout
            _ = tokio::time::sleep(HEARTBEAT_TIMEOUT) => {
                if last_activity.elapsed() >= HEARTBEAT_TIMEOUT {
                    log::warn!("heartbeat timeout for hive_id=0x{:08X}", hive_id);
                    let _ = socket.send(Message::Close(Some(CloseFrame {
                        code: CLOSE_HEARTBEAT_TIMEOUT,
                        reason: "heartbeat timeout".into(),
                    }))).await;
                    break;
                }
            }
        }
    }

    // Cleanup
    state.ws_transport.peers().remove_peer(hive_id);
    state.unregister_tg_peer(&tg_hash, hive_id).await;

    log::info!(
        "connection left tg:{} (conn=0x{:08X})",
        hex_encode(&tg_hash),
        hive_id
    );
}

/// Accept the relay-side AUTH-FREE §3.2 subscribe (R2-TRANSPORT-RELAY v0.11). Returns
/// `(tg_hash, ephemeral_conn_handle)` on accept, `None` (socket already closed) on reject.
///
/// The open is a single version-3 SUBSCRIBE `{version, trust_group, timestamp}` — no
/// `device_id`, no signature, no challenge-response. The relay authenticates NOTHING about the
/// device (trust is end-to-end at member devices); it only:
///   1. rejects retired versions 1/2 (§3.2.3, close 4401),
///   2. stateless stale-timestamp fast-rejects BEFORE allocating state (§3.2.1, close 4400),
///   3. enforces the connection-scoped cap (§3.2.1, close 4429),
///   4. resolves the `trust_group` routing hash and mints an EPHEMERAL per-connection handle
///      (no stable device identity is recorded).
///
/// The optional §3.2.2 capability token is a deferred follow-up rev: this OPEN relay ignores a
/// token field entirely (never emits close 4403). **Used-by:** [`handle_connection`].
async fn subscribe(
    socket: &mut WebSocket,
    state: &Arc<HiveState>,
) -> Option<([u8; 8], u32)> {
    let first = tokio::time::timeout(Duration::from_secs(10), socket.recv()).await;

    let text = match first {
        Ok(Some(Ok(Message::Text(text)))) => text.to_string(),
        _ => {
            close_with(socket, CLOSE_RETIRED_VERSION, "expected SUBSCRIBE").await;
            return None;
        }
    };

    let msg: ClientMessage = match serde_json::from_str(&text) {
        Ok(msg) => msg,
        Err(_) => {
            close_with(socket, CLOSE_RETIRED_VERSION, "malformed SUBSCRIBE").await;
            return None;
        }
    };

    let (version, trust_group_hex, timestamp) = match msg {
        ClientMessage::Subscribe {
            version,
            trust_group,
            timestamp,
        } => (version, trust_group, timestamp),
        _ => {
            close_with(socket, CLOSE_RETIRED_VERSION, "expected SUBSCRIBE").await;
            return None;
        }
    };

    // §3.2.3: only version 3 is served. Versions 1/2 (the retired Ed25519 device-auth
    // handshakes) — and any other version — are rejected. A conforming device opens with 3.
    if version != 3 {
        close_with(socket, CLOSE_RETIRED_VERSION, "retired protocol version").await;
        return None;
    }

    // §3.2.1: stateless stale-timestamp fast-reject, applied BEFORE allocating any connection
    // state (don't spend state on a clock-skewed / replayed open).
    let now = state.platform.now_unix();
    if timestamp > now + TIMESTAMP_WINDOW || now > timestamp + TIMESTAMP_WINDOW {
        close_with(socket, CLOSE_STALE_TIMESTAMP, "timestamp out of range").await;
        return None;
    }

    // §3.2.1: connection-scoped anti-abuse floor (no device identity is involved).
    if state.ws_transport.peers().peer_count() >= state.max_connections {
        close_with(socket, CLOSE_TOO_MANY, "too many connections").await;
        return None;
    }

    // Resolve the trust-group routing hash (exact 16-char hex or a 2-6 char word-code prefix).
    // An unresolvable group is a non-conforming open — nothing to route it into.
    let tg_hash_bytes = match state.resolve_tg_hash(&trust_group_hex).await {
        Ok(h) => h,
        Err(reason) => {
            close_with(socket, CLOSE_RETIRED_VERSION, reason).await;
            return None;
        }
    };

    // Ephemeral per-connection handle — the relay records NO stable device identity (v0.11).
    let hive_id = next_conn_handle();

    // Send WELCOME (echoes version 3).
    let peers = state.tg_peer_count(&tg_hash_bytes).await;
    let buffer_oldest = state.buffer_oldest(&tg_hash_bytes).await;
    let welcome = serde_json::to_string(&ServerMessage::Welcome {
        version: 3,
        peers,
        buffer_oldest,
    })
    .unwrap();
    if socket.send(Message::Text(welcome.into())).await.is_err() {
        return None;
    }

    Some((tg_hash_bytes, hive_id))
}

async fn close_with(socket: &mut WebSocket, code: u16, reason: &str) {
    log::warn!("closing connection: {} ({})", reason, code);
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Structural vectors from
    // r2-specifications/testing/test-vectors/r2-transport-relay-vectors.json
    // (R2-TRANSPORT-RELAY v0.11, auth-free subscribe). There is NO signing construction to
    // reproduce — the relay verifies no device key. Conformance is the accepted subscribe
    // shape + the reject-case close codes.

    /// The canonical `subscribe_example` frame deserializes to a version-3 Subscribe with only
    /// trust_group + timestamp — no device_id / signature fields exist on the type.
    #[test]
    fn subscribe_example_deserializes() {
        let frame = r#"{"type":"subscribe","version":3,"trust_group":"a1b2c3d4e5f60718","timestamp":1711900000}"#;
        match serde_json::from_str::<ClientMessage>(frame).expect("valid subscribe") {
            ClientMessage::Subscribe {
                version,
                trust_group,
                timestamp,
            } => {
                assert_eq!(version, 3);
                assert_eq!(trust_group, "a1b2c3d4e5f60718");
                assert_eq!(timestamp, 1711900000);
            }
            other => panic!("expected Subscribe, got {other:?}"),
        }
    }

    /// The WELCOME reply serializes to the vector shape (type/version/peers/buffer_oldest).
    #[test]
    fn welcome_reply_matches_vector() {
        let welcome = serde_json::to_string(&ServerMessage::Welcome {
            version: 3,
            peers: 3,
            buffer_oldest: 1711898000,
        })
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&welcome).unwrap();
        assert_eq!(v["type"], "welcome");
        assert_eq!(v["version"], 3);
        assert_eq!(v["peers"], 3);
        assert_eq!(v["buffer_oldest"], 1711898000u64);
    }

    /// Reject-case close codes pinned to the v0.11 vectors (§3.5 / §3.2).
    #[test]
    fn reject_close_codes_match_spec() {
        assert_eq!(CLOSE_STALE_TIMESTAMP, 4400); // stale_timestamp (§3.2.1)
        assert_eq!(CLOSE_RETIRED_VERSION, 4401); // retired version 1/2 (§3.2.3)
        assert_eq!(CLOSE_TOO_MANY, 4429); // connection cap (§3.2.1)
        assert_eq!(CLOSE_TOKEN_REQUIRED, 4403); // §3.2.2 token-requiring relay (unused: open)
    }

    /// A retired-version open (version 1 or 2) still parses as a Subscribe — the version-3
    /// gate is what rejects it (close 4401), not a parse failure. Guards that the wire shape is
    /// version-agnostic so the relay can emit the correct retired-version close.
    #[test]
    fn retired_versions_parse_but_are_not_v3() {
        for v in [1u32, 2] {
            let frame = format!(
                r#"{{"type":"subscribe","version":{v},"trust_group":"a1b2c3d4e5f60718","timestamp":1711900000}}"#
            );
            match serde_json::from_str::<ClientMessage>(&frame).expect("parses") {
                ClientMessage::Subscribe { version, .. } => assert_ne!(version, 3),
                other => panic!("expected Subscribe, got {other:?}"),
            }
        }
    }
}

