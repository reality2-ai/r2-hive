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
        5 => TransportKind::Mesh, // R2-TRANSPORT v0.18: EspNow → Mesh (r2-core 78a31a8)
        _ => TransportKind::Udp,
    }
}

// ───────────────────── R2-TRANSPORT §2.7 transport-profile physics helpers ─────────────────────
// The carrier-independent physics the radio-SIM (composer's bench) AND the field share — one
// source of timing/physics truth (§2.7: "the simulator and the field share one source"). Exported
// from the wasm hive so composer's sim derives synthetic link-quality from the SAME functions the
// routing layer's reachability seed uses — no drift between sim and field. These are pure math (no
// transport-seam coupling); the WS/UDP socket BINDINGS + the shared TransportProfile struct
// (single-sourced in r2-transport) land with core's host-UDP binding.

/// §2.5 / R2-ROUTE §2.6 `rssi → quality`: RSSI (dBm) → link-quality in [0,1]. Canonical clamp
/// (§2.7 table): −50 dBm → 1.0 (excellent), −80 dBm → 0.0 (link-dead); linear between, saturating
/// outside. This is the reachability-strength metric the neighbour-table confidence seed reads.
#[wasm_bindgen]
pub fn quality_from_rssi(rssi_dbm: f32) -> f32 {
    const RSSI_FULL: f32 = -50.0; // → 1.0
    const RSSI_ZERO: f32 = -80.0; // → 0.0
    ((rssi_dbm - RSSI_ZERO) / (RSSI_FULL - RSSI_ZERO)).clamp(0.0, 1.0)
}

/// §2.2 medium ids → the canonical [`r2_transport::TransportId`] (single-source; same order as
/// `kind_from_u8`): 0 Ble / 1 Wifi / 2 Lora / 3 Internet / 4 Usb / 5 Mesh / 6 Udp.
fn transport_id_from_u8(k: u8) -> r2_transport::TransportId {
    use r2_transport::TransportId;
    match k {
        0 => TransportId::Ble,
        1 => TransportId::Wifi,
        2 => TransportId::Lora,
        3 => TransportId::Internet,
        4 => TransportId::Usb,
        5 => TransportId::Mesh,
        _ => TransportId::Udp,
    }
}

/// §2.7 `range → loss` — the CANONICAL model, single-sourced from core's r2-transport (v0.19), so the
/// sim + field share one physics table (no drift). LOG-DISTANCE path-loss (§2.7 RATIFIED shape):
/// `loss = clamp(PL_ref + 10·n·log10(max(range_units, 0.001) / d_ref), 0, 160)` dB, d_ref=1,
/// n=path_loss_exponent per transport. NEAR-FIELD is MODELLED: the floor is a numerical 0.001 (not d_ref),
/// so a sub-reference distance (d<1) yields LESS loss than PL_ref (down to the 0 clamp) — closer ⇒ stronger,
/// not a PL_ref plateau. `loss(d_ref=1) == PL_ref` (log10(1)=0). Range is EMERGENT:
/// `quality_from_rssi(tx_dbm − range_to_loss_db(t, r))` crosses 0 at the −80 dBm point = the transport's
/// range. VALUES PROVISIONAL, single-sourced from core (snapshot as of profile.rs sha256 76038e63 =
/// composer theater.html byte-for-byte: PL_ref 40 dB all RF; n = LoRa 1.5 / WiFi 2.35 / Mesh 2.85 / BLE 3.4;
/// IP transports n=0 ⇒ zero loss) pending Roy field-anchor — only the numbers move, the shape is final;
/// signature stable (d_ref internal). Code is truth; this doc-list is the snapshot.
#[wasm_bindgen]
pub fn range_to_loss_db(transport_id: u8, range_units: f32) -> f32 {
    r2_transport::profile::range_to_loss_db(transport_id_from_u8(transport_id), range_units)
}

/// The full canonical §2.7 [`r2_transport::TransportProfile`] for a transport, as JSON — the shared
/// param-set the routing layer + the radio-sim both read. Fields: max_payload (MTU), power_cost,
/// decay_lambda (λ, per-transport staleness; LoRa<WiFi<BLE), reference_path_loss_db + path_loss_exponent
/// (§2.7 v0.19 log-distance two-field schema), jitter_ms, congested_jitter_ms. Composer's sim reads THIS
/// (not hard-coded copies) so there is zero sim/field drift.
#[wasm_bindgen]
pub fn transport_profile(transport_id: u8) -> String {
    let p = r2_transport::TransportProfile::for_transport(transport_id_from_u8(transport_id));
    format!(
        "{{\"transport\":{},\"max_payload\":{},\"power_cost\":{},\"decay_lambda\":{},\"reference_path_loss_db\":{},\"path_loss_exponent\":{},\"jitter_ms\":[{},{}],\"congested_jitter_ms\":[{},{}]}}",
        transport_id,
        p.max_payload,
        p.power_cost,
        p.decay_lambda,
        p.reference_path_loss_db,
        p.path_loss_exponent,
        p.jitter_ms.0,
        p.jitter_ms.1,
        p.congested_jitter_ms.0,
        p.congested_jitter_ms.1,
    )
}

/// The frame's ORIGINATOR (route_stack[0], the ROUTE-ORIGIN-1 authentic origin), or 0 if the frame
/// is route-less / undecodable. A WS-mesh client uses this to DROP its own echo — a broadcast bearer
/// rebroadcasts a relayed copy back to the originator; since an unauthenticated frame is dedup-CHECKED
/// but not dedup-RECORDED (route_inbound_sync A1), the originator would otherwise re-relay its own
/// frame's echo (wasted bandwidth). `origin == self` ⇒ drop. (source_hive stays 0 at the call site:
/// route_inbound_sync derives the true immediate-sender from route_stack[last], so F2 exclusion is
/// already correct — the echo is an originator-reprocess artefact, not a source_hive bug.)
#[wasm_bindgen]
pub fn frame_origin(frame: &[u8]) -> u32 {
    match r2_wire::decode_extended(frame) {
        Ok(m) => m.route.and_then(|r| r.origin()).unwrap_or(0),
        Err(_) => 0,
    }
}

/// The frame's FULL reverse-trail — `route_stack[0..len]` = `[origin, hop1, …, immediate_sender]`
/// (R2-WIRE §9.2: origin is immutable at [0], each relay appends itself; max 8). Empty if the frame
/// is route-less / undecodable. This is the HOP-PATH read for a bench "directed-send" event
/// (delivered/dropped/hop-path) over the REAL primitives — no plugin/event-bus fork: the path is the
/// authentic route stack the routing core itself built, decoded the same way `frame_origin` reads [0].
/// `route_hops(f)[0] == frame_origin(f)` (the originator); `.at(-1)` = the last hop that forwarded it.
#[wasm_bindgen]
pub fn route_hops(frame: &[u8]) -> Vec<u32> {
    match r2_wire::decode_extended(frame) {
        Ok(m) => match m.route {
            Some(r) => r.entries[..(r.len as usize).min(r.entries.len())].to_vec(),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
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
    /// The trust-group symmetric HMAC key (hk) — the SAME `r2_trust::GroupHmac` the
    /// firmware signs/verifies with. `Some` = this hive is a TG MEMBER: it signs its
    /// egress (`build_*`/ensemble frames) and runs the real deliver-gate in
    /// `verify_frame`. `None` = the legacy TG-agnostic sim (no gate, frames unsigned,
    /// `target_group = 0`) so the pure-routing bench is unchanged.
    group_hmac: Option<r2_trust::GroupHmac>,
    /// The TG hash (firmware `my_tg_hash`) this member stamps into `target_group` and
    /// gates inbound `target_group` against. Meaningful only when `group_hmac` is set.
    tg_hash: u32,
    /// R2-TRUST §7.5.4 posture for an UNKEYED hive (`group_hmac: None`). Default `false` = FAIL-CLOSED
    /// (`verify_frame` returns `deliver:false`) — "default-OPEN is FORBIDDEN". A pure-routing / TG-agnostic
    /// bench opts in explicitly via `setUnkeyedOpen(true)` to restore the legacy deliver-everything sim.
    unkeyed_open: bool,
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
            group_hmac: None,
            tg_hash: 0,
            unkeyed_open: false, // §7.5.4 fail-closed default; setUnkeyedOpen(true) for the pure-routing sim
        }
    }

    /// New TG-MEMBER node: the basic ensemble PLUS a trust-group identity (the SAME
    /// `GroupHmac` the DFR1195 boards run). `hk` = the 32-byte group HMAC key (the
    /// persona's `hk` — NOT `withOta`'s Ed25519 `tg_pk`; two distinct keys from one
    /// persona). `tg_hash` = the firmware `my_tg_hash` (FNV of the TG uuid). Once set,
    /// `build_*`/ensemble frames are SIGNED + stamped `target_group = tg_hash`, and
    /// `verify_frame` runs the real deliver-gate — so a browser hive's frame verifies
    /// on the real boards and theirs verify in-browser. This is the frame-crossing
    /// join. (A plain `new()` hive stays TG-agnostic.)
    #[wasm_bindgen(js_name = withGroupHmac)]
    pub fn with_group_hmac(self_hive_id: u32, hk: &[u8], tg_hash: u32) -> WasmHive {
        let mut h = WasmHive::new(self_hive_id);
        h.set_group_hmac(hk, tg_hash);
        h
    }

    /// Join (or re-key) this hive into a trust group at runtime — same effect as
    /// `withGroupHmac` but on an existing node (e.g. once the persona `hk` arrives from
    /// the bridge). `hk` shorter than 32B is zero-padded; 0 bytes is treated as
    /// "leave" (clears the key → back to TG-agnostic).
    #[wasm_bindgen(js_name = setGroupHmac)]
    pub fn set_group_hmac(&mut self, hk: &[u8], tg_hash: u32) {
        if hk.is_empty() {
            self.group_hmac = None;
            self.tg_hash = 0;
            return;
        }
        let mut key = [0u8; 32];
        let n = hk.len().min(32);
        key[..n].copy_from_slice(&hk[..n]);
        self.group_hmac = Some(r2_trust::GroupHmac::new(key));
        self.tg_hash = tg_hash;
    }

    /// R2-TRUST §7.5.4 opt-in: allow an UNKEYED hive (no `group_hmac`) to DELIVER unverified frames — the
    /// legacy TG-agnostic pure-routing sim. Default is FAIL-CLOSED; a keyed hive ignores this. Composer's
    /// pure-routing bench must call `setUnkeyedOpen(true)` to keep delivering without a group key.
    #[wasm_bindgen(js_name = setUnkeyedOpen)]
    pub fn set_unkeyed_open(&mut self, open: bool) {
        self.unkeyed_open = open;
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
            group_hmac: None,
            tg_hash: 0,
            unkeyed_open: false, // §7.5.4 fail-closed default; setUnkeyedOpen(true) for the pure-routing sim
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

    /// Run the REAL TG deliver-gate on an inbound frame — the firmware's exact check
    /// (main.rs:1751-1752): `tg_ok = target_group == my_tg_hash || target_group == 0`
    /// AND `hmac_ok = verify_extended(frame, group_hmac)`. Returns JSON
    /// `{"keyed":bool,"tg_ok":bool,"hmac_ok":bool,"deliver":bool}`. A non-member hive
    /// (no group key) is TG-agnostic → `{"keyed":false,…,"deliver":true}` (legacy sim).
    /// This is the acceptance test a browser hive applies to board frames — and the
    /// board applies the IDENTICAL code to a browser hive's frames. Same hk → both
    /// deliver; wrong hk → `tg_ok:true hmac_ok:false deliver:false` (the live carrier's
    /// exact symptom). Routing (`route_frame`) is unchanged; this gates DELIVERY, the
    /// way the firmware gates in `io_task` AFTER the flood decision.
    #[wasm_bindgen(js_name = verifyFrame)]
    pub fn verify_frame(&self, frame: &[u8]) -> String {
        use r2_wire::{decode_extended, verify_extended};
        let m = match decode_extended(frame) {
            Ok(m) => m,
            Err(_) => {
                return String::from(
                    "{\"keyed\":false,\"tg_ok\":false,\"hmac_ok\":false,\"deliver\":false,\"error\":\"decode\"}",
                )
            }
        };
        match &self.group_hmac {
            // R2-TRUST §7.5.4: an UNKEYED hive FAIL-CLOSES (deliver:false) by default — "default-OPEN is
            // FORBIDDEN". Only an explicit setUnkeyedOpen(true) (pure-routing / TG-agnostic sim) delivers.
            None => format!(
                "{{\"keyed\":false,\"tg_ok\":true,\"hmac_ok\":false,\"deliver\":{}}}",
                self.unkeyed_open
            ),
            Some(hmac) => {
                let tg = m.header.target_group;
                let tg_ok = tg == self.tg_hash || tg == 0;
                let hmac_ok = verify_extended(&m, hmac);
                let deliver = tg_ok && hmac_ok;
                format!(
                    "{{\"keyed\":true,\"tg_ok\":{tg_ok},\"hmac_ok\":{hmac_ok},\"deliver\":{deliver}}}"
                )
            }
        }
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
            0,            // target_hive 0 = broadcast
            self.tg_hash, // target_group: TG hash when a member, else 0 (no gate)
            r2_wire::MsgType::Heartbeat,
            0, // event_hash: HB carries none
            &self.self_hive_id.to_be_bytes(),
            8, // ttl
            3, // k (flood fan-out)
            seq,
            self.group_hmac.as_ref(), // sign when a TG member (firmware multitg HB form)
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
            self.tg_hash,
            r2_wire::MsgType::Event,
            event_hash,
            payload,
            8,
            3,
            seq,
            self.group_hmac.as_ref(),
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
                self.tg_hash,
                r2_wire::MsgType::Event,
                ev.hash,
                ev.payload(),
                8,
                3,
                ev.msg_id as u32,
                self.group_hmac.as_ref(),
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
        let tg = self.tg_hash;
        let signer = self.group_hmac.as_ref();
        let mut frames: Vec<String> = Vec::new();
        let mut push = |hash: u32, sdu: &[u8], seq: u32| {
            let f = encode_frame(
                me,
                target_hive,
                tg,
                r2_wire::MsgType::Event,
                hash,
                sdu,
                8,
                1,
                seq,
                signer,
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

    /// CANON: this JSON shape is **R2-DIAGNOSTICS v0.1** (specs a47ab32, ratified verbatim from this getter);
    /// the dfr1195 firmware `viz` feature emits the identical record fields for physical boards.
    /// Theater oracle: the post-route NEIGHBOUR-CLASSIFIER table — the read side of
    /// conjectures 100/103 (mobile-vs-infra classify, neighbour-evict-at-floor +
    /// rediscovery). One JSON object per tracked neighbour:
    /// `{hive_id, viable, confidence, last_seen, class:"infra"|"mobile",
    ///   duty:"unknown"|"always_on"|"intermittent", fade_remaining}`.
    /// - `viable` = `is_viable(FORWARDING_CONFIDENCE_FLOOR)` — the SAME 0.1 floor the
    ///   forwarder applies (`engine.rs:607/648`), so the oracle is the engine's truth.
    /// - `class` = `MobilityClass` (sets the decay λ: mobile fades fast, infra slow).
    /// - `fade_remaining` = seconds until confidence decays below the viability floor
    ///   (`neighbour_fade_remaining`, `t = ln(conf/floor)/λ`; 0 if already at/below;
    ///   `null` if untracked). Drag a node out of range → stop feeding it observations →
    ///   call `decay(now)` with advancing `now` → watch `confidence` fall + `viable` flip
    ///   false + the entry evict; a fresh `route_frame` from it = rediscovery.
    #[wasm_bindgen]
    pub fn neighbours(&self) -> String {
        use r2_route::constants::FORWARDING_CONFIDENCE_FLOOR;
        use r2_route::neighbour::MobilityClass;
        let mut out = String::from("[");
        let mut first = true;
        for n in self.engine.neighbours().iter() {
            if !first {
                out.push(',');
            }
            first = false;
            let class = match n.mobility {
                MobilityClass::Infrastructure => "infra",
                MobilityClass::Mobile => "mobile",
            };
            let duty = match n.duty_class {
                r2_route::neighbour::DutyClass::AlwaysOn => "always_on",
                r2_route::neighbour::DutyClass::Intermittent => "intermittent",
                r2_route::neighbour::DutyClass::Unknown => "unknown",
            };
            let fade = match self.engine.neighbour_fade_remaining(n.hive_id) {
                Some(s) => format!("{s}"),
                None => String::from("null"),
            };
            out.push_str(&format!(
                "{{\"hive_id\":{},\"viable\":{},\"confidence\":{},\"last_seen\":{},\"class\":\"{}\",\"duty\":\"{}\",\"fade_remaining\":{}}}",
                n.hive_id,
                n.is_viable(FORWARDING_CONFIDENCE_FLOOR),
                n.confidence,
                n.last_seen,
                class,
                duty,
                fade,
            ));
        }
        out.push(']');
        out
    }

    /// CANON: this JSON shape is **R2-DIAGNOSTICS v0.1** (specs a47ab32) — 1:1 with R2-ROUTE §4.2 PathTable;
    /// the dfr1195 firmware `viz` feature emits the identical record fields for physical boards.
    /// Theater oracle: the learned DIRECTED-PATH table — the read side of conjectures
    /// 200/204 (used-path-wins / idle-decays). One JSON object per path:
    /// `{destination, next_hop, confidence, last_updated, sample_count}`. A delivered
    /// frame raises the (dest,next_hop) confidence (used-path-wins); `decay(now)` lets
    /// an unused path fade (idle-decays). Pair with `route_frame`'s `outcome` — "Directed"
    /// + the send's `target` = the `directed_via` oracle; "Flooded" = the `flooded` oracle.
    #[wasm_bindgen]
    pub fn paths(&self) -> String {
        let mut out = String::from("[");
        let mut first = true;
        for p in self.engine.paths().iter() {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!(
                "{{\"destination\":{},\"next_hop\":{},\"confidence\":{},\"last_updated\":{},\"sample_count\":{}}}",
                p.destination, p.next_hop, p.confidence, p.last_updated, p.sample_count,
            ));
        }
        out.push(']');
        out
    }

    /// Theater driver: advance the decay clock to `now` — fades neighbour + path
    /// confidences and evicts stale entries (the REAL `decay_neighbours`/`decay_paths`).
    /// Confidence only falls on a decay tick (it rises on observation), so the theater
    /// calls this between range changes to animate eviction-at-floor + path idle-decay.
    #[wasm_bindgen]
    pub fn decay(&mut self, now: u32) {
        self.engine.decay_neighbours(now);
        self.engine.decay_paths(now);
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
    // TG signer: `Some` → stamp `has_hmac` + attach the 32B extended tag (the SAME
    // `sign_extended` path the firmware uses at main.rs:1011-1013), so the frame
    // verifies on real boards. `None` → unsigned (legacy TG-agnostic sim).
    signer: Option<&r2_trust::GroupHmac>,
) -> Vec<u8> {
    use r2_wire::{
        encode_extended, sign_extended, ExtendedHeader, ExtendedMessage, ExtendedRouteStack, Flags,
    };
    let mut msg = ExtendedMessage {
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
    if let Some(hmac) = signer {
        // route is self-stamped (with_origin) so the span builder has a canonical
        // origin — ROUTE-ORIGIN-1 satisfied; a route-less sign would zero-tag + drop.
        let (flags, tag) = sign_extended(&msg, hmac);
        msg.header.flags = flags;
        msg.hmac_tag = Some(tag);
    }
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

    #[test]
    fn quality_from_rssi_matches_the_50_80_clamp() {
        assert!((quality_from_rssi(-50.0) - 1.0).abs() < 1e-6); // excellent
        assert!((quality_from_rssi(-80.0) - 0.0).abs() < 1e-6); // link-dead
        assert!((quality_from_rssi(-65.0) - 0.5).abs() < 1e-6); // midpoint
        assert_eq!(quality_from_rssi(-30.0), 1.0); // saturates above −50
        assert_eq!(quality_from_rssi(-100.0), 0.0); // saturates below −80
    }

    #[test]
    fn frame_origin_reads_route_stack0_and_0_on_garbage() {
        let h = WasmHive::new(0x0000_00aa);
        let f = h.build_frame(0x0000_00bb, 0x1234, &[1, 2, 3], 7);
        assert_eq!(frame_origin(&f), 0x0000_00aa); // originator = route_stack[0] = self
        assert_eq!(frame_origin(b"not-a-frame"), 0); // undecodable → 0
        assert_eq!(frame_origin(&[]), 0);
    }

    #[test]
    fn route_hops_reads_full_trail_and_grows_by_a_hop_per_relay() {
        // A freshly-originated frame's trail is just [origin] and hops[0] == frame_origin.
        let a = WasmHive::new(0x0000_00aa);
        let f = a.build_frame(0x0000_00cc, 0x1234, &[9], 3);
        assert_eq!(route_hops(&f), vec![0x0000_00aa], "origin-only trail");
        assert_eq!(route_hops(&f).first().copied(), Some(frame_origin(&f)), "hops[0] == origin");
        // A relay appends itself: route B's frame (learn B first so it can relay to cc)…
        let mut b = WasmHive::new(0x0000_00bb);
        let learn = ext_frame(0x0000_00cc, 0x0000_0001, 5, 3, 0x1000);
        let _ = b.route_frame(0x0000_00cc, 1, &learn, 100, 0.5);
        let out = b.route_frame(0x0000_00aa, 1, &f, 200, 0.5);
        // Pull the first relayed frame's hex out of the sends JSON and read its trail.
        let hx = out.split("\"frame\":\"").nth(1).and_then(|s| s.split('"').next()).unwrap_or("");
        if !hx.is_empty() {
            let relayed: Vec<u8> = (0..hx.len() / 2)
                .map(|i| u8::from_str_radix(&hx[i * 2..i * 2 + 2], 16).unwrap())
                .collect();
            let hops = route_hops(&relayed);
            assert_eq!(hops.first().copied(), Some(0x0000_00aa), "origin immutable at [0]");
            assert_eq!(hops.last().copied(), Some(0x0000_00bb), "relayer appended itself");
            assert!(hops.len() >= 2, "trail grew by the relay hop: {hops:?}");
        }
        // Undecodable / route-less → empty.
        assert_eq!(route_hops(b"not-a-frame"), Vec::<u32>::new());
        assert_eq!(route_hops(&[]), Vec::<u32>::new());
    }

    #[test]
    fn range_to_loss_db_is_log_distance_and_lora_outranges_ble() {
        // §2.7 v0.19 LOG-DISTANCE: loss = PL_ref + 10·n·log10(d/d_ref); grows (sub-linearly) with range.
        // VALUE-AGNOSTIC by design: PL_ref + n are PROVISIONAL (single-sourced from core r2-transport;
        // they moved once already — provisional PL_ref 0 → theater.html-matched 40 in core 5e30c49 —
        // and will move again on Roy's field-anchor). So assert the ratified SHAPE, not the numbers; a
        // hard-coded value here would just re-break on the next anchor (this test IS the drift tripwire).
        assert!(range_to_loss_db(2, 100.0) > range_to_loss_db(2, 10.0)); // monotonic ↑ (LoRa, id 2)
        assert!(range_to_loss_db(2, 10.0) > range_to_loss_db(2, 1.0)); // still ↑ down to d_ref
        // LoRa's smaller n → LESS loss at the same range than BLE → longer emergent range
        assert!(range_to_loss_db(2, 50.0) < range_to_loss_db(0, 50.0)); // LoRa < BLE loss
        // NEAR-FIELD MODELLED (floor = 0.001, NOT d_ref): a sub-reference distance yields LESS loss than
        // the reference (closer ⇒ stronger), it does NOT plateau at PL_ref. This asserts the CURRENT
        // ratified floor semantics and is an intentional tripwire — it trips if core flips the floor back
        // to max(d, d_ref) (it has flip-flopped 1.0↔0.001 already; that churn is how the drift is caught).
        let at_ref = range_to_loss_db(2, 1.0); // loss at d_ref = PL_ref (log10(1)=0)
        assert!(range_to_loss_db(2, 0.5) < at_ref, "sub-reference is near-field-modelled (< PL_ref)");
        // No signal gain / no non-finite RSSI: loss is finite, ≥0, ≤160 for ANY input (incl ≤0 / huge).
        for &d in &[-5.0f32, 0.0, 0.5, 1.0, 50.0, 1.0e9] {
            let l = range_to_loss_db(2, d);
            assert!(l.is_finite() && (0.0..=160.0).contains(&l), "bounded finite loss at d={d}: {l}");
        }
        // EMERGENT range: at a long range BLE quality has decayed further than LoRa (BLE shorter range)
        let far = 1000.0;
        assert!(
            quality_from_rssi(0.0 - range_to_loss_db(0, far))
                <= quality_from_rssi(0.0 - range_to_loss_db(2, far))
        );
    }

    #[test]
    fn transport_profile_exposes_core_canonical_fields() {
        let lora = transport_profile(2);
        assert!(lora.contains("\"max_payload\":222")); // R2-LORA §5.2 MTU (core for_transport)
        assert!(lora.contains("reference_path_loss_db"));
        assert!(lora.contains("path_loss_exponent"));
        assert!(lora.contains("decay_lambda"));
    }

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
    fn group_hmac_frame_crossing_same_key_delivers_wrong_key_rejects() {
        // The frame-crossing claim: a TG member's signed frame must DELIVER on another
        // member with the SAME hk, and REJECT (hmac_ok=false) on a hive with the same
        // tg_hash but a DIFFERENT hk — the live carrier's exact symptom.
        let hk = [0x42u8; 32];
        let tg = 0xABCD_1234u32;
        let a = WasmHive::with_group_hmac(0x0000_00A1, &hk, tg);
        let b = WasmHive::with_group_hmac(0x0000_00B2, &hk, tg);
        let frame = a.build_frame(0, 0x1111_2222, b"in-TG", 7);
        assert!(!frame.is_empty(), "signed frame encoded");

        // same hk → tg_ok + hmac_ok + deliver
        let v = b.verify_frame(&frame);
        assert!(v.contains("\"keyed\":true"), "{v}");
        assert!(v.contains("\"hmac_ok\":true"), "same-key verify: {v}");
        assert!(v.contains("\"deliver\":true"), "same-key deliver: {v}");

        // same tg_hash, WRONG hk → tg_ok TRUE but hmac_ok FALSE → no deliver
        let c = WasmHive::with_group_hmac(0x0000_00C3, &[0x99u8; 32], tg);
        let v2 = c.verify_frame(&frame);
        assert!(v2.contains("\"tg_ok\":true"), "wrong-key tg still matches: {v2}");
        assert!(v2.contains("\"hmac_ok\":false"), "wrong-key reject: {v2}");
        assert!(v2.contains("\"deliver\":false"), "wrong-key no-deliver: {v2}");

        // a NON-member (unkeyed) hive FAIL-CLOSES by default (§7.5.4: default-OPEN is FORBIDDEN).
        let mut d = WasmHive::new(0x0000_00D4);
        assert!(
            d.verify_frame(&frame).contains("\"keyed\":false"),
            "unkeyed = TG-agnostic"
        );
        assert!(
            d.verify_frame(&frame).contains("\"deliver\":false"),
            "unkeyed fail-closed by default"
        );
        // explicit operator opt-in restores the legacy TG-agnostic deliver-everything (pure-routing sim).
        d.set_unkeyed_open(true);
        assert!(
            d.verify_frame(&frame).contains("\"deliver\":true"),
            "unkeyed OPEN by opt-in delivers"
        );

        // set_group_hmac at runtime = join → d now verifies like a member
        let mut d2 = WasmHive::new(0x0000_00D5);
        d2.set_group_hmac(&hk, tg);
        assert!(
            d2.verify_frame(&frame).contains("\"hmac_ok\":true"),
            "runtime join verifies"
        );
        // empty hk = leave → back to TG-agnostic
        d2.set_group_hmac(&[], 0);
        assert!(d2.verify_frame(&frame).contains("\"keyed\":false"), "leave");
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

    #[test]
    fn neighbour_oracle_learns_then_fades_below_floor() {
        // The conj-100/103 theater arc: a node learns a neighbour (viable, confidence
        // up), then dragged out of range (no more observations) it decays below the
        // forwarding floor and evicts — the getter reflects each stage.
        let peer = 0x0000_00AA;
        let mut hive = WasmHive::new(0x0000_00FF);
        assert_eq!(hive.neighbours(), "[]", "fresh hive has no neighbours");

        // Hear a frame FROM peer → learned as a viable neighbour.
        let learn = ext_frame(peer, 0x0000_0001, 5, 3, 0x1000);
        let _ = hive.route_frame(peer, 1, &learn, 100, 0.5);
        let n = hive.neighbours();
        assert!(n.contains(&format!("\"hive_id\":{peer}")), "peer not tracked: {n}");
        assert!(n.contains("\"viable\":true"), "freshly-heard peer must be viable: {n}");
        assert!(n.contains("\"fade_remaining\":"), "fade telemetry present: {n}");

        // Drag out of range: no new observations, advance the decay clock far past the
        // fade window → confidence falls below FORWARDING_CONFIDENCE_FLOOR → evicted.
        hive.decay(100 + 1_000_000);
        let after = hive.neighbours();
        assert!(
            after == "[]" || after.contains("\"viable\":false"),
            "peer must fade below the floor (evicted or non-viable), got {after}"
        );
    }
}
