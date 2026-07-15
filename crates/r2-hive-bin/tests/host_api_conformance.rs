//! Conformance test that loads the canonical
//! `r2-host-api-vectors.json` from the specifications repo and replays
//! each vector against r2-hive's encode/decode paths.
//!
//! This closes recommendation #1 from `docs/CONFORMANCE.md` — the
//! behaviour was already tested via `mgmt_integration`, but the
//! traceability gap (we re-encoded the same shapes rather than
//! replaying the canonical fixtures) is now closed.
//!
//! The replay is structural, not stateful:
//!
//! 1. Every vector's `frame_hex` decodes cleanly via
//!    `r2_wire::decode_extended`.
//! 2. The decoded `event_hash` matches `r2_hash(event_class)`.
//! 3. Every UDS frame equals `len_be32(frame_hex) || frame_hex`.
//! 4. Every payload_hex round-trips through CBOR — decoding the bytes
//!    via `r2_cbor::Decoder` reaches the end without error.
//! 5. For `app_to_hive` vectors that name an event class r2-hive
//!    dispatches, sending the frame through `handle_frame` does NOT
//!    return `unknown_event` — i.e. the dispatch wiring is intact.
//!
//! This isn't a full behavioural conformance — that would require
//! standing up state matching each vector's preconditions — but it
//! catches regressions in the wire format, hash table, and dispatch
//! switch all at once.

use serde::Deserialize;

use r2_cbor::{Decoder, Item};
use r2_fnv::r2_hash;
use r2_hive::hive::HiveState;
use r2_hive::mgmt::api::handle_frame;
use r2_hive::mgmt::state::DaemonState;

// VENDORED copy (read-only) of specs' canonical vector — see tests/vectors/_SYNC.md.
// Was a `../../../../r2-specifications/…` sibling-repo include_str! (compile-time),
// which made the whole workspace test build non-hermetic and failed hosted CI; now
// in-tree so the suite builds standalone (fresh clone, CI, open-source consumer).
const VECTORS_JSON: &str = include_str!("vectors/r2-host-api-vectors.json");

#[derive(Debug, Deserialize)]
struct VectorFile {
    spec: String,
    vectors: Vec<Vector>,
}

#[derive(Debug, Deserialize)]
struct Vector {
    id: String,
    direction: String,
    event_class: String,
    event_hash: String,
    payload_hex: String,
    frame_hex: String,
    frame_length: usize,
    uds_frame_hex: String,
    uds_frame_length: usize,
}

fn parse_hex_u32(s: &str) -> u32 {
    let s = s.trim_start_matches("0x");
    u32::from_str_radix(s, 16).expect("hex u32")
}

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex byte"))
        .collect()
}

fn load_vectors() -> VectorFile {
    serde_json::from_str(VECTORS_JSON).expect("vectors json parses")
}

#[test]
fn fixture_loads_and_is_r2_host_api() {
    let f = load_vectors();
    assert_eq!(f.spec, "R2-HOST-API");
    // Floor: don't let upstream silently drop vectors below current
    // count. Update this number when intentionally adding to the
    // fixture.
    assert!(
        f.vectors.len() >= 28,
        "vector count regressed: {}",
        f.vectors.len()
    );
}

#[test]
fn every_frame_decodes_via_r2_wire() {
    let f = load_vectors();
    for v in &f.vectors {
        let bytes = hex(&v.frame_hex);
        assert_eq!(
            bytes.len(),
            v.frame_length,
            "{}: frame_hex length {} != declared {}",
            v.id,
            bytes.len(),
            v.frame_length
        );
        r2_wire::decode_extended(&bytes)
            .unwrap_or_else(|e| panic!("{}: decode_extended failed: {e:?}", v.id));
    }
}

#[test]
fn every_event_hash_matches_event_class() {
    let f = load_vectors();
    for v in &f.vectors {
        let expected = parse_hex_u32(&v.event_hash);
        let computed = r2_hash(&v.event_class)
            .unwrap_or_else(|_| panic!("{}: event_class not valid for FNV", v.id));
        assert_eq!(
            computed, expected,
            "{}: hash of {:?} = 0x{:08X}, expected 0x{:08X}",
            v.id, v.event_class, computed, expected
        );

        let frame = hex(&v.frame_hex);
        let parsed = r2_wire::decode_extended(&frame).expect("decode");
        assert_eq!(
            parsed.header.event_hash, expected,
            "{}: frame event_hash mismatch",
            v.id
        );
    }
}

#[test]
fn every_uds_frame_is_length_prefixed_frame() {
    let f = load_vectors();
    for v in &f.vectors {
        let frame = hex(&v.frame_hex);
        let uds = hex(&v.uds_frame_hex);
        assert_eq!(
            uds.len(),
            v.uds_frame_length,
            "{}: uds_frame_hex length {} != declared {}",
            v.id,
            uds.len(),
            v.uds_frame_length
        );
        assert!(uds.len() >= 4, "{}: uds frame shorter than length prefix", v.id);
        let prefix_len = u32::from_be_bytes([uds[0], uds[1], uds[2], uds[3]]) as usize;
        assert_eq!(
            prefix_len, frame.len(),
            "{}: prefix says {} bytes, frame is {}",
            v.id, prefix_len, frame.len()
        );
        assert_eq!(
            &uds[4..],
            frame.as_slice(),
            "{}: prefix-stripped UDS != WS frame_hex",
            v.id
        );
    }
}

#[test]
fn every_payload_decodes_cleanly_as_cbor() {
    let f = load_vectors();
    for v in &f.vectors {
        let payload = hex(&v.payload_hex);
        if payload.is_empty() {
            continue;
        }
        let mut dec = Decoder::new(&payload);
        // Walk the top-level item and any nested map/array entries
        // until exhausted. Any decode error is a fixture/codec drift.
        match dec.next().unwrap_or_else(|e| panic!("{}: decode: {e:?}", v.id)) {
            Item::Map(n) => {
                for _ in 0..(n * 2) {
                    let _ = dec.next().unwrap_or_else(|e| panic!("{}: map item: {e:?}", v.id));
                }
            }
            Item::Array(n) => {
                for _ in 0..n {
                    let _ = dec.next().unwrap_or_else(|e| panic!("{}: arr item: {e:?}", v.id));
                }
            }
            _ => {}
        }
    }
}

/// Sending an `app_to_hive` vector through `handle_frame` MUST NOT
/// return `unknown_event` — that means the dispatch table no longer
/// covers an event class the spec lists. The actual response payload
/// depends on daemon state (which we don't fully reconstruct here);
/// stateful vectors are excluded from this assertion.
#[tokio::test]
async fn dispatch_switch_recognises_every_app_to_hive_event_class() {
    let f = load_vectors();
    let unknown_event_hash = r2_hash("r2.mgmt.event.error").unwrap();

    let hive = std::sync::Arc::new(HiveState::new(0xCAFEBABE, 64, 16));
    let daemon = DaemonState::new();
    daemon.attach_hive_state(hive.clone());

    for v in f.vectors.iter().filter(|v| v.direction == "app_to_hive") {
        let frame = hex(&v.frame_hex);
        let resp = handle_frame(&frame, &daemon).await;
        // Decode the response and check it isn't "unknown_event".
        let parsed = r2_wire::decode_extended(&resp).expect("decode resp");
        if parsed.header.event_hash == unknown_event_hash {
            // Inspect the error code — only the unknown_event code
            // counts as a dispatch failure. Other codes (not_in_tg,
            // peer_not_found, bad_frame, etc.) are legitimate responses
            // that the daemon emits when state preconditions aren't
            // met — we're not stateful enough here to satisfy them.
            if let Some(code) = extract_error_code(parsed.payload) {
                assert_ne!(
                    code, "unknown_event",
                    "{}: dispatch returned unknown_event for class {:?}",
                    v.id, v.event_class
                );
            }
        }
    }
}

fn extract_error_code(payload: &[u8]) -> Option<String> {
    let mut dec = Decoder::new(payload);
    let n = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..n {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(1) = k {
            if let Item::Text(s) = v {
                return Some(std::str::from_utf8(s).ok()?.to_string());
            }
        }
    }
    None
}
