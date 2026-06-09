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
//! RX task/IRQ); the host stays non-blocking. RouteEngine wiring is the next step.
//!
//! Std `String`/`Vec` here (bin crate); becomes `alloc` when this moves into the
//! `r2-hive-core` no_std crate.

use r2_route::transport::Transport as TransportKind;

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
}
