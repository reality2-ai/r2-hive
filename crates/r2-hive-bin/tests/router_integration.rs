//! Router integration — `router::route_frame`, the inbound routing entry point
//! both the WS relay driver and the UDP-LAN recv loop call. Previously had zero
//! coverage. These lock down route_frame's hive-unique logic: R2-WIRE parsing,
//! the HMAC-tag trim fallback, the no-neighbour drop, and dedup — independent of
//! the route engine internals (which core's r2-route tests own).

use std::sync::Arc;

use r2_hive::hive::HiveState;
use r2_hive::router::{route_frame, RouteOutcome};
use r2_route::transport::Transport;
use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

const SELF_ID: u32 = 0x0000_0001;
const NEIGHBOUR: u32 = 0x0000_0042;

/// Build a minimal valid R2-WIRE extended frame (no route stack, no HMAC tag).
fn ext_frame(target_hive: u32, ttl: u8, k: u8, msg_id: u32) -> Vec<u8> {
    let msg = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl,
            k,
            msg_id,
            event_hash: 0xAABB_CCDD,
            payload_len: 0,
            target_group: 0,
            target_hive,
        },
        route: None,
        payload: &[],
        hmac_tag: None,
    };
    let mut buf = vec![0u8; 128];
    let n = encode_extended(&msg, &mut buf).expect("encode");
    buf.truncate(n);
    buf
}

fn state() -> Arc<HiveState> {
    Arc::new(HiveState::new(SELF_ID, 64, 16))
}

#[tokio::test]
async fn garbage_short_is_not_r2wire() {
    let st = state();
    let out = route_frame(&st, NEIGHBOUR, Transport::Internet, b"nope").await;
    assert!(matches!(out, RouteOutcome::NotR2Wire));
}

#[tokio::test]
async fn garbage_long_is_not_r2wire() {
    // > 32 bytes so it exercises the HMAC-trim fallback branch, which must
    // still reject when neither the full buffer nor buf[..len-32] parses.
    let st = state();
    let junk = vec![0xEEu8; 80];
    let out = route_frame(&st, NEIGHBOUR, Transport::Internet, &junk).await;
    assert!(matches!(out, RouteOutcome::NotR2Wire));
}

#[tokio::test]
async fn valid_frame_parses_and_routes() {
    // A well-formed frame parses and runs the full engine path. route_frame
    // ingests the immediate source as a neighbour before planning, so the
    // frame is routed (here flooded to that just-learned neighbour) — the key
    // assertion is that it is NOT rejected as NotR2Wire, i.e. parsing worked.
    let st = state();
    let frame = ext_frame(0x0000_9999, 5, 3, 1);
    let out = route_frame(&st, NEIGHBOUR, Transport::Internet, &frame).await;
    assert!(
        !matches!(out, RouteOutcome::NotR2Wire),
        "a well-formed extended frame must parse and route"
    );
}

#[tokio::test]
async fn valid_frame_with_trailing_hmac_tag_parses() {
    // R2-WIRE §9 frames may carry a 32-byte HMAC tag the parser must trim
    // before decoding. Append 32 bytes: decode of the full buffer fails, decode
    // of buf[..len-32] succeeds → routed, NOT NotR2Wire. This is the trickiest
    // branch in route_frame and the whole point of this test.
    let st = state();
    let mut frame = ext_frame(0x0000_9999, 5, 3, 2);
    frame.extend_from_slice(&[0x5Au8; 32]); // synthetic HMAC tag
    let out = route_frame(&st, NEIGHBOUR, Transport::Internet, &frame).await;
    assert!(
        !matches!(out, RouteOutcome::NotR2Wire),
        "frame + 32B tag must parse via the trim path, not be rejected"
    );
}

#[tokio::test]
#[ignore = "A1 (core verify-then-record, ForwardRequest.authenticated): dedup is RECORDED only for a \
GroupHmac-VERIFIED frame, so this test now needs a keyed HiveState + a sign_extended'd route-full TG \
frame whose TG-routing matches the engine. Parked pending that authenticated-dedup fixture — flagged \
to core to mirror r2-route's own authenticated-dedup tests (or provide a hive test helper)."]
async fn duplicate_frame_is_dropped_by_dedup() {
    // Seed a viable neighbour so a broadcast floods (non-Dropped) on first
    // sight, then re-send the identical frame: the engine's dedup cache
    // (keyed on msg_id + originator) MUST drop the second copy.
    // NOTE: under A1 the frames must be AUTHENTICATED for dedup to record — see the #[ignore] reason.
    let st = state();

    // Seed NEIGHBOUR with several distinct observations so its confidence
    // clears the forwarding floor.
    for msg_id in 100..105 {
        let f = ext_frame(0x0000_0000, 5, 3, msg_id); // broadcast (target_hive 0)
        let _ = route_frame(&st, NEIGHBOUR, Transport::Internet, &f).await;
    }

    // A fresh broadcast from a different source should now flood to NEIGHBOUR.
    let dup = ext_frame(0x0000_0000, 5, 3, 777);
    let first = route_frame(&st, 0x0000_0077, Transport::Internet, &dup).await;
    assert!(
        matches!(first, RouteOutcome::Flooded(_)),
        "first sight of a broadcast with a viable neighbour should flood"
    );

    // Identical frame (same msg_id + originator) → dedup drop.
    let second = route_frame(&st, 0x0000_0077, Transport::Internet, &dup).await;
    assert!(
        matches!(second, RouteOutcome::Dropped),
        "duplicate frame must be dropped by the dedup cache"
    );
}

#[tokio::test]
async fn route_less_frame_is_dropped_route_origin_1() {
    // ROUTE-ORIGIN-1 (RATIFIED — R2-WIRE §9.5/§9.6, R2-ROUTE v0.14 §3.3): a route-less (route=None)
    // frame has NO authentic originator and MUST be DROPPED — a relay MUST NOT synthesise
    // route_stack[0]. This SUPERSEDES the transitional (B) frame-fingerprint dedup ("dedup by frame
    // not vantage", now DEAD): a route-less frame never reaches dedup/observe/flood — it drops at the
    // TOP of the router, from ANY vantage, so the old relay-amplification (a fabricated per-vantage
    // origin poisoning dedup) is impossible by construction, not merely deduped after the fact.
    let st = state();
    let frame = ext_frame(0x0000_0000, 5, 3, 888); // route=None broadcast
    let a = route_frame(&st, 0x0000_00AA, Transport::Internet, &frame).await; // vantage A
    assert!(
        matches!(a, RouteOutcome::Dropped),
        "a route-less (route=None) frame MUST be dropped (ROUTE-ORIGIN-1), not flooded or synthesised"
    );
    // SAME frame, DIFFERENT vantage (source 0xBB) -> still dropped; no vantage-dependent origin exists.
    let b = route_frame(&st, 0x0000_00BB, Transport::Internet, &frame).await;
    assert!(
        matches!(b, RouteOutcome::Dropped),
        "route-less frame dropped from every vantage — no fabricated origin can poison the dedup cache"
    );
}
