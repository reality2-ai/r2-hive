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
use r2_hive_core::ensemble::{
    HbSentant, MemSink, OtaConfig, OtaSentant, OCM_HASH, ODT_HASH, OST_HASH, PROGRESS_HASH,
    TICK_HASH,
};
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

    /// New OTA-CAPABLE node: the basic ensemble PLUS the OTA plugin+sentant (the pure
    /// increment-3 form), so this node can RECEIVE a signed image over the mesh and
    /// run the real R2-UPDATE verify-before-write. `tg_pk` = the 32-byte trust-group
    /// public key it accepts updates signed by (TG_SK-direct). Use this for receiver
    /// nodes in the OTA demo; the updater can be a plain `new()` node (it just builds
    /// frames via `start_ota`). FlashSink = in-memory (no flash in the browser).
    #[wasm_bindgen(js_name = withOta)]
    pub fn with_ota(self_hive_id: u32, tg_pk: &[u8]) -> WasmHive {
        let mut bus = EventBus::new();
        bus.register_sentant(Box::new(HbSentant::new(self_hive_id)));
        let mut pk = [0u8; 32];
        let n = tg_pk.len().min(32);
        pk[..n].copy_from_slice(&tg_pk[..n]);
        let cfg = OtaConfig {
            class_hash: 0,   // target_class 0 in the demo packages = any
            carrier_hash: 0, // target_carrier 0 = any
            tg_prefix: [0u8; 8],
            device_id_prefix: [0u8; 8],
            battery_pct: 100,
            tg_pk: pk,
        };
        bus.register_sentant(Box::new(OtaSentant::new(cfg, MemSink::new())));
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

    /// Run the EventBus one full cycle and return the frames the node's sentants want
    /// to BROADCAST, each a built R2-WIRE frame (hex). Two tick passes around
    /// poll_plugins so a plugin's progress (e.g. OTA) surfaces as an outbound event
    /// in the same call. Returns JSON `{"frames":["<hex>",…]}`.
    fn run_bus_cycle(&mut self) -> String {
        let mut outbound: Vec<QueuedEvent> = Vec::new();
        self.bus.tick();
        outbound.extend(self.bus.drain_outbound());
        // A plugin may buffer several progress events per step (OCM → VERIFIED then
        // APPLIED); poll_plugins drains ONE per call, so loop until nothing new
        // surfaces (bounded — at most a few per step).
        for _ in 0..8 {
            self.bus.poll_plugins();
            self.bus.tick();
            let out = self.bus.drain_outbound();
            if out.is_empty() {
                break;
            }
            outbound.extend(out);
        }
        let mut frames = String::new();
        let mut first = true;
        // STRUCTURED progress (composer renders this directly — no compact-frame parsing):
        // every r2.update.progress event decoded to {phase,bytes_done,bytes_total,reason}.
        let mut progs = String::new();
        let mut pfirst = true;
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
            if !frame.is_empty() {
                if !first {
                    frames.push(',');
                }
                first = false;
                frames.push_str(&format!("\"{}\"", hex(&frame)));
            }
            if ev.hash == PROGRESS_HASH {
                let p = ev.payload();
                if p.len() >= 10 {
                    let done = u32::from_be_bytes([p[1], p[2], p[3], p[4]]);
                    let total = u32::from_be_bytes([p[5], p[6], p[7], p[8]]);
                    if !pfirst {
                        progs.push(',');
                    }
                    pfirst = false;
                    progs.push_str(&format!(
                        "{{\"phase\":{},\"bytes_done\":{},\"bytes_total\":{},\"reason\":{}}}",
                        p[0], done, total, p[9]
                    ));
                }
            }
        }
        format!("{{\"frames\":[{frames}],\"progress\":[{progs}]}}")
    }

    /// Drive the basic ensemble one TICK: inject a host tick → run the EventBus →
    /// return the frames the node's sentants want to BROADCAST. The HB sentant emits
    /// one heartbeat per tick (a wasm node originates its HB via the SAME sentant the
    /// firmware runs). The sim floods these to neighbours (`deliver_event`).
    #[wasm_bindgen]
    pub fn tick(&mut self, seq: u32) -> String {
        self.bus
            .enqueue(QueuedEvent::new(TICK_HASH, 0xFF, false, seq as u16, &[]));
        self.run_bus_cycle()
    }

    /// Deliver an inbound R2-WIRE frame to this node's ENSEMBLE (decode → bus event)
    /// so its sentants observe peers' heartbeats and (if OTA-capable) verify an
    /// incoming OST/ODT/OCM step. This is the application layer; `route_frame` is the
    /// relay/transport layer. Returns JSON `{"frames":[…]}` = any frames this node
    /// then broadcasts (notably the OTA `r2.update.progress` events the bench renders).
    #[wasm_bindgen]
    pub fn deliver_event(&mut self, frame: &[u8]) -> String {
        if let Ok(m) = r2_wire::extended::decode_extended(frame) {
            self.bus.enqueue(QueuedEvent::new(
                m.header.event_hash,
                0xFF,
                true,
                m.header.msg_id as u16,
                m.payload,
            ));
        }
        self.run_bus_cycle()
    }

    /// UPDATER side: turn a signed R2-UPDATE package into the OST→ODT*→OCM frame
    /// sequence addressed to `target_hive`, ready to flood over the mesh. `pkg` =
    /// header(123) ++ payload ++ sig(64) (the R2-UPDATE package layout). Returns JSON
    /// `{"frames":["<hex>",…]}`. The receiver's `deliver_event` verifies each step;
    /// a bad image never `applies` (verify-before-write). chunk size 200 (= the
    /// push_ota_l2cap DEFAULT_CHUNK).
    #[wasm_bindgen(js_name = startOta)]
    pub fn start_ota(&self, target_hive: u32, pkg: &[u8]) -> String {
        use r2_update::{HEADER_LEN, SIG_LEN};
        if pkg.len() < HEADER_LEN + SIG_LEN {
            return String::from("{\"frames\":[]}");
        }
        let header = &pkg[..HEADER_LEN];
        let sig = &pkg[pkg.len() - SIG_LEN..];
        let payload = &pkg[HEADER_LEN..pkg.len() - SIG_LEN];

        let me = self.self_hive_id;
        let mut frames: Vec<String> = Vec::new();
        let mut push = |hash: u32, sdu: &[u8], seq: u32| {
            let f = encode_frame(
                me,
                target_hive,
                0,
                r2_wire::MsgType::Event,
                hash,
                sdu,
                8,
                1,
                seq,
            );
            if !f.is_empty() {
                frames.push(format!("\"{}\"", hex(&f)));
            }
        };
        // OST = header ++ sig (187B)
        let mut ost = Vec::with_capacity(HEADER_LEN + SIG_LEN);
        ost.extend_from_slice(header);
        ost.extend_from_slice(sig);
        push(OST_HASH, &ost, 0);
        // ODT = payload chunks
        for (i, chunk) in payload.chunks(200).enumerate() {
            push(ODT_HASH, chunk, (i as u32) + 1);
        }
        // OCM = commit marker
        push(OCM_HASH, &[], 0xFFFF_FFFF);
        format!("{{\"frames\":[{}]}}", frames.join(","))
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
        // A's tick frame is a real heartbeat (event_hash = HEARTBEAT_HASH).
        let m = r2_wire::extended::decode_extended(&bytes).expect("A's HB decodes");
        assert_eq!(m.header.event_hash, r2_hive_core::ensemble::HEARTBEAT_HASH);
        // Node B's ENSEMBLE accepts A's heartbeat without panic (returns valid JSON).
        let mut b = WasmHive::new(0x0000_00BB);
        let out = b.deliver_event(&bytes);
        assert!(out.starts_with("{\"frames\":["), "valid JSON: {out}");
    }

    /// On-demand: mint a signed R2-UPDATE test package + tg_pk for composer's live
    /// OTA demo. Run: `cargo test mint_ota_artifacts -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn mint_ota_artifacts() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};
        use std::io::Write;
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"R2-OTA-DEMO-IMAGE v2.0.0 ".repeat(40); // ~1000B
        let mut hh = Sha256::new();
        hh.update(&payload);
        let phash: [u8; 32] = hh.finalize().into();
        let mut header = [0u8; 123];
        header[0] = 2; // PACKAGE_VERSION
        header[41] = 0x01; // PT_FIRMWARE_FULL
        header[42..46].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        header[46..78].copy_from_slice(&phash);
        header[78..82].copy_from_slice(&1u32.to_be_bytes()); // seq 1
        header[90..122].copy_from_slice(&tg_pk);
        let sig = sk.sign(&header).to_bytes();
        let mut pkg = Vec::from(&header[..]);
        pkg.extend_from_slice(&payload); // header ++ payload ++ sig
        pkg.extend_from_slice(&sig);

        let dir = format!("{}/r2-staota-artifacts", std::env::var("HOME").unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::File::create(format!("{dir}/ota-test-pkg.bin"))
            .unwrap()
            .write_all(&pkg)
            .unwrap();
        let tg_hex: String = tg_pk.iter().map(|b| format!("{b:02x}")).collect();
        std::fs::write(format!("{dir}/ota-test-pkg.tg_pk.hex"), &tg_hex).unwrap();
        println!("MINTED pkg={} bytes payload={} tg_pk={}", pkg.len(), payload.len(), tg_hex);

        // 3rd reject-demo arm: a validly-SIGNED but WRONG-TYPE (DIFF 0x02) package —
        // must hit the A7/A8 type-confusion REJECT, never APPLIED. Same tg_pk.
        let mut dheader = header;
        dheader[41] = 0x02; // PT_FIRMWARE_DIFF (not FULL)
        let dsig = sk.sign(&dheader).to_bytes();
        let mut dpkg = Vec::from(&dheader[..]);
        dpkg.extend_from_slice(&payload);
        dpkg.extend_from_slice(&dsig);
        std::fs::File::create(format!("{dir}/ota-test-pkg-diff.bin"))
            .unwrap()
            .write_all(&dpkg)
            .unwrap();
        println!("MINTED diff pkg={} bytes (payload_type=0x02 → expect A7/A8 REJECT)", dpkg.len());
    }

    #[test]
    fn ota_over_wasm_mesh_e2e() {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};
        // Mint a signed R2-UPDATE package (header ++ payload ++ sig).
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"WASM-OTA-IMAGE-v2".repeat(20);
        let mut hh = Sha256::new();
        hh.update(&payload);
        let phash: [u8; 32] = hh.finalize().into();
        let mut header = [0u8; 123];
        header[0] = 2; // PACKAGE_VERSION
        header[41] = 0x01; // firmware-full
        header[42..46].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        header[46..78].copy_from_slice(&phash);
        header[78..82].copy_from_slice(&1u32.to_be_bytes()); // seq 1 > current 0
        header[90..122].copy_from_slice(&tg_pk);
        let sig = sk.sign(&header).to_bytes();
        let mut pkg = Vec::from(&header[..]);
        pkg.extend_from_slice(&payload);
        pkg.extend_from_slice(&sig);

        // Extract the hex frames from a {"frames":["..",..]} JSON string.
        fn frames_of(json: &str) -> Vec<Vec<u8>> {
            json.split('"')
                .filter(|s| s.len() >= 2 && s.len() % 2 == 0 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .map(|h| {
                    (0..h.len() / 2)
                        .map(|i| u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).unwrap())
                        .collect()
                })
                .collect()
        }

        // Updater builds the OST/ODT/OCM frames; receiver (OTA-capable) verifies them.
        let updater = WasmHive::new(0x0000_00AA);
        let ota_frames = frames_of(&updater.start_ota(0x0000_00BB, &pkg));
        assert!(ota_frames.len() >= 3, "OST+ODT+OCM: {}", ota_frames.len());

        let mut rx = WasmHive::with_ota(0x0000_00BB, &tg_pk);
        let mut applied = false;
        let ph_applied = r2_hive_core::ensemble::PH_APPLIED;
        let progress_hash = r2_hive_core::ensemble::PROGRESS_HASH;
        for f in &ota_frames {
            for pf in frames_of(&rx.deliver_event(f)) {
                if let Ok(m) = r2_wire::extended::decode_extended(&pf) {
                    if m.header.event_hash == progress_hash
                        && m.payload.first() == Some(&ph_applied)
                    {
                        applied = true;
                    }
                }
            }
        }
        assert!(applied, "receiver must reach APPLIED over the wasm mesh");
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
