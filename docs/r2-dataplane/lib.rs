//! r2-dataplane — the shared no_std RX data-plane + keepalive core for R2 firmware.
//!
//! HIVE-OWNED BODY (this crate's content), CORE-OWNED REPO (registered as a r2-core
//! workspace member + the per-platform call-site wired by core). Both the nRF54-LR2021
//! gateway window-scheduler AND the DFR leaf io_task call the SAME two entry points
//! (`handle_rx_frame` + `poll_keepalive`) — "A-with-reuse" (composer 876bb56): the
//! platform task owns the I/O (radio/channels/LED/clock/windows); this crate owns the
//! routing+trust LOGIC + state. One codebase, sim-testable RX pipeline (not just metal).
//!
//! WHY ITS OWN CRATE (core's location call): it depends on r2-trust (the §7.5.4
//! deliver-gate) AND r2-wire AND r2-route — but r2-route is deliberately trust-agnostic
//! ("no keys in the engine" / plane separation), so the deliver-gate ORCHESTRATION
//! cannot live in r2-route; it is a composition layer ABOVE the trust-free engine.
//!
//! STATUS: API CONTRACT scaffold (types + signatures STABLE so core can register + wire
//! the nRF54 call-site NOW, no bench needed). The fn BODIES are `todo!()` and land
//! POST-BENCH — factored from the bench-VALIDATED DFR io_task RX logic so the nRF54
//! reuses proven code, not pre-bench churn. (hive, 2026-06-24.)

#![no_std]

// Deps (Cargo.toml): r2-wire (decode_extended, classify_extended_full, FrameClass),
// r2-route (RouteEngine, ForwardRequest/Advice, Observation, DedupCache), r2-trust (GroupHmac).
// NOTE: r2-dataplane does NOT dep r2-dispatch (std/above-L4-L5) — the deliver boundary is a RAW
// channel push (deliver_out), and the CONSUMER composes it (MCU -> local engine; host -> r2-dispatch).

/// Platform-agnostic egress bitmask: bit `i` = the platform's TX-channel `i`.
/// nRF54 {bit0=LoRa, bit1=FLRC}; DFR {bit0=LoRa, bit1=ESP-NOW}. The mapping from
/// r2-route's chosen egress *transport* to a `PhyMask` bit is the PLATFORM ADAPTER —
/// it keeps this crate PHY-agnostic (NOT r2-route's `TransportSet`: FLRC+LoRa are both
/// "Lora" at the route layer but distinct platform egress channels).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PhyMask(pub u8);

impl PhyMask {
    pub const fn empty() -> Self { PhyMask(0) }
    pub const fn one(bit: u8) -> Self { PhyMask(1 << bit) }
    pub fn set(&mut self, bit: u8) { self.0 |= 1 << bit; }
    pub const fn is_set(&self, bit: u8) -> bool { self.0 & (1 << bit) != 0 }
    pub const fn any(&self) -> bool { self.0 != 0 }
}

/// Outcome of [`DataPlane::handle_rx_frame`]. The platform sends `relay_out[..relay_len]`
/// on every PHY bit in `relay_on`, and (iff `deliver`) pushes `deliver_out[..deliver_len]`
/// to its inbound DATA_RX channel. Both buffers are caller-provided.
#[derive(Clone, Copy, Debug, Default)]
pub struct RxDisposition {
    /// PHYs to re-broadcast the relay frame on (auto-bridge egress). Empty = no relay.
    pub relay_on: PhyMask,
    /// Bytes written to `relay_out` (valid iff `relay_on.any()`).
    pub relay_len: usize,
    /// `deliver_out` holds a §7.5.4-VERIFIED for-me frame. FAIL-CLOSED: false if no key.
    pub deliver: bool,
    /// Bytes written to `deliver_out` (valid iff `deliver`).
    pub deliver_len: usize,
}

/// The shared data-plane state. hive owns the const-generic sizing (`NEIGHBOURS`/`DEDUP`)
/// — sized per platform RAM. Holds the trust-free route engine + the dedup cache + the
/// group key (None => FAIL-CLOSED deliver) + the rate-decoupled v0.5 keepalive state.
///
/// FIELD TYPES finalize at integration against the exact r2-route generics (core owns the
/// API); shown here as the contract shape.
pub struct DataPlane</* const NEIGHBOURS: usize, const DEDUP: usize */> {
    // engine: r2_route::RouteEngine<NEIGHBOURS, ...>,   // the trust-free plan_forward engine
    // dedup:  r2_route::DedupCache<DEDUP>,               // (origin, msg_id) transport-agnostic
    // group:  Option<r2_trust::GroupHmac>,               // None => no key => deliver=false (fail-closed)
    // peering: heapless::Vec<r2_trust::GroupHmac, P>,    // §7.5 cross-entanglement peering keys
    // --- rate-decoupled v0.5 keepalive (own timer + loose jitter, decoupled from any data cadence) ---
    // my_hive: u32,
    // keepalive_period_ms: u64,   // §1A.1 tunable (tier-aware; ~30s always-on)
    // last_keepalive_ms: u64,     // 0 = emit-at-boot then throttle
    // jitter_lcg: u32,            // per-node loose jitter seed (my_hive)
    _private: (),
}

impl DataPlane {
    /// THE RX PIPELINE — factored POST-BENCH from the validated DFR io_task:
    ///   1. `decode_extended(frame)`                          (r2-wire; malformed => drop)
    ///   2. origin = `route_stack[0]` — `None` => DROP        (ROUTE-ORIGIN-1A; relays never synthesize)
    ///   3. `(origin, msg_id)` dedup                          (r2-route DedupCache; transport-agnostic)
    ///   4. RELAY: `plan_forward` (ingress in, engine picks egress) -> ttl-1 -> `relay_out` + `relay_on`
    ///      (auto-bridge: the egress transport -> PhyMask bit via the platform adapter)
    ///   5. DELIVER: `classify_extended_full(msg, group, peering)` -> SameGroup/CrossGroup => `deliver_out`
    ///      FAIL-CLOSED: no key / Relay / Unauthenticated => `deliver=false` (never plaintext).
    ///
    /// `ingress` = the platform PHY index the frame arrived on (so egress can exclude it).
    /// `rssi` feeds the neighbour `Observation` quality (link-quality seed).
    pub fn handle_rx_frame(
        &mut self,
        frame: &[u8],
        rssi: Option<i8>,
        ingress: u8,
        now_ms: u64,
        relay_out: &mut [u8],
        deliver_out: &mut [u8],
    ) -> RxDisposition {
        let _ = (frame, rssi, ingress, now_ms, relay_out, deliver_out);
        todo!("POST-BENCH: factor the bench-validated DFR io_task RX pipeline here")
    }

    /// The rate-decoupled R2-HEARTBEAT v0.5 keepalive: when `now_ms` crosses the node's
    /// own (period + loose-jitter) interval, fills `out` with the §1A keepalive frame and
    /// returns the PHYs to emit it on; otherwise returns an EMPTY mask (not due). Decoupled
    /// from any data-plane cadence (its own timer). Returns `(mask, len)`; `len` valid iff
    /// `mask.any()`.
    pub fn poll_keepalive(&mut self, now_ms: u64, out: &mut [u8]) -> (PhyMask, usize) {
        let _ = (now_ms, out);
        todo!("POST-BENCH: the v0.5 loose-jittered keepalive emit")
    }
}
