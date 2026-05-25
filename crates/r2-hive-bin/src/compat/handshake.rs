//! WebSocket connection handler with HELLO/WELCOME handshake.
//!
//! This is the legacy relay compatibility layer. It:
//! 1. Performs Ed25519-authenticated HELLO/WELCOME handshake
//! 2. Registers the peer with the WebSocket transport's PeerMap
//! 3. Routes binary frames through the PeerMap (broadcast to trust group)
//! 4. Handles Ping/Pong and Catchup signaling
//!
//! In future phases, step 3 will be replaced by route-engine-driven
//! forwarding. The handshake and signaling stay.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use r2_transport::transport::LinkQuality;

use super::protocol::*;
use crate::hive::HiveState;

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(90);
const TIMESTAMP_WINDOW: u64 = 60;

/// Handle a single WebSocket connection through its full lifecycle.
pub async fn handle_connection(mut socket: WebSocket, state: Arc<HiveState>) {
    // Phase 1: Handshake
    let (tg_hash, device_id, hive_id) = match handshake(&mut socket, &state).await {
        Some(result) => result,
        None => return,
    };

    log::info!(
        "device {} joined tg:{} (hive_id=0x{:08X})",
        &device_id[..16.min(device_id.len())],
        hex_encode(&tg_hash),
        hive_id
    );

    // Phase 2: Register with WebSocket transport's PeerMap
    let quality = LinkQuality {
        quality: 0.9,
        latency_ms: 5,
        ..Default::default()
    };
    let mut outbound_rx = state.ws_transport.peers().add_peer(hive_id, quality).await;

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
                        state.ws_transport.peers().push_inbound(hive_id, data.to_vec(), quality).await;

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

            // Send to WebSocket (wayfinder → client)
            frame = outbound_rx.recv() => {
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
    state.ws_transport.peers().remove_peer(hive_id).await;
    state.unregister_tg_peer(&tg_hash, hive_id).await;

    log::info!(
        "device {} left tg:{} (hive_id=0x{:08X})",
        &device_id[..16.min(device_id.len())],
        hex_encode(&tg_hash),
        hive_id
    );
}

/// Perform HELLO/WELCOME handshake. Returns (tg_hash, device_id_hex, hive_id).
async fn handshake(
    socket: &mut WebSocket,
    state: &Arc<HiveState>,
) -> Option<([u8; 8], String, u32)> {
    let hello = tokio::time::timeout(Duration::from_secs(10), socket.recv()).await;

    let hello_text = match hello {
        Ok(Some(Ok(Message::Text(text)))) => text.to_string(),
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "expected HELLO").await;
            return None;
        }
    };

    let msg: ClientMessage = match serde_json::from_str(&hello_text) {
        Ok(msg) => msg,
        Err(_) => {
            close_with(socket, CLOSE_AUTH_FAILED, "malformed HELLO").await;
            return None;
        }
    };

    let (version, trust_group_hex, device_id_hex, timestamp, signature_hex) = match msg {
        ClientMessage::Hello {
            version,
            trust_group,
            device_id,
            timestamp,
            signature,
        } => (version, trust_group, device_id, timestamp, signature),
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "expected HELLO").await;
            return None;
        }
    };

    if version != 1 {
        close_with(socket, CLOSE_AUTH_FAILED, "unsupported version").await;
        return None;
    }

    // Verify timestamp
    let now = now_unix();
    if timestamp > now + TIMESTAMP_WINDOW || now > timestamp + TIMESTAMP_WINDOW {
        close_with(socket, CLOSE_AUTH_FAILED, "timestamp out of range").await;
        return None;
    }

    // Verify Ed25519 signature
    let device_pk_bytes = match hex_decode(&device_id_hex) {
        Some(b) if b.len() == 32 => b,
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "invalid device_id").await;
            return None;
        }
    };

    let sig_bytes = match hex_decode(&signature_hex) {
        Some(b) if b.len() == 64 => b,
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "invalid signature").await;
            return None;
        }
    };

    let vk = match VerifyingKey::from_bytes(device_pk_bytes[..32].try_into().unwrap()) {
        Ok(vk) => vk,
        Err(_) => {
            close_with(socket, CLOSE_AUTH_FAILED, "invalid public key").await;
            return None;
        }
    };

    let sig = Signature::from_bytes(sig_bytes[..64].try_into().unwrap());

    let msg_to_verify = format!("{}:{}:{}", trust_group_hex, device_id_hex, timestamp);
    if vk.verify(msg_to_verify.as_bytes(), &sig).is_err() {
        close_with(socket, CLOSE_AUTH_FAILED, "signature verification failed").await;
        return None;
    }

    // Parse trust group hash (exact 16-char hex or 2-6 char prefix for word code join)
    let tg_hash_bytes = match state.resolve_tg_hash(&trust_group_hex).await {
        Ok(h) => h,
        Err(reason) => {
            close_with(socket, CLOSE_AUTH_FAILED, reason).await;
            return None;
        }
    };

    // Connection limit
    if state.ws_transport.peers().peer_count().await >= state.max_connections {
        close_with(socket, CLOSE_TOO_MANY, "too many connections").await;
        return None;
    }

    // Compute hive_id: FNV-1a of device public key bytes
    let hive_id = fnv1a_32(&device_pk_bytes);

    // Send WELCOME
    let peers = state.tg_peer_count(&tg_hash_bytes).await;
    let buffer_oldest = state.buffer_oldest(&tg_hash_bytes).await;

    let welcome = serde_json::to_string(&ServerMessage::Welcome {
        version: 1,
        peers,
        buffer_oldest,
    }).unwrap();

    if socket.send(Message::Text(welcome.into())).await.is_err() {
        return None;
    }

    Some((tg_hash_bytes, device_id_hex, hive_id))
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

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// FNV-1a 32-bit hash.
fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}
