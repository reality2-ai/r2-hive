//! `r2.mgmt.ensemble.*` handlers and the [`HiveOutboundSink`] that
//! bridges `r2-ensemble` outbound events to the wire transports.
//!
//! Wire vocabulary (R2-HIVE §5.3):
//!
//! - `r2.mgmt.ensemble.load`     — load an ensemble score
//! - `r2.mgmt.ensemble.list`     — list loaded ensembles
//! - `r2.mgmt.ensemble.info`     — info on one loaded ensemble
//! - `r2.mgmt.ensemble.stop`     — unload an ensemble
//! - `r2.mgmt.ensemble.reset`    — reset a Failed ensemble to Healthy
//!
//! Score format on the wire: a CBOR text-string with the YAML/JSON/TOML
//! source, plus a tag indicating which dialect. v0.1 only supports
//! YAML-on-the-wire; JSON/TOML can be loaded out-of-band via score
//! files in the startup folder.

use std::sync::Arc;

use async_trait::async_trait;
use r2_cbor::{Decoder, Encoder, Item, Value};
use r2_ensemble::{EnsembleStatus, OutboundEvent, OutboundSink};
use r2_engine::Target;
use r2_fnv::r2_hash;
use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

use crate::hive::HiveState;

use super::api::{build_error_response, build_response_frame_with_event, extract_correlation_id};

/// `r2.mgmt.ensemble.*` event classes.
pub const EV_ENSEMBLE_LOAD: &str = "r2.mgmt.ensemble.load";
pub const EV_ENSEMBLE_LIST: &str = "r2.mgmt.ensemble.list";
pub const EV_ENSEMBLE_INFO: &str = "r2.mgmt.ensemble.info";
pub const EV_ENSEMBLE_STOP: &str = "r2.mgmt.ensemble.stop";
pub const EV_ENSEMBLE_RESET: &str = "r2.mgmt.ensemble.reset";

/// CBOR integer keys.
const K_CORRELATION: u64 = 0;
/// `load` request: dialect ("yaml"/"json"/"toml")
const K_DIALECT: u64 = 1;
/// `load` request: source bytes (text). Optional when `path` is set.
const K_SOURCE: u64 = 2;
/// `load` request: optional filesystem path (text). When present, the
/// daemon reads the score from disk and uses the file's parent
/// directory to resolve web-plugin `bundle:` paths (R2-PLUGIN §13.2).
const K_PATH: u64 = 3;
/// `info`/`stop`/`reset` request: ensemble id
const K_ID: u64 = 1;
/// `load`/`info` response: ensemble id
const K_RESP_ID: u64 = 1;
/// `info` response: status (0=Healthy, 1=Degraded, 2=Failed)
const K_STATUS: u64 = 2;
/// `info`/`load` response: sentant count
const K_SENTANT_COUNT: u64 = 3;
/// `info`/`load` response: 32-bit score hash
const K_SCORE_HASH: u64 = 4;
/// `list` response: array of {id, status, sentant_count}
const K_ENSEMBLES: u64 = 1;

/// Handle `r2.mgmt.ensemble.load`.
pub async fn handle_load(payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let cid = extract_correlation_id(payload).unwrap_or(0);

    let req = match parse_load_request(payload) {
        Ok(p) => p,
        Err(_) => return build_error_response(cid, "bad_frame"),
    };

    // Resolve source bytes + score directory. Path-based loads give us
    // a directory we can use to mount web-plugin bundles (R2-PLUGIN §13.2).
    let (source, score_dir) = match (req.source, req.path) {
        (_, Some(path)) => match std::fs::read_to_string(&path) {
            Ok(s) => {
                let dir = std::path::Path::new(&path)
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                (s, Some(dir))
            }
            Err(e) => {
                log::warn!("ensemble.load: read {path}: {e}");
                return build_error_response(cid, "bad_score");
            }
        },
        (Some(s), None) => (s, None),
        (None, None) => return build_error_response(cid, "bad_frame"),
    };

    let score = match req.dialect.as_str() {
        "yaml" => match r2_def::parse_ensemble_yaml(&source) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("ensemble.load: yaml parse failed: {e}");
                return build_error_response(cid, "bad_score");
            }
        },
        "json" => match r2_def::parse_ensemble_json(&source) {
            Ok(s) => s,
            Err(_) => return build_error_response(cid, "bad_score"),
        },
        "toml" => match r2_def::parse_ensemble_toml(&source) {
            Ok(s) => s,
            Err(_) => return build_error_response(cid, "bad_score"),
        },
        _ => return build_error_response(cid, "unsupported_dialect"),
    };

    // Validate web-plugin manifests up-front so a bad bundle path
    // refuses load before any sentant is built.
    let mut web_mounts: Vec<r2_def::WebPluginManifest> = Vec::new();
    for plugin in &score.plugins {
        match plugin.as_web() {
            Ok(Some(m)) => web_mounts.push(m),
            Ok(None) => {}
            Err(e) => {
                log::warn!("ensemble.load: web plugin invalid: {e}");
                return build_error_response(cid, "bad_score");
            }
        }
    }
    if !web_mounts.is_empty() && score_dir.is_none() {
        log::warn!(
            "ensemble.load: score declares {} web plugin(s) but was loaded without a path; \
             bundles cannot be mounted. Reload via `r2hive ensemble load <file>` to mount.",
            web_mounts.len()
        );
    }

    match hive.ensembles.load(score) {
        Ok(id) => {
            let info = hive.ensembles.info(&id).expect("just loaded");
            // Mount web plugins now that the ensemble is registered.
            // Mount failures roll the load back so the caller sees a
            // single atomic failure rather than a partially-mounted
            // ensemble.
            if let Some(dir) = score_dir.as_ref() {
                for manifest in &web_mounts {
                    if let Err(e) = hive.web_plugins.mount(&id, manifest, dir) {
                        log::warn!(
                            "ensemble.load: web plugin {:?} mount failed: {e}",
                            manifest.name
                        );
                        hive.web_plugins.unmount_ensemble(&id);
                        let _ = hive.ensembles.stop(&id);
                        return build_error_response(cid, "bad_score");
                    }
                }
            }
            build_load_response(cid, &id, info.score_hash, info.sentants.len())
        }
        Err(e) => {
            log::warn!("ensemble.load: registry error: {e}");
            let code = match e {
                r2_ensemble::LoadError::AlreadyLoaded(_) => "already_loaded",
                r2_ensemble::LoadError::Validation(_) => "bad_score",
                r2_ensemble::LoadError::NoFactory { .. } => "no_factory",
                r2_ensemble::LoadError::ExternalIncludeUnsupported(_) => "external_unsupported",
                r2_ensemble::LoadError::BadEventClass(_) => "bad_event_class",
            };
            build_error_response(cid, code)
        }
    }
}

/// Handle `r2.mgmt.ensemble.list`.
pub async fn handle_list(payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let cid = extract_correlation_id(payload).unwrap_or(0);
    let ids = hive.ensembles.list();
    let mut buf = vec![0u8; 64 + ids.len() * 96];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.uint(K_ENSEMBLES).expect("arr key");
        enc.array(ids.len()).expect("arr");
        for id in &ids {
            let info = match hive.ensembles.info(id) {
                Some(i) => i,
                None => continue,
            };
            enc.map(3).expect("entry map");
            enc.kv(K_RESP_ID, &Value::Text(id)).expect("id");
            enc.kv(K_STATUS, &Value::UInt(status_to_u64(info.status())))
                .expect("status");
            enc.kv(K_SENTANT_COUNT, &Value::UInt(info.sentants.len() as u64))
                .expect("count");
        }
        enc.len()
    };
    build_response_frame_with_event(EV_ENSEMBLE_LIST, &buf[..used])
}

/// Handle `r2.mgmt.ensemble.info`.
pub async fn handle_info(payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let cid = extract_correlation_id(payload).unwrap_or(0);
    let id = match extract_text_field(payload, K_ID) {
        Some(s) => s,
        None => return build_error_response(cid, "bad_frame"),
    };
    let info = match hive.ensembles.info(&id) {
        Some(i) => i,
        None => return build_error_response(cid, "not_loaded"),
    };
    let mut buf = [0u8; 256];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(5).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_RESP_ID, &Value::Text(&id)).expect("id");
        enc.kv(K_STATUS, &Value::UInt(status_to_u64(info.status())))
            .expect("status");
        enc.kv(K_SENTANT_COUNT, &Value::UInt(info.sentants.len() as u64))
            .expect("count");
        enc.kv(K_SCORE_HASH, &Value::UInt(info.score_hash as u64))
            .expect("hash");
        enc.len()
    };
    build_response_frame_with_event(EV_ENSEMBLE_INFO, &buf[..used])
}

/// Handle `r2.mgmt.ensemble.stop`.
pub async fn handle_stop(payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let cid = extract_correlation_id(payload).unwrap_or(0);
    let id = match extract_text_field(payload, K_ID) {
        Some(s) => s,
        None => return build_error_response(cid, "bad_frame"),
    };
    match hive.ensembles.stop(&id) {
        Ok(()) => {
            hive.web_plugins.unmount_ensemble(&id);
            build_ack_response(EV_ENSEMBLE_STOP, cid, &id)
        }
        Err(_) => build_error_response(cid, "not_loaded"),
    }
}

/// Handle `r2.mgmt.ensemble.reset`.
pub async fn handle_reset(payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let cid = extract_correlation_id(payload).unwrap_or(0);
    let id = match extract_text_field(payload, K_ID) {
        Some(s) => s,
        None => return build_error_response(cid, "bad_frame"),
    };
    match hive.ensembles.reset(&id) {
        Ok(()) => build_ack_response(EV_ENSEMBLE_RESET, cid, &id),
        Err(_) => build_error_response(cid, "not_loaded"),
    }
}

fn build_load_response(cid: u64, id: &str, score_hash: u32, sentant_count: usize) -> Vec<u8> {
    let mut buf = [0u8; 256];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(4).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_RESP_ID, &Value::Text(id)).expect("id");
        enc.kv(K_SENTANT_COUNT, &Value::UInt(sentant_count as u64))
            .expect("count");
        enc.kv(K_SCORE_HASH, &Value::UInt(score_hash as u64)).expect("hash");
        enc.len()
    };
    build_response_frame_with_event(EV_ENSEMBLE_LOAD, &buf[..used])
}

fn build_ack_response(event: &str, cid: u64, id: &str) -> Vec<u8> {
    let mut buf = [0u8; 128];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_RESP_ID, &Value::Text(id)).expect("id");
        enc.len()
    };
    build_response_frame_with_event(event, &buf[..used])
}

struct LoadRequest {
    dialect: String,
    source: Option<String>,
    path: Option<String>,
}

fn parse_load_request(payload: &[u8]) -> Result<LoadRequest, &'static str> {
    let mut dec = Decoder::new(payload);
    let item = dec.next().map_err(|_| "decode")?;
    let entries = match item {
        Item::Map(n) => n,
        _ => return Err("not map"),
    };
    let mut dialect = String::new();
    let mut source: Option<String> = None;
    let mut path: Option<String> = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|_| "key")?;
        let val = dec.next().map_err(|_| "val")?;
        match key {
            Item::UInt(K_DIALECT) => {
                if let Item::Text(s) = val {
                    dialect = std::str::from_utf8(s).map_err(|_| "utf8")?.to_string();
                }
            }
            Item::UInt(K_SOURCE) => {
                if let Item::Text(s) = val {
                    source = Some(std::str::from_utf8(s).map_err(|_| "utf8")?.to_string());
                }
            }
            Item::UInt(K_PATH) => {
                if let Item::Text(s) = val {
                    path = Some(std::str::from_utf8(s).map_err(|_| "utf8")?.to_string());
                }
            }
            _ => {}
        }
    }
    if dialect.is_empty() || (source.is_none() && path.is_none()) {
        return Err("missing");
    }
    Ok(LoadRequest {
        dialect,
        source,
        path,
    })
}

fn extract_text_field(payload: &[u8], target_key: u64) -> Option<String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(k) = key {
            if k == target_key {
                if let Item::Text(s) = val {
                    return std::str::from_utf8(s).ok().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

fn status_to_u64(s: EnsembleStatus) -> u64 {
    match s {
        EnsembleStatus::Healthy => 0,
        EnsembleStatus::Degraded => 1,
        EnsembleStatus::Failed => 2,
    }
}

/// Build outbound CLI/test request frames.
pub fn build_load_request(cid: u64, dialect: &str, source: &str) -> Vec<u8> {
    let mut buf = vec![0u8; 64 + dialect.len() + source.len()];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(3).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_DIALECT, &Value::Text(dialect)).expect("dialect");
        enc.kv(K_SOURCE, &Value::Text(source)).expect("source");
        enc.len()
    };
    build_response_frame_with_event(EV_ENSEMBLE_LOAD, &buf[..used])
}

/// Build a path-based load request — the daemon reads the score from
/// disk and uses the file's parent directory to resolve web-plugin
/// bundles (R2-PLUGIN §13.2).
pub fn build_load_request_from_path(cid: u64, dialect: &str, path: &str) -> Vec<u8> {
    let mut buf = vec![0u8; 64 + dialect.len() + path.len()];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(3).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_DIALECT, &Value::Text(dialect)).expect("dialect");
        enc.kv(K_PATH, &Value::Text(path)).expect("path");
        enc.len()
    };
    build_response_frame_with_event(EV_ENSEMBLE_LOAD, &buf[..used])
}

pub fn build_list_request(cid: u64) -> Vec<u8> {
    let mut buf = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(1).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.len()
    };
    build_response_frame_with_event(EV_ENSEMBLE_LIST, &buf[..used])
}

fn build_id_request(event: &str, cid: u64, id: &str) -> Vec<u8> {
    let mut buf = vec![0u8; 64 + id.len()];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_ID, &Value::Text(id)).expect("id");
        enc.len()
    };
    build_response_frame_with_event(event, &buf[..used])
}

pub fn build_info_request(cid: u64, id: &str) -> Vec<u8> {
    build_id_request(EV_ENSEMBLE_INFO, cid, id)
}
pub fn build_stop_request(cid: u64, id: &str) -> Vec<u8> {
    build_id_request(EV_ENSEMBLE_STOP, cid, id)
}
pub fn build_reset_request(cid: u64, id: &str) -> Vec<u8> {
    build_id_request(EV_ENSEMBLE_RESET, cid, id)
}

// ---------------------------------------------------------------------
// OutboundSink — bridges r2-ensemble emitted events back onto the wire
// ---------------------------------------------------------------------

/// `OutboundSink` impl that forwards ensemble-emitted events via
/// `HiveState`'s transport layer.
///
/// Resolution rules (matching `r2_engine::Target`):
///
/// - `Sender` → unicast back to `event.originator` (or drop if the
///   originator is this hive itself).
/// - `Sentant(_)` / `Local` → fanned out to local mgmt subscribers via
///   `HiveState::deliver_inbound` (no wire emission).
/// - `TrustGroup` / `Broadcast` → broadcast on the active TG via
///   `broadcast_to_tg`. Broadcast outside a TG is logged-and-dropped.
pub struct HiveOutboundSink {
    pub hive: Arc<HiveState>,
}

#[async_trait]
impl OutboundSink for HiveOutboundSink {
    async fn deliver(&self, ev: OutboundEvent) {
        let frame = build_outbound_frame(&ev);

        match ev.target {
            Target::Sender => {
                if let Some(origin) = ev.originator {
                    if origin == self.hive.self_hive_id {
                        // Loopback to ourselves — feed the local fanout.
                        self.hive
                            .deliver_inbound(&frame, origin, ev.trust_group)
                            .await;
                    } else if !self.hive.send_to_hive(origin, &frame).await {
                        log::debug!(
                            "outbound Sender to hive {origin} failed (no transport)"
                        );
                    }
                } else {
                    log::debug!("outbound Sender with no originator — dropping");
                }
            }
            Target::Sentant(_) | Target::Local => {
                // Local fanout to mgmt subscribers (R2-HOST-API).
                self.hive
                    .deliver_inbound(
                        &frame,
                        ev.originator.unwrap_or(self.hive.self_hive_id),
                        ev.trust_group,
                    )
                    .await;
            }
            Target::TrustGroup | Target::Broadcast => {
                if let Some(tg) = ev.trust_group {
                    self.hive
                        .broadcast_to_tg(&tg, self.hive.self_hive_id, &frame)
                        .await;
                } else {
                    log::debug!(
                        "outbound TrustGroup/Broadcast with no TG context — dropping"
                    );
                }
            }
        }
    }
}

fn build_outbound_frame(ev: &OutboundEvent) -> Vec<u8> {
    let class_string_hash = ev.event_hash;
    let _ = r2_hash; // silence unused-import lint if guards remove use
    let target_group = ev
        .trust_group
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .unwrap_or(0);
    let msg = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0,
            k: 0,
            msg_id: ev.trigger_msg_id,
            event_hash: class_string_hash,
            payload_len: ev.payload.len() as u32,
            target_group,
            target_hive: 0,
        },
        route: None,
        payload: &ev.payload,
        hmac_tag: None,
    };
    let mut out = vec![0u8; 32 + ev.payload.len()];
    let n = encode_extended(&msg, &mut out).expect("encode_extended fits");
    out.truncate(n);
    out
}
