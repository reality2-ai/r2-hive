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
}
