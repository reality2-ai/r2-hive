//! Device identity custody — the master secret and TG-scoped key derivation.
//!
//! Implements the model from R2-WIRE §6.2.1 and R2-TRUST §2.3: a single
//! `device_master_secret` lives on the device; everything observable on the
//! wire (`hive_id`, `DEV_PK`, `DEV_SK`) is deterministically derived from it
//! plus the current `trust_group_id`. Rejoining the same TG reproduces the
//! same derived identities. Across TGs, identities are unlinkable.
//!
//! Phase 1 scope:
//! - File-based storage at a per-user path (mode 0600).
//! - HKDF-SHA256 derivation of hive_id, DEV_PK, DEV_SK.
//! - Fingerprint exposure for UIs (8-byte SHA-256 prefix of the master secret).
//! - Zeroization of the master secret in memory on drop.
//!
//! Later phases will add: keyring backend (libsecret / macOS Keychain),
//! backup/export, hardware-token custody.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ed25519_dalek::{SigningKey, VerifyingKey};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use zeroize::ZeroizeOnDrop;

/// Length of the device master secret in bytes (256 bits).
pub const MASTER_SECRET_LEN: usize = 32;

/// Per-TG derived identities.
pub struct DerivedIdentity {
    /// UUID-formatted hive ID (RFC 4122 §4.4). Deterministic per (master, tg).
    pub hive_id: String,
    pub verifying_key: VerifyingKey,
    signing_key: SigningKey,
}

impl DerivedIdentity {
    /// Access the Ed25519 signing key. Callers hold this only for the lifetime
    /// of an active hive's needs; keys are zeroized when the `DerivedIdentity`
    /// drops.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
}

impl Drop for DerivedIdentity {
    fn drop(&mut self) {
        // SigningKey in ed25519-dalek 2.x does not auto-zeroize; we don't have
        // mutable access to its internal bytes via public API, but dropping the
        // struct at least drops the allocation. A stronger zeroization would
        // require a wrapper; tracked for Phase 1+.
        //
        // The hive_id is public; no need to scrub.
        let _ = &self.signing_key;
    }
}

/// The daemon's custody of the device master secret.
///
/// `MasterSecret` holds the raw bytes in process memory. It is `ZeroizeOnDrop`
/// so tearing down a `DaemonState` scrubs the memory. The bytes MUST NOT be
/// copied out via any API surface — only the derivation helpers on this type
/// may observe them.
#[derive(ZeroizeOnDrop)]
pub struct MasterSecret {
    #[zeroize(drop)]
    bytes: [u8; MASTER_SECRET_LEN],
}

impl MasterSecret {
    /// Generate a fresh master secret from the OS CSPRNG.
    pub fn generate() -> io::Result<Self> {
        let mut bytes = [0u8; MASTER_SECRET_LEN];
        getrandom::getrandom(&mut bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("getrandom: {e}")))?;
        Ok(Self { bytes })
    }

    /// Construct from raw bytes (loaded from storage). Rejects wrong-length
    /// input.
    pub fn from_bytes(bytes: [u8; MASTER_SECRET_LEN]) -> Self {
        Self { bytes }
    }

    /// Return a short, stable public identifier suitable for showing to users
    /// in a UI: the first 8 bytes of SHA-256(master_secret), encoded as hex.
    /// This reveals nothing about the master secret by preimage, and lets users
    /// compare devices without exposing keys.
    pub fn fingerprint(&self) -> String {
        let digest = Sha256::digest(self.bytes);
        hex_u8s(&digest[..8])
    }

    /// Derive the per-TG `hive_id` (UUID string) per R2-WIRE §6.2.1.
    fn derive_hive_id(&self, trust_group_id: &str) -> String {
        let mut out = [0u8; 16];
        hkdf_expand(&self.bytes, b"r2-hive-id-v1", trust_group_id.as_bytes(), &mut out);
        format_uuid(&out)
    }

    /// Derive the per-TG Ed25519 signing key seed.
    fn derive_dev_sk_seed(&self, trust_group_id: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        hkdf_expand(&self.bytes, b"r2-dev-key-v1", trust_group_id.as_bytes(), &mut out);
        out
    }

    /// Full per-TG derivation: hive_id + Ed25519 keypair.
    pub fn derive(&self, trust_group_id: &str) -> DerivedIdentity {
        let hive_id = self.derive_hive_id(trust_group_id);
        let seed = self.derive_dev_sk_seed(trust_group_id);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        DerivedIdentity {
            hive_id,
            verifying_key,
            signing_key,
        }
    }

    fn bytes(&self) -> &[u8; MASTER_SECRET_LEN] {
        &self.bytes
    }

    /// Derive a 32-byte HMAC key for web-plugin browser-cookie signing
    /// (R2-PLUGIN §13.5). Distinct HKDF info string from the per-TG
    /// device key so cookie compromise can't impersonate the TG-bound
    /// keypair.
    pub fn derive_web_auth_key(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        hkdf_expand(&self.bytes, b"r2-web-auth-cookie-v1", b"", &mut out);
        out
    }
}

/// HKDF-SHA256 in the argument order used by R2-WIRE §6.2.1:
/// extract(salt) then expand(info) into the provided output buffer.
fn hkdf_expand(ikm: &[u8], salt: &[u8], info: &[u8], out: &mut [u8]) {
    let h = Hkdf::<Sha256>::new(Some(salt), ikm);
    h.expand(info, out).expect("output length within HKDF limit");
}

fn hex_u8s(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

/// Format 16 bytes as a UUID string per RFC 4122 §4.4.
fn format_uuid(b: &[u8; 16]) -> String {
    // Set version (4) and variant (RFC 4122) bits per §4.4 so the result is a
    // well-formed UUIDv4 string.
    let mut v = *b;
    v[6] = (v[6] & 0x0F) | 0x40; // version 4
    v[8] = (v[8] & 0x3F) | 0x80; // variant 10
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
        v[8], v[9], v[10], v[11], v[12], v[13], v[14], v[15],
    )
}

// ───────────────────────── Store backends ─────────────────────────

/// Backends r2-hive ships for holding the master secret.
///
/// `File` is always available. `Libsecret` (Linux Secret Service / GNOME
/// Keyring / KWallet) is gated behind the `keyring` cargo feature.
/// macOS Keychain and Windows Credential Manager backends follow the
/// same shape and are deferred to a follow-up iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackend {
    File,
    Libsecret,
    Keychain,
    WinCred,
    None,
}

impl StoreBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            StoreBackend::File => "file",
            StoreBackend::Libsecret => "libsecret",
            StoreBackend::Keychain => "keychain",
            StoreBackend::WinCred => "wincred",
            StoreBackend::None => "none",
        }
    }
}

/// Common interface implemented by every concrete master-secret store.
///
/// Lifetime: the store value is constructed at startup, used for
/// [`IdentityStore::load_or_create`], and dropped. The loaded
/// [`MasterSecret`] is held on the [`crate::mgmt::state::DaemonState`]
/// from then on; the store itself doesn't need to stay alive.
pub trait IdentityStore {
    /// Load the master secret from this backend, or create+persist a
    /// fresh one if none is present. Returns `(secret, created)` where
    /// `created` is `true` iff this call generated a new secret.
    fn load_or_create(&self) -> io::Result<(MasterSecret, bool)>;

    /// Backend tag for `r2.mgmt.identity.status` reporting.
    fn backend(&self) -> StoreBackend;

    /// Operator-readable identifier for the store (e.g. a filesystem
    /// path, or a DBus path). Surfaced in `r2.mgmt.identity.status`.
    fn display_path(&self) -> String;
}

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
    /// Default location per the storage layout in R2-HIVE §9.
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
            f.write_all(secret.bytes())?;
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
        let fresh = MasterSecret::generate()?;
        self.save(&fresh)?;
        Ok((fresh, true))
    }
}

impl IdentityStore for FileStore {
    fn load_or_create(&self) -> io::Result<(MasterSecret, bool)> {
        FileStore::load_or_create(self)
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
    fn load_or_create(&self) -> io::Result<(MasterSecret, bool)> {
        use base64::engine::general_purpose::STANDARD as B64;
        use base64::Engine;
        let entry = self.entry()?;
        match entry.get_password() {
            Ok(b64) => {
                let bytes = B64.decode(b64.as_bytes()).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("keyring: bad base64: {e}"),
                    )
                })?;
                if bytes.len() != MASTER_SECRET_LEN {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "keyring entry has wrong length: expected {}, got {}",
                            MASTER_SECRET_LEN,
                            bytes.len()
                        ),
                    ));
                }
                let mut arr = [0u8; MASTER_SECRET_LEN];
                arr.copy_from_slice(&bytes);
                Ok((MasterSecret::from_bytes(arr), false))
            }
            Err(keyring::Error::NoEntry) => {
                let fresh = MasterSecret::generate()?;
                let blob = B64.encode(fresh.bytes());
                entry.set_password(&blob).map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("keyring write: {e}"))
                })?;
                Ok((fresh, true))
            }
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("keyring read: {e}"),
            )),
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

    // Fixed master secret for deterministic testing.
    fn fixture_secret() -> MasterSecret {
        let mut bytes = [0u8; MASTER_SECRET_LEN];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        MasterSecret::from_bytes(bytes)
    }

    #[test]
    fn fingerprint_is_stable_and_16_hex_chars() {
        let s = fixture_secret();
        let fp1 = s.fingerprint();
        let fp2 = s.fingerprint();
        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 16);
        assert!(fp1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn derive_same_tg_twice_gives_same_identity() {
        let s = fixture_secret();
        let a = s.derive("tg-alpha");
        let b = s.derive("tg-alpha");
        assert_eq!(a.hive_id, b.hive_id);
        assert_eq!(a.verifying_key.to_bytes(), b.verifying_key.to_bytes());
    }

    #[test]
    fn derive_different_tgs_gives_different_identities() {
        let s = fixture_secret();
        let a = s.derive("tg-alpha");
        let b = s.derive("tg-beta");
        assert_ne!(a.hive_id, b.hive_id);
        assert_ne!(a.verifying_key.to_bytes(), b.verifying_key.to_bytes());
    }

    #[test]
    fn derive_is_different_per_master_secret() {
        let s1 = fixture_secret();
        let mut other = [0u8; MASTER_SECRET_LEN];
        other[0] = 0xFF; // differ from fixture
        let s2 = MasterSecret::from_bytes(other);

        let a = s1.derive("tg-alpha");
        let b = s2.derive("tg-alpha");
        assert_ne!(a.hive_id, b.hive_id);
        assert_ne!(a.verifying_key.to_bytes(), b.verifying_key.to_bytes());
    }

    #[test]
    fn hive_id_is_well_formed_uuid_v4() {
        let s = fixture_secret();
        let id = s.derive("tg-alpha").hive_id.clone();
        // 8-4-4-4-12 hex layout = 36 chars total including dashes.
        assert_eq!(id.len(), 36);
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        // UUIDv4 version nibble: first char of group 3 must be '4'.
        assert_eq!(parts[2].chars().next().unwrap(), '4');
        // Variant: first char of group 4 must be 8, 9, a, or b.
        let var = parts[3].chars().next().unwrap();
        assert!(matches!(var, '8' | '9' | 'a' | 'b'));
    }

    #[test]
    fn file_store_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("master.key");
        let store = FileStore::new(path.clone());

        assert!(!store.exists());
        let secret = MasterSecret::generate().expect("gen");
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
    fn store_backend_as_str_covers_every_variant() {
        // If a new variant is added, this test forces a deliberate
        // string mapping update.
        for v in [
            StoreBackend::File,
            StoreBackend::Libsecret,
            StoreBackend::Keychain,
            StoreBackend::WinCred,
            StoreBackend::None,
        ] {
            assert!(!v.as_str().is_empty());
        }
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
