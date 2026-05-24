//! Handlers for the R2-HOST-API `r2.api.*` application surface (R2-HOST-API §3).
//!
//! Each `handle_*` function is invoked by the api dispatcher when an inbound
//! frame matches its event hash. The function reads the request payload,
//! consults `HiveState` (route engine, transports, etc.) for the answer,
//! and returns a fully-encoded response frame.
//!
//! Phase 1 wires the handlers in. Specific event implementations land
//! incrementally; events not yet backed by daemon state return an
//! `unsupported` error frame per R2-HOST-API §6.2.

use std::sync::atomic::{AtomicU32, Ordering};

use r2_cbor::{Decoder, Encoder, Item, Value};
use r2_fnv::r2_hash;
use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

use super::api::{
    build_error_response, build_response_frame_with_event, extract_correlation_id,
    EV_CAP_QUERY, EV_EVENT_SEND, EV_EVENT_SUBSCRIBE, EV_EVENT_UNSUBSCRIBE, EV_PEER_LIST,
    EV_PEER_QUERY, EV_TG_CURRENT,
};
use super::state::DaemonState;
use super::subscriptions::{SubscriptionFilter, SubscriptionRegistry};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Monotonic msg_id counter for outbound mesh frames produced via
/// `r2.api.event.send`. v0.1 uses a process-global counter; if the daemon
/// restarts the counter resets, which is fine because msg_id only needs
/// to be unique within a recent window for dedup.
static OUTBOUND_MSG_ID: AtomicU32 = AtomicU32::new(1);

/// Default TTL for outbound mesh frames when the request does not specify
/// one. R2-WIRE doesn't pin a single default; 5 hops is what the route
/// engine uses elsewhere in this daemon.
const DEFAULT_TTL: u8 = 5;

/// Peer status values for `r2.api.peer.query` responses (R2-HOST-API §3.2).
const PEER_STATUS_UNKNOWN: u64 = 0;
const PEER_STATUS_SELF: u64 = 1;
const PEER_STATUS_NEIGHBOUR: u64 = 2;
// 3 (entangled) and 4 (relayed-only) deferred — entanglement state not yet tracked.

/// Stable transport-name strings used in `r2.api.peer.query` responses.
/// Pinned in R2-HOST-API §3.2 (peer.query response, key 4).
const TRANSPORT_NAMES: &[(&str, r2_route::transport::Transport)] = &[
    ("ble", r2_route::transport::Transport::Ble),
    ("wifi", r2_route::transport::Transport::Wifi),
    ("lora", r2_route::transport::Transport::Lora),
    ("internet", r2_route::transport::Transport::Internet),
];

/// Maximum number of peers we'll return in a single `r2.api.peer.list`
/// response. Matches the route engine's neighbour table capacity (64) so we
/// never need to truncate in v0.1; if that capacity grows the responder
/// should chunk via correlation_id-paired follow-ups (deferred to v0.2).
const PEER_LIST_MAX: usize = 64;

/// Handle `r2.api.peer.list` — return all hive_ids the route engine has
/// observed in the active TG. Per R2-HOST-API §3.2, `self_hive_id` is
/// included.
///
/// v0.1 simplification: the route engine doesn't currently scope peers by
/// trust group, so we return every neighbour in the table. This matches the
/// daemon's v0.1 single-active-TG model. Phase 2 will add per-TG filtering
/// once `HiveState` carries an active-TG attachment.
pub async fn handle_peer_list(payload: &[u8], state: &DaemonState) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let hive_state = match state.hive_state() {
        Some(hs) => hs,
        None => {
            // Mgmt-only daemon (no L1–L4 stack): no peer list available.
            // Return an empty list rather than an error — the operation is
            // legitimate, there are simply no peers to report.
            return build_peer_list_response(correlation_id, &[]);
        }
    };

    // Collect peers from the route engine, plus self.
    let engine = hive_state.route_engine.lock().await;
    let mut peers: Vec<u64> = Vec::with_capacity(PEER_LIST_MAX + 1);
    peers.push(hive_state.self_hive_id as u64);
    for entry in engine.neighbours().iter() {
        peers.push(entry.hive_id as u64);
        if peers.len() >= PEER_LIST_MAX + 1 {
            break;
        }
    }
    drop(engine);

    build_peer_list_response(correlation_id, &peers)
}

/// Build an `r2.api.peer.list` response frame.
fn build_peer_list_response(correlation_id: u64, peers: &[u64]) -> Vec<u8> {
    // Pre-size payload buffer: header + correlation_id + array header + N * 9
    // bytes for uint64-tagged entries. 16 + 9*64 = 592, round up.
    let mut payload = vec![0u8; 1024];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map header");
        enc.kv(0u64, &Value::UInt(correlation_id))
            .expect("correlation_id");
        // We need to emit `key 1 -> array of uints`. r2-cbor's Encoder::kv
        // supports scalar Value variants, but emitting an array requires
        // dropping down to lower-level methods. Encode the key manually,
        // then the array header, then each element.
        enc.uint(1).expect("array key");
        enc.array(peers.len()).expect("array header");
        for p in peers {
            enc.uint(*p).expect("peer uint");
        }
        enc.len()
    };
    build_response_frame_with_event(EV_PEER_LIST, &payload[..used])
}

/// Handle `r2.api.event.send` (R2-HOST-API §3.2). Reads the event_class
/// (key 1), payload bytes (key 2), and optional target_hive (key 3),
/// target_class (key 4), and ttl (key 5). Builds an outbound R2-WIRE
/// extended frame and dispatches it.
///
/// v0.1 simplifications:
/// - target_hive specified → `HiveState::send_to_hive_via` (route-engine
///   decides transport). Returns `peer_not_found` if every transport
///   refuses the frame.
/// - target_hive omitted → broadcasts to every observably-connected
///   WebSocket peer, and additionally to UDP/BLE peers when those
///   transports are up. Active-TG-scoped broadcast lands when the active-TG
///   slot is added to HiveState.
/// - target_class is preserved but not yet enforced; receiving daemons
///   filter on their side.
/// - HMAC tag absent (local mgmt frames don't cross the trust boundary).
pub async fn handle_event_send(payload: &[u8], state: &DaemonState) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let req = match parse_event_send_request(payload) {
        Ok(r) => r,
        Err(_) => return build_error_response(correlation_id, "bad_payload"),
    };
    if req.event_class.is_empty() {
        return build_error_response(correlation_id, "bad_payload");
    }

    let Some(hive_state) = state.hive_state() else {
        return build_error_response(correlation_id, "unsupported");
    };

    let event_hash = match r2_hash(&req.event_class) {
        Ok(h) => h,
        Err(_) => return build_error_response(correlation_id, "bad_payload"),
    };

    let msg_id = OUTBOUND_MSG_ID.fetch_add(1, Ordering::Relaxed);
    let ttl = req.ttl.unwrap_or(DEFAULT_TTL);

    // If an active TG is attached, scope the outbound frame to it via
    // target_group (first 4 bytes of the TG hash). Otherwise leave it 0
    // and fall back to the permissive fan-out path below.
    let active = hive_state.active_tg().await;
    let target_group = active
        .as_ref()
        .map(|tg| u32::from_be_bytes([tg.tg_hash[0], tg.tg_hash[1], tg.tg_hash[2], tg.tg_hash[3]]))
        .unwrap_or(0);

    let outbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl,
            k: 0,
            msg_id,
            event_hash,
            payload_len: req.payload.len() as u32,
            target_group,
            target_hive: req.target_hive.map(|h| h as u32).unwrap_or(0),
        },
        route: None,
        payload: &req.payload,
        hmac_tag: None,
    };
    let mut wire = vec![0u8; req.payload.len() + 64];
    let n = match encode_extended(&outbound, &mut wire) {
        Ok(n) => n,
        Err(_) => return build_error_response(correlation_id, "bad_payload"),
    };
    wire.truncate(n);

    // Dispatch.
    if let Some(target_hive) = req.target_hive {
        let delivered = hive_state
            .send_to_hive_via(target_hive as u32, None, &wire)
            .await
            .is_some();
        if !delivered {
            return build_error_response(correlation_id, "peer_not_found");
        }
    } else if let Some(tg) = active.as_ref() {
        // TG-scoped broadcast (R2-HOST-API §3.2). Delivers to peers
        // registered with the TG via register_tg_peer; peers learned only
        // through opportunistic neighbour observations are not on this
        // path. The sender hive_id excludes self from the broadcast set.
        hive_state
            .broadcast_to_tg(&tg.tg_hash, hive_state.self_hive_id, &wire)
            .await;
    } else {
        // No active TG: permissive fan-out across observably-connected
        // peers. Used for v0.1 testing and for direct consumers that
        // haven't created or joined a TG yet.
        let hive_ids = hive_state.ws_transport.peers().hive_ids().await;
        for hive_id in hive_ids {
            let _ = hive_state.ws_transport.peers().send(hive_id, &wire).await;
        }
        let engine = hive_state.route_engine.lock().await;
        let other_peers: Vec<u32> = engine
            .neighbours()
            .iter()
            .map(|e| e.hive_id)
            .filter(|h| *h != hive_state.self_hive_id)
            .collect();
        drop(engine);
        for hive_id in other_peers {
            let _ = hive_state.send_to_hive_via(hive_id, None, &wire).await;
        }
    }

    build_event_send_response(correlation_id, msg_id as u64)
}

/// Decoded `r2.api.event.send` request.
struct EventSendRequest {
    event_class: String,
    payload: Vec<u8>,
    target_hive: Option<u64>,
    #[allow(dead_code)] // Preserved for future class-level filtering at receiver.
    target_class: Option<String>,
    ttl: Option<u8>,
}

fn parse_event_send_request(payload: &[u8]) -> Result<EventSendRequest, String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut event_class = String::new();
    let mut inner_payload: Vec<u8> = Vec::new();
    let mut target_hive: Option<u64> = None;
    let mut target_class: Option<String> = None;
    let mut ttl: Option<u8> = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(_)) => {} // correlation_id consumed elsewhere
            (Item::UInt(1), Item::Text(b)) => {
                event_class = std::str::from_utf8(b).map_err(|e| e.to_string())?.to_string();
            }
            (Item::UInt(2), Item::Bytes(b)) => {
                inner_payload = b.to_vec();
            }
            (Item::UInt(3), Item::UInt(n)) => target_hive = Some(n),
            (Item::UInt(4), Item::Text(b)) => {
                target_class = Some(
                    std::str::from_utf8(b)
                        .map_err(|e| e.to_string())?
                        .to_string(),
                );
            }
            (Item::UInt(5), Item::UInt(n)) => ttl = Some(n.min(u8::MAX as u64) as u8),
            _ => {} // tolerant of unknown keys
        }
    }
    Ok(EventSendRequest {
        event_class,
        payload: inner_payload,
        target_hive,
        target_class,
        ttl,
    })
}

fn build_event_send_response(correlation_id: u64, msg_id: u64) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map header");
        enc.kv(0, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(1, &Value::UInt(msg_id)).expect("msg_id");
        enc.len()
    };
    build_response_frame_with_event(EV_EVENT_SEND, &payload[..used])
}

/// Decode an `r2.api.event.send` response. Used by tests / CLI.
pub fn parse_event_send_response(payload: &[u8]) -> Result<(u64, u64), String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut cid = None;
    let mut msg_id = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => cid = Some(n),
            (Item::UInt(1), Item::UInt(n)) => msg_id = Some(n),
            _ => {}
        }
    }
    Ok((
        cid.ok_or("missing correlation_id")?,
        msg_id.ok_or("missing msg_id")?,
    ))
}

/// Handle `r2.api.event.subscribe` (R2-HOST-API §3.2). Adds the requested
/// filter to the per-connection registry and returns the assigned sub_id.
///
/// The filter keys per §3.2:
///   1: event_class (text) — exact match
///   2: event_hash (uint32) — exact match (mutually exclusive with 1)
///   3: from_hive (uint64) — sender filter
///   4: from_tg (bytes, 8 bytes) — TG-hash filter
///
/// Implementations MUST tolerate unknown keys (forward-compatibility).
pub async fn handle_event_subscribe(
    payload: &[u8],
    subs: &Arc<Mutex<SubscriptionRegistry>>,
) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let filter = match parse_subscribe_filter(payload) {
        Ok(f) => f,
        Err(_) => return build_error_response(correlation_id, "bad_payload"),
    };

    let sub_id = subs.lock().await.add(filter);
    build_event_subscribe_response(correlation_id, sub_id)
}

/// Handle `r2.api.event.unsubscribe` (R2-HOST-API §3.2). Returns
/// status=0 (ok) on success, status=1 (no such subscription) on
/// unknown sub_id.
pub async fn handle_event_unsubscribe(
    payload: &[u8],
    subs: &Arc<Mutex<SubscriptionRegistry>>,
) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);
    let sub_id = match extract_uint_key(payload, 1) {
        Some(n) => n as u32,
        None => return build_error_response(correlation_id, "bad_payload"),
    };

    let removed = subs.lock().await.remove(sub_id);
    build_event_unsubscribe_response(correlation_id, if removed { 0 } else { 1 })
}

fn parse_subscribe_filter(payload: &[u8]) -> Result<SubscriptionFilter, String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("not a map".into()),
    };
    let mut filter = SubscriptionFilter::default();
    let mut saw_class = false;
    let mut saw_hash = false;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), _) => {} // correlation_id consumed elsewhere
            (Item::UInt(1), Item::Text(b)) => {
                filter.event_class =
                    Some(std::str::from_utf8(b).map_err(|e| e.to_string())?.to_string());
                saw_class = true;
            }
            (Item::UInt(2), Item::UInt(n)) => {
                filter.event_hash = Some(n.min(u32::MAX as u64) as u32);
                saw_hash = true;
            }
            (Item::UInt(3), Item::UInt(n)) => filter.from_hive = Some(n),
            (Item::UInt(4), Item::Bytes(b)) if b.len() == 8 => {
                let mut hash = [0u8; 8];
                hash.copy_from_slice(b);
                filter.from_tg = Some(hash);
            }
            _ => {} // tolerant
        }
    }
    if saw_class && saw_hash {
        return Err("event_class and event_hash are mutually exclusive".into());
    }
    Ok(filter)
}

fn build_event_subscribe_response(correlation_id: u64, sub_id: u32) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map header");
        enc.kv(0, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(1, &Value::UInt(sub_id as u64)).expect("sub_id");
        enc.len()
    };
    build_response_frame_with_event(EV_EVENT_SUBSCRIBE, &payload[..used])
}

fn build_event_unsubscribe_response(correlation_id: u64, status: u8) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map header");
        enc.kv(0, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(1, &Value::UInt(status as u64)).expect("status");
        enc.len()
    };
    build_response_frame_with_event(EV_EVENT_UNSUBSCRIBE, &payload[..used])
}

/// Decode an `r2.api.event.subscribe` response. Used by tests / CLI.
pub fn parse_event_subscribe_response(payload: &[u8]) -> Result<(u64, u32), String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("not a map".into()),
    };
    let mut cid = None;
    let mut sub_id = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => cid = Some(n),
            (Item::UInt(1), Item::UInt(n)) => sub_id = Some(n as u32),
            _ => {}
        }
    }
    Ok((cid.ok_or("missing cid")?, sub_id.ok_or("missing sub_id")?))
}

/// Decode an `r2.api.event.unsubscribe` response. Used by tests / CLI.
pub fn parse_event_unsubscribe_response(payload: &[u8]) -> Result<(u64, u8), String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("not a map".into()),
    };
    let mut cid = None;
    let mut status = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => cid = Some(n),
            (Item::UInt(1), Item::UInt(n)) => status = Some(n.min(u8::MAX as u64) as u8),
            _ => {}
        }
    }
    Ok((cid.ok_or("missing cid")?, status.ok_or("missing status")?))
}

/// Handle `r2.api.peer.query` (R2-HOST-API §3.2). Reads the requested
/// hive_id from key 1 of the payload and returns a record:
///
/// - status=1 (self) if hive_id matches `HiveState::self_hive_id`,
/// - status=2 (neighbour) with `last_seen_ms` and observed transports if
///   the route engine has the entry,
/// - status=0 (unknown) otherwise.
///
/// A request without a `hive_id` key produces `bad_payload` per §6.2.
/// A daemon without HiveState (mgmt-only) treats every hive_id as unknown.
pub async fn handle_peer_query(payload: &[u8], state: &DaemonState) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let hive_id = match extract_uint_key(payload, 1) {
        Some(n) => n,
        None => return build_error_response(correlation_id, "bad_payload"),
    };

    // Mgmt-only daemon: every hive_id is unknown.
    let Some(hive_state) = state.hive_state() else {
        return build_peer_query_response(
            correlation_id,
            hive_id,
            PEER_STATUS_UNKNOWN,
            None,
            &[],
        );
    };

    // Self check.
    if hive_id == hive_state.self_hive_id as u64 {
        return build_peer_query_response(correlation_id, hive_id, PEER_STATUS_SELF, None, &[]);
    }

    // Look up in the route engine's neighbour table.
    let engine = hive_state.route_engine.lock().await;
    let entry = engine
        .neighbours()
        .iter()
        .find(|e| e.hive_id as u64 == hive_id)
        .copied();
    drop(engine);

    match entry {
        None => build_peer_query_response(
            correlation_id,
            hive_id,
            PEER_STATUS_UNKNOWN,
            None,
            &[],
        ),
        Some(e) => {
            let last_seen_ms = (e.last_seen as u64).saturating_mul(1000);
            let mut transports: Vec<&str> = Vec::with_capacity(4);
            for (name, t) in TRANSPORT_NAMES {
                if e.transports.contains(*t) {
                    transports.push(name);
                }
            }
            build_peer_query_response(
                correlation_id,
                hive_id,
                PEER_STATUS_NEIGHBOUR,
                Some(last_seen_ms),
                &transports,
            )
        }
    }
}

fn build_peer_query_response(
    correlation_id: u64,
    hive_id: u64,
    status: u64,
    last_seen_ms: Option<u64>,
    transports: &[&str],
) -> Vec<u8> {
    let mut payload = vec![0u8; 256];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        // Map size depends on which optional keys we're emitting.
        let mut entries = 3; // 0, 1, 2 always present
        if last_seen_ms.is_some() {
            entries += 1;
        }
        // Always emit key 4 (transports) — empty array is the well-defined
        // "no observation" case per the spec note.
        entries += 1;
        enc.map(entries).expect("map header");
        enc.kv(0, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(1, &Value::UInt(hive_id)).expect("hive_id");
        enc.kv(2, &Value::UInt(status)).expect("status");
        if let Some(ms) = last_seen_ms {
            enc.kv(3, &Value::UInt(ms)).expect("last_seen");
        }
        enc.uint(4).expect("transports key");
        enc.array(transports.len()).expect("transports header");
        for t in transports {
            enc.text(t).expect("transport name");
        }
        enc.len()
    };
    build_response_frame_with_event(EV_PEER_QUERY, &payload[..used])
}

/// Decode an `r2.api.peer.query` response into its fields. Used by tests
/// and by the CLI.
pub fn parse_peer_query_response(
    payload: &[u8],
) -> Result<(u64, u64, u64, Option<u64>, Vec<String>), String> {
    let mut dec = Decoder::new(payload);
    let item = dec.next().map_err(|e| format!("cbor header: {e:?}"))?;
    let entries = match item {
        Item::Map(n) => n,
        _ => return Err("payload is not a CBOR map".to_string()),
    };
    let mut correlation_id = None;
    let mut hive_id = None;
    let mut status = None;
    let mut last_seen_ms = None;
    let mut transports: Vec<String> = Vec::new();
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => correlation_id = Some(n),
            (Item::UInt(1), Item::UInt(n)) => hive_id = Some(n),
            (Item::UInt(2), Item::UInt(n)) => status = Some(n),
            (Item::UInt(3), Item::UInt(n)) => last_seen_ms = Some(n),
            (Item::UInt(4), Item::Array(n)) => {
                for _ in 0..n {
                    match dec.next().map_err(|e| format!("cbor transport: {e:?}"))? {
                        Item::Text(b) => {
                            transports.push(
                                std::str::from_utf8(b)
                                    .map_err(|e| format!("transport utf8: {e}"))?
                                    .to_string(),
                            );
                        }
                        other => return Err(format!("transport entry not text: {other:?}")),
                    }
                }
            }
            _ => {}
        }
    }
    Ok((
        correlation_id.ok_or("missing correlation_id")?,
        hive_id.ok_or("missing hive_id")?,
        status.ok_or("missing status")?,
        last_seen_ms,
        transports,
    ))
}

/// Pull a uint at `target_key` from a CBOR map payload. Tolerant of missing
/// or non-uint values (returns `None`).
fn extract_uint_key(payload: &[u8], target_key: u64) -> Option<u64> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let (Item::UInt(k), Item::UInt(n)) = (key, val) {
            if k == target_key {
                return Some(n);
            }
        }
    }
    None
}

/// Handle `r2.api.cap.query` (R2-HOST-API §3.2). Returns the capability
/// aggregate for either a specific hive (if `target_hive` is provided) or
/// the local daemon (the active-TG aggregate, when the active-TG slot
/// lands).
///
/// v0.1: the daemon has no aggregated capability state yet (R2-CAP isn't
/// wired into HiveState's outbound advertisement path, and ensembles /
/// service-sentant advertisements aren't landed). This handler returns an
/// empty Bloom filter and an empty explicit list — the well-defined
/// "no capabilities advertised" response — rather than `unsupported`,
/// because the operation is well-defined and clients should see a stable
/// answer they can interpret.
pub async fn handle_cap_query(payload: &[u8], _state: &DaemonState) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);
    build_cap_query_response(correlation_id, &[], &[], &[])
}

/// Build an `r2.api.cap.query` response. `bloom` is the wire-format Bloom
/// filter bytes (empty in v0.1), `hashes` and `classes` are the optional
/// explicit lists.
fn build_cap_query_response(
    correlation_id: u64,
    bloom: &[u8],
    hashes: &[u32],
    classes: &[&str],
) -> Vec<u8> {
    let mut entries = 2; // 0 (cid) + 1 (bloom) always present
    if !hashes.is_empty() {
        entries += 1;
    }
    if !classes.is_empty() {
        entries += 1;
    }
    let mut payload = vec![0u8; 64 + bloom.len() + hashes.len() * 5 + classes.iter().map(|c| c.len() + 4).sum::<usize>()];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(entries).expect("map header");
        enc.kv(0, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(1, &Value::Bytes(bloom)).expect("bloom");
        if !hashes.is_empty() {
            enc.uint(2).expect("hashes key");
            enc.array(hashes.len()).expect("hashes header");
            for h in hashes {
                enc.uint(*h as u64).expect("hash uint");
            }
        }
        if !classes.is_empty() {
            enc.uint(3).expect("classes key");
            enc.array(classes.len()).expect("classes header");
            for c in classes {
                enc.text(c).expect("class text");
            }
        }
        enc.len()
    };
    build_response_frame_with_event(EV_CAP_QUERY, &payload[..used])
}

/// Decode an `r2.api.cap.query` response. Used by tests / CLI.
pub fn parse_cap_query_response(
    payload: &[u8],
) -> Result<(u64, Vec<u8>, Vec<u32>, Vec<String>), String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut cid = None;
    let mut bloom: Vec<u8> = Vec::new();
    let mut hashes: Vec<u32> = Vec::new();
    let mut classes: Vec<String> = Vec::new();
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => cid = Some(n),
            (Item::UInt(1), Item::Bytes(b)) => bloom = b.to_vec(),
            (Item::UInt(2), Item::Array(n)) => {
                for _ in 0..n {
                    match dec.next().map_err(|e| format!("hash: {e:?}"))? {
                        Item::UInt(h) => hashes.push(h as u32),
                        other => return Err(format!("hash entry: {other:?}")),
                    }
                }
            }
            (Item::UInt(3), Item::Array(n)) => {
                for _ in 0..n {
                    match dec.next().map_err(|e| format!("class: {e:?}"))? {
                        Item::Text(b) => classes.push(
                            std::str::from_utf8(b).map_err(|e| e.to_string())?.to_string(),
                        ),
                        other => return Err(format!("class entry: {other:?}")),
                    }
                }
            }
            _ => {}
        }
    }
    Ok((cid.ok_or("missing cid")?, bloom, hashes, classes))
}

/// Handle `r2.api.tg.current` (R2-HOST-API §3.2).
///
/// Returns the daemon's active trust group attachment, if any. When no
/// HiveState is attached, or when HiveState is attached but no TG is
/// active (a fresh, detached daemon), the response carries only the
/// correlation_id — matching the `TG-CUR-RESP-NONE` test vector.
pub async fn handle_tg_current(payload: &[u8], state: &DaemonState) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let active = match state.hive_state() {
        Some(hs) => hs.active_tg().await,
        None => None,
    };

    build_tg_current_response(correlation_id, active.as_ref())
}

fn build_tg_current_response(
    correlation_id: u64,
    active: Option<&crate::hive::ActiveTg>,
) -> Vec<u8> {
    let mut payload = vec![0u8; 96];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        let entries: usize = if active.is_some() { 4 } else { 1 };
        enc.map(entries).expect("map header");
        enc.kv(0, &Value::UInt(correlation_id)).expect("cid");
        if let Some(tg) = active {
            enc.kv(1, &Value::Bytes(&tg.tg_id)).expect("tg_id");
            enc.kv(2, &Value::UInt(tg.member_role.wire_value() as u64))
                .expect("member_role");
            enc.kv(3, &Value::UInt(tg.hive_id as u64)).expect("hive_id");
        }
        enc.len()
    };
    build_response_frame_with_event(EV_TG_CURRENT, &payload[..used])
}

/// Decode an `r2.api.tg.current` response. Used by tests / CLI.
/// Returns (correlation_id, optional (tg_id, member_role, hive_id)).
#[allow(clippy::type_complexity)]
pub fn parse_tg_current_response(
    payload: &[u8],
) -> Result<(u64, Option<(Vec<u8>, u8, u64)>), String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut cid = None;
    let mut tg_id: Option<Vec<u8>> = None;
    let mut role: Option<u8> = None;
    let mut hive_id: Option<u64> = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => cid = Some(n),
            (Item::UInt(1), Item::Bytes(b)) => tg_id = Some(b.to_vec()),
            (Item::UInt(2), Item::UInt(n)) => role = Some(n.min(u8::MAX as u64) as u8),
            (Item::UInt(3), Item::UInt(n)) => hive_id = Some(n),
            _ => {}
        }
    }
    let attached = match (tg_id, role, hive_id) {
        (Some(t), Some(r), Some(h)) => Some((t, r, h)),
        (None, None, None) => None,
        _ => return Err("partial tg_current payload (some keys present, others missing)".into()),
    };
    Ok((cid.ok_or("missing cid")?, attached))
}

/// Handle `r2.api.service.advertise` (R2-HOST-API §5.2, R2-PLUGIN §5).
///
/// Registers a service-sentant on this connection: the connection
/// claims to handle events of a specific class on this hive. The
/// entry is added to the per-connection subscription registry with
/// the high-bit `service_id` namespace; the existing `deliver_inbound`
/// fanout in `HiveState` then forwards every matching event to this
/// connection's mpsc channel.
///
/// Cleanup is automatic — when the connection closes, the whole
/// registry goes out of scope and the service registrations are gone.
///
/// Request payload: `{0: cid, 1: <service_class : text>, 2?: <state : text>}`.
/// Response payload: `{0: cid, 1: <service_id : uint>}` where the
/// returned id has the high bit set.
pub async fn handle_service_advertise(
    payload: &[u8],
    subs: &std::sync::Arc<tokio::sync::Mutex<crate::mgmt::subscriptions::SubscriptionRegistry>>,
) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let class = match parse_service_class(payload) {
        Some(c) => c,
        None => return build_error_response(correlation_id, "bad_frame"),
    };
    let event_hash = match r2_fnv::r2_hash(&class) {
        Ok(h) => h,
        Err(_) => return build_error_response(correlation_id, "bad_event_class"),
    };

    let service_id = {
        let mut reg = subs.lock().await;
        reg.add_service(&class, event_hash)
    };

    build_service_id_response(super::api::EV_SERVICE_ADVERTISE, correlation_id, service_id)
}

/// Handle `r2.api.service.retract`.
///
/// Drops the registration for the given service_id. Idempotent: an
/// unknown id returns success (the desired post-condition is "this
/// service is not registered", which already holds).
///
/// Request payload: `{0: cid, 1: <service_id : uint>}`.
/// Response payload: `{0: cid, 1: <service_id : uint>}`.
pub async fn handle_service_retract(
    payload: &[u8],
    subs: &std::sync::Arc<tokio::sync::Mutex<crate::mgmt::subscriptions::SubscriptionRegistry>>,
) -> Vec<u8> {
    let correlation_id = extract_correlation_id(payload).unwrap_or(0);

    let service_id = match extract_uint_field(payload, 1) {
        Some(n) => n as u32,
        None => return build_error_response(correlation_id, "bad_frame"),
    };

    {
        let mut reg = subs.lock().await;
        reg.remove(service_id);
    }

    build_service_id_response(super::api::EV_SERVICE_RETRACT, correlation_id, service_id)
}

/// Pull the `service_class` field (key 1, text) out of a CBOR request
/// payload.
fn parse_service_class(payload: &[u8]) -> Option<String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(1) = key {
            if let Item::Text(s) = val {
                return std::str::from_utf8(s).ok().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Generic uint extractor used by retract.
fn extract_uint_field(payload: &[u8], target: u64) -> Option<u64> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(k) = key {
            if k == target {
                if let Item::UInt(n) = val {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Build a `{0: cid, 1: <id : uint>}` response frame.
fn build_service_id_response(event_class: &str, cid: u64, service_id: u32) -> Vec<u8> {
    use r2_cbor::{Encoder, Value};
    let mut buf = [0u8; 64];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(0, &Value::UInt(cid)).expect("cid");
        enc.kv(1, &Value::UInt(service_id as u64)).expect("service_id");
        enc.len()
    };
    super::api::build_response_frame_with_event(event_class, &buf[..used])
}

/// Decode an `r2.api.peer.list` response payload back to `(correlation_id, peer_list)`.
/// Used by the CLI and integration tests.
pub fn parse_peer_list_response(payload: &[u8]) -> Result<(u64, Vec<u64>), String> {
    let mut dec = Decoder::new(payload);
    let item = dec.next().map_err(|e| format!("cbor header: {e:?}"))?;
    let entries = match item {
        Item::Map(n) => n,
        _ => return Err("payload is not a CBOR map".to_string()),
    };
    let mut correlation_id: Option<u64> = None;
    let mut peers: Vec<u64> = Vec::new();
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(0), Item::UInt(n)) => correlation_id = Some(n),
            (Item::UInt(1), Item::Array(n)) => {
                for _ in 0..n {
                    match dec.next().map_err(|e| format!("cbor peer: {e:?}"))? {
                        Item::UInt(p) => peers.push(p),
                        other => return Err(format!("peer entry not uint: {other:?}")),
                    }
                }
            }
            _ => {}
        }
    }
    Ok((correlation_id.ok_or("missing correlation_id")?, peers))
}
