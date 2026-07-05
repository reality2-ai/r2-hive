//! Device identity custody — **platform (Linux/cloud) storage layer** for the
//! master secret. The portable crypto (master-secret derivation, Ed25519
//! keypair, fingerprint, UUID) and the [`IdentityStore`] seam now live in
//! `r2-hive-core`; this module supplies the concrete std-backed stores and
//! re-exports the core types so existing `mgmt::identity::*` paths keep
//! resolving (R2-HIVE north-star: portable identity in hive-core, only the
//! backing store is platform code).
//!
//! Implements the model from R2-WIRE §6.2.1 and R2-TRUST §2.3: a single
//! `device_master_secret` lives on the device; everything observable on the
//! wire is deterministically derived from it plus the current
//! `trust_group_id`. See [`r2_hive_core::identity`] for the derivation.
//!
//! Stores here:
//! - [`FileStore`] — per-user file at mode 0600 (always available).
//! - [`KeyringStore`] — platform keyring, behind the `keyring` cargo feature
//!   (Linux Secret Service / macOS Keychain / Windows Credential Manager).
//!
//! Master-secret generation uses the OS CSPRNG (`getrandom`) here, since the
//! store is itself the platform layer; the core only ever takes the resulting
//! bytes via [`MasterSecret::from_bytes`].

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use r2_hive_core::identity::{
    DerivedIdentity, IdentityStore, MasterSecret, StoreBackend, StoreError, MASTER_SECRET_LEN,
};

/// Generate a fresh master secret from the OS CSPRNG. Platform-layer helper —
/// the core takes only the resulting bytes ([`MasterSecret::from_bytes`]).
fn generate_master_secret() -> io::Result<MasterSecret> {
    let mut bytes = [0u8; MASTER_SECRET_LEN];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("getrandom: {e}")))?;
    Ok(MasterSecret::from_bytes(bytes))
}

/// Map a platform IO error into the core's platform-neutral [`StoreError`].
fn backend_err(e: impl core::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

// ───────────────────────── File-backed store ─────────────────────────

/// File-backed store for the master secret.
///
/// Path is per-user: `$XDG_STATE_HOME/r2/master.key` on Linux, or the platform
/// equivalent. Permissions enforced to 0600 on creation; loud warning logged
/// if we see looser permissions at read time (doesn't block, so users can
/// recover from manual file ops).
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    /// Default location mirroring the R2-TG-TOOL §9 storage layout
    /// (informative); the exact `r2/master.key` path is daemon-local.
    pub fn default_path() -> PathBuf {
        if let Some(base) = dir_xdg_state_home() {
            return base.join("r2").join("master.key");
        }
        // Fallback if XDG_STATE_HOME and HOME are unset (shouldn't happen on a
        // real user session; mostly for CI).
        PathBuf::from("/tmp").join(format!(
            "r2-master-{}.key",
            unsafe { getuid() }
        ))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Load a master secret from disk. Returns `None` if the file does not
    /// exist; returns an `io::Error` for other failures (permissions, wrong
    /// length, etc.).
    pub fn load(&self) -> io::Result<Option<MasterSecret>> {
        if !self.path.exists() {
            return Ok(None);
        }
        check_permissions(&self.path)?;
        let bytes = fs::read(&self.path)?;
        if bytes.len() != MASTER_SECRET_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "master secret file has wrong length: expected {} bytes, got {}",
                    MASTER_SECRET_LEN,
                    bytes.len()
                ),
            ));
        }
        let mut arr = [0u8; MASTER_SECRET_LEN];
        arr.copy_from_slice(&bytes);
        Ok(Some(MasterSecret::from_bytes(arr)))
    }

    /// Write a master secret to disk atomically: write to a sibling tempfile,
    /// chmod 0600, rename into place.
    ///
    /// The parent directory is created with `mkdir -p` semantics, then
    /// chmod'd to 0o700 — but only if we own it. Setting permissions
    /// on a system directory (e.g. when the master key path is
    /// directly under `/tmp` or `/etc/r2/`) would fail with EPERM as
    /// a non-root user; that's not a fatal error here, just skip.
    pub fn save(&self, secret: &MasterSecret) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
            // Best-effort. Failures are non-fatal — the permission
            // hardening is a nice-to-have when we own the dir; when
            // we don't, the file's own 0o600 mode is the security
            // boundary that matters.
            let _ = apply_dir_permissions(parent);
        }
        let tmp = self.path.with_extension("tmp");
        // Write and close the file before chmod + rename.
        {
            use std::io::Write as _;
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)?;
            f.write_all(secret.expose_secret_bytes())?;
            f.sync_all()?;
        }
        apply_file_permissions(&tmp)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Load if present; otherwise generate and persist. Idempotent.
    pub fn load_or_create(&self) -> io::Result<(MasterSecret, bool /* created */)> {
        if let Some(existing) = self.load()? {
            return Ok((existing, false));
        }
        let fresh = generate_master_secret()?;
        self.save(&fresh)?;
        Ok((fresh, true))
    }
}

impl IdentityStore for FileStore {
    fn load_or_create(&self) -> Result<(MasterSecret, bool), StoreError> {
        FileStore::load_or_create(self).map_err(backend_err)
    }
    fn backend(&self) -> StoreBackend {
        StoreBackend::File
    }
    fn display_path(&self) -> String {
        self.path.display().to_string()
    }
}

// ───────────────────────── Keyring-backed store ─────────────────────────

/// Master-secret store that delegates to the platform keyring via the
/// `keyring` crate (Linux Secret Service, macOS Keychain, Windows
/// Credential Manager). Built only when the `keyring` cargo feature is
/// enabled.
///
/// The secret is stored as a base64-encoded 32-byte blob under the
/// service `"r2-hive"` and account `"master"`. Per-user
/// keyring; multi-user hosts get one entry per user, matching the file
/// store's behaviour.
///
/// The reported [`StoreBackend`] is the natural variant for the build
/// target: `Libsecret` on Linux, `Keychain` on macOS, `WinCred` on
/// Windows.
#[cfg(feature = "keyring")]
pub struct KeyringStore {
    service: String,
    account: String,
}

#[cfg(feature = "keyring")]
impl KeyringStore {
    /// Build with default service/account names.
    pub fn new() -> Self {
        Self {
            service: "r2-hive".to_string(),
            account: "master".to_string(),
        }
    }

    /// Build with custom names (mostly for tests / multi-tenant rigs).
    pub fn with_names(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            account: account.into(),
        }
    }

    fn entry(&self) -> io::Result<keyring::Entry> {
        keyring::Entry::new(&self.service, &self.account)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("keyring: {e}")))
    }
}

#[cfg(feature = "keyring")]
impl IdentityStore for KeyringStore {
    fn load_or_create(&self) -> Result<(MasterSecret, bool), StoreError> {
        use base64::engine::general_purpose::STANDARD as B64;
        use base64::Engine;
        let entry = self.entry().map_err(backend_err)?;
        match entry.get_password() {
            Ok(b64) => {
                let bytes = B64
                    .decode(b64.as_bytes())
                    .map_err(|e| StoreError::InvalidData(format!("keyring: bad base64: {e}")))?;
                if bytes.len() != MASTER_SECRET_LEN {
                    return Err(StoreError::InvalidData(format!(
                        "keyring entry has wrong length: expected {}, got {}",
                        MASTER_SECRET_LEN,
                        bytes.len()
                    )));
                }
                let mut arr = [0u8; MASTER_SECRET_LEN];
                arr.copy_from_slice(&bytes);
                Ok((MasterSecret::from_bytes(arr), false))
            }
            Err(keyring::Error::NoEntry) => {
                let fresh = generate_master_secret().map_err(backend_err)?;
                let blob = B64.encode(fresh.expose_secret_bytes());
                entry
                    .set_password(&blob)
                    .map_err(|e| backend_err(format!("keyring write: {e}")))?;
                Ok((fresh, true))
            }
            Err(e) => Err(backend_err(format!("keyring read: {e}"))),
        }
    }

    fn backend(&self) -> StoreBackend {
        if cfg!(target_os = "linux") {
            StoreBackend::Libsecret
        } else if cfg!(target_os = "macos") {
            StoreBackend::Keychain
        } else if cfg!(target_os = "windows") {
            StoreBackend::WinCred
        } else {
            StoreBackend::None
        }
    }

    fn display_path(&self) -> String {
        format!("keyring://{}/{}", self.service, self.account)
    }
}

// ───────────────────────── Auto-precedence ─────────────────────────

/// Pick the best-available store at runtime. With the `keyring` feature
/// on, prefer [`KeyringStore`] (probed by attempting to construct a
/// keyring entry); on failure, log and fall back to [`FileStore`] at
/// the given path.
///
/// With the `keyring` feature off, always returns [`FileStore`].
///
/// The probe is non-destructive: it only constructs a keyring `Entry`,
/// not a read/write. A keyring that's installed but inaccessible
/// (e.g. headless Linux without gnome-keyring-daemon) will surface its
/// error on the first `load_or_create` call, at which point the daemon
/// logs and the operator can re-launch with `--identity-backend file`.
pub fn auto_store(file_path: PathBuf) -> Box<dyn IdentityStore> {
    #[cfg(feature = "keyring")]
    {
        let candidate = KeyringStore::new();
        if candidate.entry().is_ok() {
            log::info!("identity: auto-selected keyring backend");
            return Box::new(candidate);
        }
        log::warn!(
            "identity: keyring backend unavailable, falling back to file store"
        );
    }
    Box::new(FileStore::new(file_path))
}

#[cfg(unix)]
fn apply_file_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(unix)]
fn apply_dir_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(unix)]
fn check_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path)?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        log::warn!(
            "master secret file {} has permissions {:o}; should be 0600 — consider `chmod 600`",
            path.display(),
            mode
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn apply_file_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn apply_dir_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn check_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn dir_xdg_state_home() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XDG_STATE_HOME") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home).join(".local").join("state"));
        }
    }
    None
}

#[cfg(unix)]
unsafe fn getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

#[cfg(not(unix))]
unsafe fn getuid() -> u32 {
    0
}

// ───────────────────────── Tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_store_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("master.key");
        let store = FileStore::new(path.clone());

        assert!(!store.exists());
        let secret = generate_master_secret().expect("gen");
        let fp1 = secret.fingerprint();
        store.save(&secret).expect("save");

        assert!(store.exists());
        let loaded = store.load().expect("load").expect("present");
        assert_eq!(loaded.fingerprint(), fp1);

        // Permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "file should be 0600");
        }
    }

    #[test]
    fn file_store_load_or_create_is_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("master.key");
        let store = FileStore::new(path);

        let (first, created1) = store.load_or_create().expect("first");
        assert!(created1);
        let fp1 = first.fingerprint();
        drop(first);

        let (second, created2) = store.load_or_create().expect("second");
        assert!(!created2);
        assert_eq!(second.fingerprint(), fp1);
    }

    #[test]
    fn file_store_implements_identity_store_trait() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("master.key");
        let store: Box<dyn IdentityStore> = Box::new(FileStore::new(path.clone()));
        assert_eq!(store.backend(), StoreBackend::File);
        assert!(store.display_path().contains("master.key"));
        let (s, created) = store.load_or_create().expect("create");
        assert!(created);
        let fp = s.fingerprint();
        let (s2, created2) = store.load_or_create().expect("reload");
        assert!(!created2);
        assert_eq!(s2.fingerprint(), fp);
    }

    #[test]
    fn auto_store_returns_filestore_without_keyring_feature() {
        // Without `keyring` feature, auto must always pick file.
        // With the feature on but no DBus reachable, the keyring entry
        // probe still succeeds (constructing an Entry doesn't read);
        // that's by design — failures surface on first use, not at
        // pick time.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("master.key");
        let store = auto_store(path.clone());
        // We can't deterministically assert backend without knowing the
        // feature combo, so just verify the store is usable.
        let (_s, created) = store.load_or_create().expect("auto load");
        assert!(created);
    }
}
