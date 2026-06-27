//! r2-hive configuration file loader (Phase 4b of the plan).
//!
//! Layered defaults — every field has a compiled-in default, overridden
//! by `$XDG_CONFIG_HOME/r2/hive.toml` (or `~/.config/r2/hive.toml`),
//! further overridden by CLI flags in `main.rs`.
//!
//! The file is optional; a missing or empty file is not an error and
//! the daemon runs on compiled-in defaults. A malformed file IS an
//! error — operators expect a clear failure rather than silent
//! defaults when their config is wrong.
//!
//! See `packaging/defaults/hive.toml` for a fully-commented example.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level config layout. Every section is optional; missing sections
/// fall back to compiled-in defaults.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct HiveConfig {
    pub daemon: DaemonConfig,
    pub transports: TransportsConfig,
    pub identity: IdentityConfig,
    pub management: ManagementConfig,
}

/// `[daemon]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DaemonConfig {
    /// Device name (mDNS/BLE advertisement; influences hive_id).
    pub name: String,
    /// Bind address for HTTP/WS listener.
    pub bind: String,
    /// Listener port.
    pub port: u16,
    /// Per-TG event buffer.
    pub buffer_size: usize,
    /// Maximum concurrent connections.
    pub max_connections: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            name: "r2-hive".to_string(),
            bind: "127.0.0.1".to_string(),
            port: 21042,
            buffer_size: 1000,
            max_connections: 10000,
        }
    }
}

/// `[transports]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransportsConfig {
    /// When true, run `autoconfig::detect_profile()` at startup and
    /// honour its suggestions for any transport not explicitly enabled
    /// here.
    pub auto: bool,
    /// Force-enable UDP LAN transport + mDNS/UDP beacon. Honoured
    /// regardless of `auto`.
    pub lan: bool,
    /// Force-enable BLE transport (requires `ble` cargo feature).
    pub ble: bool,
    /// Force-enable LoRa transport via arduino-router IPC (requires
    /// `lora` cargo feature).
    pub lora: bool,
    /// Path to the arduino-router socket. Only used when `lora` is on.
    pub lora_socket: PathBuf,
}

impl Default for TransportsConfig {
    fn default() -> Self {
        Self {
            auto: false,
            lan: false,
            ble: false,
            lora: false,
            lora_socket: PathBuf::from("/var/run/arduino-router.sock"),
        }
    }
}

/// `[identity]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IdentityConfig {
    /// Backend selection — `auto`, `file`, or `keyring`.
    pub backend: String,
    /// File-store path. Only honoured when the resolved backend is
    /// `file`. `None` falls back to `$XDG_STATE_HOME/r2/master.key`.
    pub store: Option<PathBuf>,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            backend: "auto".to_string(),
            store: None,
        }
    }
}

/// `[management]` section.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManagementConfig {
    /// Whether the local management API is started. False is
    /// equivalent to `--no-mgmt`.
    pub enabled: bool,
    /// Override the management socket path. `None` resolves at
    /// runtime to `${XDG_RUNTIME_DIR}/r2-hive.sock` (Linux) or
    /// `${TMPDIR}/r2-hive.sock` (macOS).
    pub socket: Option<PathBuf>,
}

impl Default for ManagementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            socket: None,
        }
    }
}

/// Errors from config loading. `Missing` is not surfaced to callers —
/// `load_optional` swallows it and returns defaults.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// I/O error reading the config file.
    #[error("config: read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// TOML parse / structural validation failure.
    #[error("config: parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

impl HiveConfig {
    /// Default location: `$XDG_CONFIG_HOME/r2/hive.toml`, or
    /// `~/.config/r2/hive.toml` if XDG_CONFIG_HOME is unset.
    pub fn default_path() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
            if !p.is_empty() {
                return Some(PathBuf::from(p).join("r2").join("hive.toml"));
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return Some(
                    PathBuf::from(home)
                        .join(".config")
                        .join("r2")
                        .join("hive.toml"),
                );
            }
        }
        None
    }

    /// Load from `path`. Returns `Err` if the file exists but is
    /// unreadable or malformed. A non-existent file returns the
    /// compiled-in defaults (Ok).
    pub fn load_optional(path: &Path) -> Result<Self, ConfigError> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(ConfigError::Read {
                    path: path.to_path_buf(),
                    source: e,
                })
            }
        };
        let s = String::from_utf8(bytes).map_err(|e| ConfigError::Read {
            path: path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;
        Self::from_toml_str(&s).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Load from a TOML string. Useful for tests and `--config-string`
    /// style inline config.
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_pure_defaults() {
        let c = HiveConfig::default();
        assert_eq!(c.daemon.name, "r2-hive");
        assert_eq!(c.daemon.port, 21042);
        assert_eq!(c.identity.backend, "auto");
        assert!(!c.transports.lan);
        assert!(c.management.enabled);
    }

    #[test]
    fn empty_toml_yields_defaults() {
        let c = HiveConfig::from_toml_str("").expect("parse");
        assert_eq!(c.daemon.name, "r2-hive");
        assert_eq!(c.transports.lora_socket, PathBuf::from("/var/run/arduino-router.sock"));
    }

    #[test]
    fn full_toml_round_trips() {
        let toml = r#"
[daemon]
name = "alfred"
port = 23000
bind = "127.0.0.1"
buffer_size = 256
max_connections = 32

[transports]
auto = true
lan = true
ble = false
lora = true
lora_socket = "/tmp/arduino.sock"

[identity]
backend = "keyring"

[management]
enabled = false
"#;
        let c = HiveConfig::from_toml_str(toml).expect("parse");
        assert_eq!(c.daemon.name, "alfred");
        assert_eq!(c.daemon.port, 23000);
        assert_eq!(c.daemon.bind, "127.0.0.1");
        assert!(c.transports.auto);
        assert!(c.transports.lan);
        assert!(!c.transports.ble);
        assert!(c.transports.lora);
        assert_eq!(c.transports.lora_socket, PathBuf::from("/tmp/arduino.sock"));
        assert_eq!(c.identity.backend, "keyring");
        assert!(!c.management.enabled);
    }

    #[test]
    fn unknown_section_fails_loudly() {
        // deny_unknown_fields catches typo'd sections so operators
        // don't silently keep running on defaults after a config
        // refactor.
        let toml = r#"
[transports]
internet_relay = true
"#;
        let err = HiveConfig::from_toml_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("internet_relay") || msg.contains("unknown field"));
    }

    #[test]
    fn load_optional_missing_file_yields_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does-not-exist.toml");
        let c = HiveConfig::load_optional(&path).expect("ok");
        assert_eq!(c.daemon.name, "r2-hive");
    }

    #[test]
    fn load_optional_reads_real_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hive.toml");
        std::fs::write(&path, r#"[daemon]
name = "fixture"
"#).unwrap();
        let c = HiveConfig::load_optional(&path).expect("ok");
        assert_eq!(c.daemon.name, "fixture");
    }

    #[test]
    fn packaged_default_example_parses() {
        // The packaged example must always parse as valid TOML so
        // distros can ship it verbatim.
        const PACKAGED: &str =
            include_str!("../packaging/defaults/hive.toml");
        let c = HiveConfig::from_toml_str(PACKAGED).expect("packaged config parses");
        // Spot-check a few fields to confirm the example matches
        // documented defaults.
        assert_eq!(c.daemon.name, "r2-hive");
        assert_eq!(c.daemon.port, 21042);
        assert_eq!(c.identity.backend, "auto");
        assert!(c.management.enabled);
    }

    #[test]
    fn load_optional_malformed_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hive.toml");
        std::fs::write(&path, "this is not toml = = =").unwrap();
        let err = HiveConfig::load_optional(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
    }
}
