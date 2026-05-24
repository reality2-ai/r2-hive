//! Local management surface for r2-hive: device-scoped identity custody and
//! the `r2.mgmt.*` event vocabulary served over a Unix-domain socket.
//!
//! Specified in R2-HIVE §§3 (identity), §5 (local API), §6.3 (pairing gate).
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
#[cfg(target_os = "linux")]
pub mod usb;
pub mod ws;

pub use identity::{FileStore, MasterSecret, StoreBackend};
pub use state::DaemonState;

/// Management-socket path on the current platform/user.
///
/// Linux: `${XDG_RUNTIME_DIR}/r2-hive.sock`.
/// macOS: `${TMPDIR}/r2-hive.sock`.
///
/// Falls back to `/tmp/r2-hive-<uid>.sock` if neither env var is set.
pub fn default_socket_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(dir).join("r2-hive.sock");
    }
    if let Ok(dir) = std::env::var("TMPDIR") {
        return std::path::PathBuf::from(dir).join("r2-hive.sock");
    }
    let uid = unsafe { libc_uid() };
    std::path::PathBuf::from(format!("/tmp/r2-hive-{uid}.sock"))
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
