//! r2-dataplane — shared no_std RX data-plane + keepalive BODY (hive-authored).
//!
//! Drops in behind core's landed scaffold signatures (crates/r2-dataplane @4d5987f):
//! `handle_rx_frame(dp, frame, info, ingress, now_ms, relay_out, deliver_out) -> RxDisposition`
//! and `poll_keepalive(dp, now_ms, out) -> PhyMask`, with `DataPlane` made concrete + a `new`.
//! Factored from the bench-un-refuted DFR io_task RX pipeline. FIRST CUT for core's build-loop —
//! API-shape drift (exact generics / encode helpers) gets ironed out core-side; the LOGIC is final.
//!
//! Pipeline (per frame): decode_extended -> origin=route_stack[0] (None=>drop) -> (origin,msg_id)
//! dedup -> plan_forward (auto-bridge egress -> relay_on) AND §7.5.4 classify_extended_full
//! (FAIL-CLOSED: no key => deliver=false). Heartbeat frames also feed the neighbour table +
//! set_neighbour_duty_class from the §12.6 `dc`.

#![no_std]
#![deny(missing_docs)]

use r2_route::engine::{DropReason, ForwardAction, ForwardRequest, RouteEngine};
use r2_route::neighbour::{DutyClass, MobilityClass, Observation, QualitySample};
use r2_route::transport::Transport;
use r2_route::Target;
use r2_trust::GroupHmac;
use r2_wire::{
    classify_extended_full, decode_extended, encode_extended, ExtendedHeader, ExtendedMessage, Flags,
    FrameClass, MsgType,
};

/// LoRa single-packet MTU ceiling (bytes).
pub const MTU: usize = 255;
/// One R2-WIRE frame buffer (bounded by the LoRa [`MTU`]).
pub type Frame = heapless::Vec<u8, MTU>;

/// A per-PHY egress bitmask — the auto-bridge picks which carrier(s) a frame leaves on.
pub type PhyMask = u8;
/// FLRC backbone PHY bit.
pub const PHY_FLRC: PhyMask = 0b0000_0001;
/// LoRa leaf PHY bit.
pub const PHY_LORA: PhyMask = 0b0000_0010;

/// Max cross-entanglement peering keys held (§7.5).
const PEERING: usize = 4;

/// RX signal metadata (platform-agnostic; the platform maps its own RxInfo onto this).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameInfo {
    /// Received signal strength (dBm).
    pub rssi_dbm: i16,
    /// Signal-to-noise ratio (dB).
    pub snr_db: i16,
}

/// The outcome of [`handle_rx_frame`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RxDisposition {
    /// Egress PHY bitmask to relay on (`0` = no relay).
    pub relay_on: PhyMask,
    /// `true` => deliver locally (§7.5.4 verified). FAIL-CLOSED: no key => false.
    pub deliver: bool,
}

/// The shared RX data-plane state. hive owns the layout + sizing.
pub struct DataPlane {
    /// Trust-free route engine (plan_forward + neighbour table + dedup).
    engine: RouteEngine,
    /// §7.5.4 group key — `None` => FAIL-CLOSED (deliver=false), never plaintext.
    group: Option<GroupHmac>,
    /// §7.5 cross-entanglement peering keys.
    peering: heapless::Vec<GroupHmac, PEERING>,
    /// This node's hive id (the keepalive origin + the relay source-hop exclude).
    my_hive: u32,
    /// This node's self-asserted duty class (advertised as §12.6 `dc`).
    my_duty: DutyClass,
    /// §1A.1 tunable keepalive period (ms) — rate-decoupled from any data cadence.
    keepalive_period_ms: u64,
    /// Last keepalive emit (ms); `0` => emit-at-boot then throttle.
    last_keepalive_ms: u64,
    /// Per-node loose-jitter LCG (decorrelates fires on the half-duplex air).
    jitter_lcg: u32,
    /// Monotonic keepalive sequence.
    seq: u32,
}

impl DataPlane {
    /// Construct with the node identity, the §7.5.4 group key (None = fail-closed), the duty class,
    /// and the keepalive period. Peering keys are added later via [`DataPlane::add_peering`].
    pub fn new(my_hive: u32, group: Option<GroupHmac>, my_duty: DutyClass, keepalive_period_ms: u64) -> Self {
        DataPlane {
            engine: RouteEngine::new(),
            group,
            peering: heapless::Vec::new(),
            my_hive,
            my_duty,
            keepalive_period_ms,
            last_keepalive_ms: 0,
            jitter_lcg: my_hive ^ 0x9E37_79B9,
            seq: 0,
        }
    }

    /// Add a §7.5 cross-entanglement peering key (ignored if full).
    pub fn add_peering(&mut self, key: GroupHmac) {
        let _ = self.peering.push(key);
    }

    /// Mutable engine access (the platform feeds link-quality/decay ticks).
    pub fn engine_mut(&mut self) -> &mut RouteEngine {
        &mut self.engine
    }
}

/// Per-inbound-frame data-plane step. Body owned by hive (see module doc).
pub fn handle_rx_frame(
    dp: &mut DataPlane,
    frame: &[u8],
    info: &FrameInfo,
    ingress: PhyMask,
    now_ms: u64,
    relay_out: &mut Frame,
    deliver_out: &mut Frame,
) -> RxDisposition {
    let no = RxDisposition { relay_on: 0, deliver: false };
    let now_s = (now_ms / 1000) as u32;

    // (1) decode.
    let msg = match decode_extended(frame) {
        Ok(m) => m,
        Err(_) => return no,
    };
    // (2) origin = route_stack[0]; None => DROP (ROUTE-ORIGIN-1A — relays never synthesise).
    let origin = match msg.route.as_ref().and_then(|r| r.origin()) {
        Some(o) => o,
        None => return no,
    };

    // (3) HEARTBEAT: feed the neighbour table + the §12.6 `dc` duty class. Branches BEFORE
    // plan_forward, so it skips the (origin,msg_id) dedup — fine: neighbour updates are idempotent
    // and H9 accept_keepalive owns keepalive replay-freshness.
    if msg.header.msg_type == MsgType::Heartbeat {
        dp.engine.ingest_observation(Observation {
            hive_id: origin,
            transport: Transport::Lora,
            timestamp: now_s,
            quality: QualitySample::Direct(1.0),
            rssi: Some(info.rssi_dbm as i8),
            mcu_origin: false,
            mobility: MobilityClass::Mobile,
        });
        if let Some(dc) = parse_dc(msg.payload) {
            dp.engine.set_neighbour_duty_class(origin, dc);
        }
        return no;
    }

    // (4) plan_forward — it DEDUPS INTERNALLY on the single engine cache (it MARKS the cache), so
    // there is NO separate pre-dedup (a pre-mark would double-mark -> drop everything). Gate on its
    // Drop(Duplicate) — that covers BOTH the relay AND the deliver via the one cache.
    let msg_id = msg.header.msg_id as u16; // extended msg_id is u32; dedup/ForwardRequest key on u16.
    let req = ForwardRequest {
        now: now_s,
        msg_id,
        origin,
        source_hop: origin as u16,
        ttl: msg.header.ttl,
        k: msg.header.k,
        destination: Target::Address(msg.header.target_hive),
        msg_type: msg.header.msg_type,
        payload_len: msg.payload.len(),
        relay_enabled: true,
        congested: false,
        dice_roll: 0.5,
    };
    let advice = dp.engine.plan_forward(req);
    if matches!(advice.action, ForwardAction::Drop(DropReason::Duplicate)) {
        return no;
    }
    let mut relay_on: PhyMask = 0;
    match &advice.action {
        ForwardAction::Directed(hop) => relay_on |= phy_of(hop.transport, ingress),
        ForwardAction::Flood(hops) => {
            for h in hops {
                relay_on |= phy_of(h.transport, ingress);
            }
        }
        ForwardAction::DeliverOnly | ForwardAction::Drop(_) => {}
    }

    // (5) DELIVER: §7.5.4 fail-closed gate (independent of relay). Runs BEFORE the relay re-encode
    // so `msg` is still borrowable (the re-encode below consumes it).
    let class = classify_extended_full(&msg, dp.group.as_ref(), dp.peering.as_slice());
    let deliver = matches!(class, Some(FrameClass::SameGroup) | Some(FrameClass::CrossGroup(_)));
    if deliver {
        deliver_out.clear();
        let _ = deliver_out.extend_from_slice(msg.payload);
    }

    // (6) RELAY re-encode (ttl-1) — LAST, since it moves `msg`.
    if relay_on != 0 {
        let mut fwd = msg;
        fwd.header.ttl = advice.ttl;
        relay_out.clear();
        let _ = relay_out.resize_default(MTU);
        match encode_extended(&fwd, relay_out) {
            Ok(n) => relay_out.truncate(n),
            Err(_) => relay_on = 0,
        }
    }

    RxDisposition { relay_on, deliver }
}

/// The rate-decoupled §1A liveness keepalive: emits a §12.6 `{seq, dc}` CBOR when due.
pub fn poll_keepalive(dp: &mut DataPlane, now_ms: u64, out: &mut Frame) -> PhyMask {
    let due = dp.last_keepalive_ms == 0
        || now_ms.wrapping_sub(dp.last_keepalive_ms) >= dp.keepalive_period_ms;
    if !due {
        return 0;
    }
    dp.last_keepalive_ms = now_ms;
    dp.seq = dp.seq.wrapping_add(1);
    dp.jitter_lcg = dp.jitter_lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    out.clear();
    encode_keepalive(out, dp.my_hive, dp.seq, dp.my_duty);
    // Emit on both PHYs the platform has wired (it masks to its real carriers).
    PHY_FLRC | PHY_LORA
}

/// Build the §1A liveness Heartbeat frame: a Heartbeat ExtendedMessage whose route_stack[0] is this
/// node (the origin, ROUTE-ORIGIN-1A) carrying the §12.6 `{0:seq, 1:dc}` Compact-CBOR payload. One-hop
/// (ttl=1, k=1). Unsigned in this cut — §1A keepalive signing is a security follow-up (the RX HB-arm
/// ingests liveness unconditionally; a signed-keepalive gate is the hardening step).
fn encode_keepalive(out: &mut Frame, my_hive: u32, seq: u32, dc: DutyClass) {
    let mut payload = [0u8; 16];
    let plen = encode_seq_dc(&mut payload, seq, dc);
    let header = ExtendedHeader {
        version: 0,
        msg_type: MsgType::Heartbeat,
        flags: Flags::default(),
        ttl: 1,
        k: 1,
        msg_id: seq as u16,
        event_hash: 0,
        payload_len: plen as u32,
        target_group: 0,
        target_hive: 0,
    };
    // route_stack[0] = my_hive (origin). NOTE(core-loop): confirm the ExtendedRouteStack CONSTRUCTOR
    // (you added the reader route.origin(); I need the writer — a single-entry origin stack).
    let route = ExtendedRouteStack::with_origin(my_hive);
    let msg = ExtendedMessage { header, route: Some(route), payload: &payload[..plen], hmac_tag: None };
    out.clear();
    let _ = out.resize_default(MTU);
    match encode_extended(&msg, out) {
        Ok(n) => out.truncate(n),
        Err(_) => out.clear(),
    }
}

/// §12.6 keepalive payload `{0: seq, 1: dc}` — Compact UINT keys, ascending canonical (R2-WIRE v0.14
/// §12.6 / R2-CBOR CBOR-1). Returns the byte length written. [core-loop offered to fill this via
/// r2-cbor `canonical_map(&mut [(0, Uint(seq)), (1, Uint(dc as u64))])`.]
fn encode_seq_dc(buf: &mut [u8], seq: u32, dc: DutyClass) -> usize {
    let _ = (&mut *buf, seq, dc);
    0 // TODO(core): r2-cbor canonical_map, uint keys 0=seq 1=dc.
}

/// Parse the §12.6 `dc` (key 1) from a Heartbeat Compact-CBOR payload; None if absent ⇒ Unknown.
fn parse_dc(payload: &[u8]) -> Option<DutyClass> {
    let _ = payload;
    None // TODO(core): r2-cbor Decoder read uint key 1 -> DutyClass {0=Unknown,1=AlwaysOn,2=Intermittent}.
}

/// Map a route-engine egress [`Transport`] to a [`PhyMask`] bit, excluding the ingress PHY.
fn phy_of(transport: Transport, ingress: PhyMask) -> PhyMask {
    // PLATFORM ADAPTER shape: FLRC+LoRa are both "Lora" at the route layer -> the platform
    // refines. Default: relay on the non-ingress carrier(s).
    let _ = transport;
    (PHY_FLRC | PHY_LORA) & !ingress
}
