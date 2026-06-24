//! Router — R2-WIRE header parsing and route-engine-driven forwarding.
//!
//! Parses incoming R2-WIRE extended frames (header only, not payload),
//! feeds observations into the RouteEngine, and executes forwarding
//! decisions via the transport's PeerMap.
//!
//! **Layering (R2-INTRO trust boundary, R2-ROUTE §1.1):** routing operates at
//! L3, below the trust boundary. There are two distinct cases:
//!
//! 1. **Inter-trust-group routing** — relay traffic that crosses trust group
//!    boundaries, or traffic for a trust group this hive is not a member of.
//!    The router has no trust context here and must work without it. Decisions
//!    are based purely on the wire header, the engine's observed-path
//!    confidence, and the transport peer maps. This is the trustless shared
//!    mesh promise from R2-INTRO.
//!
//! 2. **Intra-trust-group routing** — when this hive is a member of the
//!    destination trust group, additional information becomes available
//!    (membership list, capability bloom filters, sentant locations, prior
//!    intra-group delivery history). This information may *enrich* routing
//!    decisions but does not change them at the base level. The enrichment is
//!    additive: the trust-agnostic decision is computed first, then the
//!    caller layers on TG-specific extras (e.g. flooding to TG members the
//!    engine hasn't yet observed).
//!
//! `route_frame` here implements case 1 — it is trust-agnostic in signature.
//! Case 2 enrichment is the caller's responsibility, performed on the
//! `RouteOutcome` returned. The `compat::handshake` handler has TG context
//! from the WebSocket auth and applies intra-TG enrichment on top of the base
//! decision. UDP and BLE inbound handlers do not have TG context for the
//! frames they relay and apply no enrichment.
//!
//! **Sources of routing hints (additive on top of the base engine):**
//!
//! 1. **Observed-path confidence** (R2-ROUTE §4) — the engine's own learning
//!    based on actual delivery success per neighbour and transport. Always
//!    available to the router. This is the default routing signal.
//!
//! 2. **Entanglement** (R2-CAP §12.5, R2-TRUST §7) — persistent, scoped
//!    peering with a heartbeat that reinforces route observations on a
//!    specific path, keeping it "warm" between active uses.
//!
//! There is a deeper unification waiting in the spec set that is worth
//! recording here as a future direction: **intra-trust-group membership and
//! inter-trust-group entanglement are structurally the same routing hint**.
//! Members of a trust group already exchange HEARTBEAT messages
//! (R2-WIRE §3.6), each of which is an observation that reinforces
//! observed-path confidence — that is exactly what entanglement does too.
//! Today the implementation has separate concepts (`tg_map` for intra-TG
//! membership, future entanglement table for inter-TG), but both produce the
//! same routing artifact: warm paths kept alive by heartbeats. A unified
//! model — "entanglement is the universal mechanism for keeping a path warm,
//! and a trust group is just every member implicitly entangled with every
//! other member" — would collapse them into one routing input. R2-TRUST and
//! R2-CAP §12 would need a small clarification to make this explicit, but
//! the routing engine would get a single, consistent enrichment hook instead
//! of two parallel ones.
//!
//! Currently, the only enrichment hook implemented is `flood_tg_peers_not_in`
//! applied by `compat::handshake` after the WS auth — the intra-TG case. The
//! inter-TG entanglement hook will join it when entanglements are
//! implemented, and the unified collapse can happen as a follow-up cleanup.
//!
//! Caller responsibilities for non-routing fallbacks:
//! - Legacy 0xFF join messages on the WebSocket compat path → caller falls
//!   back to `state.broadcast_to_tg()` on `RouteOutcome::NotR2Wire`
//! - Anything else with trust-group-specific semantics → caller decides

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use r2_route::engine::{DirectedHop, ForwardAction, ForwardRequest, Target};
use r2_route::neighbour::{MobilityClass, Observation};
use r2_route::transport::{QualitySample, Transport};
use r2_wire::extended::{decode_extended, prepare_relay_extended};
use r2_wire::hmac::{classify_extended_full, FrameClass};
use r2_wire::types::WireError;
use r2_trust::wire_hmac::GroupHmac;

use crate::hive::HiveState;

/// What the router did with a frame, returned so callers can add their own
/// transport-specific or trust-group-specific behaviour on top.
pub enum RouteOutcome {
    /// Frame did not parse as R2-WIRE. The caller decides what to do
    /// (e.g. WS handshake handler falls back to legacy broadcast for 0xFF
    /// join messages; UDP/BLE callers simply drop).
    NotR2Wire,
    /// Engine decided to drop the frame (TTL exhausted, dedup hit, etc.).
    Dropped,
    /// Engine decided this frame is for local delivery only.
    DeliverOnly,
    /// Engine forwarded the frame to a single directed neighbour.
    Directed,
    /// Engine flooded the frame to a set of hops. The caller MAY consult the
    /// hops list to add transport- or trust-group-specific extras (e.g. flood
    /// to freshly-connected TG peers the engine doesn't yet know about).
    Flooded(Vec<DirectedHop>),
}

/// Route an inbound frame: parse R2-WIRE header, feed the route engine,
/// execute the forwarding decision via `state.send_to_hive` (which uses the
/// multi-transport fallback chain).
///
/// This function is **trust-agnostic** — it does not consult or require any
/// trust group context. Callers that have trust group context (e.g. the
/// WebSocket handshake handler) may use the returned `RouteOutcome` to add
/// trust-group-specific fallbacks.
pub async fn route_frame(
    state: &Arc<HiveState>,
    source_hive: u32,
    transport: Transport,
    frame: &[u8],
) -> RouteOutcome {
    // Try to parse as R2-WIRE extended. Frame may include 32-byte HMAC tag.
    // We need the FULL message (not just header) so we can read the route
    // stack to identify the originator for dedup (R2-WIRE §8.2).
    let trimmed = if let Ok(_) = decode_extended(frame) {
        frame
    } else if frame.len() > 32 {
        if decode_extended(&frame[..frame.len() - 32]).is_ok() {
            &frame[..frame.len() - 32]
        } else {
            return RouteOutcome::NotR2Wire;
        }
    } else {
        return RouteOutcome::NotR2Wire;
    };
    let msg = match decode_extended(trimmed) {
        Ok(m) => m,
        Err(_) => return RouteOutcome::NotR2Wire,
    };
    let header = msg.header;

    // R2-WIRE §8.2: dedup key is (msg_id, originator) — originator is route_stack[0] (the
    // frame-carried origin).
    //
    // ROUTE-ORIGIN-1 (RATIFIED — R2-WIRE §9.5/§9.6, R2-ROUTE v0.14 §3.3): a route-less (R=0 /
    // route=None) frame has NO authentic originator, and a relay MUST NOT synthesise route_stack[0].
    // EARLY-DROP it here — BEFORE the (msg_id,origin) dedup (a fabricated origin would POISON the
    // dedup cache so each vantage re-forwards = relay amplification the gateless relay can't catch)
    // and BEFORE the neighbour-observe below (a route-less frame must not seed the engine). This
    // SUPERSEDES the transitional (B) frame-fingerprint dedup (event_hash ^ target_hive), now DEAD:
    // the mandate-route_stack[0]+drop ruling (A) subsumes it, and r2-wire (6e0aea4) backs it — decode
    // gives route=None + verify_extended returns false on a route-less frame.
    let originator = match &msg.route {
        Some(r) if r.len > 0 => r.entries[0],
        _ => return RouteOutcome::Dropped,
    };

    // Immediate source — the peer we just heard from. The transport layer
    // may not know this (broadcast mediums like LoRa report 0). When that
    // happens, fall back to the LAST entry of the route stack, which by
    // R2-WIRE §8.3 is the most recent relayer (or the originator itself on
    // first hop).
    let immediate_source = if source_hive != 0 {
        source_hive
    } else {
        match &msg.route {
            Some(r) if r.len > 0 => r.entries[(r.len - 1) as usize],
            _ => source_hive,
        }
    };

    let now_secs = now_monotonic();

    // Feed observation to route engine — based on the IMMEDIATE source (the
    // peer we just heard from), not the originator. The engine learns about
    // direct neighbours, not end-to-end paths.
    {
        let mut engine = state.route_engine.lock().await;
        let obs = Observation {
            hive_id: immediate_source,
            transport,
            timestamp: now_secs,
            quality: QualitySample::Direct(0.9),
            rssi: None,
            mcu_origin: header.flags.mcu_origin,
            mobility: MobilityClass::Infrastructure,
        };
        engine.ingest_observation(obs);
    }

    // Re-fan to mgmt-API subscribers (R2-HOST-API §3.2 event.delivery /
    // §4 subscription mechanics). Frames matching any active subscription
    // get pushed to that connection's outbound channel. Source TG is not
    // recoverable from the v0.1 wire frame (only the 4-byte target_group
    // is on the wire); from_tg subscription filters therefore won't match
    // until the L5 trust path provides full TG context here.
    //
    // R2-TRUST §7.5.4 DELIVER-GATE — verify GroupHmac before LOCAL dispatch.
    // The gate is tier/transport-agnostic (this is the LAN/Internet tier; the MCU
    // tier verifies in firmware). It guards DELIVERY only — the relay/forward path
    // below is untouched (trust-agnostic carry, §7.5.4). Classify against the
    // frame's target-group key:
    //   SameGroup / CrossGroup -> verified -> deliver.
    //   None -> a tag is present + we hold the key but nothing verifies = forgery
    //           aimed at us -> DROP (do not deliver).
    //   Relay -> we hold no key for this TG -> transit -> don't deliver (the relay
    //            path forwards it opaquely).
    //   Unauthenticated -> no tag while we hold keys -> drop.
    // EMPTY group_hmacs = migration mode (no keys configured) -> deliver + LOUD
    // warn, so existing no-key daemons don't break (production MUST configure HK).
    // §7.5.4 deliver-gate + A1 authenticated flag: classify the frame ONCE (against the frame's
    // target-group key), then derive both the delivery decision and the dedup-record gate below.
    let class = if state.group_hmacs.is_empty() {
        None // no group keys configured (dev/migration) — nothing can be authenticated
    } else {
        classify_extended_full(
            &msg,
            state.group_hmacs.get(&header.target_group),
            &[] as &[GroupHmac], // cross-TG peering = live entanglement table (follow-up)
        )
    };
    // A1 (verify-then-record): the (origin,msg_id) dedup is RECORDED only for a GroupHmac-VERIFIED
    // frame — a keyless forged frame must not poison the cache (else each vantage re-forwards).
    let authenticated = matches!(
        class,
        Some(FrameClass::SameGroup) | Some(FrameClass::CrossGroup(_))
    );
    let gate_deliver = if state.group_hmacs.is_empty() {
        log::warn!(
            "§7.5.4 deliver-gate INACTIVE (no group keys configured) — delivering UNVERIFIED \
             msg_id={} tg={:08x} (dev/migration; production MUST configure a sealed HK)",
            header.msg_id, header.target_group
        );
        true
    } else {
        match class {
            None => log::warn!(
                "§7.5.4 DROP: forgery — tag present, no key verifies for tg={:08x} (msg_id={})",
                header.target_group, header.msg_id
            ),
            Some(FrameClass::Unauthenticated) => log::warn!(
                "§7.5.4 drop: untagged frame for tg={:08x} while holding keys (msg_id={})",
                header.target_group, header.msg_id
            ),
            Some(FrameClass::Relay) => {} // transit (no key for this TG) — relay forwards, don't deliver
            _ => {}
        }
        gate_should_deliver(false, class)
    };
    if gate_deliver {
        state.deliver_inbound(trimmed, originator, None).await;
    }

    // Build forwarding request
    let destination = if header.target_group != 0 {
        Target::from(header.target_group)
    } else {
        Target::from(header.target_hive)
    };

    let req = ForwardRequest {
        now: now_secs,
        msg_id: header.msg_id, // full 32-bit dedup id (F3: u16 made (origin,msg_id) collisions cheap)
        origin: originator,
        source_hop: immediate_source, // the IMMEDIATE sender, to exclude the inbound peer (F2)
        authenticated,                // A1: dedup recorded only for a verified frame
        ttl: header.ttl,
        k: header.k,
        destination,
        msg_type: header.msg_type,
        payload_len: frame.len(),
        relay_enabled: true,
        congested: false,
        dice_roll: pseudo_random(),
    };

    // Get forwarding decision
    let advice = {
        let mut engine = state.route_engine.lock().await;
        engine.plan_forward(req)
    };

    // For Directed/Flood we need to mutate the frame for relay (R2-WIRE §8.3,
    // §8.4, §9.2): decrement TTL, halve K, append our hive_id to route stack,
    // set R flag. This is encapsulated in r2-wire's prepare_relay_extended.
    // If preparation fails (TTL exhausted, route stack full), we drop.
    let prepare_relay = || -> Result<Vec<u8>, WireError> {
        prepare_relay_extended(trimmed, state.self_hive_id, source_hive)
    };

    // Execute the decision and return what happened
    match advice.action {
        ForwardAction::Drop(reason) => {
            log::debug!(
                "route: DROP({:?}) from=0x{:08X} ttl={} k={}",
                reason, source_hive, header.ttl, header.k
            );
            RouteOutcome::Dropped
        }
        ForwardAction::DeliverOnly => {
            log::debug!("route: DELIVER_ONLY from=0x{:08X}", source_hive);
            // Hand to the ensemble registry's DispatchTarget impl so any
            // loaded sentants whose subscriptions match get the event.
            // Errors here are non-fatal — `NoHandler` just means no
            // ensemble cares; the route engine has already done its
            // work.
            let envelope = r2_dispatch::DispatchEnvelope {
                originator,
                target_hive: header.target_hive,
                target_group: header.target_group,
                event_hash: header.event_hash,
                payload: msg.payload,
                msg_id: header.msg_id,
                mcu_origin: header.flags.mcu_origin,
                received_at: now_secs as u32,
                trust_group: None,
            };
            use r2_dispatch::DispatchTarget;
            let _ = state.ensembles.dispatch(envelope).await;
            RouteOutcome::DeliverOnly
        }
        ForwardAction::Directed(hop) => {
            match prepare_relay() {
                Ok(relayed) => {
                    let used = state
                        .send_to_hive_via(hop.neighbour, Some(hop.transport), &relayed)
                        .await;
                    match used {
                        Some(t) => {
                            reinforce_delivery(state, hop.neighbour, t, now_secs).await;
                            log::info!(
                                "decision: dst=0x{:08X} via=0x{:08X} hint={:?} used={:?} conf={:.2} outcome=ok",
                                header.target_hive, hop.neighbour, hop.transport, t, hop.confidence
                            );
                        }
                        None => {
                            log::info!(
                                "decision: dst=0x{:08X} via=0x{:08X} hint={:?} used=none conf={:.2} outcome=fail",
                                header.target_hive, hop.neighbour, hop.transport, hop.confidence
                            );
                        }
                    }
                }
                Err(e) => {
                    log::info!(
                        "decision: dst=0x{:08X} via=0x{:08X} hint={:?} outcome=drop reason={:?}",
                        header.target_hive, hop.neighbour, hop.transport, e
                    );
                }
            }
            RouteOutcome::Directed
        }
        ForwardAction::Flood(hops) => {
            let hops_owned: Vec<DirectedHop> = hops.iter().copied().collect();
            match prepare_relay() {
                Ok(relayed) => {
                    for hop in &hops_owned {
                        if hop.neighbour != source_hive {
                            let used = state
                                .send_to_hive_via(hop.neighbour, Some(hop.transport), &relayed)
                                .await;
                            match used {
                                Some(t) => {
                                    reinforce_delivery(state, hop.neighbour, t, now_secs).await;
                                    log::info!(
                                        "decision: dst=0x{:08X} via=0x{:08X} hint={:?} used={:?} conf={:.2} outcome=flood-ok",
                                        header.target_hive, hop.neighbour, hop.transport, t, hop.confidence
                                    );
                                }
                                None => {
                                    log::info!(
                                        "decision: dst=0x{:08X} via=0x{:08X} hint={:?} used=none conf={:.2} outcome=flood-fail",
                                        header.target_hive, hop.neighbour, hop.transport, hop.confidence
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::info!(
                        "decision: dst=0x{:08X} outcome=flood-drop reason={:?} hops={}",
                        header.target_hive, e, hops_owned.len()
                    );
                }
            }
            RouteOutcome::Flooded(hops_owned)
        }
    }
}

/// Reinforce the route engine after a successful outbound delivery: update the
/// neighbour table for the transport that worked (feeds the EWMA used by
/// `best_transport()`), and mark the path to that neighbour as positive.
async fn reinforce_delivery(
    state: &Arc<HiveState>,
    neighbour: u32,
    transport: Transport,
    now_secs: u32,
) {
    let mut engine = state.route_engine.lock().await;
    engine.ingest_observation(Observation {
        hive_id: neighbour,
        transport,
        timestamp: now_secs,
        quality: QualitySample::Direct(0.95),
        rssi: None,
        mcu_origin: false,
        mobility: MobilityClass::Infrastructure,
    });
    engine.record_delivery_success(neighbour, neighbour, now_secs);
}

/// Log route engine neighbour table state.
pub async fn log_neighbours(state: &Arc<HiveState>) {
    let engine = state.route_engine.lock().await;
    let table = engine.neighbours();
    let count = table.len();
    if count > 0 {
        log::info!("route-engine: {} neighbours tracked", count);
        for entry in table.iter() {
            log::info!(
                "  hive=0x{:08X} conf={:.3} transports={:?} last_seen={}s ago samples={}",
                entry.hive_id, entry.confidence,
                entry.transports, now_monotonic().saturating_sub(entry.last_seen),
                entry.sample_count
            );
        }
    }
}

/// Periodic route engine maintenance (decay + logging).
pub async fn maintenance_loop(state: Arc<HiveState>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        let now = now_monotonic();
        {
            let mut engine = state.route_engine.lock().await;
            engine.decay_neighbours(now);
            engine.decay_paths(now);
        }
        log_neighbours(&state).await;
    }
}

fn now_monotonic() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

fn pseudo_random() -> f32 {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (t % 1000) as f32 / 1000.0
}

/// §7.5.4 deliver decision (pure, testable): deliver iff the frame VERIFIED
/// (SameGroup / CrossGroup). When no group keys are configured (migration mode)
/// deliver everything — the caller logs the UNVERIFIED warning — so existing
/// no-key daemons keep working. A forgery (`None`), transit (`Relay`, no key for
/// that TG), or untagged (`Unauthenticated`) frame is NOT delivered.
fn gate_should_deliver(keys_empty: bool, class: Option<FrameClass>) -> bool {
    if keys_empty {
        return true; // migration mode — no keys configured
    }
    matches!(
        class,
        Some(FrameClass::SameGroup) | Some(FrameClass::CrossGroup(_))
    )
}

#[cfg(test)]
mod gate_tests {
    use super::*;

    #[test]
    fn migration_mode_delivers_everything() {
        // No keys configured -> deliver regardless of class (back-compat).
        assert!(gate_should_deliver(true, None));
        assert!(gate_should_deliver(true, Some(FrameClass::SameGroup)));
        assert!(gate_should_deliver(true, Some(FrameClass::Unauthenticated)));
    }

    #[test]
    fn enforcing_delivers_only_verified() {
        assert!(gate_should_deliver(false, Some(FrameClass::SameGroup)));
        assert!(gate_should_deliver(false, Some(FrameClass::CrossGroup(0))));
    }

    #[test]
    fn enforcing_drops_forgery_transit_and_untagged() {
        assert!(!gate_should_deliver(false, None)); // forgery aimed at us -> DROP
        assert!(!gate_should_deliver(false, Some(FrameClass::Relay))); // transit (no key) -> don't deliver
        assert!(!gate_should_deliver(false, Some(FrameClass::Unauthenticated))); // untagged -> drop
    }
}
