//! Sync transport seam — the no_std/MCU host-loop side of the transport contract.
//!
//! **TRANSITIONAL local mirror** of the R2-TRANSPORT sync seam core+hive agreed
//! for the DFR1195 firmware (R2-DISCOVERY §5: no_std uses the sync interface, not
//! async r2-discovery §4). core will EXTEND r2-transport's `Transport` trait with
//! `poll_recv` (default `None`) plus `TransportAddr`/`InboundFrame`; when that
//! lands, delete this mirror and import `r2_transport::{…}`. Kept here meanwhile
//! (per the supervisor's "use your sync-stub") so the host-loop logic is built and
//! Linux-verified now.
//!
//! Host loop (each tick): for every sync driver, [`SyncTransport::poll_recv`] →
//! resolve `source_addr` → hive_id → feed `RouteEngine` → forwarding decision →
//! [`SyncTransport::send`]. The driver owns its RX buffer (filled by the embassy
//! RX task/IRQ); the host stays non-blocking. The routing core is
//! [`route_inbound_sync`].
//!
//! `no_std` + `alloc` (this is the `r2-hive-core` crate) — proves the routing
//! host-loop is genuinely platform-portable (MCU → cloud).

use alloc::string::String;
use alloc::vec::Vec;

use r2_route::engine::{ForwardAction, ForwardRequest, RouteEngine, Target};
use r2_route::neighbour::{MobilityClass, Observation};
use r2_route::transport::{QualitySample, Transport as TransportKind};
use r2_wire::extended::{decode_extended, prepare_relay_extended};

/// Transport-layer peer address, driver-stamped on inbound frames (the host, not
/// the dumb driver, resolves this to a canonical hive_id — see R2-DISCOVERY §3).
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
pub fn provisional_hive_id(addr: &TransportAddr) -> u32 {
    r2_fnv::fnv1a_32(addr.canonical().as_bytes())
}

/// The no_std radio-driver contract the MCU host loop polls each tick. One impl
/// per medium (WiFi-UDP / BLE5 / SX1262-LoRa); the host loop is driver-agnostic.
/// (Maps onto core's R2-TRANSPORT `Transport` + the agreed `poll_recv` extension.)
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
/// to a (provisional) source hive_id. The MCU host loop calls this each tick and
/// feeds the result to the `RouteEngine` (wiring is the next increment). Address-
/// map / trust resolution replaces the provisional id once that lands.
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

/// Send `frame` to `target` over the sync transport whose `kind()` matches.
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
pub fn route_inbound_sync(
    engine: &mut RouteEngine<64, 64, 64>,
    self_hive_id: u32,
    transports: &[&dyn SyncTransport],
    source_hive: u32,
    transport: TransportKind,
    frame: &[u8],
    now: u32,
    dice_roll: f32,
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

    // Dedup originator + immediate source (R2-WIRE §8.2/§8.3).
    let originator = match &msg.route {
        Some(r) if r.len > 0 => r.entries[0],
        _ => source_hive,
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
        msg_id: header.msg_id as u16,
        // R2-WIRE v0.4 dedup origin (originator hive) — sync_host has the real originator, so per-origin
        // multi-hop dedup works across paths (unlike the MCU firmware's route:None=0 placeholder).
        origin: originator,
        source_hop: (originator >> 16) as u16,
        ttl: header.ttl,
        k: header.k,
        destination,
        msg_type: header.msg_type,
        payload_len: frame.len(),
        relay_enabled: true,
        congested: false,
        dice_roll,
    });

    // Relay frames mutate the header per R2-WIRE §8.3/§8.4/§9.2 (TTL--, K split,
    // route-stack append) via r2-wire's prepare_relay_extended.
    let relay = || prepare_relay_extended(trimmed, self_hive_id, source_hive);

    match advice.action {
        ForwardAction::Drop(_) => SyncRouteOutcome::Dropped,
        ForwardAction::DeliverOnly => SyncRouteOutcome::DeliverOnly,
        ForwardAction::Directed(hop) => match relay() {
            Ok(bytes) => SyncRouteOutcome::Directed {
                sent: send_via_kind(transports, hop.transport, hop.neighbour, &bytes),
            },
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

    fn ext_frame(target_hive: u32, ttl: u8, k: u8, msg_id: u32) -> Vec<u8> {
        use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};
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
        let mut buf = vec![0u8; 64];
        let n = encode_extended(&msg, &mut buf).expect("encode");
        buf.truncate(n);
        buf
    }

    #[test]
    fn route_garbage_is_not_r2wire() {
        let mut engine = RouteEngine::<64, 64, 64>::new();
        let stub = StubTransport::new(TransportKind::Wifi, vec![]);
        let out = route_inbound_sync(
            &mut engine, 0xCAFE, &[&stub], 0xBEEF, TransportKind::Wifi, b"nope", 1, 0.0,
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
        let frame = ext_frame(target, 5, 3, 0x1234);
        let out = route_inbound_sync(
            &mut engine, 0x0000_00FF, &[&stub], 0x0000_00BB, TransportKind::Wifi, &frame, 200, 0.5,
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
}
