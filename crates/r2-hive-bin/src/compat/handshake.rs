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
/// Max lifetime of a v0.2 challenge nonce — the device must return its AUTH
/// within this window or the handshake is rejected (R2-TRANSPORT-RELAY §3.2.1).
const CHALLENGE_TTL: Duration = Duration::from_secs(10);

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

/// Perform the relay-side HELLO/WELCOME handshake. Returns (tg_hash,
/// device_id_hex, hive_id).
///
/// Supports both relay-protocol versions (R2-TRANSPORT-RELAY §3.2):
///   * **v0.1** — single HELLO carrying the Ed25519 signature over
///     `<trust_group>:<device_id>:<timestamp>`. Kept for legacy clients.
///   * **v0.2** — device-first challenge-response. HELLO carries no signature;
///     the relay replies with a single-use CHALLENGE nonce, and the device
///     returns an AUTH signed over
///     `<nonce>:<trust_group>:<device_id>:<timestamp>`.
///
/// This is the relay (server) half: the device's `v2-pinned` downgrade
/// protection (§3.2.2) lives on the client and is not implemented here.
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

    let (version, trust_group_hex, device_id_hex, timestamp, hello_signature) = match msg {
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

    // Stateless timestamp fast-reject (±60s), applied BEFORE any challenge
    // state is issued or consumed (R2-TRANSPORT-RELAY §3.2 step 4 / §3.2.1).
    let now = now_unix();
    if timestamp > now + TIMESTAMP_WINDOW || now > timestamp + TIMESTAMP_WINDOW {
        close_with(socket, CLOSE_AUTH_FAILED, "timestamp out of range").await;
        return None;
    }

    // Device public key (Ed25519) — the claimed identity, common to both versions.
    let device_pk_bytes = match hex_decode(&device_id_hex) {
        Some(b) if b.len() == 32 => b,
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "invalid device_id").await;
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

    // Resolve the signature and the exact message it must sign, per version.
    let (signature_hex, msg_to_verify) = match version {
        1 => {
            // v0.1: signature is inline in HELLO; signs tg:device_id:timestamp.
            let sig = match hello_signature {
                Some(s) => s,
                None => {
                    close_with(socket, CLOSE_AUTH_FAILED, "missing signature").await;
                    return None;
                }
            };
            let msg = format!("{}:{}:{}", trust_group_hex, device_id_hex, timestamp);
            (sig, msg)
        }
        2 => {
            // v0.2: device-first challenge-response. Issue a single-use nonce,
            // then read the AUTH carrying the echoed nonce + signature.
            let nonce_hex = match issue_nonce() {
                Some(n) => n,
                None => {
                    close_with(socket, CLOSE_AUTH_FAILED, "nonce generation failed").await;
                    return None;
                }
            };

            let challenge = serde_json::to_string(&ServerMessage::Challenge {
                version: 2,
                nonce: nonce_hex.clone(),
            })
            .unwrap();
            if socket.send(Message::Text(challenge.into())).await.is_err() {
                return None;
            }

            // The AUTH must arrive within the challenge lifetime (≤10s, §3.2.1).
            let auth = tokio::time::timeout(CHALLENGE_TTL, socket.recv()).await;
            let auth_text = match auth {
                Ok(Some(Ok(Message::Text(text)))) => text.to_string(),
                _ => {
                    close_with(socket, CLOSE_AUTH_FAILED, "expected AUTH").await;
                    return None;
                }
            };
            let auth_msg: ClientMessage = match serde_json::from_str(&auth_text) {
                Ok(m) => m,
                Err(_) => {
                    close_with(socket, CLOSE_AUTH_FAILED, "malformed AUTH").await;
                    return None;
                }
            };
            let (auth_nonce, auth_sig) = match auth_msg {
                ClientMessage::Auth {
                    nonce, signature, ..
                } => (nonce, signature),
                _ => {
                    close_with(socket, CLOSE_AUTH_FAILED, "expected AUTH").await;
                    return None;
                }
            };

            // The echoed nonce MUST match the one we issued to this connection
            // (single-use, unexpired). The nonce is public, so a plain compare
            // is sufficient — there is no secret to leak by timing.
            if auth_nonce != nonce_hex {
                close_with(socket, CLOSE_AUTH_FAILED, "nonce mismatch").await;
                return None;
            }

            let msg = format!(
                "{}:{}:{}:{}",
                nonce_hex, trust_group_hex, device_id_hex, timestamp
            );
            (auth_sig, msg)
        }
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "unsupported version").await;
            return None;
        }
    };

    // Verify the Ed25519 signature over the version-specific message.
    let sig_bytes = match hex_decode(&signature_hex) {
        Some(b) if b.len() == 64 => b,
        _ => {
            close_with(socket, CLOSE_AUTH_FAILED, "invalid signature").await;
            return None;
        }
    };
    let sig = Signature::from_bytes(sig_bytes[..64].try_into().unwrap());
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

    // Send WELCOME, echoing the negotiated protocol version.
    let peers = state.tg_peer_count(&tg_hash_bytes).await;
    let buffer_oldest = state.buffer_oldest(&tg_hash_bytes).await;

    let welcome = serde_json::to_string(&ServerMessage::Welcome {
        version,
        peers,
        buffer_oldest,
    }).unwrap();

    if socket.send(Message::Text(welcome.into())).await.is_err() {
        return None;
    }

    Some((tg_hash_bytes, device_id_hex, hive_id))
}

/// Generate a single-use 32-byte challenge nonce (lowercase hex) from the OS
/// CSPRNG (R2-TRANSPORT-RELAY §3.2.1). Returns None if the RNG is unavailable.
fn issue_nonce() -> Option<String> {
    let mut nonce = [0u8; 32];
    getrandom::getrandom(&mut nonce).ok()?;
    Some(hex_encode(&nonce))
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

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical `valid_handshake` vector from
    // r2-specifications/testing/test-vectors/r2-transport-relay-vectors.json
    // (R2-TRANSPORT-RELAY v0.2 §3.2). TEST-ONLY key — never used in production.
    const VEC_NONCE: &str =
        "0f1e2d3c4b5a69788796a5b4c3d2e1f00112233445566778899aabbccddeeff00";
    const VEC_TRUST_GROUP: &str = "a1b2c3d4e5f60718";
    const VEC_DEVICE_ID: &str =
        "b52557a04646443e40a591f0f5d9ab81b7d66155e72890c75d288d37bebbb49e";
    const VEC_TIMESTAMP: u64 = 1711900000;
    const VEC_SIGNATURE: &str = "f51b2ca7825ef7cdbb779b3cbd8d9c22cec326ee4e5f61184f675f7ca04f0c105797340b445dcdd49d55c4f7625468eb9d716821d228b709873e58bd250abd02";

    fn v2_signing_message() -> String {
        // Mirrors the relay's construction in `handshake()` for version 2.
        format!(
            "{}:{}:{}:{}",
            VEC_NONCE, VEC_TRUST_GROUP, VEC_DEVICE_ID, VEC_TIMESTAMP
        )
    }

    fn verify(sig_hex: &str) -> bool {
        let pk = hex_decode(VEC_DEVICE_ID).unwrap();
        let vk = VerifyingKey::from_bytes(pk[..32].try_into().unwrap()).unwrap();
        let sig_bytes = hex_decode(sig_hex).unwrap();
        let sig = Signature::from_bytes(sig_bytes[..64].try_into().unwrap());
        vk.verify(v2_signing_message().as_bytes(), &sig).is_ok()
    }

    /// The 4-field signing message must match the vector byte-for-byte.
    #[test]
    fn v2_signing_message_matches_vector() {
        assert_eq!(
            v2_signing_message(),
            "0f1e2d3c4b5a69788796a5b4c3d2e1f00112233445566778899aabbccddeeff00:\
a1b2c3d4e5f60718:\
b52557a04646443e40a591f0f5d9ab81b7d66155e72890c75d288d37bebbb49e:\
1711900000"
        );
    }

    /// A conforming relay MUST accept the canonical signature (`expect: accept`).
    #[test]
    fn v2_valid_signature_accepts() {
        assert!(verify(VEC_SIGNATURE));
    }

    /// `tampered_signature` reject case: last byte 0x02 -> 0x03 MUST fail (4401).
    #[test]
    fn v2_tampered_signature_rejected() {
        let tampered = "f51b2ca7825ef7cdbb779b3cbd8d9c22cec326ee4e5f61184f675f7ca04f0c105797340b445dcdd49d55c4f7625468eb9d716821d228b709873e58bd250abd03";
        assert!(!verify(tampered));
    }
}
