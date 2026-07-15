//! Daemon state: version / build metadata, identity custody, uptime.
//!
//! Optionally carries an `Arc<HiveState>` so primitive (`r2.api.*`) handlers
//! per R2-HOST-API can reach the wire / route / transport layer. Tests and
//! mgmt-only deployments can construct a `DaemonState` without it via
//! [`DaemonState::new`].
//!
//! ## Interlinks + canon
//!
//! Constructed in `main.rs` via `with_identity_store` (identity custody
//! before anything mgmt-facing); cloned into `socket.rs`/`ws.rs`
//! connections; `attach_hive_state` links it to the mesh half so
//! `primitive.rs` can route. `derive_web_auth_key` seeds `web_auth.rs`
//! from the master secret (R2-PLUGIN §13.5). Custody canon: R2-TG-TOOL §3
//! + R2-WIRE §6.2.1 —
//! `r2-specifications/specs/r2-core/{R2-TG-TOOL,R2-WIRE}.md`.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use crate::hive::HiveState;

use super::identity::{FileStore, IdentityStore, MasterSecret, StoreBackend};

/// R2-BUILDMODE §6.3: the version string carries the BUILD MODE so the
/// artifact and every runtime surface that echoes it (daemon.status, logs)
/// declare which-code-was-flashed. PROD = bare semver (absence-is-prod,
/// mirroring the beacon rule); DEV = "+dev" suffix.
#[cfg(feature = "dev")]
pub const BUILD_MODE_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+dev");
#[cfg(not(feature = "dev"))]
pub const BUILD_MODE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct DaemonState {
    inner: Arc<Inner>,
    /// HiveState reference, set once after construction so primitive
    /// (`r2.api.*`) handlers can reach the wire / route / transport layer.
    /// Stored separately from `Inner` so attachment doesn't require
    /// cloning the (zeroize-on-drop) master secret.
    hive_state: Arc<OnceLock<Arc<HiveState>>>,
}

struct Inner {
    pub version: &'static str,
    pub build_hash: &'static str,
    pub started_at: Instant,
    pub identity: Option<IdentityHandle>,
}

pub struct IdentityHandle {
    pub master_secret: MasterSecret,
    pub backend: StoreBackend,
    /// Operator-readable identifier for the backing store. Filesystem
    /// path for `FileStore`, `keyring://service/account` for keyring
    /// backends. Empty for `StoreBackend::None`.
    pub path: PathBuf,
    pub created_this_start: bool,
}

impl DaemonState {
    /// Construct a daemon state without identity custody and without a
    /// HiveState reference (used by tests that don't need either; the main
    /// binary uses [`Self::with_identity`] then attaches a HiveState via
    /// [`Self::attach_hive_state`]).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                version: BUILD_MODE_VERSION,
                build_hash: option_env!("R2TGD_BUILD_HASH").unwrap_or("unversioned"),
                started_at: Instant::now(),
                identity: None,
            }),
            hive_state: Arc::new(OnceLock::new()),
        }
    }

    /// Construct a daemon state with identity loaded/created from the given
    /// file store. Returns an error if the store can't be read/written.
    /// HiveState is unset; attach via [`Self::attach_hive_state`] to enable
    /// the primitive (`r2.api.*`) surface.
    pub fn with_identity(store: &FileStore) -> std::io::Result<Self> {
        Self::with_identity_store(store)
    }

    /// Polymorphic variant — accepts any [`IdentityStore`] implementation
    /// (FileStore, KeyringStore, …). Used by `main.rs` after
    /// `--identity-backend` resolution.
    pub fn with_identity_store<S: IdentityStore + ?Sized>(store: &S) -> std::io::Result<Self> {
        // The store seam now lives in r2-hive-core with a platform-neutral
        // `StoreError`; map it back to `io::Error` for the daemon's startup path.
        let (master_secret, created) = store
            .load_or_create()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let identity = IdentityHandle {
            master_secret,
            backend: store.backend(),
            path: PathBuf::from(store.display_path()),
            created_this_start: created,
        };
        Ok(Self {
            inner: Arc::new(Inner {
                version: BUILD_MODE_VERSION,
                build_hash: option_env!("R2TGD_BUILD_HASH").unwrap_or("unversioned"),
                started_at: Instant::now(),
                identity: Some(identity),
            }),
            hive_state: Arc::new(OnceLock::new()),
        })
    }

    /// Attach a HiveState reference, enabling the `r2.api.*` primitive
    /// surface. Idempotent: subsequent calls are silently ignored
    /// (OnceLock semantics). Clones of the `DaemonState` made before this
    /// call will see the attached HiveState because the OnceLock is shared
    /// by Arc.
    pub fn attach_hive_state(&self, hive: Arc<HiveState>) {
        let _ = self.hive_state.set(hive);
    }

    /// Borrow the attached HiveState if one was provided, else `None`.
    /// Primitive handlers use this to gate behaviour: when no HiveState is
    /// attached, peer-list returns just self, send returns `unsupported`,
    /// etc.
    pub fn hive_state(&self) -> Option<&Arc<HiveState>> {
        self.hive_state.get()
    }

    /// Daemon semver (compile-time `CARGO_PKG_VERSION`).
    ///
    /// **Used-by:** `api.rs` (`r2.mgmt.hello` / status responses).
    pub fn version(&self) -> &'static str {
        self.inner.version
    }

    /// Build identifier baked in at compile time.
    ///
    /// **Used-by:** `api.rs` status responses.
    pub fn build_hash(&self) -> &'static str {
        self.inner.build_hash
    }

    /// Seconds since this DaemonState was constructed (daemon start).
    ///
    /// **Used-by:** `api.rs` status responses.
    pub fn uptime_seconds(&self) -> u64 {
        self.inner.started_at.elapsed().as_secs()
    }

    /// Identity presence flag — `true` iff the daemon is holding a master
    /// secret in memory.
    pub fn identity_present(&self) -> bool {
        self.inner.identity.is_some()
    }

    /// Short public identifier of the master secret — 16-hex-char SHA-256
    /// prefix. Empty string if no identity is loaded. Safe for UI display;
    /// reveals no preimage information.
    pub fn identity_fingerprint(&self) -> String {
        self.inner
            .identity
            .as_ref()
            .map(|h| h.master_secret.fingerprint())
            .unwrap_or_default()
    }

    /// Derive the web-auth cookie HMAC key (R2-PLUGIN §13.5) from the
    /// loaded master secret, or `None` if no identity is loaded.
    pub fn derive_web_auth_key(&self) -> Option<[u8; 32]> {
        self.inner
            .identity
            .as_ref()
            .map(|h| h.master_secret.derive_web_auth_key())
    }

    /// Backend name (e.g. `"file"`) for the identity store, or `"none"` when
    /// no identity is loaded.
    pub fn identity_backend(&self) -> &'static str {
        self.inner
            .identity
            .as_ref()
            .map(|h| h.backend.as_str())
            .unwrap_or("none")
    }

    /// Path the identity store reads/writes. Empty when none.
    pub fn identity_path(&self) -> String {
        self.inner
            .identity
            .as_ref()
            .map(|h| h.path.display().to_string())
            .unwrap_or_default()
    }

    /// `true` iff this daemon start generated a fresh master secret (first
    /// boot on this storage).
    pub fn identity_created_this_start(&self) -> bool {
        self.inner
            .identity
            .as_ref()
            .map(|h| h.created_this_start)
            .unwrap_or(false)
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}
