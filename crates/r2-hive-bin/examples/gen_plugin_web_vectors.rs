//! Generator for `r2-plugin-web-vectors.json` — R2-WEB §4.2/§4.5 WebSocket
//! message-auth conformance fixtures. Each WS message is a JSON envelope
//! `{device_id, timestamp, signature, payload}` where
//! `signature = Ed25519-Sign(DEV_SK, "<device_id>:<timestamp>:<payload>")`
//! (R2-WEB §4.2 v0.3 — Ed25519, NOT HMAC; the same correction as the relay
//! handshake). Covers the §4.5 message types both directions + reject cases.
//!
//! Run: `cargo run --example gen_plugin_web_vectors`
//! Writes `r2-specifications/testing/test-vectors/r2-plugin-web-vectors.json`.
//!
//! SPEC-DRIVEN, not impl-cross-checked: r2-hive's §4.2 per-message auth is a
//! future v0.2 deliverable (mgmt/ws.rs) — these vectors encode the canonical
//! spec algorithm (ed25519-dalek signing) and become the fixture r2-hive's
//! §4.2 impl is built against. specs independently verifies each signature
//! against device_id + lands canonical. TEST-ONLY key.

use std::fs;
use std::path::PathBuf;

use ed25519_dalek::{Signer, SigningKey};

const TIMESTAMP: u64 = 1711900000;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    // Deterministic TEST-ONLY browser device key (Ed25519 seed, §1.3).
    let seed: [u8; 32] = [
        0xa5, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f,
    ];
    let sk = SigningKey::from_bytes(&seed);
    let device_id = hex(sk.verifying_key().as_bytes());

    // (id, direction, message type, payload JSON string) — §4.5 message types.
    let msgs: &[(&str, &str, &str, &str)] = &[
        ("PW1", "browser_to_plugin", "command", r#"{"type":"command","event":"set_level","args":{"v":42}}"#),
        ("PW2", "browser_to_plugin", "query", r#"{"type":"query","query":"{ sentants { id } }"}"#),
        ("PW3", "browser_to_plugin", "subscribe", r#"{"type":"subscribe","filter":{"event":"read_level"}}"#),
        ("PW4", "browser_to_plugin", "unsubscribe", r#"{"type":"unsubscribe","filter":{"event":"read_level"}}"#),
        ("PW5", "browser_to_plugin", "ping", r#"{"type":"ping"}"#),
        ("PW6", "plugin_to_browser", "event", r#"{"type":"event","event":"read_level","data":{"v":42}}"#),
        ("PW7", "plugin_to_browser", "state", r#"{"type":"state","snapshot":{"level":42}}"#),
        ("PW8", "plugin_to_browser", "error", r#"{"type":"error","code":"not_found"}"#),
        ("PW9", "plugin_to_browser", "pong", r#"{"type":"pong"}"#),
    ];

    let mut vectors = Vec::new();

    for (id, direction, mtype, payload) in msgs {
        let signing_message = format!("{device_id}:{TIMESTAMP}:{payload}");
        let sig = sk.sign(signing_message.as_bytes());
        vectors.push(serde_json::json!({
            "id": id,
            "direction": direction,
            "message_type": mtype,
            "device_id": device_id,
            "timestamp": TIMESTAMP,
            "payload": payload,
            "signing_message": signing_message,
            "signature": hex(&sig.to_bytes()),
            "expect": "accept",
        }));
    }

    // Reject cases (R2-WEB §4.2: failing messages are silently dropped).
    let good_payload = r#"{"type":"ping"}"#;
    let good_msg = format!("{device_id}:{TIMESTAMP}:{good_payload}");
    let good_sig = sk.sign(good_msg.as_bytes());

    // PW10 — tampered signature (last byte flipped) MUST NOT verify.
    let mut tampered = good_sig.to_bytes();
    tampered[63] ^= 0x01;
    vectors.push(serde_json::json!({
        "id": "PW10",
        "direction": "browser_to_plugin",
        "message_type": "ping",
        "device_id": device_id,
        "timestamp": TIMESTAMP,
        "payload": good_payload,
        "signing_message": good_msg,
        "signature": hex(&tampered),
        "expect": "reject",
        "reason": "Ed25519 verification fails against device_id (tampered signature).",
    }));

    // PW11 — stale timestamp (>60s) MUST be rejected before signature check.
    let stale_ts = TIMESTAMP - 120;
    let stale_msg = format!("{device_id}:{stale_ts}:{good_payload}");
    let stale_sig = sk.sign(stale_msg.as_bytes()); // validly signed, but stale
    vectors.push(serde_json::json!({
        "id": "PW11",
        "direction": "browser_to_plugin",
        "message_type": "ping",
        "device_id": device_id,
        "timestamp": stale_ts,
        "payload": good_payload,
        "signing_message": stale_msg,
        "signature": hex(&stale_sig.to_bytes()),
        "expect": "reject",
        "reason": "timestamp outside the ±60s replay window (R2-WEB §4.2).",
    }));

    let doc = serde_json::json!({
        "spec": "R2-WEB",
        "version": "0.3",
        "description": "R2-WEB §4.2/§4.5 WebSocket message-auth conformance vectors. \
            Each message is a JSON envelope {device_id,timestamp,signature,payload} with \
            signature = Ed25519-Sign(DEV_SK, device_id:timestamp:payload) (Ed25519, NOT \
            HMAC — §4.2 v0.3). GENERATED from ed25519-dalek via \
            examples/gen_plugin_web_vectors.rs — do not hand-edit. Spec-driven (r2-hive's \
            §4.2 impl is a future v0.2 deliverable); TEST-ONLY key.",
        "device_key_seed_hex": hex(&seed),
        "vectors": vectors,
    });

    let out: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..", "..", "..", "r2-specifications", "testing", "test-vectors",
        "r2-plugin-web-vectors.json",
    ]
    .iter()
    .collect();

    let json = serde_json::to_string_pretty(&doc).expect("serialize");
    fs::write(&out, json + "\n").unwrap_or_else(|e| panic!("write {out:?}: {e}"));
    println!("wrote {} plugin-web vectors to {}", doc["vectors"].as_array().unwrap().len(), out.display());
}
