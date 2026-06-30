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

/// One hive node in the browser sim — owns the real routing engine.
#[wasm_bindgen]
pub struct WasmHive {
    engine: RouteEngine<64, 64, 64>,
    self_hive_id: u32,
}

#[wasm_bindgen]
impl WasmHive {
    /// New hive node with the given canonical hive id (R2-WIRE §8.2).
    #[wasm_bindgen(constructor)]
    pub fn new(self_hive_id: u32) -> WasmHive {
        WasmHive {
            engine: RouteEngine::new(),
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
