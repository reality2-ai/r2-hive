//! Local management surface for r2-hive: device-scoped identity custody and
//! the `r2.mgmt.*` event vocabulary served over a Unix-domain socket.
//!
//! Canon anchors: identity custody R2-TG-TOOL §3 + R2-WIRE §6.2.1;
//! local-API socket R2-TG-TOOL §5 + R2-HOST-API §2.2/§2.4; pairing gate
//! R2-PROVISION §5.3.4.
//! This module is the substrate-layer half of the r2-hive process; it runs
//! alongside the L1–L4 mesh machinery (route engine, WebSocket transport,
//! BLE/LoRa bindings) in the sibling modules.

pub mod api;
pub mod ensemble;
pub mod framing;
pub mod identity;
pub mod primitive;
pub mod socket;
pub mod state;
pub mod subscriptions;
pub mod transport_policy;
#[cfg(target_os = "linux")]
pub mod usb;
pub mod ws;

pub use identity::{FileStore, MasterSecret, StoreBackend};
pub use state::DaemonState;

/// Management-socket path on the current platform/user.
///
/// Linux: `${XDG_RUNTIME_DIR}/r2tgd.sock`.
/// macOS: `${TMPDIR}/r2tgd.sock`.
///
/// The FILENAME is normative, not just the directory discipline:
/// R2-TG-TOOL §5.1 (v0.3, specs fa94443) pins `r2tgd.sock` as the
/// well-known address so any spec-built UI reaches the daemon with zero
/// configuration — path + 0600 + same-UID + filename are one contract.
/// (Renamed from the daemon-local `r2-hive.sock`; specs fix_impl ruling.)
///
/// Falls back to `/tmp/r2tgd-<uid>.sock` if neither env var is set
/// (no-env fallback is outside the §5.1 table; name kept consistent).
///
/// **Used-by:** `main.rs` (daemon bind default) and `r2hive-cli`
/// (client connect default) — one function, both sides, so they cannot
/// disagree.
pub fn default_socket_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(dir).join("r2tgd.sock");
    }
    if let Ok(dir) = std::env::var("TMPDIR") {
        return std::path::PathBuf::from(dir).join("r2tgd.sock");
    }
    let uid = unsafe { libc_uid() };
    std::path::PathBuf::from(format!("/tmp/r2tgd-{uid}.sock"))
}

#[cfg(unix)]
unsafe fn libc_uid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

#[cfg(not(unix))]
unsafe fn libc_uid() -> u32 {
    0
}
