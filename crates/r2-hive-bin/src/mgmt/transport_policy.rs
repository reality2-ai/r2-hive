//! Local management handlers for the node-wide transport egress allow mask.
//!
//! This is intentionally local management API only. It delegates the effective
//! policy to `r2-route::RouteEngine::transport_allow_mask` and does not install
//! any mesh-received control frame semantics.

use std::sync::Arc;

use r2_cbor::{Decoder, Encoder, Item, Value};

use crate::hive::{HiveState, TransportPolicyAck, TransportPolicySnapshot};

use super::api::{build_error_response, build_response_frame_with_event};

pub const EV_TRANSPORT_ALLOW_MASK_STATE: &str = "r2.mgmt.transport.allow_mask.state";
pub const EV_TRANSPORT_ALLOW_MASK_SET: &str = "r2.mgmt.transport.allow_mask.set";
pub const EV_TRANSPORT_ALLOW_MASK_CLEAR: &str = "r2.mgmt.transport.allow_mask.clear";

const KEY_CID: u64 = 0;
const KEY_REQUESTED_MASK: u64 = 1;
const KEY_ACCEPTED_MASK: u64 = 2;
const KEY_EFFECTIVE_MASK: u64 = 3;
const KEY_ALL_MASK: u64 = 4;
const KEY_LEASE_ID: u64 = 5;
const KEY_SOURCE: u64 = 6;
const KEY_ACTIVE_LEASE: u64 = 7;

pub async fn handle_state(correlation_id: u64, hive: &Arc<HiveState>) -> Vec<u8> {
    let snapshot = hive.transport_policy_snapshot().await;
    build_snapshot_response(EV_TRANSPORT_ALLOW_MASK_STATE, correlation_id, &snapshot)
}

pub async fn handle_set(correlation_id: u64, payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let req = match parse_set_request(payload) {
        Ok(req) => req,
        Err(_) => return build_error_response(correlation_id, "bad_payload"),
    };
    let snapshot = hive.transport_policy_snapshot().await;
    if snapshot
        .active_lease
        .as_ref()
        .is_some_and(|lease| lease.lease_id != req.lease_id)
    {
        return build_error_response(correlation_id, "lease_conflict");
    }
    let ack = hive
        .set_transport_policy_lease(req.lease_id, req.source, req.requested_mask)
        .await;
    build_ack_response(EV_TRANSPORT_ALLOW_MASK_SET, correlation_id, &ack)
}

pub async fn handle_clear(correlation_id: u64, payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let lease_id = match parse_clear_request(payload) {
        Ok(id) => id,
        Err(_) => return build_error_response(correlation_id, "bad_payload"),
    };

    if let Some(lease_id) = lease_id {
        let snapshot = hive.transport_policy_snapshot().await;
        if snapshot.active_lease.as_ref().map(|lease| lease.lease_id) != Some(lease_id) {
            return build_error_response(correlation_id, "lease_not_found");
        }
    }

    let snapshot = hive.clear_transport_policy().await;
    build_snapshot_response(EV_TRANSPORT_ALLOW_MASK_CLEAR, correlation_id, &snapshot)
}

struct SetRequest {
    requested_mask: u8,
    lease_id: u64,
    source: String,
}

fn parse_set_request(payload: &[u8]) -> Result<SetRequest, String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut requested_mask = None;
    let mut lease_id = None;
    let mut source = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(KEY_CID), _) => {}
            (Item::UInt(1), Item::UInt(n)) if n <= u8::MAX as u64 => {
                requested_mask = Some(n as u8);
            }
            (Item::UInt(2), Item::UInt(n)) => lease_id = Some(n),
            (Item::UInt(3), Item::Text(bytes)) => {
                let parsed = std::str::from_utf8(bytes)
                    .map_err(|e| format!("source utf8: {e}"))?
                    .to_string();
                if parsed.is_empty() {
                    return Err("empty source".into());
                }
                source = Some(parsed);
            }
            _ => {}
        }
    }
    Ok(SetRequest {
        requested_mask: requested_mask.ok_or("missing mask")?,
        lease_id: lease_id.ok_or("missing lease_id")?,
        source: source.ok_or("missing source")?,
    })
}

fn parse_clear_request(payload: &[u8]) -> Result<Option<u64>, String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut lease_id = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(KEY_CID), _) => {}
            (Item::UInt(1), Item::UInt(n)) => lease_id = Some(n),
            _ => {}
        }
    }
    Ok(lease_id)
}

fn build_ack_response(event_class: &str, correlation_id: u64, ack: &TransportPolicyAck) -> Vec<u8> {
    let mut payload = vec![0u8; 128 + ack.source.len()];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(8).expect("map header");
        enc.kv(KEY_CID, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(KEY_REQUESTED_MASK, &Value::UInt(ack.requested_mask as u64))
            .expect("requested_mask");
        enc.kv(KEY_ACCEPTED_MASK, &Value::UInt(ack.accepted_mask as u64))
            .expect("accepted_mask");
        enc.kv(KEY_EFFECTIVE_MASK, &Value::UInt(ack.effective_mask as u64))
            .expect("effective_mask");
        enc.kv(KEY_ALL_MASK, &Value::UInt(ack.all_mask as u64))
            .expect("all_mask");
        enc.kv(KEY_LEASE_ID, &Value::UInt(ack.lease_id))
            .expect("lease_id");
        enc.kv(KEY_SOURCE, &Value::Text(&ack.source))
            .expect("source");
        enc.kv(KEY_ACTIVE_LEASE, &Value::Bool(true))
            .expect("active_lease");
        enc.len()
    };
    build_response_frame_with_event(event_class, &payload[..used])
}

fn build_snapshot_response(
    event_class: &str,
    correlation_id: u64,
    snapshot: &TransportPolicySnapshot,
) -> Vec<u8> {
    let source_len = snapshot
        .active_lease
        .as_ref()
        .map(|lease| lease.source.len())
        .unwrap_or(0);
    let mut payload = vec![0u8; 128 + source_len];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        let lease = snapshot.active_lease.as_ref();
        let entries = if lease.is_some() { 8 } else { 4 };
        enc.map(entries).expect("map header");
        enc.kv(KEY_CID, &Value::UInt(correlation_id)).expect("cid");
        if let Some(lease) = lease {
            enc.kv(
                KEY_REQUESTED_MASK,
                &Value::UInt(lease.requested_mask as u64),
            )
            .expect("requested_mask");
            enc.kv(KEY_ACCEPTED_MASK, &Value::UInt(lease.accepted_mask as u64))
                .expect("accepted_mask");
        }
        enc.kv(
            KEY_EFFECTIVE_MASK,
            &Value::UInt(snapshot.effective_mask as u64),
        )
        .expect("effective_mask");
        enc.kv(KEY_ALL_MASK, &Value::UInt(snapshot.all_mask as u64))
            .expect("all_mask");
        if let Some(lease) = lease {
            enc.kv(KEY_LEASE_ID, &Value::UInt(lease.lease_id))
                .expect("lease_id");
            enc.kv(KEY_SOURCE, &Value::Text(&lease.source))
                .expect("source");
        }
        enc.kv(KEY_ACTIVE_LEASE, &Value::Bool(lease.is_some()))
            .expect("active_lease");
        enc.len()
    };
    build_response_frame_with_event(event_class, &payload[..used])
}

pub fn build_state_request(correlation_id: u64) -> Vec<u8> {
    build_empty_request(EV_TRANSPORT_ALLOW_MASK_STATE, correlation_id)
}

pub fn build_set_request(
    correlation_id: u64,
    requested_mask: u8,
    lease_id: u64,
    source: &str,
) -> Vec<u8> {
    let mut payload = vec![0u8; 64 + source.len()];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(4).expect("map header");
        enc.kv(KEY_CID, &Value::UInt(correlation_id)).expect("cid");
        enc.kv(1, &Value::UInt(requested_mask as u64))
            .expect("mask");
        enc.kv(2, &Value::UInt(lease_id)).expect("lease_id");
        enc.kv(3, &Value::Text(source)).expect("source");
        enc.len()
    };
    build_response_frame_with_event(EV_TRANSPORT_ALLOW_MASK_SET, &payload[..used])
}

pub fn build_clear_request(correlation_id: u64, lease_id: Option<u64>) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        let entries = if lease_id.is_some() { 2 } else { 1 };
        enc.map(entries).expect("map header");
        enc.kv(KEY_CID, &Value::UInt(correlation_id)).expect("cid");
        if let Some(lease_id) = lease_id {
            enc.kv(1, &Value::UInt(lease_id)).expect("lease_id");
        }
        enc.len()
    };
    build_response_frame_with_event(EV_TRANSPORT_ALLOW_MASK_CLEAR, &payload[..used])
}

fn build_empty_request(event_class: &str, correlation_id: u64) -> Vec<u8> {
    let mut payload = [0u8; 16];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(1).expect("map header");
        enc.kv(KEY_CID, &Value::UInt(correlation_id)).expect("cid");
        enc.len()
    };
    build_response_frame_with_event(event_class, &payload[..used])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportPolicyResponse {
    pub correlation_id: u64,
    pub requested_mask: Option<u8>,
    pub accepted_mask: Option<u8>,
    pub effective_mask: u8,
    pub all_mask: u8,
    pub lease_id: Option<u64>,
    pub source: Option<String>,
    pub active_lease: bool,
}

pub fn parse_response(payload: &[u8]) -> Result<TransportPolicyResponse, String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().map_err(|e| format!("cbor: {e:?}"))? {
        Item::Map(n) => n,
        _ => return Err("payload not a map".into()),
    };
    let mut correlation_id = None;
    let mut requested_mask = None;
    let mut accepted_mask = None;
    let mut effective_mask = None;
    let mut all_mask = None;
    let mut lease_id = None;
    let mut source = None;
    let mut active_lease = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("val: {e:?}"))?;
        match (key, val) {
            (Item::UInt(KEY_CID), Item::UInt(n)) => correlation_id = Some(n),
            (Item::UInt(KEY_REQUESTED_MASK), Item::UInt(n)) if n <= u8::MAX as u64 => {
                requested_mask = Some(n as u8);
            }
            (Item::UInt(KEY_ACCEPTED_MASK), Item::UInt(n)) if n <= u8::MAX as u64 => {
                accepted_mask = Some(n as u8);
            }
            (Item::UInt(KEY_EFFECTIVE_MASK), Item::UInt(n)) if n <= u8::MAX as u64 => {
                effective_mask = Some(n as u8);
            }
            (Item::UInt(KEY_ALL_MASK), Item::UInt(n)) if n <= u8::MAX as u64 => {
                all_mask = Some(n as u8);
            }
            (Item::UInt(KEY_LEASE_ID), Item::UInt(n)) => lease_id = Some(n),
            (Item::UInt(KEY_SOURCE), Item::Text(bytes)) => {
                source = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|e| format!("source utf8: {e}"))?
                        .to_string(),
                );
            }
            (Item::UInt(KEY_ACTIVE_LEASE), Item::Bool(b)) => active_lease = Some(b),
            _ => {}
        }
    }
    Ok(TransportPolicyResponse {
        correlation_id: correlation_id.ok_or("missing correlation_id")?,
        requested_mask,
        accepted_mask,
        effective_mask: effective_mask.ok_or("missing effective_mask")?,
        all_mask: all_mask.ok_or("missing all_mask")?,
        lease_id,
        source,
        active_lease: active_lease.ok_or("missing active_lease")?,
    })
}
