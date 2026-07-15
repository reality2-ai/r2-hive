//! Golden compact-frame emitter — a byte-exact R2-USB DATA record for cross-repo
//! decode verification (handed to android to validate `decode_compact` OFFLINE,
//! independent of live LoRa capture drops).
//!
//! WHY: android's live `dd bs=1` captures byte-DROP the XIAO USB egress (measured
//! 27–30B where the true frame is a fixed 31B), so a naive decode hit
//! `InvalidRouteLen` — inconclusive (capture tooling, not necessarily the decoder).
//! This emits ONE canonical `[payload_len u16 LE][compact frame]` straight from the
//! CANONICAL `r2_wire::encode_compact`, so a round-trip `decode_compact` is a clean
//! byte-exact reference with zero capture in the loop.
//!
//! The fields reproduce the STRUCTURE android saw on air: byte0=0x06
//! (ver0 | Event | route+hmac), byte1=0x53 (ttl5/k3), event_hash/target reused
//! from their capture. The 8-byte HMAC is arbitrary — `decode_compact` only SLICES
//! the trailing tag; HMAC *verification* is `verify_compact`, a separate step.
//!
//! Run: `cargo run -p r2-hive-bin --example gen_golden_compact_frame`

use r2_wire::{encode_compact, CompactHeader, CompactMessage, CompactRouteStack, Flags, MsgType};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    // Deterministic fields matching the structure android decoded on air.
    let header = CompactHeader {
        version: 0,
        msg_type: MsgType::Event, // type 0 — the "DATA" frame android saw (byte0>>6==0)
        flags: Flags {
            has_route: true, // R bit → route stack present (byte0 bit2)
            has_hmac: true,  // H bit → 8-byte trailing tag (byte0 bit1)
            mcu_origin: false,
        },
        ttl: 5,               // byte1 high nibble
        k: 3,                 // byte1 low nibble  → byte1 = 0x53
        msg_id: 0x0001,       // dedup counter (bytes 2..4, BE)
        event_hash: 0x64cedbf3, // reused from android's capture (bytes 4..8, BE)
        target: 0x05fe0701,   // reused from android's capture (bytes 8..12, BE)
    };

    // Single-entry route stack: [len=1][entry BE] = 3 bytes.
    let mut route = CompactRouteStack::new();
    route.len = 1;
    route.entries[0] = 0x1234; // origin hive id (upper-16 of FNV32), test value

    // 8-byte payload so the full record is a fixed 31B (== the clean length android
    // expected before capture drops): 12 header + 3 route + 8 payload + 8 hmac = 31.
    let payload: [u8; 8] = [0xa1, 0x00, 0x18, 0xea, 0xa1, 0x01, 0x18, 0x2a];
    let hmac_tag: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    let msg = CompactMessage {
        header,
        route: Some(route),
        payload: &payload,
        hmac_tag: Some(hmac_tag),
    };

    let mut buf = [0u8; 256];
    let n = encode_compact(&msg, &mut buf).expect("encode golden compact frame");
    let frame = &buf[..n];

    // R2-USB DATA record = [payload_len u16 LE][payload=frame].
    let mut record = Vec::with_capacity(2 + n);
    record.extend_from_slice(&(n as u16).to_le_bytes());
    record.extend_from_slice(frame);

    // Round-trip: prove decode_compact accepts our own encoder's bytes.
    let back = r2_wire::decode_compact(frame).expect("decode_compact round-trips golden frame");
    assert_eq!(back.header.event_hash, 0x64cedbf3);
    assert_eq!(back.route.map(|r| r.len), Some(1));
    assert_eq!(back.payload, &payload);
    assert_eq!(back.hmac_tag, Some(hmac_tag));

    println!("compact frame ({} bytes):        {}", n, hex(frame));
    println!("R2-USB DATA record ({} bytes):   {}", record.len(), hex(&record));
    println!("round-trip decode_compact:       OK (event_hash/route/payload/hmac match)");
}
