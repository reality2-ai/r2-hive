//! r2-hive library crate — the R2 software stack on general-purpose hosts.
//!
//! See R2-HIVE in the specifications. The library exposes the daemon's
//! components so that the binary (`src/main.rs`) is a thin composition
//! layer, and so integration tests / alternative front-ends can drive
//! pieces in process.
//!
//! Modules:
//! - `mgmt` — local management API (R2-HIVE §5) and primitive application
//!   surface (R2-HOST-API §3). Identity custody, Unix-domain-socket
//!   listener, request dispatch.
//! - `hive` — `HiveState` owning transports and routing. Required by `mgmt`
//!   for the `r2.api.*` primitive surface.
//! - `compat`, `plugins`, `router` — mesh-side modules used by the binary.
//!   Exposed so external test rigs can reach them; not yet a stable API.

pub mod autoconfig;
pub mod compat;
pub mod config;
pub mod hive;
pub mod mgmt;
pub mod platform;
pub mod plugins;
pub mod router;
/// Platform-agnostic hive core, re-exported so `r2_hive::sync_host` (etc.)
/// resolves while the convergence migrates modules into `r2-hive-core`.
pub use r2_hive_core::sync_host;
pub mod systemd;
pub mod transport_seam;
pub mod usb;
#[cfg(target_os = "linux")]
pub mod usb_hotplug;
pub mod usb_pair;
#[cfg(target_os = "linux")]
pub mod usb_serial;
pub mod web;
pub mod web_auth;

pub use mgmt::default_socket_path;
