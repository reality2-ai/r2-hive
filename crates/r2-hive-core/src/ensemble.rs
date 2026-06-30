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
use crate::ota::{FirmwareSink, OtaError};
use r2_engine::plugin::{PluginResponse, PluginResult};
use r2_engine::{Plugin, PluginCommand, PluginId};
use r2_update::{reject_reason, verify_header, DeviceContext, PayloadVerifier, HEADER_LEN, SIG_LEN};

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

const OTA_CMD_OST: PluginCommand = 1;
const OTA_CMD_ODT: PluginCommand = 2;
const OTA_CMD_OCM: PluginCommand = 3;

/// The inactive-slot write seam is the canonical [`crate::ota::FirmwareSink`]
/// (slot_capacity/begin/write_chunk/finalize/abort) — the ONE platform seam per
/// core's OTA ruling: real `esp_ota_*` flash on the DFR1195, an in-memory buffer in
/// wasm. One OTA plugin, the sink swaps.

/// In-memory `FirmwareSink` — the wasm/host platform sink (no real flash). Holds the
/// written image so a sim/test can prove what was installed. (Board impl wraps
/// esp_ota_* in the firmware.)
pub struct MemSink {
    image: Vec<u8>,
    cap: u32,
    finalized: bool,
}
impl Default for MemSink {
    fn default() -> Self {
        Self::new()
    }
}
impl MemSink {
    pub fn new() -> Self {
        Self { image: Vec::new(), cap: 4 * 1024 * 1024, finalized: false }
    }
    /// The image written so far (the installed bytes after a successful finalize).
    pub fn image(&self) -> &[u8] {
        &self.image
    }
    pub fn finalized(&self) -> bool {
        self.finalized
    }
}
impl FirmwareSink for MemSink {
    fn slot_capacity(&self) -> u32 {
        self.cap
    }
    fn begin(&mut self, _image_len: u32) -> Result<(), OtaError> {
        self.image.clear();
        self.finalized = false;
        Ok(())
    }
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), OtaError> {
        self.image.extend_from_slice(chunk);
        Ok(())
    }
    fn finalize(&mut self) -> Result<(), OtaError> {
        self.finalized = true;
        Ok(())
    }
    fn abort(&mut self) {
        self.image.clear();
        self.finalized = false;
    }
}

/// Map a sink error to a progress `reason` byte (kept distinct from the
/// VerifyError `reject_reason` space, which is 1..=12).
fn sink_reason(_e: OtaError) -> u8 {
    0x70 // SINK_FAIL
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

/// OTA receiver PLUGIN — verify-before-write over OST/ODT/OCM commands (the mesh
/// framing of R2-UPDATE §3.1.2.3 CMD_START_SIGNED: OST = header(123)‖sig(64), ODT =
/// payload stream, OCM = close/commit). Reuses the r2-update verify primitives
/// verbatim (core ruling: r2-update stays verify-only) + the canonical
/// [`crate::ota::FirmwareSink`] seam; a bad image never reaches `finalize()`.
pub struct OtaPlugin<S: FirmwareSink> {
    cfg: OtaConfig,
    sink: S,
    pv: Option<PayloadVerifier>,
    total: u32,
    done: u32,
    queue: Vec<[u8; 10]>, // progress events drained by poll()
    last: [u8; 10],
}
impl<S: FirmwareSink> OtaPlugin<S> {
    pub fn new(cfg: OtaConfig, sink: S) -> Self {
        Self { cfg, sink, pv: None, total: 0, done: 0, queue: Vec::new(), last: [0u8; 10] }
    }
    /// The platform sink (e.g. read the installed image from a MemSink after APPLIED).
    pub fn sink(&self) -> &S {
        &self.sink
    }
    fn push(&mut self, p: [u8; 10]) {
        self.queue.push(p);
    }
}
impl<S: FirmwareSink> Plugin for OtaPlugin<S> {
    fn execute(&mut self, command: PluginCommand, data: &[u8]) -> PluginResult {
        match command {
            OTA_CMD_OST => {
                self.pv = None;
                self.done = 0;
                self.sink.abort(); // discard any half-staged prior attempt
                if data.len() < HEADER_LEN + SIG_LEN {
                    self.push(progress(PH_REJECT, 0, 0, 0));
                    return PluginResult::Ok(PluginResponse::empty());
                }
                let header = &data[..HEADER_LEN];
                let sig = &data[HEADER_LEN..HEADER_LEN + SIG_LEN];
                // verify_header (4-gate/Ed25519/anti-rollback) BEFORE any sink.begin.
                match verify_header(header, sig, &self.cfg.ctx()) {
                    Ok(vh) => {
                        self.total = vh.payload_len as u32;
                        // slot_capacity TOO_BIG precheck, then begin (only after verify Ok).
                        if self.total > self.sink.slot_capacity() {
                            self.push(progress(PH_REJECT, 0, self.total, sink_reason(OtaError::TooBig)));
                        } else if let Err(e) = self.sink.begin(self.total) {
                            self.push(progress(PH_REJECT, 0, self.total, sink_reason(e)));
                        } else {
                            self.pv = Some(PayloadVerifier::new(&vh));
                            self.push(progress(PH_START_OK, 0, self.total, 0));
                        }
                    }
                    Err(e) => self.push(progress(PH_REJECT, 0, 0, reject_reason(e))),
                }
            }
            OTA_CMD_ODT => {
                if let Some(pv) = self.pv.as_mut() {
                    // verify chunk (running hash, length-bounded) FIRST, then write.
                    match pv.update(data) {
                        Ok(()) => match self.sink.write_chunk(data) {
                            Ok(()) => {
                                self.done += data.len() as u32;
                                let (d, t) = (self.done, self.total);
                                self.push(progress(PH_DATA, d, t, 0));
                            }
                            Err(e) => {
                                self.pv = None;
                                self.sink.abort();
                                let (d, t) = (self.done, self.total);
                                self.push(progress(PH_REJECT, d, t, sink_reason(e)));
                            }
                        },
                        Err(e) => {
                            self.pv = None;
                            self.sink.abort();
                            let r = reject_reason(e);
                            let (d, t) = (self.done, self.total);
                            self.push(progress(PH_REJECT, d, t, r));
                        }
                    }
                }
            }
            OTA_CMD_OCM => {
                if let Some(pv) = self.pv.take() {
                    let (d, t) = (self.done, self.total);
                    match pv.finish() {
                        Ok(()) => {
                            self.push(progress(PH_VERIFIED, d, t, 0));
                            // hash-confirmed → activate. Else nothing is activated.
                            match self.sink.finalize() {
                                Ok(()) => self.push(progress(PH_APPLIED, d, t, 0)),
                                Err(e) => {
                                    self.sink.abort();
                                    self.push(progress(PH_REJECT, d, t, sink_reason(e)));
                                }
                            }
                        }
                        Err(e) => {
                            self.sink.abort();
                            self.push(progress(PH_REJECT, d, t, reject_reason(e)));
                        }
                    }
                }
            }
            _ => {}
        }
        PluginResult::Ok(PluginResponse::empty())
    }
    fn name(&self) -> &str {
        "ota"
    }
    fn id(&self) -> PluginId {
        0
    }
    fn poll(&mut self) -> Option<(u32, &[u8])> {
        if self.queue.is_empty() {
            return None;
        }
        self.last = self.queue.remove(0);
        Some((PROGRESS_HASH, &self.last))
    }
}

const OTA_CLASS: u32 = r2_fnv::fnv1a_32(b"ai.reality2.sentant.ota");
const OTA_SUBS: [u32; 4] = [OST_HASH, ODT_HASH, OCM_HASH, PROGRESS_HASH];

/// OTA control SENTANT — routes OST/ODT/OCM events to the OTA plugin, and
/// re-broadcasts the plugin's PROGRESS so peers / the bench observe the transfer.
pub struct OtaSentant {
    ota_plugin: PluginId,
}
impl OtaSentant {
    pub fn new(ota_plugin: PluginId) -> Self {
        Self { ota_plugin }
    }
}
impl Sentant for OtaSentant {
    fn handle_event(&mut self, event: &Event, actions: &mut ActionBuf) {
        let cmd = match event.hash {
            h if h == OST_HASH => OTA_CMD_OST,
            h if h == ODT_HASH => OTA_CMD_ODT,
            h if h == OCM_HASH => OTA_CMD_OCM,
            h if h == PROGRESS_HASH => {
                // The plugin produced progress (via poll → local event); broadcast it.
                actions.push(Action::Send {
                    target: Target::Broadcast,
                    event_hash: PROGRESS_HASH,
                    payload: PayloadBuf::from_slice(event.payload),
                });
                return;
            }
            _ => return,
        };
        actions.push(Action::PluginCall {
            plugin_id: self.ota_plugin,
            command: cmd,
            data: PayloadBuf::from_slice(event.payload),
        });
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

    fn drain_progress<S: FirmwareSink>(p: &mut OtaPlugin<S>) -> Vec<[u8; 10]> {
        let mut v = Vec::new();
        while let Some((h, pl)) = p.poll() {
            assert_eq!(h, PROGRESS_HASH);
            let mut a = [0u8; 10];
            a.copy_from_slice(pl);
            v.push(a);
        }
        v
    }

    #[test]
    fn ota_plugin_verifies_and_applies() {
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"FIRMWARE-IMAGE-BYTES-v2-the-new-build".repeat(8);
        let header = mint(&payload, 1, tg_pk);
        let sig = sk.sign(&header).to_bytes();

        let mut p = OtaPlugin::new(ota_cfg(tg_pk), MemSink::new());
        // OST = header(123)+sig(64)
        let mut ost = Vec::from(&header[..]);
        ost.extend_from_slice(&sig);
        p.execute(OTA_CMD_OST, &ost);
        // ODT chunks (200B like push_ota_l2cap DEFAULT_CHUNK)
        for chunk in payload.chunks(200) {
            p.execute(OTA_CMD_ODT, chunk);
        }
        p.execute(OTA_CMD_OCM, &[]);

        let progs = drain_progress(&mut p);
        let phases: Vec<u8> = progs.iter().map(|x| x[0]).collect();
        assert!(phases.contains(&PH_START_OK), "start ok: {phases:?}");
        assert!(phases.contains(&PH_VERIFIED), "verified: {phases:?}");
        assert!(phases.contains(&PH_APPLIED), "applied: {phases:?}");
        assert!(!phases.contains(&PH_REJECT), "no reject: {phases:?}");
        // The verified image was written to the sink (verify-before-write held).
        assert!(p.sink().finalized());
        assert_eq!(p.sink().image(), &payload[..]);
    }

    #[test]
    fn ota_plugin_rejects_tampered_payload() {
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"GOOD-IMAGE".repeat(20);
        let header = mint(&payload, 1, tg_pk);
        let sig = sk.sign(&header).to_bytes();

        let mut p = OtaPlugin::new(ota_cfg(tg_pk), MemSink::new());
        let mut ost = Vec::from(&header[..]);
        ost.extend_from_slice(&sig);
        p.execute(OTA_CMD_OST, &ost);
        // Tamper: flip a byte in the streamed payload — hash won't match the signed header.
        let mut bad = payload.clone();
        bad[5] ^= 0xFF;
        for chunk in bad.chunks(200) {
            p.execute(OTA_CMD_ODT, chunk);
        }
        p.execute(OTA_CMD_OCM, &[]);

        let phases: Vec<u8> = drain_progress(&mut p).iter().map(|x| x[0]).collect();
        assert!(phases.contains(&PH_REJECT), "tampered must reject: {phases:?}");
        assert!(!phases.contains(&PH_APPLIED), "must NOT apply: {phases:?}");
        // verify-before-write: a rejected image is never finalized/activated.
        assert!(!p.sink().finalized(), "bad image must not finalize");
    }

    #[test]
    fn ota_rejects_replayed_seq() {
        let sk = SigningKey::from_bytes(&[0xF0; 32]);
        let tg_pk = sk.verifying_key().to_bytes();
        let payload = b"x".repeat(50);
        let header = mint(&payload, 0, tg_pk); // seq 0 <= current_seq 0 → StaleSeq
        let sig = sk.sign(&header).to_bytes();
        let mut p = OtaPlugin::new(ota_cfg(tg_pk), MemSink::new());
        let mut ost = Vec::from(&header[..]);
        ost.extend_from_slice(&sig);
        p.execute(OTA_CMD_OST, &ost);
        let phases: Vec<u8> = drain_progress(&mut p).iter().map(|x| x[0]).collect();
        assert!(phases.contains(&PH_REJECT), "stale seq must reject at OST: {phases:?}");
    }
}
