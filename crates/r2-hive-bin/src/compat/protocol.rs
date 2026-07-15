//! WebSocket signaling protocol — JSON messages for the AUTH-FREE §3.2 relay subscribe.
//!
//! R2-TRANSPORT-RELAY v0.11 (Roy ruling 2026-07-07): the below-TG relay authenticates
//! NOTHING about the device. A connection opens with a version-3 SUBSCRIBE carrying only a
//! `trust_group` routing hash + `timestamp` — no `device_id`, no signature, no
//! challenge-response. Trust is end-to-end (TG-HMAC + §7.5.4 deliver-gate at member devices),
//! never device-to-relay. The former Ed25519 device-first handshake (v0.2–v0.10) was removed:
//! it violated the R2-WIRE §6.2.2 `device_id`-off-air MUST and gated nothing (the relay holds
//! no TG secret, so it cannot distinguish a member from a non-member). Conformance is now
//! structural — the accepted subscribe shape + the reject cases below.
//!
//! Vectors: `r2-specifications/testing/test-vectors/r2-transport-relay-vectors.json`.

use serde::{Deserialize, Serialize};

/// Messages sent by the client (device/browser) to the relay.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// §3.2 step 1 — the auth-free open. The relay reads only `version` (MUST be 3),
    /// `trust_group` (routing hash), and `timestamp` (stale-reject window). No identity,
    /// no signature: the relay records no stable device key.
    #[serde(rename = "subscribe")]
    Subscribe {
        version: u32,
        trust_group: String,
        timestamp: u64,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "catchup")]
    Catchup { since: u64 },
}

/// Messages sent by the relay to the client.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// §3.2 step 2 — the relay's reply to an accepted SUBSCRIBE (echoes version 3).
    #[serde(rename = "welcome")]
    Welcome {
        version: u32,
        peers: usize,
        buffer_oldest: u64,
    },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "catchup_incomplete")]
    CatchupIncomplete { oldest: u64 },
}

// ── WebSocket close codes per R2-TRANSPORT-RELAY §3.5 / §3.2 ──
/// §3.2.1 stateless stale-timestamp fast-reject (applied before allocating connection state).
pub const CLOSE_STALE_TIMESTAMP: u16 = 4400;
/// §3.2.3 retired protocol version (the 1/2 device-auth handshakes) — only version 3 is
/// served. Also the catch-all for a non-conforming open (malformed / non-subscribe / an
/// unresolvable `trust_group`): there is no valid version-3 subscribe to serve.
pub const CLOSE_RETIRED_VERSION: u16 = 4401;
/// §3.2.2 a token-requiring relay rejects a missing/invalid capability token. This OPEN relay
/// never emits it — the blinded token is a deferred follow-up rev (spec'd, wire/vectors TBD).
#[allow(dead_code)]
pub const CLOSE_TOKEN_REQUIRED: u16 = 4403;
pub const CLOSE_HEARTBEAT_TIMEOUT: u16 = 4408;
/// §3.2.1 connection-scoped anti-abuse floor (per-IP / per-`trust_group` cap). No identity.
pub const CLOSE_TOO_MANY: u16 = 4429;
