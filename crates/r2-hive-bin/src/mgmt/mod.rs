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

/// Whether `path` is the WORLD-WRITABLE /tmp fallback socket (the no-env
/// arm of [`default_socket_path`]): parent is exactly `/tmp` and the file
/// name matches `r2tgd-<uid>.sock`.
///
/// Why it matters (R2-TG-TOOL §5.1 v0.4, specs 8ea8e22): /tmp is
/// world-writable, so a foreign-UID process can PRE-BIND this name and
/// impersonate the daemon to a connecting client — the daemon-side
/// same-UID accept check does not protect the CLIENT side. Canon MUSTs:
/// (a) clients connecting via this path MUST peer-verify the socket's UID
/// (SO_PEERCRED) before trusting it; (b) a daemon finding the name
/// squatted MUST fail loudly and never silently pick another name. The
/// XDG_RUNTIME_DIR / TMPDIR arms are per-user directories and need
/// neither check.
///
/// **Used-by:** `r2hive-cli::connect` (applies MUST (a)) and
/// `mgmt/socket.rs::spawn` (applies MUST (b)).
pub fn is_tmp_fallback_socket(path: &std::path::Path) -> bool {
    path.parent() == Some(std::path::Path::new("/tmp"))
        && path
            .file_name()
            .and_then(|f| f.to_str())
            .is_some_and(|f| {
                f.strip_prefix("r2tgd-")
                    .and_then(|rest| rest.strip_suffix(".sock"))
                    .is_some_and(|uid| !uid.is_empty() && uid.bytes().all(|b| b.is_ascii_digit()))
            })
}

/// This process's real UID — pub(crate) so the squat guard and peer
/// checks compare against the same identity the socket paths embed.
///
/// **Used-by:** [`default_socket_path`] (fallback name),
/// `mgmt/socket.rs` (squat-guard owner compare).
#[cfg(unix)]
pub(crate) unsafe fn libc_uid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

#[cfg(not(unix))]
unsafe fn libc_uid() -> u32 {
    0
}

#[cfg(test)]
mod fallback_path_tests {
    use super::is_tmp_fallback_socket;
    use std::path::Path;

    /// Pin the §5.1 v0.4 fallback-detector shape: exactly /tmp parent +
    /// r2tgd-<digits>.sock. Per-user dirs and lookalike names must NOT
    /// trigger the world-writable-path guards.
    #[test]
    fn detects_only_the_real_tmp_fallback() {
        assert!(is_tmp_fallback_socket(Path::new("/tmp/r2tgd-1000.sock")));
        assert!(is_tmp_fallback_socket(Path::new("/tmp/r2tgd-0.sock")));
        // Per-user runtime dirs — not the fallback.
        assert!(!is_tmp_fallback_socket(Path::new("/run/user/1000/r2tgd.sock")));
        // TMPDIR-style per-user path under /tmp is still not the bare-/tmp shape.
        assert!(!is_tmp_fallback_socket(Path::new("/tmp/user-1000/r2tgd.sock")));
        // Lookalikes.
        assert!(!is_tmp_fallback_socket(Path::new("/tmp/r2tgd.sock")));
        assert!(!is_tmp_fallback_socket(Path::new("/tmp/r2tgd-abc.sock")));
        assert!(!is_tmp_fallback_socket(Path::new("/tmp/r2tgd-.sock")));
        assert!(!is_tmp_fallback_socket(Path::new("/tmp/r2tgd-1000.sock.bak")));
    }
}
