//! `r2.mgmt.usb.*` event handlers (Phase USB-4b).
//!
//! Surface for the operator-side USB peripheral controls: list
//! devices, prepare a path (extend the explicit allowlist), confirm
//! / abort a pending SAS prompt, unpair a previously-paired
//! peripheral. Drives the [`crate::usb_hotplug::UsbBringupHandle`]
//! captured during daemon bring-up.
//!
//! Linux-only — `r2-hive` on macOS/Windows doesn't run the USB
//! watcher, so these events return `usb_disabled`.
//!
//! Wire vocabulary (R2-HOST-API §4 `r2.mgmt.*` namespace extension):
//!
//! ```text
//! r2.mgmt.usb.list      — request: {0: cid}
//!                       — response: {0: cid, 1: [<device>...]}
//! r2.mgmt.usb.prepare   — request: {0: cid, 1: <path: text>}
//!                       — response: {0: cid, 1: <path: text>}
//! r2.mgmt.usb.confirm   — request: {0: cid, 1: <path: text>}
//!                       — response: {0: cid, 1: <accepted: bool>}
//! r2.mgmt.usb.abort     — request: {0: cid, 1: <path: text>}
//!                       — response: {0: cid, 1: <accepted: bool>}
//! r2.mgmt.usb.unpair    — request: {0: cid, 1: <hive_id_bytes: bytes(16)>}
//!                       — response: {0: cid, 1: <hive_id_bytes: bytes(16)>}
//! ```
//!
//! `<device>` (in the `list` response) is an integer-keyed CBOR map:
//!
//! ```text
//! {
//!   1: <path : text>,
//!   2: <session_state : uint>,         // 0=Initial 1=SyncSent 2=AwaitingCaps
//!                                       // 3=Reconnecting 4=PairingHelloSent
//!                                       // 5=PairingCommitReceived
//!                                       // 6=PairingAwaitingUser
//!                                       // 7=PairingConfirmSent 8=Active 9=Closed
//!   3?: <hive_id_bytes : bytes(16)>,       // present after CAPS
//!   4?: <firmware_id : text>,          // present after CAPS
//!   5?: <pending_sas : uint>,          // 6-digit SAS code awaiting confirm
//!   6?: <last_error : text>,           // most recent failure summary
//!   7?: <vid : uint>,                  // USB descriptor — present when /sys readable
//!   8?: <pid : uint>,                  // USB descriptor — present when /sys readable
//!   9?: <manufacturer : text>,
//!  10?: <product : text>
//! }
//! ```
//!
//! ## Interlinks + canon
//!
//! Dispatched from `api.rs`; every handler reaches the watcher through
//! `HiveState::usb_handle()` (installed by `main.rs`; absent →
//! `usb_disabled`). The confirm/abort verbs are the operator half of the
//! SAS flow whose crypto lives in `usb_pair.rs` and whose state machine is
//! `usb.rs::UsbSession` (crate root). Canon: R2-HOST-API §4 vocabulary —
//! `r2-specifications/specs/r2-core/R2-HOST-API.md`; R2-PROVISION §5.3.4
//! (SAS) — `r2-specifications/specs/r2-core/R2-PROVISION.md`.

use std::path::PathBuf;
use std::sync::Arc;

use r2_cbor::{Decoder, Encoder, Item, Value};

use super::api::{build_error_response, build_response_frame_with_event, extract_correlation_id};
use crate::hive::HiveState;
use crate::usb::SessionState;
use crate::usb_hotplug::DeviceStatus;

const K_CORRELATION: u64 = 0;
const K_DEVICES: u64 = 1;

const K_PATH: u64 = 1;
const K_STATE: u64 = 2;
const K_DEVICE_ID: u64 = 3;
const K_FIRMWARE_ID: u64 = 4;
const K_PENDING_SAS: u64 = 5;
const K_LAST_ERROR: u64 = 6;
const K_VID: u64 = 7;
const K_PID: u64 = 8;
const K_MANUFACTURER: u64 = 9;
const K_PRODUCT: u64 = 10;

const K_REQ_PATH: u64 = 1;
const K_REQ_DEVICE_ID: u64 = 1;

pub const EV_USB_LIST: &str = "r2.mgmt.usb.list";
pub const EV_USB_PREPARE: &str = "r2.mgmt.usb.prepare";
pub const EV_USB_CONFIRM: &str = "r2.mgmt.usb.confirm";
pub const EV_USB_ABORT: &str = "r2.mgmt.usb.abort";
pub const EV_USB_UNPAIR: &str = "r2.mgmt.usb.unpair";

fn session_state_code(s: SessionState) -> u64 {
    use SessionState::*;
    match s {
        Initial => 0,
        SyncSent => 1,
        AwaitingCaps => 2,
        Reconnecting => 3,
        PairingHelloSent => 4,
        PairingCommitReceived => 5,
        PairingAwaitingUser => 6,
        PairingConfirmSent => 7,
        Active => 8,
        Closed => 9,
    }
}

/// `r2.mgmt.usb.list` — snapshot every watched device (state, CAPS,
/// pairing status). `usb_disabled` when no watcher handle is installed.
///
/// **Used-by:** the `api.rs` dispatcher.
pub async fn handle_list(cid: u64, hive: &Arc<HiveState>) -> Vec<u8> {
    let handle = match hive.usb_handle() {
        Some(h) => h,
        None => return build_error_response(cid, "usb_disabled"),
    };
    let devices = handle.status();
    let mut buf = vec![0u8; 256 + devices.len() * 256];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("response map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.uint(K_DEVICES).expect("devices key");
        enc.array(devices.len()).expect("devices array");
        for d in &devices {
            encode_device(&mut enc, d);
        }
        enc.len()
    };
    build_response_frame_with_event(EV_USB_LIST, &buf[..used])
}

fn encode_device(enc: &mut Encoder<'_>, d: &DeviceStatus) {
    let mut nfields = 1u64; // path
    if d.session_state.is_some() {
        nfields += 1;
    }
    if d.hive_id_bytes.is_some() {
        nfields += 1;
    }
    if d.firmware_id.is_some() {
        nfields += 1;
    }
    if d.pending_sas.is_some() {
        nfields += 1;
    }
    if d.last_error.is_some() {
        nfields += 1;
    }
    if let Some(desc) = &d.descriptor {
        nfields += 2; // vid + pid
        if desc.manufacturer.is_some() {
            nfields += 1;
        }
        if desc.product.is_some() {
            nfields += 1;
        }
    }
    enc.map(nfields as usize).expect("device map");
    let path_str = d.path.to_string_lossy();
    enc.kv(K_PATH, &Value::Text(&path_str)).expect("path");
    if let Some(s) = d.session_state {
        enc.kv(K_STATE, &Value::UInt(session_state_code(s)))
            .expect("state");
    }
    if let Some(id) = &d.hive_id_bytes {
        enc.kv(K_DEVICE_ID, &Value::Bytes(id)).expect("hive_id_bytes");
    }
    if let Some(fw) = &d.firmware_id {
        enc.kv(K_FIRMWARE_ID, &Value::Text(fw)).expect("fw_id");
    }
    if let Some(sas) = d.pending_sas {
        enc.kv(K_PENDING_SAS, &Value::UInt(sas as u64))
            .expect("sas");
    }
    if let Some(err) = &d.last_error {
        enc.kv(K_LAST_ERROR, &Value::Text(err)).expect("err");
    }
    if let Some(desc) = &d.descriptor {
        enc.kv(K_VID, &Value::UInt(desc.vid as u64)).expect("vid");
        enc.kv(K_PID, &Value::UInt(desc.pid as u64)).expect("pid");
        if let Some(m) = &desc.manufacturer {
            enc.kv(K_MANUFACTURER, &Value::Text(m)).expect("mfr");
        }
        if let Some(p) = &desc.product {
            enc.kv(K_PRODUCT, &Value::Text(p)).expect("product");
        }
    }
}

/// `r2.mgmt.usb.prepare` — add a device path to the watcher's explicit
/// allowlist (the operator's "yes, talk to this one" ahead of a VID:PID
/// entry).
///
/// **Used-by:** the `api.rs` dispatcher.
pub async fn handle_prepare(cid: u64, payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let handle = match hive.usb_handle() {
        Some(h) => h,
        None => return build_error_response(cid, "usb_disabled"),
    };
    let path = match extract_text_field(payload, K_REQ_PATH) {
        Some(p) => p,
        None => return build_error_response(cid, "bad_frame"),
    };
    handle.prepare(PathBuf::from(&path));
    build_path_ack_response(EV_USB_PREPARE, cid, &path)
}

/// `r2.mgmt.usb.confirm` — operator confirms the pending SAS prompt for
/// `path` (the human half of R2-PROVISION §5.3.4).
///
/// **Used-by:** the `api.rs` dispatcher.
pub async fn handle_confirm(cid: u64, payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let handle = match hive.usb_handle() {
        Some(h) => h,
        None => return build_error_response(cid, "usb_disabled"),
    };
    let path = match extract_text_field(payload, K_REQ_PATH) {
        Some(p) => p,
        None => return build_error_response(cid, "bad_frame"),
    };
    let accepted = handle.confirm(std::path::Path::new(&path)).await;
    build_bool_ack_response(EV_USB_CONFIRM, cid, accepted)
}

/// `r2.mgmt.usb.abort` — operator rejects the pending SAS prompt (codes
/// didn't match).
///
/// **Used-by:** the `api.rs` dispatcher.
pub async fn handle_abort(cid: u64, payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let handle = match hive.usb_handle() {
        Some(h) => h,
        None => return build_error_response(cid, "usb_disabled"),
    };
    let path = match extract_text_field(payload, K_REQ_PATH) {
        Some(p) => p,
        None => return build_error_response(cid, "bad_frame"),
    };
    let accepted = handle.abort(std::path::Path::new(&path)).await;
    build_bool_ack_response(EV_USB_ABORT, cid, accepted)
}

/// `r2.mgmt.usb.unpair` — forget a stored link key by peer hive-id bytes;
/// the device re-pairs from scratch on next attach.
///
/// **Used-by:** the `api.rs` dispatcher.
pub async fn handle_unpair(cid: u64, payload: &[u8], hive: &Arc<HiveState>) -> Vec<u8> {
    let handle = match hive.usb_handle() {
        Some(h) => h,
        None => return build_error_response(cid, "usb_disabled"),
    };
    let hive_id_bytes = match extract_bstr16_field(payload, K_REQ_DEVICE_ID) {
        Some(d) => d,
        None => return build_error_response(cid, "bad_frame"),
    };
    handle.unpair(&hive_id_bytes);
    let mut buf = [0u8; 64];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_REQ_DEVICE_ID, &Value::Bytes(&hive_id_bytes))
            .expect("hive_id_bytes");
        enc.len()
    };
    build_response_frame_with_event(EV_USB_UNPAIR, &buf[..used])
}

fn build_path_ack_response(event: &str, cid: u64, path: &str) -> Vec<u8> {
    let mut buf = vec![0u8; 64 + path.len()];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_REQ_PATH, &Value::Text(path)).expect("path");
        enc.len()
    };
    build_response_frame_with_event(event, &buf[..used])
}

fn build_bool_ack_response(event: &str, cid: u64, accepted: bool) -> Vec<u8> {
    let mut buf = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(1u64, &Value::Bool(accepted)).expect("accepted");
        enc.len()
    };
    build_response_frame_with_event(event, &buf[..used])
}

fn extract_text_field(payload: &[u8], target: u64) -> Option<String> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(kk) = k {
            if kk == target {
                if let Item::Text(s) = v {
                    return std::str::from_utf8(s).ok().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

fn extract_bstr16_field(payload: &[u8], target: u64) -> Option<[u8; 16]> {
    let mut dec = Decoder::new(payload);
    let entries = match dec.next().ok()? {
        Item::Map(n) => n,
        _ => return None,
    };
    for _ in 0..entries {
        let k = dec.next().ok()?;
        let v = dec.next().ok()?;
        if let Item::UInt(kk) = k {
            if kk == target {
                if let Item::Bytes(b) = v {
                    if b.len() == 16 {
                        let mut a = [0u8; 16];
                        a.copy_from_slice(b);
                        return Some(a);
                    }
                }
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────
// Outbound request builders — used by the CLI and by mgmt-test code.
// ─────────────────────────────────────────────────────────────────────

/// Client-side encoder for `r2.mgmt.usb.list`.
///
/// **Used-by:** `r2hive-cli` (`usb list`) and `tests/usb_mgmt_integration.rs`.
pub fn build_list_request(cid: u64) -> Vec<u8> {
    build_empty_request(EV_USB_LIST, cid)
}

/// Client-side encoder for `r2.mgmt.usb.prepare`.
///
/// **Used-by:** `r2hive-cli` (`usb prepare`) and the usb mgmt tests.
pub fn build_prepare_request(cid: u64, path: &str) -> Vec<u8> {
    build_path_ack_response(EV_USB_PREPARE, cid, path)
}

/// Client-side encoder for `r2.mgmt.usb.confirm`.
///
/// **Used-by:** `r2hive-cli` (`usb confirm`) and the usb mgmt tests.
pub fn build_confirm_request(cid: u64, path: &str) -> Vec<u8> {
    build_path_ack_response(EV_USB_CONFIRM, cid, path)
}

/// Client-side encoder for `r2.mgmt.usb.abort`.
///
/// **Used-by:** `r2hive-cli` (`usb abort`) and the usb mgmt tests.
pub fn build_abort_request(cid: u64, path: &str) -> Vec<u8> {
    build_path_ack_response(EV_USB_ABORT, cid, path)
}

/// Client-side encoder for `r2.mgmt.usb.unpair` (16-byte peer id).
///
/// **Used-by:** `r2hive-cli` (`usb unpair`) and the usb mgmt tests.
pub fn build_unpair_request(cid: u64, hive_id_bytes: &[u8; 16]) -> Vec<u8> {
    let mut buf = [0u8; 64];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(2).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.kv(K_REQ_DEVICE_ID, &Value::Bytes(hive_id_bytes))
            .expect("hive_id_bytes");
        enc.len()
    };
    build_response_frame_with_event(EV_USB_UNPAIR, &buf[..used])
}

fn build_empty_request(event: &str, cid: u64) -> Vec<u8> {
    let mut buf = [0u8; 32];
    let used = {
        let mut enc = Encoder::new(&mut buf);
        enc.map(1).expect("map");
        enc.kv(K_CORRELATION, &Value::UInt(cid)).expect("cid");
        enc.len()
    };
    build_response_frame_with_event(event, &buf[..used])
}

#[allow(dead_code)]
fn extract_cid_for_test(payload: &[u8]) -> u64 {
    extract_correlation_id(payload).unwrap_or(0)
}
