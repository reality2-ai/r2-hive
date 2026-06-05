//! WebSocket signaling protocol — JSON messages for HELLO/WELCOME handshake.
//!
//! This is the legacy relay protocol used by existing clients (Notekeeper).
//! It's WebSocket-specific and NOT part of the R2 transport abstraction.

use serde::{Deserialize, Serialize};

/// Messages sent by the client (device/browser) to the relay.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "hello")]
    Hello {
        version: u32,
        trust_group: String,
        device_id: String,
        timestamp: u64,
        // v0.1 carries the Ed25519 signature inline in HELLO. v0.2 (device-first
        // ordering, R2-TRANSPORT-RELAY §3.2) omits it — the signature arrives
        // later in the AUTH message after the relay issues a CHALLENGE nonce.
        #[serde(default)]
        signature: Option<String>,
    },
    /// v0.2 challenge-response: the device's reply to a CHALLENGE, carrying the
    /// echoed nonce and the Ed25519 signature over
    /// `<nonce>:<trust_group>:<device_id>:<timestamp>` (R2-TRANSPORT-RELAY §3.2).
    #[serde(rename = "auth")]
    Auth {
        version: u32,
        nonce: String,
        signature: String,
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
    #[serde(rename = "welcome")]
    Welcome {
        version: u32,
        peers: usize,
        buffer_oldest: u64,
    },
    /// v0.2 device-first handshake: the relay's reply to a v2 HELLO, carrying a
    /// single-use challenge nonce the device must sign (R2-TRANSPORT-RELAY §3.2).
    #[serde(rename = "challenge")]
    Challenge {
        version: u32,
        /// 32-byte CSPRNG nonce, lowercase hex.
        nonce: String,
    },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "catchup_incomplete")]
    CatchupIncomplete { oldest: u64 },
}

/// WebSocket close codes per R2-TRANSPORT-RELAY §3.5.
pub const CLOSE_AUTH_FAILED: u16 = 4401;
#[allow(dead_code)]
pub const CLOSE_BANNED: u16 = 4403;
pub const CLOSE_HEARTBEAT_TIMEOUT: u16 = 4408;
pub const CLOSE_TOO_MANY: u16 = 4429;
