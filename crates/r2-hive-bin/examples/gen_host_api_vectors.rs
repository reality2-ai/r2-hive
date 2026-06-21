//! Generator for `r2-host-api-vectors.json` (R2-HOST-API v0.1 conformance
//! fixtures). Emits byte-exact vectors from the CANONICAL encoders so the
//! fixture can never drift from r2-hive's wire path:
//!
//! - frame  = `r2_hive::mgmt::api::build_response_frame_with_event` →
//!   `r2_wire::encode_extended` (EXTENDED R2-WIRE frame, 22-byte header)
//! - event_hash = `r2_fnv::r2_hash(event_class)`
//! - payload = `r2_cbor::Encoder` map `{0: correlation_id}`
//! - uds_frame = `len_be32(frame) || frame` (R2-HOST-API §2 UDS framing)
//!
//! Run: `cargo run --example gen_host_api_vectors`
//! Writes to `r2-specifications/testing/test-vectors/r2-host-api-vectors.json`.
//! specs independently verifies the 5 invariants before it lands canonical.
//!
//! `direction`:
//! - `app_to_hive` — request classes (R2-HOST-API §3.1/§4). Per R2-HOST-API §6,
//!   platform-gating does NOT change direction: `r2.mgmt.usb.*` are `app_to_hive`
//!   requests on every platform and the fixture MUST label them so, even though
//!   their handler dispatch is `cfg(target_os = "linux")`.
//! - `hive_to_app` — responses, deliveries, and the error class.

use std::fs;
use std::path::PathBuf;

use r2_cbor::Encoder;
use r2_fnv::r2_hash;
use r2_hive::mgmt::api::build_response_frame_with_event;

/// Classes r2-hive dispatches as application/management requests (R2-HOST-API
/// §3 r2.api.* + R2-TG-TOOL §5 r2.mgmt.* reachable with hive state attached).
const APP_TO_HIVE: &[&str] = &[
    "r2.api.peer.list",
    "r2.api.peer.query",
    "r2.api.tg.current",
    "r2.api.event.send",
    "r2.api.event.subscribe",
    "r2.api.event.unsubscribe",
    "r2.api.cap.query",
    "r2.api.service.advertise",
    "r2.api.service.retract",
    "r2.mgmt.daemon.status",
    "r2.mgmt.identity.status",
    "r2.mgmt.web.provision",
    "r2.mgmt.ensemble.deploy",
    "r2.mgmt.ensemble.list",
    "r2.mgmt.ensemble.info",
    "r2.mgmt.ensemble.stop",
    "r2.mgmt.ensemble.remove",
    // usb.* are app_to_hive requests on every platform (R2-HOST-API §6 —
    // platform-gating does not change direction). Handler dispatch is
    // cfg(target_os = "linux"); the conformance test runs on Linux.
    "r2.mgmt.usb.list",
    "r2.mgmt.usb.prepare",
    "r2.mgmt.usb.confirm",
    "r2.mgmt.usb.abort",
    "r2.mgmt.usb.unpair",
];

/// Responses, hive→app deliveries, and the error class (§6).
const HIVE_TO_APP: &[&str] = &[
    "r2.api.event.delivery",
    "r2.mgmt.event.error",
    "r2.api.peer.list",
    "r2.api.tg.current",
    "r2.api.cap.query",
    "r2.api.peer.query",
];

/// Representative valid CBOR payload: map `{0: correlation_id}` (key 0 is the
/// correlation id, R2-HOST-API §6). Built via the canonical r2_cbor encoder.
fn cbor_payload(correlation_id: u64) -> Vec<u8> {
    let mut buf = [0u8; 32];
    let mut enc = Encoder::new(&mut buf);
    enc.map(1).expect("map header");
    enc.uint(0).expect("key");
    enc.uint(correlation_id).expect("value");
    enc.as_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn main() {
    let mut vectors = Vec::new();
    let mut idx: u64 = 0;

    for (direction, classes) in [("app_to_hive", APP_TO_HIVE), ("hive_to_app", HIVE_TO_APP)] {
        for class in classes {
            idx += 1;
            let payload = cbor_payload(idx);
            let frame = build_response_frame_with_event(class, &payload);
            let mut uds = (frame.len() as u32).to_be_bytes().to_vec();
            uds.extend_from_slice(&frame);
            let event_hash = r2_hash(class).expect("event class canonicalises");

            vectors.push(serde_json::json!({
                "id": format!("HA{idx:02}"),
                "direction": direction,
                "event_class": class,
                "event_hash": format!("0x{event_hash:08X}"),
                "payload_hex": hex(&payload),
                "frame_hex": hex(&frame),
                "frame_length": frame.len(),
                "uds_frame_hex": hex(&uds),
                "uds_frame_length": uds.len(),
            }));
        }
    }

    let doc = serde_json::json!({
        "spec": "R2-HOST-API",
        "version": "0.2",
        "description": "Conformance vectors for the R2-HOST-API r2.api.* / r2.mgmt.* \
            event surface. Each vector is a byte-exact EXTENDED R2-WIRE frame \
            (r2_wire::encode_extended) with event_hash == r2_fnv::r2_hash(event_class), \
            a representative CBOR payload, and the UDS framing len_be32(frame)||frame \
            (R2-HOST-API §2). GENERATED from r2-hive's canonical encoders via \
            examples/gen_host_api_vectors.rs — do not hand-edit. [v0.2, 2026-06-22: \
            ensemble verbs synced to R2-TG-TOOL §5.3 canonical set — HA13 load->deploy \
            (0xAFA2A20A), HA17 reset->remove (0xE1336311); FNV-verified, payload ordinals \
            unchanged. r2-hive MUST sync examples/gen_host_api_vectors.rs ensemble verb \
            names so a regen reproduces these bytes.]",
        "vectors": vectors,
    });

    let out: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..", "..", "..", "r2-specifications", "testing", "test-vectors",
        "r2-host-api-vectors.json",
    ]
    .iter()
    .collect();

    let json = serde_json::to_string_pretty(&doc).expect("serialize");
    fs::write(&out, json + "\n").unwrap_or_else(|e| panic!("write {out:?}: {e}"));
    println!("wrote {} vectors to {}", vectors.len(), out.display());
}
