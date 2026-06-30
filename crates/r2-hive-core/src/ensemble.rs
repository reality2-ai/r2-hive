//! The **basic ensemble** — the sentants/plugins every hive platform runs on the
//! `r2_engine` EventBus. R2-HIVE north-star: the SAME ensemble code on the browser
//! wasm-hive, the Linux/cloud host, and the ESP32-S3/DFR1195 firmware; only the
//! platform host (transport, clock, storage) differs.
//!
//! Increment 1 (here): the **heartbeat sentant** — on each host `TICK` it broadcasts
//! a heartbeat carrying this node's `hive_id`. The platform host turns the resulting
//! *outbound* bus event into a wire frame and floods it over its transport (the wasm
//! virtual-mesh, ESP-NOW, …) — so the ensemble is bearer-agnostic.
//!
//! Next: the **OTA plugin + sentant** (R2-UPDATE OST/ODT/OCM, verify-before-write,
//! 4-gate / Ed25519 / anti-rollback) — the *pure* OTA form (complex work in the
//! plugin, control via sentant+events), bearer = whatever transport the host binds.
//! That is held pending core's OTA-plugin-shape ruling + the codex refute of the
//! firmware's `ota_receive_over_coc` (this ensemble OTA is TEST/validation, not a
//! substitute for that refute).

use r2_engine::action::PayloadBuf;
use r2_engine::{Action, ActionBuf, Event, Sentant, StateId, Target};

/// Bus event the platform host injects each tick to drive periodic sentant work.
pub const TICK_HASH: u32 = r2_fnv::fnv1a_32(b"r2.hive.tick");
/// Heartbeat event a node broadcasts; payload = origin `hive_id` BE32 (the firmware
/// heartbeat wire form — see the DFR1195 ESP-NOW HB payload).
pub const HEARTBEAT_HASH: u32 = r2_fnv::fnv1a_32(b"r2.hb.heartbeat");

const HB_CLASS: u32 = r2_fnv::fnv1a_32(b"ai.reality2.sentant.heartbeat");
const HB_SUBS: [u32; 1] = [TICK_HASH];

/// Heartbeat sentant — emits a broadcast heartbeat on every `TICK`.
pub struct HbSentant {
    hive_id: u32,
    beats: u32,
}

impl HbSentant {
    pub fn new(hive_id: u32) -> Self {
        Self { hive_id, beats: 0 }
    }
    /// Heartbeats emitted so far (sim/telemetry).
    pub fn beats(&self) -> u32 {
        self.beats
    }
}

impl Sentant for HbSentant {
    fn handle_event(&mut self, event: &Event, actions: &mut ActionBuf) {
        if event.hash == TICK_HASH {
            self.beats = self.beats.wrapping_add(1);
            actions.push(Action::Send {
                target: Target::Broadcast,
                event_hash: HEARTBEAT_HASH,
                payload: PayloadBuf::from_slice(&self.hive_id.to_be_bytes()),
            });
        }
    }
    fn state(&self) -> StateId {
        0
    }
    fn class_hash(&self) -> u32 {
        HB_CLASS
    }
    fn name(&self) -> &str {
        "heartbeat"
    }
    fn subscriptions(&self) -> &[u32] {
        &HB_SUBS
    }
}

// ───────────────────────────── OTA plugin + sentant ─────────────────────────────
//
// The PURE OTA form (R2-HIVE increment-3): the PLUGIN does the complex work
// (R2-UPDATE verify-before-write: verify_header / PayloadVerifier / Ed25519 / 4-gate
// / anti-rollback, reused VERBATIM from r2-update), the SENTANT + events do control.
// Bearer-agnostic: the same plugin verifies an image arriving over a BLE L2CAP CoC
// (DFR1195), the wasm virtual-mesh (browser sim), or UDP (host hive) — only the
// frame the sentant receives differs. Wasm nodes OTA each other = a software e2e of
// the REAL flow. (This is TEST/validation; NOT a substitute for the codex refute of
// the firmware's `ota_receive_over_coc`.)

use alloc::vec::Vec;
// Core's canonical OTA orchestrator (the shared verify-before-write RCE-guard ordering)
// + the platform sink trait. Per core's OTA ruling (OTA_PLUGIN_SHAPE.md, STATE B): the
// ORDERING is shared in core (SignedOtaApply), hive impls ImageSink per platform + the
// OtaSentant control surface. r2-update stays the verify primitive owner.
use r2_update::apply::{ApplyError, ImageSink, SignedOtaApply};
use r2_update::{reject_reason, verify_header, DeviceContext, HEADER_LEN, PT_FIRMWARE_FULL, SIG_LEN};

/// OTA wire steps (event_hash discriminators on the mesh; SDU 3-byte tags on a CoC).
pub const OST_HASH: u32 = r2_fnv::fnv1a_32(b"r2.update.ost"); // start: header(123)+sig(64)
pub const ODT_HASH: u32 = r2_fnv::fnv1a_32(b"r2.update.odt"); // data: one payload chunk
pub const OCM_HASH: u32 = r2_fnv::fnv1a_32(b"r2.update.ocm"); // commit: finish+activate
/// Progress event the receiver emits — payload `[phase][bytes_done BE32][total BE32][reason]`.
pub const PROGRESS_HASH: u32 = r2_fnv::fnv1a_32(b"r2.update.progress");

/// OTA progress phases (progress payload byte 0).
pub const PH_START_OK: u8 = 0;
pub const PH_DATA: u8 = 1;
pub const PH_VERIFIED: u8 = 2;
pub const PH_APPLIED: u8 = 3;
pub const PH_REJECT: u8 = 0xFF;

/// In-memory [`ImageSink`] — the wasm/host platform sink (no real flash). Stages the
/// VERIFIED image so a sim/test can prove what was installed. (Board impl wraps
/// `esp_ota_begin/write/end` + `set_boot_partition` in the firmware — same trait.)
pub struct MemSink {
    staged: Vec<u8>,
    activated: bool,
}
impl Default for MemSink {
    fn default() -> Self {
        Self::new()
    }
}
impl MemSink {
    pub fn new() -> Self {
        Self { staged: Vec::new(), activated: false }
    }
    /// The activated image (the installed bytes after a successful `activate`).
    pub fn image(&self) -> &[u8] {
        &self.staged
    }
    /// Whether `activate` ran (the verified image is now pending-boot).
    pub fn activated(&self) -> bool {
        self.activated
    }
}
impl ImageSink for MemSink {
    type Error = ();
    fn begin(&mut self, total_len: usize) -> Result<(), ()> {
        self.staged = Vec::with_capacity(total_len);
        self.activated = false;
        Ok(())
    }
    fn write(&mut self, chunk: &[u8]) -> Result<(), ()> {
        self.staged.extend_from_slice(chunk);
        Ok(())
    }
    fn activate(&mut self, _seq: u32, _payload_hash: &[u8; 32]) -> Result<(), ()> {
        self.activated = true;
        Ok(())
    }
}

/// Map an apply error to a progress `reason` byte: a VerifyError via `reject_reason`
/// (1..=12), a sink error to 0x70.
fn apply_reason<E>(e: &ApplyError<E>) -> u8 {
    match e {
        ApplyError::Verify(v) => reject_reason(*v),
        ApplyError::Sink(_) => 0x70,
    }
}

/// Owned device-side OTA gate inputs — built into a [`DeviceContext`] per verify.
/// (certs/revocation omitted for the v1 demo = TG_SK-direct signer only.)
#[derive(Clone)]
pub struct OtaConfig {
    pub class_hash: u32,
    pub carrier_hash: u32,
    pub tg_prefix: [u8; 8],
    pub device_id_prefix: [u8; 8],
    pub current_seq: u32,
    pub battery_pct: u8,
    pub tg_pk: [u8; 32],
}
impl OtaConfig {
    fn ctx(&self) -> DeviceContext<'_> {
        DeviceContext {
            class_hash: self.class_hash,
            carrier_hash: self.carrier_hash,
            tg_prefix: self.tg_prefix,
            device_id_prefix: self.device_id_prefix,
            current_seq: self.current_seq,
            battery_pct: self.battery_pct,
            tg_pk: self.tg_pk,
            update_authority_certs: &[],
            revocation_gset: &[],
            authority_epoch_floor: 0,
        }
    }
}

fn progress(phase: u8, done: u32, total: u32, reason: u8) -> [u8; 10] {
    let mut p = [0u8; 10];
    p[0] = phase;
    p[1..5].copy_from_slice(&done.to_be_bytes());
    p[5..9].copy_from_slice(&total.to_be_bytes());
    p[9] = reason;
    p
}

/// OTA receiver applier — buffers the OST/ODT/OCM mesh framing of R2-UPDATE §3.1.2.3
/// CMD_START_SIGNED (OST = header(123)‖sig(64), ODT = payload stream, OCM = commit) and
/// runs core's canonical [`SignedOtaApply`] orchestrator on commit (the shared
/// verify-before-write RCE-guard ordering — NOT re-implemented here). The unverified
/// payload sits in a RAM buffer; only VERIFIED chunks are written to the `ImageSink`,
/// and the sink is activated ONLY after the full hash confirms. Bearer-agnostic.
///
/// (Buffer-then-apply-on-commit is the event-model realization: `SignedOtaApply`
/// borrows the sink for its lifetime + `finish` consumes it, so it cannot persist
/// across discrete OST/ODT/OCM events — the MCU streaming receiver drives the SAME
/// orchestrator in a single streaming loop instead. Same shared ordering, both.)
pub struct OtaApplier<S: ImageSink> {
    cfg: OtaConfig,
    sink: S,
    ost: Vec<u8>, // header(123) ‖ sig(64)
    buf: Vec<u8>, // payload stream
    total: u32,
    header_ok: bool,
}
impl<S: ImageSink> OtaApplier<S> {
    pub fn new(cfg: OtaConfig, sink: S) -> Self {
        Self { cfg, sink, ost: Vec::new(), buf: Vec::new(), total: 0, header_ok: false }
    }
    /// The platform sink (read the installed image from a MemSink after APPLIED).
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// OST: stash header‖sig + an EARLY `verify_header` for fast reject feedback (the
    /// authoritative verify+apply runs on commit via `SignedOtaApply`). Returns a
    /// progress record.
    pub fn on_ost(&mut self, data: &[u8]) -> [u8; 10] {
        self.ost.clear();
        self.buf.clear();
        self.header_ok = false;
        self.total = 0;
        if data.len() < HEADER_LEN + SIG_LEN {
            return progress(PH_REJECT, 0, 0, 0);
        }
        self.ost.extend_from_slice(&data[..HEADER_LEN + SIG_LEN]);
        let header = &self.ost[..HEADER_LEN];
        let sig = &self.ost[HEADER_LEN..HEADER_LEN + SIG_LEN];
        match verify_header(header, sig, &self.cfg.ctx()) {
            Ok(vh) => {
                self.total = vh.payload_len as u32;
                self.header_ok = true;
                progress(PH_START_OK, 0, self.total, 0)
            }
            Err(e) => progress(PH_REJECT, 0, 0, reject_reason(e)),
        }
    }

    /// ODT: buffer one payload chunk (unverified — verification happens on commit).
    pub fn on_odt(&mut self, chunk: &[u8]) -> [u8; 10] {
        if self.header_ok {
            self.buf.extend_from_slice(chunk);
        }
        progress(PH_DATA, self.buf.len() as u32, self.total, 0)
    }

    /// OCM: run core's canonical verify-before-write apply over the buffered stream:
    /// `start` (verify_header 4-gate/Ed25519/anti-rollback + type-gate + sink.begin) →
    /// `feed` (verify-then-write per chunk) → `finish` (hash-confirm THEN sink.activate).
    /// A bad image never activates. Returns the progress sequence (VERIFIED+APPLIED or
    /// a single REJECT).
    pub fn on_ocm(&mut self) -> Vec<[u8; 10]> {
        let d = self.buf.len() as u32;
        if !self.header_ok || self.ost.len() < HEADER_LEN + SIG_LEN {
            return alloc::vec![progress(PH_REJECT, d, self.total, 0)];
        }
        let ctx = self.cfg.ctx();
        let header = &self.ost[..HEADER_LEN];
        let sig = &self.ost[HEADER_LEN..HEADER_LEN + SIG_LEN];
        let apply = SignedOtaApply::start(header, sig, &ctx, PT_FIRMWARE_FULL, &mut self.sink);
        let mut apply = match apply {
            Ok(a) => a,
            Err(e) => return alloc::vec![progress(PH_REJECT, 0, self.total, apply_reason(&e))],
        };
        if let Err(e) = apply.feed(&self.buf) {
            // sink.begin ran but we never activate → staged bytes are discardable.
            return alloc::vec![progress(PH_REJECT, d, self.total, apply_reason(&e))];
        }
        match apply.finish() {
            Ok(_) => alloc::vec![progress(PH_VERIFIED, d, self.total, 0), progress(PH_APPLIED, d, self.total, 0)],
            Err(e) => alloc::vec![progress(PH_REJECT, d, self.total, apply_reason(&e))],
        }
    }
}

const OTA_CLASS: u32 = r2_fnv::fnv1a_32(b"ai.reality2.sentant.ota");
const OTA_SUBS: [u32; 3] = [OST_HASH, ODT_HASH, OCM_HASH];

/// OTA control SENTANT (the EventBus surface) — owns the platform [`ImageSink`] via an
/// [`OtaApplier`], drives the canonical apply on OST/ODT/OCM events, and BROADCASTS the
/// `r2.update.progress` so peers / the bench observe the transfer. "Complex work in the
/// (core) driver, control via sentant+events."
pub struct OtaSentant<S: ImageSink> {
    applier: OtaApplier<S>,
}
impl<S: ImageSink> OtaSentant<S> {
    pub fn new(cfg: OtaConfig, sink: S) -> Self {
        Self { applier: OtaApplier::new(cfg, sink) }
    }
    /// The platform sink (test/inspection).
    pub fn sink(&self) -> &S {
        self.applier.sink()
    }
    fn emit(actions: &mut ActionBuf, p: [u8; 10]) {
        actions.push(Action::Send {
            target: Target::Broadcast,
            event_hash: PROGRESS_HASH,
            payload: PayloadBuf::from_slice(&p),
        });
    }
}
impl<S: ImageSink> Sentant for OtaSentant<S> {
    fn handle_event(&mut self, event: &Event, actions: &mut ActionBuf) {
        match event.hash {
            h if h == OST_HASH => Self::emit(actions, self.applier.on_ost(event.payload)),
            h if h == ODT_HASH => Self::emit(actions, self.applier.on_odt(event.payload)),
            h if h == OCM_HASH => {
                for p in self.applier.on_ocm() {
                    Self::emit(actions, p);
                }
            }
            _ => {}
        }
    }
    fn state(&self) -> StateId {
        0
    }
    fn class_hash(&self) -> u32 {
        OTA_CLASS
    }
    fn name(&self) -> &str {
        "ota"
    }
    fn subscriptions(&self) -> &[u32] {
        &OTA_SUBS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2_engine::queue::QueuedEvent;
    use r2_engine::EventBus;

    #[test]
    fn hb_sentant_emits_on_tick() {
        let mut bus = EventBus::new();
        bus.register_sentant(alloc::boxed::Box::new(HbSentant::new(0x0000_00AA)));
        bus.init_all();
        // Inject a TICK → the sentant should broadcast a heartbeat outbound.
        bus.enqueue(QueuedEvent::new(TICK_HASH, 0xFF, false, 0, &[]));
        bus.tick();
        let out = bus.drain_outbound();
        assert_eq!(out.len(), 1, "one outbound heartbeat");
        assert_eq!(out[0].hash, HEARTBEAT_HASH);
        assert_eq!(out[0].payload(), 0x0000_00AAu32.to_be_bytes());
    }

    // ── OTA plugin: mint a real signed package, drive verify-before-write ──
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    fn sha256(d: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(d);
        h.finalize().into()
    }

    /// Mirror of r2-update's test header layout (offsets per UpdateHeader::parse).
    fn mint(payload: &[u8], seq: u32, issuer_pk: [u8; 32]) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[0] = r2_update::PACKAGE_VERSION; // = 2
        // package_id [1..17] left zero
        // target_class [17..21] = 0 (any); target_carrier [21..25] = 0 (any)
        // target_tg [25..33] = 0 (any member); target_device [33..41] = 0 (any)
        h[41] = 0x01; // payload_type = firmware-full
        h[42..46].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        h[46..78].copy_from_slice(&sha256(payload));
        h[78..82].copy_from_slice(&seq.to_be_bytes());
        // created_at [82..90] = 0
        h[90..122].copy_from_slice(&issuer_pk);
        h[122] = 0; // min_battery
        h
    }

    fn ota_cfg(tg_pk: [u8; 32]) -> OtaConfig {
        OtaConfig {
            class_hash: 0xDEAD_BEEF,
            carrier_hash: 0xCA44_1E20,
            tg_prefix: [0x11; 8],
            device_id_prefix: [0x22; 8],
            current_seq: 0,
            battery_pct: 100,
            tg_pk,
        }
    }

    fn run_apply(a: &mut OtaApplier<MemSink>, header: &[u8], sig: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut ost = Vec::from(header);
        ost.extend_from_slice(sig);
        let mut phases = alloc::vec![a.on_ost(&ost)[0]];
        for chunk in payload.chunks(200) {
            phases.push(a.on_odt(chunk)[0]);
        }
        for p in a.on_ocm() {
            phases.push(p[0]);
        }
        phases
    }

    #[test]
    fn ota_applier_verifies_and_applies() {
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"FIRMWARE-IMAGE-BYTES-v2-the-new-build".repeat(8);
        let header = mint(&payload, 1, tg_pk);
        let sig = sk.sign(&header).to_bytes();

        let mut a = OtaApplier::new(ota_cfg(tg_pk), MemSink::new());
        let phases = run_apply(&mut a, &header, &sig, &payload);
        assert!(phases.contains(&PH_START_OK), "start ok: {phases:?}");
        assert!(phases.contains(&PH_VERIFIED), "verified: {phases:?}");
        assert!(phases.contains(&PH_APPLIED), "applied: {phases:?}");
        assert!(!phases.contains(&PH_REJECT), "no reject: {phases:?}");
        // verify-before-write: the VERIFIED image was staged + activated.
        assert!(a.sink().activated());
        assert_eq!(a.sink().image(), &payload[..]);
    }

    #[test]
    fn ota_applier_rejects_tampered_payload() {
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"GOOD-IMAGE".repeat(20);
        let header = mint(&payload, 1, tg_pk);
        let sig = sk.sign(&header).to_bytes();
        let mut bad = payload.clone();
        bad[5] ^= 0xFF; // hash won't match the signed header

        let mut a = OtaApplier::new(ota_cfg(tg_pk), MemSink::new());
        let phases = run_apply(&mut a, &header, &sig, &bad);
        assert!(phases.contains(&PH_REJECT), "tampered must reject: {phases:?}");
        assert!(!phases.contains(&PH_APPLIED), "must NOT apply: {phases:?}");
        // verify-before-write: a rejected image is never activated.
        assert!(!a.sink().activated(), "bad image must not activate");
    }

    #[test]
    fn ota_rejects_replayed_seq() {
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"x".repeat(50);
        let header = mint(&payload, 0, tg_pk); // seq 0 <= current_seq 0 → StaleSeq
        let sig = sk.sign(&header).to_bytes();
        let mut a = OtaApplier::new(ota_cfg(tg_pk), MemSink::new());
        // early OST verify rejects a stale seq immediately.
        let p = a.on_ost(&{
            let mut o = Vec::from(&header[..]);
            o.extend_from_slice(&sig);
            o
        });
        assert_eq!(p[0], PH_REJECT, "stale seq must reject at OST");
    }
}
