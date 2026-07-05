//! # sync_host — the platform-portable routing host-loop (sync tier)
//!
//! ## Why this file exists
//!
//! The R2-HIVE north-star is ONE hive codebase everywhere: the same routing
//! behaviour on a Linux daemon, a browser wasm hive, and a 256 KiB
//! microcontroller. Async runtimes don't exist on the MCU tier, so the mesh
//! needs a **sync** expression of the inbound pipeline — poll a driver,
//! resolve the sender, drive the RouteEngine, execute the forwarding
//! decision — with zero platform dependencies. This file is that expression:
//! [`route_inbound_sync`] is the routing core, [`SyncTransport`] is the
//! driver contract it runs against, and everything here is `no_std` + alloc
//! so the identical code is Linux-testable and MCU-deployable. If wasm and
//! firmware ever route differently, the mesh partitions on behavioural
//! mismatch — this file existing once is the guard against that.
//!
//! **TRANSITIONAL local mirror note:** the seam types (`SyncTransport`,
//! `TransportAddr`, `InboundFrame`) mirror the R2-TRANSPORT sync extension
//! core+hive agreed (R2-DISCOVERY §5: no_std uses the sync interface, not
//! async r2-discovery §4). When core lands `poll_recv` on r2-transport's
//! `Transport` trait, delete the mirror and import `r2_transport::{…}`.
//!
//! ## How it interlinks (grep-verified)
//!
//! - **`r2-hive-wasm`** is the production caller: its `route_frame` wraps
//!   [`route_inbound_sync`] verbatim (same engine, same trail hooks), which
//!   is what makes the browser /proof hive a real refutation instrument
//!   rather than a simulation of one.
//! - **MCU firmware** (dfr1195/rak4630 branches) is the intended second
//!   caller once the io_task → dataplane wiring lands (hive ledger task
//!   #32); [`poll_inbound`] is the host-loop entry built for that wiring —
//!   no production caller yet, exercised by unit tests here.
//! - **Depends on** r2-route (`RouteEngine`, `TrailReinforcer`), r2-wire
//!   (`decode_extended`, `prepare_relay_extended`), r2-fnv (provisional
//!   ids). Holds NO keys and does NO crypto — §7.5.4 verification happens a
//!   layer up; the outcome arrives as the `congested`/`authenticated`
//!   inputs documented at their parameters.
//! - The Linux daemon does NOT use this path — its async twin lives in
//!   `r2-hive-bin/src/router.rs` (route_frame); the two implement the same
//!   canon and must not drift (each file head cross-references the other).
//!
//! ## Canon (r2-specifications)
//!
//! - R2-ROUTE §3 (forwarding), §3A (congestion), §4.3.4/§4.6.1 (trails +
//!   retrace) — `r2-specifications/specs/r2-core/R2-ROUTE.md`.
//! - R2-WIRE §8.2/§8.3 (dedup keying, TTL), §8.4/§8.4a (amplification
//!   caps), §9.2/§9.5/§9.6 (relay append, route-origin rules) —
//!   `r2-specifications/specs/r2-core/R2-WIRE.md`.
//! - R2-DISCOVERY §3.3 (provisional ids), §5 (sync tier) —
//!   `r2-specifications/specs/r2-core/R2-DISCOVERY.md`.
//! - R2-TRANSPORT §2.1.3 (canonical address form) —
//!   `r2-specifications/specs/r2-core/R2-TRANSPORT.md`.

use alloc::string::String;
use alloc::vec::Vec;

use r2_route::engine::{ForwardAction, ForwardRequest, RouteEngine, Target};
use r2_route::neighbour::{MobilityClass, Observation};
use r2_route::trail::TrailReinforcer;
use r2_route::transport::{QualitySample, Transport as TransportKind};
use r2_route::DropReason;
use r2_wire::extended::{decode_extended, prepare_relay_extended};

/// Transport-layer peer address, driver-stamped on inbound frames (the host, not
/// the dumb driver, resolves this to a canonical hive_id — see R2-DISCOVERY §3).
///
/// **Used-by:** [`InboundFrame`] (every inbound frame carries one),
/// [`provisional_hive_id`], and `r2-hive-wasm` (MAC-derived peer ids).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TransportAddr {
    /// BLE MAC.
    Mac([u8; 6]),
    /// LoRa node id.
    LoRaNode(u32),
    /// UDP source (WiFi-LAN).
    Udp { ip: [u8; 4], port: u16 },
}

impl TransportAddr {
    /// Canonical address string (R2-TRANSPORT §2.1.3 form) — the input to the
    /// provisional-id hash, and the key into the host's address→hive_id map.
    pub fn canonical(&self) -> String {
        match self {
            TransportAddr::Mac(m) => m
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(":"),
            TransportAddr::LoRaNode(n) => format!("lora:{n}"),
            TransportAddr::Udp { ip, port } => {
                format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port)
            }
        }
    }
}

/// One frame received from a sync driver: the transport source address (driver-
/// stamped) + owned R2-WIRE bytes (≤256B at this tier).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InboundFrame {
    /// Driver-stamped transport-layer source.
    pub source_addr: TransportAddr,
    /// Complete R2-WIRE frame bytes (compact or extended per the transport).
    pub data: Vec<u8>,
}

/// Provisional hive_id for an unknown advertiser — FNV-1a of the canonical
/// transport address (R2-WIRE §8.2 / R2-TRANSPORT §2.1.3), used until the trust
/// layer resolves the canonical id (R2-DISCOVERY §3.3).
///
/// **Used-by:** [`poll_inbound`] (tags each drained frame) and
/// `r2-hive-wasm::peer_id_from_mac` (JS-visible peer identity).
pub fn provisional_hive_id(addr: &TransportAddr) -> u32 {
    r2_fnv::fnv1a_32(addr.canonical().as_bytes())
}

/// The no_std radio-driver contract the MCU host loop polls each tick. One impl
/// per medium (WiFi-UDP / BLE5 / SX1262-LoRa); the host loop is driver-agnostic.
/// (Maps onto core's R2-TRANSPORT `Transport` + the agreed `poll_recv` extension.)
///
/// **Used-by:** [`route_inbound_sync`] / [`poll_inbound`] as `&dyn` sets;
/// implemented by `r2-hive-wasm`'s CaptureTransport (virtual mesh) and the
/// stub driver in this file's tests; MCU driver impls arrive with the
/// firmware io_task wiring (hive ledger task #32).
pub trait SyncTransport {
    /// Which transport this is.
    fn kind(&self) -> TransportKind;
    /// Send a complete R2-WIRE frame to `target` (0 = broadcast). Fire-and-forget:
    /// `Ok` = accepted for transmission, not delivered (R2-TRANSPORT).
    fn send(&self, target: u32, frame: &[u8]) -> Result<(), ()>;
    /// Non-blocking: the next received frame, if any. The driver owns the buffer.
    fn poll_recv(&self) -> Option<InboundFrame>;
}

/// Drain all pending inbound frames across a set of sync drivers, resolving each
/// to a (provisional) source hive_id. Address-map / trust resolution replaces
/// the provisional id once that lands.
///
/// **Used-by:** no production caller yet — this is the MCU host-loop entry
/// built for the firmware io_task wiring (hive ledger task #32); wasm drives
/// [`route_inbound_sync`] directly instead (its virtual mesh delivers frames
/// by call, not by poll). Exercised by the unit tests below; kept because it
/// is the designed tick entry for the pending consumer, not dead code.
pub fn poll_inbound(transports: &[&dyn SyncTransport]) -> Vec<(u32, InboundFrame)> {
    let mut out = Vec::new();
    for t in transports {
        while let Some(frame) = t.poll_recv() {
            let hive_id = provisional_hive_id(&frame.source_addr);
            out.push((hive_id, frame));
        }
    }
    out
}

/// Outcome of routing one inbound frame through the engine on the sync host loop.
///
/// **Used-by:** returned by [`route_inbound_sync`]; `r2-hive-wasm` maps each
/// variant into its JSON telemetry (`outcome_tag`) so JS scenes render the
/// real engine decision, and this file's tests assert on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SyncRouteOutcome {
    /// Frame did not parse as R2-WIRE extended.
    NotR2Wire,
    /// Engine dropped it (TTL exhausted, dedup hit, relay disabled, …).
    Dropped,
    /// Local delivery only — this hive is the destination.
    DeliverOnly,
    /// Relayed to one directed neighbour; `sent` = whether the send was accepted.
    Directed { sent: bool },
    /// Flooded to a set of neighbours; `sent` = how many sends were accepted.
    Flooded { sent: usize },
}

/// Send `frame` to `target` over the sync transport whose `kind()` matches
/// (first match wins; a missing driver for the engine's chosen kind means
/// the send silently fails and the outcome reports `sent: false`).
///
/// **Used-by:** [`route_inbound_sync`] Directed/Flood arms only.
fn send_via_kind(
    transports: &[&dyn SyncTransport],
    kind: TransportKind,
    target: u32,
    frame: &[u8],
) -> bool {
    transports
        .iter()
        .find(|t| t.kind() == kind)
        .is_some_and(|t| t.send(target, frame).is_ok())
}

/// Route one inbound frame through the engine and execute the forwarding decision
/// over the sync transports — the routing-only MCU host-loop core (R2-DISCOVERY §5
/// sync tier; host-centralised resolution, conformant per specs).
///
/// Mirrors the host's `router::route_frame` decision logic, but is **sync**, drives
/// the engine directly (no async lock), and omits host-only behaviour (mgmt-API
/// subscribers, ensemble dispatch, trust-group broadcast, WS compat) that a
/// routing+transport MCU hive does not run. `now` (monotonic seconds) and
/// `dice_roll` (spray draw) are caller-provided (from the Platform).
///
/// **Dependencies:** the engine's `plan_forward` (all R2-ROUTE §3 policy
/// lives THERE, none here), r2-wire decode + `prepare_relay_extended`
/// (header mutation per §8.3/§8.4/§9.2), the reinforcer's `on_received` /
/// `note_forwarded` hooks (§4.3.4 — no trail policy in this glue either).
///
/// **Used-by:** `r2-hive-wasm::route_frame` (the browser /proof hive — its
/// only routing path), and this file's invariant tests; the MCU firmware
/// adopts it with the task #32 io_task wiring.
pub fn route_inbound_sync(
    engine: &mut RouteEngine<64, 64, 64>,
    self_hive_id: u32,
    transports: &[&dyn SyncTransport],
    source_hive: u32,
    transport: TransportKind,
    frame: &[u8],
    now: u32,
    dice_roll: f32,
    // §3A: the caller's REAL congestion latch (drive it only through the core sensor —
    // DataPlane::observe_queue_occupancy; local authority only, never a wire field).
    congested: bool,
    reinforcer: &mut TrailReinforcer<256>,
) -> SyncRouteOutcome {
    // Parse R2-WIRE extended (frame may carry a trailing 32-byte HMAC tag).
    let trimmed = if decode_extended(frame).is_ok() {
        frame
    } else if frame.len() > 32 && decode_extended(&frame[..frame.len() - 32]).is_ok() {
        &frame[..frame.len() - 32]
    } else {
        return SyncRouteOutcome::NotR2Wire;
    };
    let msg = match decode_extended(trimmed) {
        Ok(m) => m,
        Err(_) => return SyncRouteOutcome::NotR2Wire,
    };
    let header = msg.header;

    // Dedup originator = route_stack[0] (R2-WIRE §8.2/§8.3). ROUTE-ORIGIN-1 (RATIFIED — R2-WIRE
    // §9.5/§9.6, R2-ROUTE v0.14 §3.3): a route-less frame has no authentic origin and MUST be DROPPED
    // here (pre-dedup, pre-neighbour-observe) — a relay MUST NOT synthesise route_stack[0]. The old
    // `_ => source_hive` fallback was the SYNC-1 vuln (vantage-dependent origin -> dedup poisoning).
    let originator = match &msg.route {
        Some(r) if r.len > 0 => r.entries[0],
        _ => return SyncRouteOutcome::Dropped,
    };
    let immediate_source = if source_hive != 0 {
        source_hive
    } else {
        match &msg.route {
            Some(r) if r.len > 0 => r.entries[(r.len - 1) as usize],
            _ => source_hive,
        }
    };

    // Learn the immediate neighbour (the peer we just heard from).
    engine.ingest_observation(Observation {
        hive_id: immediate_source,
        transport,
        timestamp: now,
        quality: QualitySample::Direct(0.9),
        rssi: None,
        mcu_origin: header.flags.mcu_origin,
        mobility: MobilityClass::Infrastructure,
    });

    let destination = if header.target_group != 0 {
        Target::from(header.target_group)
    } else {
        Target::from(header.target_hive)
    };

    let advice = engine.plan_forward(ForwardRequest {
        now,
        // §2.3B (r2-core consolidation bf1bf3b): ForwardRequest gained arrival_transport: Option<Transport>.
        // None = BEHAVIOUR-PRESERVING (engine skips the §2.3B arrival-reachability drop, engine.rs:492) — this is
        // a build-compat fix for the core API change, NOT a silent enablement of faked-distance on the host sync
        // tier. The arrival `transport` IS in scope here (see the Observation ingest above); whether the sync
        // tier should apply §2.3B virtual-reachability = a deliberate semantic decision (FLAG FOR CORE, like the
        // A1 `authenticated` note below), not a build-fix. Left None until that's ruled.
        arrival_transport: None,
        msg_id: header.msg_id, // full 32-bit dedup id (F3)
        // R2-WIRE v0.4 dedup origin (originator hive) — sync_host has the real originator, so per-origin
        // multi-hop dedup works across paths (unlike the MCU firmware's route:None=0 placeholder).
        origin: originator,
        source_hop: immediate_source, // the IMMEDIATE sender, to exclude the inbound peer (F2)
        // A1 RULED (core, §4.3.4 trail pass): the sync tier's frames are sim/local-origin — legitimately
        // trusted on this tier (no radio attacker inside the harness) — so `authenticated: true`: the
        // engine RECORDS dedup and a replayed (origin,msg_id) copy returns Drop(Duplicate). LOAD-BEARING
        // for the trail invariant (a) below: reinforcement fires at most once per copy-set per dedup
        // window (a replayed copy re-reinforcing forever is a black-hole-builder primitive).
        authenticated: true,
        ttl: header.ttl,
        k: header.k,
        destination,
        msg_type: header.msg_type,
        // §8.4a (R2-WIRE v0.27 / R2-ROUTE §3.4 v0.53): the broadcast/flood amplification cap keys on the REAL
        // payload size, NOT the whole frame (was frame.len() — over-counted by the ~header+route+hmac overhead,
        // which would wrongly OversizeBroadcast-drop a broadcast whose payload is under BROADCAST_PAYLOAD_MAX=512).
        payload_len: msg.payload.len(),
        relay_enabled: true,
        congested,
        dice_roll,
    });

    // §4.3.4 trail reinforcement — POST-dedup-accept ONLY (core invariant (a),
    // security-load-bearing: a replayed copy must never re-reinforce, else an
    // attacker/chatty bridge pumps (origin,via) confidence unboundedly = a
    // black-hole-builder primitive). With `authenticated: true` above the engine
    // RECORDS dedup, so a duplicate copy comes back Drop(Duplicate) — skip it;
    // every other outcome is the accepted first copy. The hook does weak
    // (forward ⇒ record_indirect toward origin) / strong (reply-marker for a msg
    // we forwarded ⇒ record_delivery toward the replier) internally — no trail
    // policy lives in this glue (core invariant (d)).
    // v0.64 (core 1cc8cd1): on_received now takes my_hive for §4.6.1 retrace — strong credit goes
    // to the RECORDED successor from the forward ring, never the radio immediate-sender (sender
    // only SELECTS among fan-out records). Caller contract unchanged: carried frames only, and
    // this post-dedup sync ingest is carried by construction.
    if !matches!(advice.action, ForwardAction::Drop(DropReason::Duplicate)) {
        // is_reply rides the frame TYPE field (3d43838, codex HIGH): a marker-shaped
        // payload inside an Event/GroupMgmt frame must NOT spoof a retraced reply
        // (strong-reinforce + consume a forwarded record = trail-poisoning lever).
        reinforcer.on_received(
            engine,
            originator,
            msg.payload,
            immediate_source,
            self_hive_id,
            header.msg_type == r2_wire::MsgType::Reply,
            now,
        );
    }

    // Relay frames mutate the header per R2-WIRE §8.3/§8.4/§9.2 (TTL--, K split,
    // route-stack append) via r2-wire's prepare_relay_extended.
    let relay = || prepare_relay_extended(trimmed, self_hive_id, source_hive);

    match advice.action {
        ForwardAction::Drop(_) => SyncRouteOutcome::Dropped,
        ForwardAction::DeliverOnly => SyncRouteOutcome::DeliverOnly,
        ForwardAction::Directed(hop) => match relay() {
            Ok(bytes) => {
                let sent = send_via_kind(transports, hop.transport, hop.neighbour, &bytes);
                if sent {
                    // §4.3.4: relaying puts us on this msg's forward path — note
                    // (origin, msg_id, successor) so the returning reply strong-
                    // reinforces the RECORDED successor (v0.64 §4.6.1: this copy's
                    // actual next-hop, never the reply's radio sender).
                    reinforcer.note_forwarded(originator, header.msg_id, hop.neighbour);
                }
                SyncRouteOutcome::Directed { sent }
            }
            Err(_) => SyncRouteOutcome::Dropped,
        },
        ForwardAction::Flood(hops) => match relay() {
            Ok(bytes) => {
                let mut sent = 0;
                for hop in hops.iter() {
                    if hop.neighbour != source_hive
                        && send_via_kind(transports, hop.transport, hop.neighbour, &bytes)
                    {
                        sent += 1;
                        // v0.64 fan-out rule: one note PER FORWARDED COPY, each with
                        // its own recorded successor — the returning reply's sender
                        // then SELECTS which record earns the strong credit.
                        reinforcer.note_forwarded(originator, header.msg_id, hop.neighbour);
                    }
                }
                SyncRouteOutcome::Flooded { sent }
            }
            Err(_) => SyncRouteOutcome::Dropped,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// Linux sync-stub driver: a canned inbound queue + a sent-frame log.
    struct StubTransport {
        kind: TransportKind,
        inbound: RefCell<VecDeque<InboundFrame>>,
        sent: RefCell<Vec<(u32, Vec<u8>)>>,
    }
    impl StubTransport {
        fn new(kind: TransportKind, inbound: Vec<InboundFrame>) -> Self {
            Self {
                kind,
                inbound: RefCell::new(inbound.into()),
                sent: RefCell::new(Vec::new()),
            }
        }
    }
    impl SyncTransport for StubTransport {
        fn kind(&self) -> TransportKind {
            self.kind
        }
        fn send(&self, target: u32, frame: &[u8]) -> Result<(), ()> {
            self.sent.borrow_mut().push((target, frame.to_vec()));
            Ok(())
        }
        fn poll_recv(&self) -> Option<InboundFrame> {
            self.inbound.borrow_mut().pop_front()
        }
    }

    #[test]
    fn poll_recv_drains_then_empties() {
        let stub = StubTransport::new(
            TransportKind::Lora,
            vec![InboundFrame {
                source_addr: TransportAddr::LoRaNode(7),
                data: vec![1, 2, 3],
            }],
        );
        let t: &dyn SyncTransport = &stub;
        assert_eq!(t.poll_recv().unwrap().data, vec![1, 2, 3]);
        assert!(t.poll_recv().is_none());
    }

    #[test]
    fn provisional_id_is_stable_and_addr_specific() {
        let a = TransportAddr::Udp { ip: [192, 168, 1, 5], port: 21042 };
        let b = TransportAddr::Udp { ip: [192, 168, 1, 6], port: 21042 };
        assert_eq!(provisional_hive_id(&a), provisional_hive_id(&a));
        assert_ne!(provisional_hive_id(&a), provisional_hive_id(&b));
    }

    #[test]
    fn poll_inbound_drains_all_drivers_with_resolved_ids() {
        let udp = StubTransport::new(
            TransportKind::Wifi,
            vec![InboundFrame {
                source_addr: TransportAddr::Udp { ip: [10, 0, 0, 2], port: 21042 },
                data: vec![0xAA],
            }],
        );
        let lora = StubTransport::new(
            TransportKind::Lora,
            vec![InboundFrame { source_addr: TransportAddr::LoRaNode(42), data: vec![0xBB] }],
        );
        let drained = poll_inbound(&[&udp, &lora]);
        assert_eq!(drained.len(), 2);
        // Each frame is tagged with the provisional id of its source address.
        assert_eq!(drained[0].0, provisional_hive_id(&TransportAddr::Udp { ip: [10, 0, 0, 2], port: 21042 }));
        assert_eq!(drained[1].0, provisional_hive_id(&TransportAddr::LoRaNode(42)));
    }

    #[test]
    fn send_records_target_and_frame() {
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        stub.send(0xABCD, b"wire").unwrap();
        assert_eq!(stub.sent.borrow()[0], (0xABCD, b"wire".to_vec()));
    }

    fn ext_frame(origin_hive: u32, target_hive: u32, ttl: u8, k: u8, msg_id: u32) -> Vec<u8> {
        use r2_wire::{
            encode_extended, ExtendedHeader, ExtendedMessage, ExtendedRouteStack, Flags, MsgType,
        };
        let msg = ExtendedMessage {
            header: ExtendedHeader {
                version: 0,
                msg_type: MsgType::Event,
                flags: Flags {
                    has_route: true,
                    ..Flags::default()
                },
                ttl,
                k,
                msg_id,
                event_hash: 0xAABB_CCDD,
                payload_len: 0,
                target_group: 0,
                target_hive,
            },
            route: Some(ExtendedRouteStack::with_origin(origin_hive)),
            payload: &[],
            hmac_tag: None,
        };
        let mut buf = vec![0u8; 64];
        let n = encode_extended(&msg, &mut buf).expect("encode");
        buf.truncate(n);
        buf
    }

    #[test]
    fn route_garbage_is_not_r2wire() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        let mut r = TrailReinforcer::<256>::new();
        let out = route_inbound_sync(
            &mut engine, 0xCAFE, &[&stub], 0xBEEF, TransportKind::Wifi, b"nope", 1, 0.0, false, &mut r,
        );
        assert_eq!(out, SyncRouteOutcome::NotR2Wire);
    }

    #[test]
    fn route_relays_to_known_neighbour() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let target = 0x0000_00AA;
        // Give the engine a route to `target` on Wifi.
        engine.ingest_observation(Observation {
            hive_id: target,
            transport: TransportKind::Wifi,
            timestamp: 100,
            quality: QualitySample::Direct(0.95),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Infrastructure,
        });
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        let source = 0x0000_00BB;
        let frame = ext_frame(source, target, 5, 3, 0x1234);
        let mut r = TrailReinforcer::<256>::new();
        let out = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], source, TransportKind::Wifi, &frame, 200, 0.5,
            false, &mut r,
        );
        // The engine reached a relay decision and the frame went to `target` over
        // the matching sync transport (the whole point of the host-loop wiring).
        assert!(
            matches!(out, SyncRouteOutcome::Directed { .. } | SyncRouteOutcome::Flooded { .. }),
            "expected a relay decision, got {out:?}",
        );
        let sent = stub.sent.borrow();
        assert!(
            sent.iter().any(|(t, _)| *t == target),
            "expected a relay send to target, sent={sent:?}",
        );
    }

    #[test]
    fn route_respects_transport_allow_mask_before_sync_send() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let target = 0x0000_00AA;
        engine.ingest_observation(Observation {
            hive_id: target,
            transport: TransportKind::Lora,
            timestamp: 100,
            quality: QualitySample::Direct(1.0),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Infrastructure,
        });
        engine.ingest_observation(Observation {
            hive_id: target,
            transport: TransportKind::Wifi,
            timestamp: 100,
            quality: QualitySample::Direct(0.4),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Infrastructure,
        });
        engine.set_transport_allow_mask_bits(TransportKind::Wifi.bit());

        let wifi = StubTransport::new(TransportKind::Wifi, vec![]);
        let lora = StubTransport::new(TransportKind::Lora, vec![]);
        let source = 0x0000_00BB;
        let frame = ext_frame(source, target, 5, 3, 0x1235);
        let mut r = TrailReinforcer::<256>::new();
        let out = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&wifi, &lora], source, TransportKind::Wifi, &frame, 200, 0.5,
            false, &mut r,
        );

        assert!(
            matches!(
                out,
                SyncRouteOutcome::Directed { sent: true } | SyncRouteOutcome::Flooded { sent: 1 }
            ),
            "expected exactly one accepted relay over the allowed transport, got {out:?}"
        );
        assert!(
            wifi.sent.borrow().iter().any(|(t, _)| *t == target),
            "policy should leave Wifi as the only egress"
        );
        assert!(
            lora.sent.borrow().is_empty(),
            "masked LoRa must not be sent even though it scores better"
        );
    }

    #[test]
    fn route_drops_when_mask_removes_only_sync_candidate() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let target = 0x0000_00AA;
        engine.ingest_observation(Observation {
            hive_id: target,
            transport: TransportKind::Lora,
            timestamp: 100,
            quality: QualitySample::Direct(1.0),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Infrastructure,
        });
        engine.set_transport_allow_mask_bits(TransportKind::Wifi.bit());

        let wifi = StubTransport::new(TransportKind::Wifi, vec![]);
        let lora = StubTransport::new(TransportKind::Lora, vec![]);
        let source = 0x0000_00BB;
        let frame = ext_frame(source, target, 5, 3, 0x1236);
        let mut r = TrailReinforcer::<256>::new();
        let out = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&wifi, &lora], source, TransportKind::Wifi, &frame, 200, 0.5,
            false, &mut r,
        );

        assert_eq!(out, SyncRouteOutcome::Dropped);
        assert!(wifi.sent.borrow().is_empty());
        assert!(lora.sent.borrow().is_empty());
    }

    /// §4.3.4 invariant (a) — POST-dedup-accept only: the first copy of an
    /// (origin,msg_id) lays exactly one weak trail; a replayed duplicate copy
    /// must NOT reinforce again (replay-pumped confidence = a black-hole-builder
    /// primitive). With `authenticated: true` the engine records dedup, so the
    /// duplicate comes back Drop(Duplicate) and the hook is skipped.
    #[test]
    fn duplicate_copy_reinforces_at_most_once() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        let mut r = TrailReinforcer::<256>::new();
        let origin = 0x0000_00AA;
        let sender = 0x0000_00BB;
        // A second known neighbour so the broadcast has a viable flood hop (the
        // sender itself is excluded as the inbound peer).
        engine.ingest_observation(Observation {
            hive_id: 0x0000_00CC,
            transport: TransportKind::Wifi,
            timestamp: 90,
            quality: QualitySample::Direct(0.9),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Infrastructure,
        });
        // Broadcast (target 0) so the frame floods/deliver-onlys rather than needing a route.
        let frame = ext_frame(origin, 0, 5, 3, 0x0000_7001);

        let out1 = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], sender, TransportKind::Wifi, &frame, 100, 0.5,
            false, &mut r,
        );
        assert_ne!(out1, SyncRouteOutcome::Dropped, "first copy must be accepted");
        let c1 = engine
            .paths()
            .best_for(origin)
            .map(|p| p.confidence)
            .expect("weak trail toward origin after the first accepted copy");

        // Same (origin,msg_id) again — the replayed copy must dedup-drop and NOT re-reinforce.
        let out2 = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], sender, TransportKind::Wifi, &frame, 101, 0.5,
            false, &mut r,
        );
        assert_eq!(out2, SyncRouteOutcome::Dropped, "duplicate must dedup-drop");
        let c2 = engine
            .paths()
            .best_for(origin)
            .map(|p| p.confidence)
            .expect("trail entry persists");
        assert_eq!(c1, c2, "a replayed copy must not move confidence");
    }

    /// §4.3.4 strong trail: a node that RELAYED (origin,msg_id) — noted via the
    /// forward arms — receiving the retracing reply marker strong-reinforces
    /// toward the replier via the immediate sender (used-path-wins).
    #[test]
    fn reply_marker_strong_reinforces_through_forwarder() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        let mut r = TrailReinforcer::<256>::new();
        let origin = 0x0000_00AA; // request originator
        let replier = 0x0000_00DD; // destination that replies
        let upstream = 0x0000_00BB; // neighbour the request arrived from
        let downstream = 0x0000_00CC; // neighbour the reply arrives from
        let req_id = 0x0000_7002u32;

        // Request (origin → broadcast) arrives via upstream and floods on: the
        // Flood arm notes (origin, req_id) in the reinforcer.
        engine.ingest_observation(Observation {
            hive_id: downstream,
            transport: TransportKind::Wifi,
            timestamp: 90,
            quality: QualitySample::Direct(0.9),
            rssi: None,
            mcu_origin: false,
            mobility: MobilityClass::Infrastructure,
        });
        let req = ext_frame(origin, 0, 5, 3, req_id);
        let out = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], upstream, TransportKind::Wifi, &req, 100, 0.5,
            false, &mut r,
        );
        assert!(
            matches!(out, SyncRouteOutcome::Flooded { sent } if sent > 0),
            "request must flood on (we are a forwarder), got {out:?}"
        );

        // Reply frame: originated by the replier, payload = the §4.3.4 marker for
        // the noted (origin, req_id), arriving from `downstream`. Same construction
        // idiom as ext_frame (has_route + with_origin — a route-less frame would
        // ROUTE-ORIGIN-1 early-drop before the gate).
        let marker = r2_route::trail::reply_marker(origin, req_id);
        let reply = {
            use r2_wire::{
                encode_extended, ExtendedHeader, ExtendedMessage, ExtendedRouteStack, Flags,
                MsgType,
            };
            let msg = ExtendedMessage {
                header: ExtendedHeader {
                    version: 0,
                    // Reply-ness MUST ride the type field (3d43838): an Event-typed
                    // frame with a marker payload now yields weak evidence at most.
                    msg_type: MsgType::Reply,
                    flags: Flags {
                        has_route: true,
                        ..Flags::default()
                    },
                    ttl: 5,
                    k: 3,
                    msg_id: r2_route::trail::reply_msg_id_ext(req_id),
                    event_hash: 0x5EED_0001,
                    payload_len: marker.as_bytes().len() as u32,
                    target_group: 0,
                    target_hive: origin,
                },
                route: Some(ExtendedRouteStack::with_origin(replier)),
                payload: marker.as_bytes(),
                hmac_tag: None,
            };
            let mut buf = vec![0u8; 128];
            let n = encode_extended(&msg, &mut buf).expect("encode reply");
            buf.truncate(n);
            buf
        };
        let buf = reply;

        let _ = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], downstream, TransportKind::Wifi, &buf, 110, 0.5,
            false, &mut r,
        );

        // Strong trail: best path toward the REPLIER goes via the reply's sender.
        let p = engine
            .paths()
            .best_for(replier)
            .expect("strong trail toward the replier after the retracing reply");
        assert_eq!(p.next_hop, downstream, "reply retrace reinforces via its sender");
    }

    /// §4.3.4 weak trail is ONE-WAY: an accepted forward lays weak evidence
    /// toward its ORIGIN via the sender — and nothing toward the frame's dest
    /// (the black-hole guard lives inside trail.rs; this pins it end-to-end).
    #[test]
    fn weak_trail_is_toward_origin_only() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        let mut r = TrailReinforcer::<256>::new();
        let origin = 0x0000_00AA;
        let dest = 0x0000_00EE;
        let sender = 0x0000_00BB;
        let frame = ext_frame(origin, dest, 5, 3, 0x0000_7003);

        let _ = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], sender, TransportKind::Wifi, &frame, 100, 0.5,
            false, &mut r,
        );

        let toward_origin = engine.paths().best_for(origin);
        assert!(
            toward_origin.is_some_and(|p| p.next_hop == sender),
            "weak trail toward origin via the sender must exist"
        );
        assert!(
            engine.paths().best_for(dest).is_none(),
            "no strong-reinforce toward the frame's dest on a forward (black-hole guard)"
        );
    }
}
