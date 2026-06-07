//! API dispatcher — recognises `r2.mgmt.*` and `r2.api.*` event classes and
//! produces responses.
//!
//! `r2.mgmt.*` is the management vocabulary (R2-HIVE §5.3) used by UIs.
//! `r2.api.*` is the application vocabulary (R2-HOST-API §3) used by R2-guest
//! apps. Both transit the same socket; this module routes by event hash.
//!
//! Unknown event hashes return a `r2.mgmt.event.error` frame with
//! `code = "unknown_event"`.
//!
//! Request/response pairs carry a `correlation_id` per R2-HIVE §5.4. In the
//! CBOR payload map the correlation id is encoded under integer key `0`.

use r2_cbor::{Decoder, Encoder, Item, Value};
use r2_fnv::r2_hash;
use r2_wire::{
    encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType,
};

use super::ensemble as ens;
use super::primitive;
use super::state::DaemonState;
#[cfg(target_os = "linux")]
use super::usb;

/// `r2.mgmt.*` management event classes.
pub const EV_DAEMON_STATUS: &str = "r2.mgmt.daemon.status";
pub const EV_IDENTITY_STATUS: &str = "r2.mgmt.identity.status";
pub const EV_WEB_PROVISION: &str = "r2.mgmt.web.provision";
/// Phase USB-4 — operator opt-in surface for USB peripherals
/// (R2-USB §3 + R2-HIVE §6.4). Five events:
///
/// - `list` — return every device the watcher is tracking + its
///   current state and any pending SAS prompt.
/// - `prepare` — add a path to the watcher's explicit allowlist.
/// - `confirm` — operator confirms the SAS code on a paired device
///   (drives [`crate::usb_serial::SessionControl::UserConfirms`]).
/// - `abort` — operator rejects the SAS code or otherwise cancels.
/// - `unpair` — forget the link key for a previously-paired device,
///   forcing fresh §6.4.3 first-attach pairing on next reconnect.
pub const EV_USB_LIST: &str = "r2.mgmt.usb.list";
pub const EV_USB_PREPARE: &str = "r2.mgmt.usb.prepare";
pub const EV_USB_CONFIRM: &str = "r2.mgmt.usb.confirm";
pub const EV_USB_ABORT: &str = "r2.mgmt.usb.abort";
pub const EV_USB_UNPAIR: &str = "r2.mgmt.usb.unpair";
/// Event class for structured errors returned to UIs and apps (R2-HOST-API §6).
pub const EV_ERROR: &str = "r2.mgmt.event.error";

/// `r2.api.*` application event classes (R2-HOST-API §3).
pub const EV_TG_CURRENT: &str = "r2.api.tg.current";
pub const EV_EVENT_SEND: &str = "r2.api.event.send";
pub const EV_EVENT_SUBSCRIBE: &str = "r2.api.event.subscribe";
pub const EV_EVENT_UNSUBSCRIBE: &str = "r2.api.event.unsubscribe";
pub const EV_EVENT_DELIVERY: &str = "r2.api.event.delivery";
pub const EV_PEER_LIST: &str = "r2.api.peer.list";
pub const EV_PEER_QUERY: &str = "r2.api.peer.query";
pub const EV_CAP_QUERY: &str = "r2.api.cap.query";
pub const EV_SERVICE_ADVERTISE: &str = "r2.api.service.advertise";
pub const EV_SERVICE_RETRACT: &str = "r2.api.service.retract";

/// CBOR integer key used for the correlation id field in request/response payloads.
const CBOR_KEY_CORRELATION_ID: u64 = 0;
/// CBOR integer keys for daemon-status response fields.
const CBOR_KEY_VERSION: u64 = 1;
const CBOR_KEY_BUILD_HASH: u64 = 2;
const CBOR_KEY_UPTIME_SECONDS: u64 = 3;
/// CBOR integer keys for identity-status response fields (numbered to avoid
/// collision with daemon-status so a future combined view is mechanical).
const CBOR_KEY_IDENTITY_PRESENT: u64 = 1;
const CBOR_KEY_IDENTITY_FINGERPRINT: u64 = 2;
const CBOR_KEY_IDENTITY_BACKEND: u64 = 3;
const CBOR_KEY_IDENTITY_PATH: u64 = 4;
const CBOR_KEY_IDENTITY_CREATED_THIS_START: u64 = 5;
/// CBOR integer key used for the `r2.mgmt.event.error`-style error code.
const CBOR_KEY_ERR_CODE: u64 = 1;

/// Parse a single incoming R2-WIRE extended frame and produce a response
/// frame. The per-connection subscription registry is required for
/// subscribe/unsubscribe handlers; other handlers ignore it.
///
/// Errors during decode produce an error-response frame (not a Rust `Err`),
/// so a misbehaving peer doesn't tear down the socket session.
///
/// Dispatch is by `event_hash`. The function is async because primitive
/// (`r2.api.*`) handlers may need to lock async-only state in `HiveState`
/// (route engine, transports). Management (`r2.mgmt.*`) handlers are
/// synchronous because they read from cheap `DaemonState` accessors.
pub async fn handle_frame_with_subs(
    input: &[u8],
    state: &DaemonState,
    subs: &std::sync::Arc<tokio::sync::Mutex<crate::mgmt::subscriptions::SubscriptionRegistry>>,
) -> Vec<u8> {
    let msg = match r2_wire::decode_extended(input) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("decode_extended failed: {e:?}");
            return build_error_response(0, "bad_frame");
        }
    };

    let correlation_id = extract_correlation_id(msg.payload).unwrap_or(0);
    let h = msg.header.event_hash;

    // r2.mgmt.* — management vocabulary (R2-HIVE §5.3).
    if h == r2_hash(EV_DAEMON_STATUS).expect("known-good event name") {
        return build_status_response(correlation_id, state);
    }
    if h == r2_hash(EV_IDENTITY_STATUS).expect("known-good event name") {
        return build_identity_status_response(correlation_id, state);
    }

    // r2.api.* — application vocabulary (R2-HOST-API §3).
    if h == r2_hash(EV_PEER_LIST).expect("known-good event name") {
        return primitive::handle_peer_list(msg.payload, state).await;
    }
    if h == r2_hash(EV_PEER_QUERY).expect("known-good event name") {
        return primitive::handle_peer_query(msg.payload, state).await;
    }
    if h == r2_hash(EV_TG_CURRENT).expect("known-good event name") {
        return primitive::handle_tg_current(msg.payload, state).await;
    }
    if h == r2_hash(EV_EVENT_SEND).expect("known-good event name") {
        return primitive::handle_event_send(msg.payload, state).await;
    }
    if h == r2_hash(EV_EVENT_SUBSCRIBE).expect("known-good event name") {
        return primitive::handle_event_subscribe(msg.payload, subs).await;
    }
    if h == r2_hash(EV_EVENT_UNSUBSCRIBE).expect("known-good event name") {
        return primitive::handle_event_unsubscribe(msg.payload, subs).await;
    }
    if h == r2_hash(EV_CAP_QUERY).expect("known-good event name") {
        return primitive::handle_cap_query(msg.payload, state).await;
    }
    if h == r2_hash(EV_SERVICE_ADVERTISE).expect("known-good event name") {
        return primitive::handle_service_advertise(msg.payload, subs).await;
    }
    if h == r2_hash(EV_SERVICE_RETRACT).expect("known-good event name") {
        return primitive::handle_service_retract(msg.payload, subs).await;
    }

    // r2.mgmt.usb.* are app_to_hive management requests on every platform
    // (R2-HOST-API §4/§6 — platform-gating does not change direction), but the
    // USB stack is Linux-only. Off Linux, recognise these classes and reply with
    // a structured `unsupported` error — never a silent `unknown_event` (§6).
    #[cfg(not(target_os = "linux"))]
    {
        for usb_class in [EV_USB_LIST, EV_USB_PREPARE, EV_USB_CONFIRM, EV_USB_ABORT, EV_USB_UNPAIR] {
            if h == r2_hash(usb_class).expect("known-good event name") {
                return build_error_response(correlation_id, "unsupported");
            }
        }
    }

    // r2.mgmt.ensemble.* — ensemble lifecycle (R2-HIVE §5.3, R2-ENSEMBLE).
    if let Some(hive) = state.hive_state() {
        let hive = hive.clone();
        if h == r2_hash(ens::EV_ENSEMBLE_LOAD).expect("known-good event name") {
            return ens::handle_load(msg.payload, &hive).await;
        }
        if h == r2_hash(ens::EV_ENSEMBLE_LIST).expect("known-good event name") {
            return ens::handle_list(msg.payload, &hive).await;
        }
        if h == r2_hash(ens::EV_ENSEMBLE_INFO).expect("known-good event name") {
            return ens::handle_info(msg.payload, &hive).await;
        }
        if h == r2_hash(ens::EV_ENSEMBLE_STOP).expect("known-good event name") {
            return ens::handle_stop(msg.payload, &hive).await;
        }
        if h == r2_hash(ens::EV_ENSEMBLE_RESET).expect("known-good event name") {
            return ens::handle_reset(msg.payload, &hive).await;
        }
        if h == r2_hash(EV_WEB_PROVISION).expect("known-good event name") {
            return handle_web_provision(correlation_id, &hive);
        }
        // r2.mgmt.usb.* — Phase USB-4. Linux-only; on other targets
        // the USB watcher is absent and these all return
        // `usb_disabled`.
        #[cfg(target_os = "linux")]
        {
            if h == r2_hash(EV_USB_LIST).expect("known-good event name") {
                return usb::handle_list(correlation_id, &hive).await;
            }
            if h == r2_hash(EV_USB_PREPARE).expect("known-good event name") {
                return usb::handle_prepare(correlation_id, msg.payload, &hive).await;
            }
            if h == r2_hash(EV_USB_CONFIRM).expect("known-good event name") {
                return usb::handle_confirm(correlation_id, msg.payload, &hive).await;
            }
            if h == r2_hash(EV_USB_ABORT).expect("known-good event name") {
                return usb::handle_abort(correlation_id, msg.payload, &hive).await;
            }
            if h == r2_hash(EV_USB_UNPAIR).expect("known-good event name") {
                return usb::handle_unpair(correlation_id, msg.payload, &hive).await;
            }
        }
    }

    log::debug!(
        "unknown event hash 0x{:08X} — returning unknown_event",
        h
    );
    build_error_response(correlation_id, "unknown_event")
}

/// Convenience wrapper for callers that don't have a per-connection
/// subscription registry (single-shot tests, the WS handler that builds
/// its own per-call registry, etc). subscribe/unsubscribe routed through
/// this path operate on a throwaway registry — the subscriptions don't
/// outlive the call.
pub async fn handle_frame(input: &[u8], state: &DaemonState) -> Vec<u8> {
    let subs = std::sync::Arc::new(tokio::sync::Mutex::new(
        crate::mgmt::subscriptions::SubscriptionRegistry::new(),
    ));
    handle_frame_with_subs(input, state, &subs).await
}

/// Build an `r2.mgmt.identity.status` response frame.
fn build_identity_status_response(correlation_id: u64, state: &DaemonState) -> Vec<u8> {
    let mut payload = [0u8; 512];
    let fingerprint = state.identity_fingerprint();
    let backend = state.identity_backend();
    let path = state.identity_path();
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(6).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(
            CBOR_KEY_IDENTITY_PRESENT,
            &Value::Bool(state.identity_present()),
        )
        .expect("present");
        enc.kv(CBOR_KEY_IDENTITY_FINGERPRINT, &Value::Text(&fingerprint))
            .expect("fingerprint");
        enc.kv(CBOR_KEY_IDENTITY_BACKEND, &Value::Text(backend))
            .expect("backend");
        enc.kv(CBOR_KEY_IDENTITY_PATH, &Value::Text(&path))
            .expect("path");
        enc.kv(
            CBOR_KEY_IDENTITY_CREATED_THIS_START,
            &Value::Bool(state.identity_created_this_start()),
        )
        .expect("created_this_start");
        enc.len()
    };
    build_response_frame(EV_IDENTITY_STATUS, &payload[..used])
}

/// Build an `r2.mgmt.daemon.status` response frame.
fn build_status_response(correlation_id: u64, state: &DaemonState) -> Vec<u8> {
    let mut payload = [0u8; 512];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(4).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(CBOR_KEY_VERSION, &Value::Text(state.version()))
            .expect("version");
        enc.kv(CBOR_KEY_BUILD_HASH, &Value::Text(state.build_hash()))
            .expect("build_hash");
        enc.kv(CBOR_KEY_UPTIME_SECONDS, &Value::UInt(state.uptime_seconds()))
            .expect("uptime_seconds");
        enc.len()
    };
    build_response_frame(EV_DAEMON_STATUS, &payload[..used])
}

/// Mint a single-use word code for browser provisioning
/// (R2-PLUGIN §13.5). The response payload is `{0: cid, 1: <words>}`.
/// Returns `auth_unavailable` when the hive is in dev-mode (no master
/// secret loaded).
fn handle_web_provision(correlation_id: u64, hive: &std::sync::Arc<crate::hive::HiveState>) -> Vec<u8> {
    let Some(auth) = hive.web_auth() else {
        return build_error_response(correlation_id, "auth_unavailable");
    };
    let words = auth.mint_provision_code();
    let mut payload = [0u8; 128];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("cid");
        enc.kv(1u64, &Value::Text(&words)).expect("words");
        enc.len()
    };
    build_response_frame(EV_WEB_PROVISION, &payload[..used])
}

/// Build an `r2.mgmt.event.error` response frame. Public so primitive
/// handlers (and any future module) can reuse it for the standard
/// error-code vocabulary in R2-HOST-API §6.
pub fn build_error_response(correlation_id: u64, code: &str) -> Vec<u8> {
    let mut payload = [0u8; 128];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(CBOR_KEY_ERR_CODE, &Value::Text(code))
            .expect("err code");
        enc.len()
    };
    build_response_frame(EV_ERROR, &payload[..used])
}

/// Package an event class + payload into an R2-WIRE extended frame.
///
/// Local management frames skip HMAC (local dispatch; no trust-boundary crossing).
/// Public alias for use by other modules in this crate (e.g. `primitive.rs`).
pub fn build_response_frame_with_event(event_class: &str, payload: &[u8]) -> Vec<u8> {
    build_response_frame(event_class, payload)
}

/// Package an event class + payload into an R2-WIRE extended frame.
///
/// Local management frames skip HMAC (local dispatch; no trust-boundary crossing).
fn build_response_frame(event_class: &str, payload: &[u8]) -> Vec<u8> {
    let event_hash = r2_hash(event_class).expect("event class canonicalises");
    let msg = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0,
            k: 0,
            msg_id: 0,
            event_hash,
            payload_len: payload.len() as u32,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload,
        hmac_tag: None,
    };
    // 22-byte header + payload + margin for any encoder surplus.
    let mut out = vec![0u8; 32 + payload.len()];
    let n = encode_extended(&msg, &mut out).expect("encode_extended fits");
    out.truncate(n);
    out
}

/// Pull the correlation_id field (integer key 0) out of a CBOR request payload.
/// Tolerant of missing/garbled payloads — returns `None` and lets the
/// caller decide whether to still respond. Public so primitive handlers
/// can reuse it.
pub fn extract_correlation_id(payload: &[u8]) -> Option<u64> {
    let mut dec = Decoder::new(payload);
    let item = dec.next().ok()?;
    let entries = match item {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let key = dec.next().ok()?;
        let val = dec.next().ok()?;
        if let Item::UInt(CBOR_KEY_CORRELATION_ID) = key {
            if let Item::UInt(n) = val {
                return Some(n);
            }
        }
    }
    None
}

/// Construct an outgoing `r2.mgmt.daemon.status` request frame (used by the CLI).
pub fn build_status_request(correlation_id: u64) -> Vec<u8> {
    build_empty_request(EV_DAEMON_STATUS, correlation_id)
}

/// Construct an outgoing `r2.mgmt.identity.status` request frame.
pub fn build_identity_status_request(correlation_id: u64) -> Vec<u8> {
    build_empty_request(EV_IDENTITY_STATUS, correlation_id)
}

/// Construct an outgoing `r2.mgmt.web.provision` request frame.
/// Mints a one-time browser-provisioning word code (R2-PLUGIN §13.5).
pub fn build_web_provision_request(correlation_id: u64) -> Vec<u8> {
    build_empty_request(EV_WEB_PROVISION, correlation_id)
}

/// Construct an outgoing `r2.api.peer.list` request frame (R2-HOST-API §3.2).
pub fn build_peer_list_request(correlation_id: u64) -> Vec<u8> {
    build_empty_request(EV_PEER_LIST, correlation_id)
}

/// Construct an outgoing `r2.api.peer.query` request frame (R2-HOST-API §3.2).
pub fn build_peer_query_request(correlation_id: u64, hive_id: u64) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(2).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(1, &Value::UInt(hive_id)).expect("hive_id");
        enc.len()
    };
    build_response_frame(EV_PEER_QUERY, &payload[..used])
}

/// Construct an outgoing `r2.api.tg.current` request frame (R2-HOST-API §3.2).
pub fn build_tg_current_request(correlation_id: u64) -> Vec<u8> {
    build_empty_request(EV_TG_CURRENT, correlation_id)
}

/// Construct an outgoing `r2.api.event.subscribe` request frame
/// (R2-HOST-API §3.2). At most one of `event_class` / `event_hash` may be
/// specified per spec; this helper does not enforce that — callers are
/// trusted to send well-formed filters.
pub fn build_event_subscribe_request(
    correlation_id: u64,
    event_class: Option<&str>,
    event_hash: Option<u32>,
    from_hive: Option<u64>,
) -> Vec<u8> {
    let mut buf = vec![0u8; 64 + event_class.map(|s| s.len() + 4).unwrap_or(0)];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        let mut entries: usize = 1;
        if event_class.is_some() {
            entries += 1;
        }
        if event_hash.is_some() {
            entries += 1;
        }
        if from_hive.is_some() {
            entries += 1;
        }
        enc.map(entries).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("cid");
        if let Some(c) = event_class {
            enc.kv(1, &Value::Text(c)).expect("event_class");
        }
        if let Some(h) = event_hash {
            enc.kv(2, &Value::UInt(h as u64)).expect("event_hash");
        }
        if let Some(h) = from_hive {
            enc.kv(3, &Value::UInt(h)).expect("from_hive");
        }
        enc.len()
    };
    build_response_frame(EV_EVENT_SUBSCRIBE, &buf[..used])
}

/// Construct an outgoing `r2.api.event.unsubscribe` request frame.
pub fn build_event_unsubscribe_request(correlation_id: u64, sub_id: u32) -> Vec<u8> {
    let mut buf = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("cid");
        enc.kv(1, &Value::UInt(sub_id as u64)).expect("sub_id");
        enc.len()
    };
    build_response_frame(EV_EVENT_UNSUBSCRIBE, &buf[..used])
}

/// Construct an outgoing `r2.api.service.advertise` request frame (R2-HOST-API §5.2).
pub fn build_service_advertise_request(correlation_id: u64, service_class: &str) -> Vec<u8> {
    let mut buf = vec![0u8; 64 + service_class.len()];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(1, &Value::Text(service_class)).expect("service_class");
        enc.len()
    };
    build_response_frame(EV_SERVICE_ADVERTISE, &buf[..used])
}

/// Construct an outgoing `r2.api.service.retract` request frame.
pub fn build_service_retract_request(correlation_id: u64, service_id: u32) -> Vec<u8> {
    let mut buf = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(1, &Value::UInt(service_id as u64)).expect("service_id");
        enc.len()
    };
    build_response_frame(EV_SERVICE_RETRACT, &buf[..used])
}

/// Construct an outgoing `r2.api.cap.query` request frame (R2-HOST-API §3.2).
pub fn build_cap_query_request(correlation_id: u64, target_hive: Option<u64>) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        let entries: usize = if target_hive.is_some() { 2 } else { 1 };
        enc.map(entries).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        if let Some(h) = target_hive {
            enc.kv(1, &Value::UInt(h)).expect("target_hive");
        }
        enc.len()
    };
    build_response_frame(EV_CAP_QUERY, &payload[..used])
}

/// Construct an outgoing `r2.api.event.send` request frame (R2-HOST-API §3.2).
/// `inner_payload` is the CBOR (or otherwise opaque) bytes the receiving
/// event handler sees in its `params` map.
pub fn build_event_send_request(
    correlation_id: u64,
    event_class: &str,
    inner_payload: &[u8],
    target_hive: Option<u64>,
    target_class: Option<&str>,
) -> Vec<u8> {
    // Pre-size: header + small CBOR scaffolding + class + payload + class.
    let mut buf = vec![0u8; 64 + event_class.len() + inner_payload.len() + target_class.map(|s| s.len() + 4).unwrap_or(0)];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        let mut entries: usize = 3; // 0, 1, 2 always present
        if target_hive.is_some() {
            entries += 1;
        }
        if target_class.is_some() {
            entries += 1;
        }
        enc.map(entries).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.kv(1, &Value::Text(event_class)).expect("event_class");
        enc.kv(2, &Value::Bytes(inner_payload)).expect("payload");
        if let Some(h) = target_hive {
            enc.kv(3, &Value::UInt(h)).expect("target_hive");
        }
        if let Some(c) = target_class {
            enc.kv(4, &Value::Text(c)).expect("target_class");
        }
        enc.len()
    };
    build_response_frame(EV_EVENT_SEND, &buf[..used])
}

/// Request with only a correlation_id in the payload.
fn build_empty_request(event_class: &str, correlation_id: u64) -> Vec<u8> {
    let mut payload = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut payload);
        enc.map(1).expect("map header");
        enc.kv(CBOR_KEY_CORRELATION_ID, &Value::UInt(correlation_id))
            .expect("correlation_id");
        enc.len()
    };
    build_response_frame(event_class, &payload[..used])
}

/// Status-response fields, decoded from a response frame's payload.
#[derive(Debug)]
pub struct DaemonStatusResponse {
    pub correlation_id: u64,
    pub version: String,
    pub build_hash: String,
    pub uptime_seconds: u64,
}

/// Decode a response frame back into a [`DaemonStatusResponse`].
pub fn parse_status_response(frame: &[u8]) -> Result<DaemonStatusResponse, String> {
    let msg = r2_wire::decode_extended(frame).map_err(|e| format!("decode_extended: {e:?}"))?;
    let status_hash = r2_hash(EV_DAEMON_STATUS).expect("known-good event name");
    if msg.header.event_hash != status_hash {
        return Err(format!(
            "unexpected event_hash: 0x{:08X}",
            msg.header.event_hash
        ));
    }
    let mut dec = Decoder::new(msg.payload);
    let item = dec.next().map_err(|e| format!("cbor header: {e:?}"))?;
    let entries = match item {
        Item::Map(n) => n,
        _ => return Err("payload is not a CBOR map".to_string()),
    };
    let mut correlation_id: Option<u64> = None;
    let mut version: Option<String> = None;
    let mut build_hash: Option<String> = None;
    let mut uptime_seconds: Option<u64> = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        let Item::UInt(k) = key else { continue };
        match (k, val) {
            (CBOR_KEY_CORRELATION_ID, Item::UInt(n)) => correlation_id = Some(n),
            (CBOR_KEY_VERSION, Item::Text(bytes)) => {
                version = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|e| format!("version utf8: {e}"))?
                        .to_string(),
                );
            }
            (CBOR_KEY_BUILD_HASH, Item::Text(bytes)) => {
                build_hash = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|e| format!("build_hash utf8: {e}"))?
                        .to_string(),
                );
            }
            (CBOR_KEY_UPTIME_SECONDS, Item::UInt(n)) => uptime_seconds = Some(n),
            _ => {}
        }
    }
    Ok(DaemonStatusResponse {
        correlation_id: correlation_id.ok_or("missing correlation_id")?,
        version: version.ok_or("missing version")?,
        build_hash: build_hash.ok_or("missing build_hash")?,
        uptime_seconds: uptime_seconds.ok_or("missing uptime_seconds")?,
    })
}

/// Identity-status response fields, decoded from a response frame's payload.
#[derive(Debug)]
pub struct IdentityStatusResponse {
    pub correlation_id: u64,
    pub present: bool,
    pub fingerprint: String,
    pub backend: String,
    pub path: String,
    pub created_this_start: bool,
}

/// Decode an identity-status response frame.
pub fn parse_identity_status_response(frame: &[u8]) -> Result<IdentityStatusResponse, String> {
    let msg = r2_wire::decode_extended(frame).map_err(|e| format!("decode_extended: {e:?}"))?;
    let expected = r2_hash(EV_IDENTITY_STATUS).expect("known-good event name");
    if msg.header.event_hash != expected {
        return Err(format!(
            "unexpected event_hash: 0x{:08X}",
            msg.header.event_hash
        ));
    }
    let mut dec = Decoder::new(msg.payload);
    let item = dec.next().map_err(|e| format!("cbor header: {e:?}"))?;
    let entries = match item {
        Item::Map(n) => n,
        _ => return Err("payload is not a CBOR map".to_string()),
    };
    let mut correlation_id: Option<u64> = None;
    let mut present: Option<bool> = None;
    let mut fingerprint: Option<String> = None;
    let mut backend: Option<String> = None;
    let mut path: Option<String> = None;
    let mut created_this_start: Option<bool> = None;
    for _ in 0..entries {
        let key = dec.next().map_err(|e| format!("cbor key: {e:?}"))?;
        let val = dec.next().map_err(|e| format!("cbor val: {e:?}"))?;
        let Item::UInt(k) = key else { continue };
        match (k, val) {
            (CBOR_KEY_CORRELATION_ID, Item::UInt(n)) => correlation_id = Some(n),
            (CBOR_KEY_IDENTITY_PRESENT, Item::Bool(b)) => present = Some(b),
            (CBOR_KEY_IDENTITY_FINGERPRINT, Item::Text(bytes)) => {
                fingerprint = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|e| format!("fingerprint utf8: {e}"))?
                        .to_string(),
                );
            }
            (CBOR_KEY_IDENTITY_BACKEND, Item::Text(bytes)) => {
                backend = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|e| format!("backend utf8: {e}"))?
                        .to_string(),
                );
            }
            (CBOR_KEY_IDENTITY_PATH, Item::Text(bytes)) => {
                path = Some(
                    std::str::from_utf8(bytes)
                        .map_err(|e| format!("path utf8: {e}"))?
                        .to_string(),
                );
            }
            (CBOR_KEY_IDENTITY_CREATED_THIS_START, Item::Bool(b)) => {
                created_this_start = Some(b)
            }
            _ => {}
        }
    }
    Ok(IdentityStatusResponse {
        correlation_id: correlation_id.ok_or("missing correlation_id")?,
        present: present.ok_or("missing present")?,
        fingerprint: fingerprint.ok_or("missing fingerprint")?,
        backend: backend.ok_or("missing backend")?,
        path: path.ok_or("missing path")?,
        created_this_start: created_this_start.ok_or("missing created_this_start")?,
    })
}
