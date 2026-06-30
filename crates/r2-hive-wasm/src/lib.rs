//! # r2-hive-wasm — the wasm platform layer of the one-codebase hive.
//!
//! R2-HIVE north-star: ONE hive codebase everywhere = the no_std `r2-hive-core`
//! crates + a thin per-platform host. This crate is that host for the **browser**:
//! it wraps [`r2_hive_core::sync_host::route_inbound_sync`] — the SAME current-TN
//! routing core the Linux host and the ESP32-S3 firmware run — and exposes it to
//! JavaScript via `wasm-bindgen`, so a browser bench/sim drives REAL current-TN
//! routing (not a re-implementation).
//!
//! It is deliberately tiny: a [`WasmHive`] owns a `RouteEngine` + self-id; JS feeds
//! it inbound R2-WIRE frames (`route_frame`) and reads back the forwarding decision
//! plus every frame the engine would relay (captured per-medium). No tokio/sockets:
//! the sim IS the network — it moves the captured outbound frames between hive
//! instances itself.
//!
//! This crate is EXCLUDED from the r2-hive workspace (it is wasm/std + wasm-bindgen)
//! so it never touches host CI. Build with:
//!   cargo build -p r2-hive-wasm --target wasm32-unknown-unknown --release
//!   wasm-bindgen target/wasm32-unknown-unknown/release/r2_hive_wasm.wasm \
//!     --out-dir pkg --target web

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use r2_engine::queue::QueuedEvent;
use r2_engine::EventBus;
use r2_hive_core::ensemble::{HbSentant, TICK_HASH};
use r2_hive_core::sync_host::{
    provisional_hive_id, route_inbound_sync, InboundFrame, SyncRouteOutcome, SyncTransport,
    TransportAddr,
};
use r2_route::engine::RouteEngine;
use r2_route::transport::Transport as TransportKind;

/// R2-TRANSPORT §2.2 medium ids — the `arrival_kind` / `sends[].kind` wire codes.
fn kind_from_u8(k: u8) -> TransportKind {
    match k {
        0 => TransportKind::Ble,
        1 => TransportKind::Wifi,
        2 => TransportKind::Lora,
        3 => TransportKind::Internet,
        4 => TransportKind::Usb,
        5 => TransportKind::EspNow,
        _ => TransportKind::Udp,
    }
}

/// A sim-side transport: it never receives (the sim injects via `route_frame`); it
/// only CAPTURES what the engine decides to send, so JS can move it on-wire itself.
/// Mirror of `sync_host`'s test `StubTransport`.
struct CaptureTransport {
    kind: TransportKind,
    sent: RefCell<Vec<(u32, Vec<u8>)>>,
}

impl SyncTransport for CaptureTransport {
    fn kind(&self) -> TransportKind {
        self.kind
    }
    fn send(&self, target: u32, frame: &[u8]) -> Result<(), ()> {
        self.sent.borrow_mut().push((target, frame.to_vec()));
        Ok(())
    }
    fn poll_recv(&self) -> Option<InboundFrame> {
        None
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn outcome_tag(o: &SyncRouteOutcome) -> (&'static str, i64) {
    match o {
        SyncRouteOutcome::NotR2Wire => ("NotR2Wire", -1),
        SyncRouteOutcome::Dropped => ("Dropped", -1),
        SyncRouteOutcome::DeliverOnly => ("DeliverOnly", -1),
        SyncRouteOutcome::Directed { sent } => ("Directed", *sent as i64),
        SyncRouteOutcome::Flooded { sent } => ("Flooded", *sent as i64),
    }
}

/// One hive node in the browser sim — the UNIFIED hive: the real routing engine
/// (`route_frame`) PLUS the basic-ensemble runtime (`tick`/`deliver_event` over the
/// `r2_engine` EventBus), the SAME ensemble the DFR1195 firmware runs.
#[wasm_bindgen]
pub struct WasmHive {
    engine: RouteEngine<64, 64, 64>,
    bus: EventBus,
    self_hive_id: u32,
}

#[wasm_bindgen]
impl WasmHive {
    /// New hive node with the given canonical hive id (R2-WIRE §8.2). Boots the
    /// basic ensemble (heartbeat sentant) on its EventBus.
    #[wasm_bindgen(constructor)]
    pub fn new(self_hive_id: u32) -> WasmHive {
        let mut bus = EventBus::new();
        bus.register_sentant(Box::new(HbSentant::new(self_hive_id)));
        bus.init_all();
        WasmHive {
            engine: RouteEngine::new(),
            bus,
            self_hive_id,
        }
    }

    /// This node's hive id.
    #[wasm_bindgen(getter)]
    pub fn hive_id(&self) -> u32 {
        self.self_hive_id
    }

    /// Route one inbound R2-WIRE extended frame through the REAL current-TN core.
    ///
    /// - `source_hive`: immediate sender's hive id (0 = derive from route-stack).
    /// - `arrival_kind`: R2-TRANSPORT §2.2 medium id the frame arrived on.
    /// - `frame`: complete R2-WIRE extended bytes (optional trailing 32B HMAC ok).
    /// - `now`: monotonic seconds (sim clock). `dice`: spray draw in [0,1).
    ///
    /// Returns JSON: `{"outcome":"Flooded","sent":2,"sends":[{"kind":2,
    /// "target":123,"frame":"<hex>"}]}`. `sends` are the frames this node would
    /// transmit — the sim delivers them to neighbour `WasmHive`s itself.
    #[wasm_bindgen]
    pub fn route_frame(
        &mut self,
        source_hive: u32,
        arrival_kind: u8,
        frame: &[u8],
        now: u32,
        dice: f32,
    ) -> String {
        // One capture transport per medium so a Directed/Flood decision on ANY
        // kind is recorded (send_via_kind matches kind() to the chosen hop).
        let transports: Vec<CaptureTransport> = (0u8..=6)
            .map(|k| CaptureTransport {
                kind: kind_from_u8(k),
                sent: RefCell::new(Vec::new()),
            })
            .collect();
        let refs: Vec<&dyn SyncTransport> =
            transports.iter().map(|t| t as &dyn SyncTransport).collect();

        let outcome = route_inbound_sync(
            &mut self.engine,
            self.self_hive_id,
            &refs,
            source_hive,
            kind_from_u8(arrival_kind),
            frame,
            now,
            dice,
        );

        let (tag, sent) = outcome_tag(&outcome);

        let mut sends = String::new();
        let mut first = true;
        for t in &transports {
            let kind_code = t.kind() as u8;
            for (target, bytes) in t.sent.borrow().iter() {
                if !first {
                    sends.push(',');
                }
                first = false;
                sends.push_str(&format!(
                    "{{\"kind\":{kind_code},\"target\":{target},\"frame\":\"{}\"}}",
                    hex(bytes)
                ));
            }
        }

        format!("{{\"outcome\":\"{tag}\",\"sent\":{sent},\"sends\":[{sends}]}}")
    }

    /// Build a Heartbeat frame ORIGINATED by this node — origin = self in the route
    /// stack, payload = self hive_id (BE32, the firmware HB wire form). `seq` is the
    /// msg_id (dedup discriminator; pass a per-node counter or the sim tick). The sim
    /// feeds these bytes to neighbours' `route_frame` so each node floods its OWN HB
    /// (realistic per-node origin), not a fixed fixture. Returns raw R2-WIRE bytes.
    #[wasm_bindgen]
    pub fn build_heartbeat(&self, seq: u32) -> Vec<u8> {
        encode_frame(
            self.self_hive_id,
            0, // target_hive 0 = broadcast
            0, // target_group 0 = no TG gate in the sim
            r2_wire::MsgType::Heartbeat,
            0, // event_hash: HB carries none
            &self.self_hive_id.to_be_bytes(),
            8, // ttl
            3, // k (flood fan-out)
            seq,
        )
    }

    /// Build a generic Event frame from this node to `target_hive` (0 = broadcast),
    /// carrying `payload`, discriminated by `event_hash`. `seq` = msg_id. Origin =
    /// self in the route stack. Returns raw R2-WIRE bytes (empty on encode error).
    #[wasm_bindgen]
    pub fn build_frame(&self, target_hive: u32, event_hash: u32, payload: &[u8], seq: u32) -> Vec<u8> {
        encode_frame(
            self.self_hive_id,
            target_hive,
            0,
            r2_wire::MsgType::Event,
            event_hash,
            payload,
            8,
            3,
            seq,
        )
    }

    /// Drive the basic ensemble one TICK: inject a host tick → run the EventBus →
    /// return the frames the node's sentants want to BROADCAST, each a built R2-WIRE
    /// frame (hex). The sim floods these to neighbours (their `deliver_event` +
    /// `route_frame`) — so a wasm node originates its heartbeat via the SAME sentant
    /// the firmware runs. Returns JSON `{"frames":["<hex>",…]}`.
    #[wasm_bindgen]
    pub fn tick(&mut self, seq: u32) -> String {
        self.bus
            .enqueue(QueuedEvent::new(TICK_HASH, 0xFF, false, seq as u16, &[]));
        self.bus.tick();
        self.bus.poll_plugins();
        let outbound = self.bus.drain_outbound();
        let mut frames = String::new();
        let mut first = true;
        for ev in &outbound {
            let frame = encode_frame(
                self.self_hive_id,
                0, // broadcast
                0,
                r2_wire::MsgType::Event,
                ev.hash,
                ev.payload(),
                8,
                3,
                ev.msg_id as u32,
            );
            if frame.is_empty() {
                continue;
            }
            if !first {
                frames.push(',');
            }
            first = false;
            frames.push_str(&format!("\"{}\"", hex(&frame)));
        }
        format!("{{\"frames\":[{frames}]}}")
    }

    /// Deliver an inbound R2-WIRE frame to this node's ENSEMBLE (decode → bus event)
    /// so its sentants observe peers' heartbeats/events. This is the application
    /// layer; `route_frame` is the relay/transport layer. Returns the delivered
    /// event_hash (0 if the frame didn't decode).
    #[wasm_bindgen]
    pub fn deliver_event(&mut self, frame: &[u8]) -> u32 {
        match r2_wire::extended::decode_extended(frame) {
            Ok(m) => {
                let h = m.header.event_hash;
                self.bus.enqueue(QueuedEvent::new(
                    h,
                    0xFF,
                    true,
                    m.header.msg_id as u16,
                    m.payload,
                ));
                self.bus.tick();
                h
            }
            Err(_) => 0,
        }
    }
}

/// Encode one R2-WIRE extended frame (origin in the route stack) — the same
/// `encode_extended` the firmware uses, so sim traffic is wire-identical.
#[allow(clippy::too_many_arguments)]
fn encode_frame(
    origin: u32,
    target_hive: u32,
    target_group: u32,
    msg_type: r2_wire::MsgType,
    event_hash: u32,
    payload: &[u8],
    ttl: u8,
    k: u8,
    msg_id: u32,
) -> Vec<u8> {
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, ExtendedRouteStack, Flags};
    let msg = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type,
            flags: Flags { has_route: true, ..Flags::default() },
            ttl,
            k,
            msg_id,
            event_hash,
            payload_len: payload.len() as u32,
            target_group,
            target_hive,
        },
        route: Some(ExtendedRouteStack::with_origin(origin)),
        payload,
        hmac_tag: None,
    };
    let mut buf = [0u8; 512];
    match encode_extended(&msg, &mut buf) {
        Ok(n) => buf[..n].to_vec(),
        Err(_) => Vec::new(),
    }
}

/// Provisional hive id (FNV-1a of canonical transport address) for an unknown
/// advertiser — R2-WIRE §8.2 / R2-TRANSPORT §2.1.3. `mac` is 6 bytes.
#[wasm_bindgen]
pub fn provisional_id_mac(mac: &[u8]) -> u32 {
    let mut m = [0u8; 6];
    let n = mac.len().min(6);
    m[..n].copy_from_slice(&mac[..n]);
    provisional_hive_id(&TransportAddr::Mac(m))
}

/// Crate version string — lets the sim assert it loaded the current-TN build.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real R2-WIRE extended frame builder (mirrors r2-hive-core's sync_host test).
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
    fn encode_helpers_roundtrip() {
        let a = WasmHive::new(0x0000_00AA);
        let hb = a.build_heartbeat(0x10);
        assert!(!hb.is_empty(), "heartbeat encoded");
        // A different node routes A's HB — must parse as R2-WIRE (not NotR2Wire).
        let mut b = WasmHive::new(0x0000_00BB);
        let out = b.route_frame(0, 5, &hb, 1, 0.5);
        assert!(!out.contains("NotR2Wire"), "HB must parse: {out}");
        // Generic Event frame to a target also parses.
        let ev = a.build_frame(0x0000_00CC, 0xAABB_CCDD, b"hi", 0x11);
        assert!(!ev.is_empty(), "event encoded");
        let out2 = b.route_frame(0, 5, &ev, 2, 0.5);
        assert!(!out2.contains("NotR2Wire"), "event must parse: {out2}");
    }

    #[test]
    fn ensemble_tick_emits_heartbeat_to_peer() {
        // Node A's ensemble TICK → a broadcast heartbeat frame.
        let mut a = WasmHive::new(0x0000_00AA);
        let out = a.tick(1);
        assert!(out.contains("\"frames\":[\""), "tick emits a frame: {out}");
        let hexframe = out.split('"').nth(3).unwrap_or("");
        assert!(!hexframe.is_empty(), "frame hex present");
        let bytes: Vec<u8> = (0..hexframe.len() / 2)
            .map(|i| u8::from_str_radix(&hexframe[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        // Node B's ENSEMBLE receives A's heartbeat (decode → bus event).
        let mut b = WasmHive::new(0x0000_00BB);
        let h = b.deliver_event(&bytes);
        assert_eq!(h, r2_hive_core::ensemble::HEARTBEAT_HASH, "B got A's HB");
    }

    #[test]
    fn garbage_is_not_r2wire_json() {
        let mut hive = WasmHive::new(0xCAFE);
        let out = hive.route_frame(0xBEEF, 1, b"nope", 1, 0.0);
        assert!(out.contains("\"outcome\":\"NotR2Wire\""), "{out}");
        assert!(out.contains("\"sends\":[]"), "{out}");
    }

    #[test]
    fn positive_relay_populates_sends_json() {
        let target = 0x0000_00AA;
        let mut hive = WasmHive::new(0x0000_00FF);
        // Round 1: hear a frame FROM `target` on Wifi(=1) so the engine learns it
        // as a reachable neighbour (immediate_source observation).
        let learn = ext_frame(target, 0x0000_0001, 5, 3, 0x1000);
        let _ = hive.route_frame(target, 1, &learn, 100, 0.5);
        // Round 2: a frame addressed TO `target` must now relay to it, and the
        // sends-JSON must carry the target + a non-empty hex frame.
        let frame = ext_frame(0x0000_00BB, target, 5, 3, 0x1234);
        let out = hive.route_frame(0x0000_00BB, 1, &frame, 200, 0.5);
        assert!(
            out.contains("\"outcome\":\"Directed\"") || out.contains("\"outcome\":\"Flooded\""),
            "expected a relay decision, got {out}"
        );
        assert!(out.contains(&format!("\"target\":{target}")), "no relay to target: {out}");
        assert!(!out.contains("\"sends\":[]"), "sends must be populated: {out}");
    }
}
