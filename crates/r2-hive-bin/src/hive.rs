//! # HiveState — the Linux daemon's shared state hub
//!
//! ## Why this file exists
//!
//! Every long-lived subsystem of the daemon — the inbound frame router, the
//! legacy browser-WebSocket compat layer, the management API, the web-plugin
//! surface, the USB peripheral watcher — needs the same set of shared
//! objects: the transports, the route engine, the trust-group keys, the
//! mgmt-API subscriber list. This file defines [`HiveState`], the single
//! `Arc`-shared object that owns all of them. The rule it enforces: state
//! visible to more than one task lives HERE, under one locking discipline;
//! state private to one subsystem stays in that subsystem. Without this hub,
//! each transport task would need bespoke plumbing to every consumer and the
//! locking would drift per-path.
//!
//! ## How it interlinks (grep-verified)
//!
//! - Constructed once in `main.rs` (`HiveState::new`) and cloned as
//!   `Arc<HiveState>` into every spawned task and axum route.
//! - `router.rs` — the daemon's single inbound decision point — consumes
//!   [`HiveState::deliver_inbound`] (local dispatch fan-out),
//!   [`HiveState::deny_inbound`] (the structured §3.2.1 deny event),
//!   [`HiveState::send_to_hive_via`] (multi-transport egress with fallback),
//!   the `group_hmacs` / `deliver_unkeyed_open` deliver-gate inputs, and the
//!   `route_engine` lock.
//! - `compat/handshake.rs` (legacy browser protocol) uses the TG-compat
//!   surface: `register_tg_peer` / `unregister_tg_peer` / `broadcast_to_tg` /
//!   `buffer_frame` / `catchup_frames` / `tg_peer_count` / `buffer_oldest` /
//!   `resolve_tg_hash` / `flood_tg_peers_not_in`, plus the shared
//!   `hex_encode` / `hex_decode` helpers.
//! - `mgmt/socket.rs` + `mgmt/ws.rs` register/unregister mgmt subscribers;
//!   `mgmt/primitive.rs` reads `active_tg` and sends via `send_to_hive_via` /
//!   `broadcast_to_tg`; `mgmt/transport_policy.rs` drives the egress-mask
//!   lease; `mgmt/usb.rs` drives `usb_handle`; `mgmt/ensemble.rs` emits
//!   sentant output through `deliver_inbound` / `send_to_hive` /
//!   `broadcast_to_tg`.
//! - `web.rs`, `mgmt/ws.rs` and `mgmt/api.rs` gate browser-facing surfaces
//!   on [`HiveState::web_auth`] / [`HiveState::web_dev_mode`].
//! - `HiveState` implements [`crate::transport_seam::HiveTransports`]
//!   (trait defined in `r2-hive-core`) — the seam that lets the
//!   platform-independent forwarding core run unchanged on Linux, MCU and
//!   wasm hosts.
//!
//! ## Canon (r2-specifications)
//!
//! - R2-TRUST §2.2–2.3 (TG identity/roles), §7.5.4 (inbound deliver-gate),
//!   §13.2 (single active hive) — `r2-specifications/specs/r2-core/R2-TRUST.md`.
//! - R2-HOST-API §3.2 (`event.delivery`), §3.2.1 (`event.delivery.denied`),
//!   §6.2 (mgmt error frames) — `r2-specifications/specs/r2-core/R2-HOST-API.md`.
//! - R2-WIRE §4.3.5 (extended↔compact transcoding at transport boundaries),
//!   §6.2.1 (per-TG hive identifiers) — `r2-specifications/specs/r2-core/R2-WIRE.md`.
//! - R2-USB §3.5 (dongle byte stream), Appendix A (transport-kind
//!   enumeration) — `r2-specifications/specs/r2-core/R2-USB.md`.
//! - R2-KEYSTORE §4 (sealed key custody — the production posture for
//!   `group_hmacs`) — `r2-specifications/specs/r2-core/R2-KEYSTORE.md`.
//! - R2-TRANSPORT §2.2 (the 7-transport canon) —
//!   `r2-specifications/specs/r2-core/R2-TRANSPORT.md`.
//!
//! **Citation note (specs-ruled):** no R2-HIVE spec exists (implementation
//! repo name — `r2-specifications/specs/r2-core/README.md`). Former
//! "R2-HIVE §…" cites in this crate are RE-ANCHORED to the real canon:
//! single-active-TG → R2-TRUST §13.2 + R2-TG-TOOL §7; mgmt socket/API →
//! R2-TG-TOOL §5 + R2-HOST-API §2.2/§2.4; identity custody → R2-TG-TOOL §3 +
//! R2-WIRE §6.2.1. Only genuinely daemon-local choices (backend selection,
//! concrete file paths) remain marked as such.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{mpsc, Mutex, RwLock};

use std::sync::Arc;
use r2_discovery::AsyncTransport;
use r2_discovery::WebSocketTransport;
#[cfg(feature = "transport-udp")]
use r2_discovery::bindings::udp_lan::UdpLanTransport;
#[cfg(feature = "transport-ble")]
use r2_discovery::bindings::ble::BleTransport;
#[cfg(feature = "transport-lora")]
use r2_discovery::bindings::lora::LoraTransport;
use r2_route::engine::RouteEngine;
use r2_route::transport::{Transport, TransportSet};
use r2_trust::wire_hmac::GroupHmac;

use crate::compat::buffer::RingBuffer;
use crate::mgmt::subscriptions::SubscriptionRegistry;
use crate::plugins::word_codes::WordCodeStore;
use r2_ensemble::EnsembleRegistry;

/// One connected mgmt-API consumer. Holds the per-connection subscription
/// state and the channel used to push unsolicited notifications
/// (`r2.api.event.delivery`, `r2.mgmt.event.error` for backpressure) back
/// to the connection's writer task.
///
/// **Used-by:** created in [`HiveState::register_subscriber`] (called from
/// `mgmt/socket.rs` for UDS connections and `mgmt/ws.rs` for the loopback
/// mgmt WebSocket); consumed by [`HiveState::deliver_inbound`] and
/// [`HiveState::deny_inbound`] when fanning notifications out.
pub struct Subscriber {
    pub id: u64,
    pub subs: Arc<tokio::sync::Mutex<SubscriptionRegistry>>,
    pub tx: mpsc::Sender<Vec<u8>>,
}

/// Trust group hash: first 8 bytes of SHA-256(TG_PK) — the on-wire scoping
/// identifier used by the legacy compat layer (`compat/handshake.rs`) and
/// the `tg_map` machinery below. Distinct from the u32 WIRE `target_group`
/// that keys `group_hmacs`.
pub type TrustGroupHash = [u8; 8];

/// Per-trust-group state for legacy compat routing: which hive_ids are
/// members (for broadcast fan-out) plus the frame catchup ring buffer.
/// Only ever touched through the `tg_map` methods on [`HiveState`].
struct TrustGroupCompat {
    /// Hive IDs of peers in this trust group.
    peers: HashSet<u32>,
    /// Frame catchup buffer.
    buffer: RingBuffer,
}

/// Snapshot of the daemon's currently-attached trust group.
///
/// Per R2-TRUST §13.2 (Single-Active-Hive Rule) the daemon may have at
/// most one TG attachment active at a time; R2-TG-TOOL §7 pins the daemon
/// as that rule's enforcer. When `active_tg` is
/// `None` the daemon is detached — fresh device, no `r2hive tg create` yet.
///
/// `tg_id` is the 32-byte TG public key (R2-TRUST §2.2). `tg_hash` is the
/// first 8 bytes of SHA-256(TG_PK), the on-wire scoping identifier used by
/// the legacy `tg_map` / `register_tg_peer` machinery. `hive_id` is the
/// per-TG hive identifier (R2-WIRE §6.2.1).
///
/// v0.1: this struct is populated by future TG creation / join flows; the
/// daemon currently boots detached. The structure is here so primitive
/// handlers (`r2.api.tg.current`, future TG-scoped broadcast) have a
/// stable surface to query.
#[derive(Debug, Clone)]
pub struct ActiveTg {
    pub tg_id: [u8; 32],
    pub tg_hash: TrustGroupHash,
    pub member_role: TgMemberRole,
    pub hive_id: u32,
}

/// Member role within the active trust group, per R2-TRUST §2.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TgMemberRole {
    /// Standard member; cannot issue invites.
    Member,
    /// Holds TG_SK; can provision new devices into the group.
    KeyHolder,
}

impl TgMemberRole {
    /// Wire-format value per R2-HOST-API §3.2 (key 2 of `tg.current` response).
    ///
    /// **Used-by:** `mgmt/primitive.rs` when encoding the `r2.api.tg.current`
    /// response payload.
    pub fn wire_value(self) -> u8 {
        match self {
            TgMemberRole::Member => 1,
            TgMemberRole::KeyHolder => 2,
        }
    }
}

/// Local management lease for the node-wide transport egress allow mask.
///
/// The routing semantics live in `r2-route::RouteEngine` (the mask itself is
/// stored there, single-sourced); this is only local ACK/state metadata for
/// the management surface — who last set the mask, and what core accepted.
///
/// **Used-by:** `mgmt/transport_policy.rs` (the `r2.mgmt.transport.*`
/// handlers) via [`HiveState::set_transport_policy_lease`] /
/// [`HiveState::transport_policy_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportPolicyLease {
    pub lease_id: u64,
    pub source: String,
    pub requested_mask: u8,
    pub accepted_mask: u8,
}

/// Read-only view of the current egress policy, returned by
/// [`HiveState::transport_policy_snapshot`] and
/// [`HiveState::clear_transport_policy`] for the mgmt query/clear handlers
/// in `mgmt/transport_policy.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportPolicySnapshot {
    pub effective_mask: u8,
    pub all_mask: u8,
    pub active_lease: Option<TransportPolicyLease>,
}

/// Acknowledgement returned by [`HiveState::set_transport_policy_lease`]:
/// echoes what was requested and reports the mask core actually accepted
/// (core may trim bits it cannot honour). Consumed by
/// `mgmt/transport_policy.rs` when encoding the set-response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportPolicyAck {
    pub requested_mask: u8,
    pub accepted_mask: u8,
    pub effective_mask: u8,
    pub all_mask: u8,
    pub lease_id: u64,
    pub source: String,
}

/// Global wayfinder state.
pub struct HiveState {
    /// This hive's own canonical 32-bit identifier. Set once at startup from
    /// `--name` and used by the router to populate the route stack on relay.
    pub self_hive_id: u32,
    /// WebSocket transport (Internet transport, extended format).
    pub ws_transport: WebSocketTransport,
    /// UDP LAN transport (Internet transport, extended format).
    /// None until --lan is enabled.
    #[cfg(feature = "transport-udp")]
    pub udp_transport: RwLock<Option<Arc<UdpLanTransport>>>,
    /// BLE transport (compact format, L2CAP CoC).
    /// None until --ble is enabled.
    #[cfg(feature = "transport-ble")]
    pub ble_transport: RwLock<Option<Arc<BleTransport>>>,
    /// LoRa transport via arduino-router IPC (compact format).
    /// None until --lora is enabled.
    #[cfg(feature = "transport-lora")]
    pub lora_transport: RwLock<Option<Arc<LoraTransport>>>,
    /// R2-ROUTE engine — Layer 2-4 routing decisions.
    pub route_engine: Mutex<RouteEngine<64, 64, 64>>,
    /// Local management lease metadata for the node-wide egress allow mask.
    ///
    /// The effective policy is kept in `route_engine.transport_allow_mask()` so
    /// route selection remains single-sourced in core. This field only lets the
    /// local management API acknowledge and report the current local writer.
    transport_policy_lease: RwLock<Option<TransportPolicyLease>>,
    /// §7.5.4 deliver-gate: GroupHmac per trust group, keyed by the WIRE
    /// `target_group` (u32). The inbound deliver-gate verifies a frame against
    /// `group_hmacs[target_group]` before local dispatch (R2-TRUST §7.5.4).
    /// EMPTY = migration mode (gate inactive — deliver unverified + warn), so
    /// existing no-key daemons don't break. Populated at startup from a sealed
    /// keystore (production, R2-KEYSTORE §4) or — for the FR-2b bench only — the
    /// C3-flagged plaintext `R2_GROUP_KEYS_BENCH` json (dev-only, never prod).
    pub group_hmacs: HashMap<u32, GroupHmac>,
    /// R2-TRUST §7.5.4 posture when `group_hmacs` is EMPTY (no keys configured). Default `false` =
    /// FAIL-CLOSED (drop, do not deliver unverified) — "default-OPEN is FORBIDDEN". An operator may set
    /// `R2_DELIVER_UNKEYED_OPEN=1` to explicitly opt into the legacy migration behaviour (deliver + loud
    /// warn) for a keyless dev/bring-up daemon; production never sets it.
    pub deliver_unkeyed_open: bool,
    /// Currently-attached trust group, if any (R2-TRUST §13.2 single-active
    /// rule; arbitration per R2-TG-TOOL §7).
    /// `None` means the daemon is detached — fresh device, no TG joined.
    active_tg: RwLock<Option<ActiveTg>>,
    /// Per-connection mgmt-API subscribers (R2-HOST-API §3.2 event.subscribe /
    /// §4 subscription mechanics). Each entry is a connection on the UDS or
    /// loopback WebSocket; the connection handler registers on open and
    /// unregisters on close. `deliver_inbound` re-fans matching frames to
    /// the entries here.
    subscribers: Mutex<Vec<Subscriber>>,
    /// Monotonic counter for assigning Subscriber IDs.
    next_subscriber_id: AtomicU64,
    /// Trust group compat map — tracks which hive_ids are in which TG.
    tg_map: RwLock<HashMap<TrustGroupHash, TrustGroupCompat>>,
    /// Word code plugin state.
    pub word_codes: WordCodeStore,
    /// Ensemble registry — owns loaded ensembles and is the
    /// `DispatchTarget` for the route engine's `DeliverOnly`
    /// decisions. Held as `Arc` so the registry can be cloned into
    /// dispatcher / mgmt handlers without locking the parent.
    pub ensembles: Arc<EnsembleRegistry>,
    /// Web plugin registry — mount/unmount of bundle directories served
    /// under `/ensemble/<name>` per R2-PLUGIN §13.
    pub web_plugins: Arc<crate::web::WebPluginRegistry>,
    /// Browser-device credentials and cookie signing key
    /// (R2-PLUGIN §13.5). Set after `HiveState::new` via
    /// [`HiveState::set_web_auth`] once the master secret is loaded. If
    /// absent, web surfaces fail closed unless [`HiveState::set_web_dev_mode`]
    /// has explicitly enabled the development bypass.
    web_auth: std::sync::RwLock<Option<Arc<crate::web_auth::WebAuth>>>,
    /// Explicit development bypass for web-plugin static assets. Defaults
    /// false; production must install `web_auth` instead.
    web_dev_mode: AtomicBool,
    /// USB peripheral bring-up handle. Linux-only and gated on
    /// `--no-usb` / runtime presence; mgmt-event handlers reach into
    /// it for the `r2.mgmt.usb.*` vocabulary. `None` when the
    /// watcher is disabled.
    #[cfg(target_os = "linux")]
    usb_handle: std::sync::RwLock<Option<crate::usb_hotplug::UsbBringupHandle>>,
    /// Config
    pub buffer_size: usize,
    pub max_connections: usize,
    /// Stats
    pub frames_routed: AtomicU64,
    pub connections_total: AtomicU64,
    pub started_at: Instant,
    /// Platform abstraction (clock / RNG / …) — the north-star seam that lets the
    /// same hive-core run on Linux/cloud, ESP32, Uno-Q, and wasm. Defaults to
    /// [`crate::platform::LinuxPlatform`]; a platform layer can inject its own.
    pub platform: Arc<dyn crate::platform::Platform>,
}

// The transport seam (R2-HIVE north-star): hive-core forwarding targets
// `HiveTransports`, not the concrete transport set. On Linux this delegates to
// the async r2-discovery transports below; the MCU platform will provide its own
// impl over core's no_std R2-TRANSPORT sync drivers. See `transport_seam`.
#[async_trait::async_trait]
impl crate::transport_seam::HiveTransports for HiveState {
    async fn send_to_hive(&self, hive_id: u32, frame: &[u8]) -> bool {
        HiveState::send_to_hive(self, hive_id, frame).await
    }
    async fn send_to_hive_via(
        &self,
        hive_id: u32,
        hint: Option<r2_route::transport::Transport>,
        frame: &[u8],
    ) -> Option<r2_route::transport::Transport> {
        HiveState::send_to_hive_via(self, hive_id, hint, frame).await
    }
}

impl HiveState {
    /// Construct the daemon's single shared state object.
    ///
    /// **Purpose:** initialise every field to its boot posture — no optional
    /// transports attached, detached from any TG, deliver-gate keys loaded
    /// (bench env var only; empty otherwise), fail-closed unkeyed posture
    /// unless `R2_DELIVER_UNKEYED_OPEN` explicitly opts out.
    ///
    /// **Dependencies:** `load_bench_group_hmacs` (env-gated key load),
    /// `crate::platform::linux()` (default Platform impl), the
    /// `R2_DELIVER_UNKEYED_OPEN` env var (R2-TRUST §7.5.4 posture).
    ///
    /// **Used-by:** `main.rs` once at startup; unit/integration tests
    /// construct throwaway instances directly.
    pub fn new(self_hive_id: u32, buffer_size: usize, max_connections: usize) -> Self {
        HiveState {
            self_hive_id,
            ws_transport: WebSocketTransport::new(4096),
            #[cfg(feature = "transport-udp")]
            udp_transport: RwLock::new(None),
            #[cfg(feature = "transport-ble")]
            ble_transport: RwLock::new(None),
            #[cfg(feature = "transport-lora")]
            lora_transport: RwLock::new(None),
            route_engine: Mutex::new(RouteEngine::new()),
            transport_policy_lease: RwLock::new(None),
            // R2-BUILDMODE §5.1: both env-var inputs below exist ONLY in dev
            // builds. A prod binary reads neither — it boots unkeyed +
            // fail-closed (relay-only) until R2-KEYSTORE §4 sealed custody
            // lands. Structural absence: no env read, no config path.
            #[cfg(feature = "dev")]
            group_hmacs: load_bench_group_hmacs(),
            #[cfg(not(feature = "dev"))]
            group_hmacs: HashMap::new(),
            // R2-TRUST §7.5.4: fail-closed by default; the operator opt-in is
            // a DEV-build capability only (specs v0.3 ruling: the migration
            // fail-open predates the compile-time split; refined, it lives in
            // DEV images — no contradiction).
            #[cfg(feature = "dev")]
            deliver_unkeyed_open: std::env::var("R2_DELIVER_UNKEYED_OPEN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            #[cfg(not(feature = "dev"))]
            deliver_unkeyed_open: false,
            active_tg: RwLock::new(None),
            subscribers: Mutex::new(Vec::new()),
            next_subscriber_id: AtomicU64::new(1),
            tg_map: RwLock::new(HashMap::new()),
            word_codes: WordCodeStore::new(),
            ensembles: Arc::new(EnsembleRegistry::new()),
            web_plugins: Arc::new(crate::web::WebPluginRegistry::new()),
            web_auth: std::sync::RwLock::new(None),
            web_dev_mode: AtomicBool::new(false),
            #[cfg(target_os = "linux")]
            usb_handle: std::sync::RwLock::new(None),
            buffer_size,
            max_connections,
            frames_routed: AtomicU64::new(0),
            connections_total: AtomicU64::new(0),
            started_at: Instant::now(),
            platform: crate::platform::linux(),
        }
    }

    /// Install the web-plugin browser-auth registry (R2-PLUGIN §13.5).
    ///
    /// **Used-by:** `main.rs` after the master secret is loaded (auth key is
    /// derived from it); `mgmt/ws.rs` tests install throwaway keys.
    pub fn set_web_auth(&self, auth: Arc<crate::web_auth::WebAuth>) {
        *self.web_auth.write().expect("web_auth lock") = Some(auth);
    }

    /// Borrow the auth registry, if installed. `None` means browser-facing
    /// surfaces must fail closed (unless [`Self::web_dev_mode`] is on).
    ///
    /// **Used-by:** `web.rs` (asset + provisioning gates), `mgmt/api.rs`
    /// (provision handler), `mgmt/ws.rs` (`authorize_upgrade`).
    pub fn web_auth(&self) -> Option<Arc<crate::web_auth::WebAuth>> {
        self.web_auth.read().expect("web_auth lock").clone()
    }

    /// Enable or disable the explicit web development bypass. DEV BUILDS
    /// ONLY (R2-BUILDMODE §5.1) — the setter does not exist in a prod
    /// binary, so nothing can ever flip the flag there.
    ///
    /// **Used-by:** `main.rs` when `--web-dev-mode` is passed (dev builds).
    #[cfg(feature = "dev")]
    pub fn set_web_dev_mode(&self, enabled: bool) {
        self.web_dev_mode.store(enabled, Ordering::Relaxed);
    }

    /// Returns true only when the operator explicitly enabled the web
    /// development bypass. In a PROD build this is a compile-time `false`
    /// (no setter exists; the field is write-never) — the consuming branch
    /// in `web.rs` folds away.
    ///
    /// **Used-by:** `web.rs` when deciding whether an unauthenticated asset
    /// request may still be served.
    pub fn web_dev_mode(&self) -> bool {
        #[cfg(feature = "dev")]
        {
            self.web_dev_mode.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "dev"))]
        {
            false
        }
    }

    /// Install the USB peripheral bring-up handle. Until set, the
    /// `r2.mgmt.usb.*` event surface returns `usb_disabled`.
    ///
    /// **Used-by:** `main.rs` after `spawn_usb_watcher` returns (unless
    /// `--no-usb`).
    #[cfg(target_os = "linux")]
    pub fn set_usb_handle(&self, h: crate::usb_hotplug::UsbBringupHandle) {
        *self.usb_handle.write().expect("usb_handle lock") = Some(h);
    }

    /// Cheap clone of the USB handle, if installed.
    ///
    /// **Used-by:** `mgmt/usb.rs` (every `r2.mgmt.usb.*` handler) and
    /// `try_send_via_dongle` (egress via a paired dongle).
    #[cfg(target_os = "linux")]
    pub fn usb_handle(&self) -> Option<crate::usb_hotplug::UsbBringupHandle> {
        self.usb_handle.read().expect("usb_handle lock").clone()
    }

    /// Set the UDP transport.
    ///
    /// **Used-by:** `main.rs::start_lan_discovery` when `--lan` is enabled.
    #[cfg(feature = "transport-udp")]
    pub async fn set_udp_transport(&self, udp: Arc<UdpLanTransport>) {
        *self.udp_transport.write().await = Some(udp);
    }

    /// Set the BLE transport.
    ///
    /// **Used-by:** `main.rs::start_ble` when `--ble` is enabled.
    #[cfg(feature = "transport-ble")]
    pub async fn set_ble_transport(&self, ble: Arc<BleTransport>) {
        *self.ble_transport.write().await = Some(ble);
    }

    /// Set the LoRa transport.
    ///
    /// **Used-by:** `main.rs::start_lora` when `--lora` is enabled.
    #[cfg(feature = "transport-lora")]
    pub async fn set_lora_transport(&self, lora: Arc<LoraTransport>) {
        *self.lora_transport.write().await = Some(lora);
    }

    /// Snapshot of the currently-attached trust group, if any.
    /// Returns a clone so callers don't hold a read lock across awaits.
    ///
    /// **Used-by:** `mgmt/primitive.rs` (`r2.api.tg.current` and the
    /// TG-scoped `event.send` broadcast path).
    pub async fn active_tg(&self) -> Option<ActiveTg> {
        self.active_tg.read().await.clone()
    }

    /// Attach a trust group as active. Replaces any prior attachment.
    /// v0.1: there is no enforcement of R2-TRUST §13.2 single-active-hive
    /// at this method; the caller is expected to stop the previous
    /// attachment before swapping. Phase 2 supervisor will gate this.
    ///
    /// **Used-by:** no production caller yet — the TG create/join flow that
    /// writes this is future work; `tests/mgmt_integration.rs` uses it to
    /// stage an attached-TG scenario. (Its former `clear_active_tg`
    /// counterpart had zero callers and was removed; detach lands with the
    /// TG lifecycle flow.)
    pub async fn set_active_tg(&self, tg: ActiveTg) {
        *self.active_tg.write().await = Some(tg);
    }

    /// Snapshot the local transport egress allow policy.
    ///
    /// **Used-by:** `mgmt/transport_policy.rs` (query handler + tests).
    pub async fn transport_policy_snapshot(&self) -> TransportPolicySnapshot {
        let effective_mask = self.route_engine.lock().await.transport_allow_mask().bits();
        let active_lease = self.transport_policy_lease.read().await.clone();
        TransportPolicySnapshot {
            effective_mask,
            all_mask: TransportSet::ALL_BITS,
            active_lease,
        }
    }

    /// Install/refresh the single local transport-policy lease and ACK the
    /// canonical mask that core accepted (core stays the single source of
    /// truth for the mask itself; this only records lease metadata).
    ///
    /// **Used-by:** `mgmt/transport_policy.rs` set handler.
    pub async fn set_transport_policy_lease(
        &self,
        lease_id: u64,
        source: String,
        requested_mask: u8,
    ) -> TransportPolicyAck {
        let accepted_mask = self
            .route_engine
            .lock()
            .await
            .set_transport_allow_mask_bits(requested_mask)
            .bits();
        let lease = TransportPolicyLease {
            lease_id,
            source: source.clone(),
            requested_mask,
            accepted_mask,
        };
        *self.transport_policy_lease.write().await = Some(lease);
        TransportPolicyAck {
            requested_mask,
            accepted_mask,
            effective_mask: accepted_mask,
            all_mask: TransportSet::ALL_BITS,
            lease_id,
            source,
        }
    }

    /// Clear the current local transport-policy lease and restore the canonical
    /// default all-on mask.
    ///
    /// **Used-by:** `mgmt/transport_policy.rs` clear handler.
    pub async fn clear_transport_policy(&self) -> TransportPolicySnapshot {
        let effective_mask = self
            .route_engine
            .lock()
            .await
            .clear_transport_allow_mask()
            .bits();
        *self.transport_policy_lease.write().await = None;
        TransportPolicySnapshot {
            effective_mask,
            all_mask: TransportSet::ALL_BITS,
            active_lease: None,
        }
    }

    /// Register a new mgmt-API subscriber. Returns the Subscriber's ID;
    /// callers retain it so they can `unregister_subscriber` on connection
    /// close. The returned `Arc<Mutex<SubscriptionRegistry>>` is the same
    /// one carried in the registered Subscriber, so the connection handler
    /// can mutate the registry (subscribe/unsubscribe handlers do this)
    /// without going back through HiveState.
    ///
    /// **Used-by:** `mgmt/socket.rs` (UDS connection open) and `mgmt/ws.rs`
    /// (loopback mgmt-WebSocket connection open).
    pub async fn register_subscriber(
        &self,
        tx: mpsc::Sender<Vec<u8>>,
    ) -> (u64, Arc<tokio::sync::Mutex<SubscriptionRegistry>>) {
        let id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        let subs = Arc::new(tokio::sync::Mutex::new(SubscriptionRegistry::new()));
        self.subscribers.lock().await.push(Subscriber {
            id,
            subs: subs.clone(),
            tx,
        });
        (id, subs)
    }

    /// Unregister a subscriber by ID. Idempotent — unknown IDs are
    /// silently ignored (the connection handler may call this during
    /// teardown after the registry has already been pruned by
    /// `clear_dead_subscribers`).
    ///
    /// **Used-by:** `mgmt/socket.rs` and `mgmt/ws.rs` on connection close.
    pub async fn unregister_subscriber(&self, id: u64) {
        self.subscribers.lock().await.retain(|s| s.id != id);
    }

    /// Deliver an inbound frame to any subscribers whose filter matches —
    /// the daemon's "local dispatch" endpoint (R2-HOST-API §3.2).
    ///
    /// **Used-by:** `router.rs::route_frame` after the §7.5.4 deliver-gate
    /// passes, and `mgmt/ensemble.rs` when sentant output targets this hive
    /// locally (no wire emission).
    ///
    /// This decodes the frame minimally — just enough to extract event
    /// hash, event class (if known), and source — then calls each
    /// subscriber's `SubscriptionRegistry::iter()` looking for matches.
    /// On match it builds an `r2.api.event.delivery` notification and
    /// pushes through the subscriber's mpsc::Sender. If the channel is
    /// full the delivery is dropped and a `backpressure` error is queued
    /// instead (best-effort — if even the error fails to enqueue, we move
    /// on).
    pub async fn deliver_inbound(
        &self,
        frame: &[u8],
        source_hive: u32,
        source_tg: Option<TrustGroupHash>,
    ) {
        // Cheap parse: only the extended-frame header needs reading. If
        // decode fails, there are no deliveries — let the existing route
        // engine paths log the bad frame.
        let msg = match r2_wire::decode_extended(frame) {
            Ok(m) => m,
            Err(_) => return,
        };
        let event_hash = msg.header.event_hash;
        let payload = msg.payload.to_vec();
        let msg_id = msg.header.msg_id as u64;

        // Snapshot the subscribers list while holding the lock briefly.
        let subscribers = self.subscribers.lock().await;
        if subscribers.is_empty() {
            return;
        }

        for subscriber in subscribers.iter() {
            let registry = subscriber.subs.lock().await;
            for sub in registry.iter() {
                // Filter match: see SubscriptionFilter docs.
                let f = &sub.filter;
                if let Some(h) = f.event_hash {
                    if h != event_hash {
                        continue;
                    }
                }
                if let Some(class) = &f.event_class {
                    // Class-string filter: hash and compare. v0.1 has no
                    // canonical-form lookup table, so we trust the
                    // subscriber's class string is canonical.
                    if r2_fnv::r2_hash(class).ok() != Some(event_hash) {
                        continue;
                    }
                }
                if let Some(h) = f.from_hive {
                    if h != source_hive as u64 {
                        continue;
                    }
                }
                if let Some(tg) = f.from_tg {
                    if Some(tg) != source_tg {
                        continue;
                    }
                }

                // Match: build delivery notification.
                let class_string = f.event_class.as_deref().unwrap_or("");
                let delivery_frame = build_delivery_frame(
                    sub.sub_id,
                    class_string,
                    event_hash,
                    &payload,
                    source_hive as u64,
                    source_tg,
                    msg_id,
                );

                match subscriber.tx.try_send(delivery_frame) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Best-effort backpressure error.
                        let err_frame = build_backpressure_error(sub.sub_id);
                        let _ = subscriber.tx.try_send(err_frame);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Connection has closed; the writer task drained
                        // and exited. The connection handler will call
                        // unregister_subscriber. Move on.
                        break;
                    }
                }
            }
        }
    }

    /// Report a §7.5.4 deliver-gate REJECT to matching mgmt-API subscribers as
    /// an `r2.api.event.delivery.denied` notification (R2-HOST-API §3.2.1) —
    /// the observable counterpart of `deliver_inbound`, so a rejected frame is
    /// a real event, not just a log line. The denied frame's payload is never
    /// forwarded (unverified attacker-controlled bytes, §3.2.1 omits key 4).
    ///
    /// `reason` is the ratified text taxonomy: `"forgery"` |
    /// `"unauthenticated"` | `"fail_closed"`. Never called for Relay/transit
    /// (not a local reject) or for a frame that was actually delivered (the
    /// keyless operator-opt-in path delivers, so it must not deny).
    ///
    /// **Used-by:** `router.rs::route_frame` only — the three deliver-gate
    /// reject arms (bad tag, missing tag, unkeyed fail-closed).
    ///
    /// Subscription hygiene (channel isolation): an UNFILTERED subscription
    /// shares its bounded channel between denies and legit deliveries, so a
    /// forged-frame flood can crowd deliveries out via try_send-Full.
    /// Confidence-surface subscribers should use a dedicated deny-filtered
    /// subscription (event_class = the denied class); delivery consumers
    /// should filter by their own event class (a class-filtered deliveries-
    /// only subscription never receives denies).
    pub async fn deny_inbound(
        &self,
        msg_id: u64,
        target_group: u32,
        reason: &str,
        from_hive: u64,
    ) {
        let denied_hash =
            r2_fnv::r2_hash(crate::mgmt::api::EV_EVENT_DELIVERY_DENIED).expect("known event");

        let subscribers = self.subscribers.lock().await;
        if subscribers.is_empty() {
            return;
        }

        // A denial is not tied to a subscription (§3.2.1 omits sub_id), so the
        // frame is identical for every match — build once, clone per send.
        let denied_frame = build_denied_frame(from_hive, msg_id, target_group, reason);

        for subscriber in subscribers.iter() {
            let registry = subscriber.subs.lock().await;
            for sub in registry.iter() {
                // Filter match mirrors deliver_inbound, with the notification's
                // own class standing in as the event: a subscription naming the
                // denied class (by hash or string) matches; an unfiltered
                // subscription matches everything, denies included
                // (distinguishable by the event_class at key 2).
                let f = &sub.filter;
                if let Some(h) = f.event_hash {
                    if h != denied_hash {
                        continue;
                    }
                }
                if let Some(class) = &f.event_class {
                    if r2_fnv::r2_hash(class).ok() != Some(denied_hash) {
                        continue;
                    }
                }
                if let Some(h) = f.from_hive {
                    if h != from_hive {
                        continue;
                    }
                }
                // A denied frame's from_tg claim is exactly what's untrusted
                // (§3.2.1 omits key 6) — a from_tg-filtered subscription never
                // matches a deny.
                if f.from_tg.is_some() {
                    continue;
                }

                match subscriber.tx.try_send(denied_frame.clone()) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Best-effort backpressure error (same as deliveries).
                        let err_frame = build_backpressure_error(sub.sub_id);
                        let _ = subscriber.tx.try_send(err_frame);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Connection has closed; the handler will unregister.
                        break;
                    }
                }
            }
        }
    }

    /// Send a frame to a hive via any transport that can reach it —
    /// convenience wrapper over [`Self::send_to_hive_via`] with no hint.
    /// Used when the caller has no route-engine recommendation (broadcasts,
    /// legacy paths).
    ///
    /// **Used-by:** [`Self::broadcast_to_tg`] fan-out, `mgmt/ensemble.rs`
    /// directed sentant output, and hive-core code via the
    /// `HiveTransports` seam.
    pub async fn send_to_hive(&self, hive_id: u32, frame: &[u8]) -> bool {
        self.send_to_hive_via(hive_id, None, frame).await.is_some()
    }

    /// Send a frame to a hive, preferring the route-engine's recommended
    /// transport. Returns `Some(transport_used)` on success, `None` if every
    /// transport failed. When `hint` is provided, that transport is tried
    /// first; remaining transports are tried in priority order as fallback.
    /// Every attempt is filtered through the node-wide egress allow mask
    /// (single-sourced in `route_engine.transport_allow_mask()`).
    ///
    /// **Dependencies:** `try_send_on` per transport; the route
    /// engine lock (mask read only — do not call while holding it).
    ///
    /// **Used-by:** `router.rs` relay arms (Directed/Flood hops, with the
    /// engine's per-hop transport as `hint`), `mgmt/primitive.rs`
    /// (`event.send` directed + TG-peer fan-out), [`Self::send_to_hive`],
    /// and hive-core via the `HiveTransports` seam.
    pub async fn send_to_hive_via(
        &self,
        hive_id: u32,
        hint: Option<Transport>,
        frame: &[u8],
    ) -> Option<Transport> {
        // Build attempt order: hint first (if any), then default priority,
        // skipping duplicates.
        let default_order = [
            Transport::Internet,
            Transport::Wifi,
            Transport::Ble,
            Transport::Lora,
            Transport::Usb,
            Transport::WifiMesh,
            Transport::Udp,
        ];
        let mut order: Vec<Transport> = Vec::with_capacity(Transport::COUNT);
        if let Some(h) = hint {
            order.push(h);
        }
        for t in default_order {
            if Some(t) != hint {
                order.push(t);
            }
        }
        let allow_mask = self.route_engine.lock().await.transport_allow_mask();
        for transport in order {
            if !allow_mask.contains(transport) {
                log::debug!(
                    "send: {:?} disabled by local transport_allow_mask=0x{:02X}",
                    transport,
                    allow_mask.bits()
                );
                continue;
            }
            if self.try_send_on(transport, hive_id, frame).await {
                return Some(transport);
            }
        }
        None
    }

    /// Attempt one send on one concrete transport, applying the per-medium
    /// wire format (extended verbatim for Internet/WiFi; extended→compact
    /// transcode per R2-WIRE §4.3.5 for BLE/LoRa) and falling back to a
    /// paired USB dongle advertising that transport kind.
    ///
    /// **Dependencies:** the optional transport slots on `self`, the
    /// r2-wire transcoder, [`Self::try_send_via_dongle`].
    ///
    /// **Used-by:** [`Self::send_to_hive_via`] only (its per-transport
    /// attempt loop).
    async fn try_send_on(
        &self,
        transport: r2_route::transport::Transport,
        hive_id: u32,
        frame: &[u8],
    ) -> bool {
        use r2_route::transport::Transport;
        match transport {
            // r2-route gained Usb/WifiMesh/Udp (R2-TRANSPORT §2.2 7-transport canon; WifiMesh = the
            // v0.37 §2.2A rename of Mesh — the "R2-Mesh" proper noun is RETIRED; canonical display label
            // "wifi-mesh"; discriminant 5 unchanged). Host-side handling per specs steer (these are
            // HOST-IMPL, not spec; conform to the refs when built out):
            //  - WifiMesh (ESP-NOW is the reference PHY): correctly UNAVAILABLE on a Linux/cloud host
            //    (peer radio) — leave stubbed ("not available on this platform", R2-TRANSPORT §2.2 role).
            //  - Udp: the IP/global transport + the WiFi-UDP critical path — SHOULD become real next
            //    (route via udp_transport, R2-ROUTE §5.7.1 selection); compile-stub false for now.
            //  - Usb: implement the R2-USB framer + R2-PROVISION §5.3.x pairing when built out.
            Transport::Usb | Transport::WifiMesh | Transport::Udp => false,
            Transport::Internet => {
                if self.ws_transport.send(hive_id, frame).await.is_ok() {
                    return true;
                }
                // Internet via USB-attached dongle (if any). R2-USB
                // §3.5 carries the byte stream verbatim; the dongle's
                // own bridge wraps Internet in whatever the
                // peripheral implements (rare; mostly here for
                // symmetry).
                self.try_send_via_dongle(transport, frame, false).await
            }
            Transport::Wifi => {
                #[cfg(feature = "transport-udp")]
                if let Some(udp) = self.udp_transport.read().await.as_ref() {
                    if udp.send(hive_id, frame).await.is_ok() {
                        return true;
                    }
                }
                self.try_send_via_dongle(transport, frame, false).await
            }
            Transport::Ble => {
                #[cfg(feature = "transport-ble")]
                if let Some(ble) = self.ble_transport.read().await.as_ref() {
                    if ble.send(hive_id, frame).await.is_ok() {
                        return true;
                    }
                }
                // BLE on the air is compact format per R2-BLE.
                self.try_send_via_dongle(transport, frame, true).await
            }
            Transport::Lora => {
                #[cfg(feature = "transport-lora")]
                if let Some(lora) = self.lora_transport.read().await.as_ref() {
                    // Frames on the engine's side are extended; LoRa carries
                    // compact. Transcode at the transport boundary per
                    // R2-WIRE §4.3.5. If transcoding fails, drop — an
                    // extended frame that can't be compressed isn't a LoRa
                    // frame we can put on the air.
                    let mut buf = vec![0u8; frame.len()];
                    match r2_wire::transcode::transcode_extended_to_compact(frame, &mut buf) {
                        Ok(n) => {
                            if lora.send(hive_id, &buf[..n]).await.is_ok() {
                                return true;
                            }
                        }
                        Err(e) => {
                            log::debug!(
                                "send: LoRa extended→compact transcode failed: {:?}",
                                e
                            );
                        }
                    }
                }
                // LoRa via USB-attached dongle. Compact format on the
                // air (same rationale as the native binding).
                self.try_send_via_dongle(transport, frame, true).await
            }
        }
    }

    /// Attempt to send `frame` (extended-format R2-WIRE) via a paired
    /// USB peripheral that advertises the requested transport kind in
    /// its CAPS. Returns `false` on Linux without a configured USB
    /// watcher, on non-Linux platforms, when no paired dongle
    /// advertises that kind, or when the session's control channel
    /// has been dropped.
    ///
    /// `compact_on_wire = true` means transcode to compact format
    /// before sending (BLE, LoRa). `false` means send the extended
    /// frame verbatim (Internet, WiFi).
    #[cfg(target_os = "linux")]
    async fn try_send_via_dongle(
        &self,
        transport: r2_route::transport::Transport,
        frame: &[u8],
        compact_on_wire: bool,
    ) -> bool {
        let handle = match self.usb_handle() {
            Some(h) => h,
            None => return false,
        };
        let kind = transport_to_caps_kind(transport);
        let (path, local_id) = match handle.find_dongle_for_kind(&kind) {
            Some(p) => p,
            None => return false,
        };
        let body = if compact_on_wire {
            let mut buf = vec![0u8; frame.len()];
            match r2_wire::transcode::transcode_extended_to_compact(frame, &mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    buf
                }
                Err(e) => {
                    log::debug!(
                        "send: {:?} extended→compact transcode failed: {:?}",
                        transport,
                        e
                    );
                    return false;
                }
            }
        } else {
            frame.to_vec()
        };
        handle.send_via_path(&path, local_id, body).await
    }

    #[cfg(not(target_os = "linux"))]
    async fn try_send_via_dongle(
        &self,
        _transport: r2_route::transport::Transport,
        _frame: &[u8],
        _compact_on_wire: bool,
    ) -> bool {
        false
    }

    // ── Legacy TG-compat surface ────────────────────────────────────────
    // The methods from here to `resolve_tg_hash` serve the legacy browser
    // WebSocket protocol in `compat/handshake.rs` (their only production
    // caller unless noted): membership tracking, TG broadcast fan-out, and
    // the frame catchup buffer. They predate the route engine and remain
    // until browser clients migrate to R2-ROUTE proper.

    /// Register a peer as a member of a trust group (creating the group's
    /// compat entry, including its catchup buffer, on first sight).
    ///
    /// **Used-by:** `compat/handshake.rs` on legacy-client join.
    pub async fn register_tg_peer(&self, tg_hash: TrustGroupHash, hive_id: u32) {
        let mut map = self.tg_map.write().await;
        let entry = map.entry(tg_hash).or_insert_with(|| TrustGroupCompat {
            peers: HashSet::new(),
            buffer: RingBuffer::new(self.buffer_size),
        });
        entry.peers.insert(hive_id);
        self.connections_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Unregister a peer from a trust group (dropping the group's compat
    /// entry when the last member leaves).
    ///
    /// **Used-by:** `compat/handshake.rs` on legacy-client disconnect.
    pub async fn unregister_tg_peer(&self, tg_hash: &TrustGroupHash, hive_id: u32) {
        let mut map = self.tg_map.write().await;
        if let Some(entry) = map.get_mut(tg_hash) {
            entry.peers.remove(&hive_id);
            if entry.peers.is_empty() {
                map.remove(tg_hash);
            }
        }
    }

    /// Broadcast a frame to all peers in a trust group except the sender,
    /// using the full multi-transport fallback per peer.
    ///
    /// **Used-by:** `compat/handshake.rs` (legacy broadcast),
    /// `mgmt/primitive.rs` (`event.send` with no target hive), and
    /// `mgmt/ensemble.rs` (sentant `Action::Send` broadcast).
    pub async fn broadcast_to_tg(&self, tg_hash: &TrustGroupHash, sender: u32, frame: &[u8]) {
        let map = self.tg_map.read().await;
        if let Some(entry) = map.get(tg_hash) {
            let peer_count = entry.peers.len();
            let mut sent = 0;
            for &hive_id in &entry.peers {
                if hive_id != sender {
                    if self.send_to_hive(hive_id, frame).await {
                        sent += 1;
                    } else {
                        log::debug!("broadcast: 0x{:08X} unreachable on any transport", hive_id);
                    }
                }
            }
            log::debug!("broadcast: {} bytes from 0x{:08X} to {}/{} peers in tg:{}",
                frame.len(), sender, sent, peer_count - 1,
                hex_encode(tg_hash));
        } else {
            log::warn!("broadcast: no trust group found for tg:{}", hex_encode(tg_hash));
        }
    }

    /// Buffer a frame in the TG's catchup ring so late-joining legacy
    /// clients can replay what they missed.
    ///
    /// **Used-by:** `compat/handshake.rs` alongside each legacy broadcast.
    pub async fn buffer_frame(&self, tg_hash: &TrustGroupHash, data: Vec<u8>) {
        let mut map = self.tg_map.write().await;
        if let Some(entry) = map.get_mut(tg_hash) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            entry.buffer.push(data, now);
        }
    }

    /// Get catchup frames buffered since a UNIX timestamp.
    ///
    /// **Used-by:** `compat/handshake.rs` when a legacy client asks to
    /// catch up after (re)connecting.
    pub async fn catchup_frames(&self, tg_hash: &TrustGroupHash, since: u64) -> Vec<Vec<u8>> {
        let map = self.tg_map.read().await;
        match map.get(tg_hash) {
            Some(entry) => entry.buffer.since(since).map(|f| f.data.clone()).collect(),
            None => Vec::new(),
        }
    }

    /// Number of peers currently registered in a trust group.
    ///
    /// **Used-by:** `compat/handshake.rs` (join-response stats).
    pub async fn tg_peer_count(&self, tg_hash: &TrustGroupHash) -> usize {
        let map = self.tg_map.read().await;
        map.get(tg_hash).map(|e| e.peers.len()).unwrap_or(0)
    }

    /// Oldest buffered catchup timestamp for a trust group (0 if none).
    ///
    /// **Used-by:** `compat/handshake.rs` (join-response stats, so clients
    /// know how far back catchup can reach).
    pub async fn buffer_oldest(&self, tg_hash: &TrustGroupHash) -> u64 {
        let map = self.tg_map.read().await;
        map.get(tg_hash).map(|e| e.buffer.oldest_timestamp()).unwrap_or(0)
    }

    /// Flood to TG peers that the route engine didn't cover.
    /// When the engine says FLOOD to N hops, there may be freshly connected
    /// peers it doesn't know about yet (no observation ingested). Send to those too.
    ///
    /// **Used-by:** `compat/handshake.rs` after a `RouteOutcome::Flooded`
    /// from `router::route_frame` (intra-TG flood enrichment).
    pub async fn flood_tg_peers_not_in(
        &self,
        tg_hash: &TrustGroupHash,
        sender: u32,
        covered_hops: &[r2_route::engine::DirectedHop],
        frame: &[u8],
    ) {
        let map = self.tg_map.read().await;
        if let Some(entry) = map.get(tg_hash) {
            for &hive_id in &entry.peers {
                if hive_id == sender { continue; }
                if covered_hops.iter().any(|h| h.neighbour == hive_id) { continue; }
                log::debug!("route: flood-extra 0x{:08X} (not in engine hop list)", hive_id);
                let _ = self.ws_transport.send(hive_id, frame).await;
            }
        }
    }

    /// Resolve a trust group hash from a hex string.
    /// Accepts exact 16-char hex or a 2–6 char prefix (matched against the
    /// currently-registered groups — the word-code join flow sends prefixes).
    ///
    /// **Dependencies:** `hex_decode` / `hex_encode`; the `tg_map` read lock.
    ///
    /// **Used-by:** `compat/handshake.rs` when a legacy client names a TG.
    pub async fn resolve_tg_hash(&self, hex: &str) -> Result<TrustGroupHash, &'static str> {
        if hex.len() == 16 {
            // Exact match
            let bytes = hex_decode(hex).ok_or("invalid hex")?;
            if bytes.len() != 8 { return Err("invalid trust_group"); }
            let mut h = [0u8; 8];
            h.copy_from_slice(&bytes);
            Ok(h)
        } else if hex.len() >= 2 && hex.len() <= 6 {
            // Prefix match against active trust groups
            let map = self.tg_map.read().await;
            let matches: Vec<TrustGroupHash> = map.keys()
                .filter(|h| hex_encode(*h).starts_with(hex))
                .copied()
                .collect();
            match matches.len() {
                0 => Err("no matching trust group"),
                1 => Ok(matches[0]),
                _ => Err("ambiguous trust group prefix"),
            }
        } else {
            Err("invalid trust_group length")
        }
    }
}

/// Decode a hex string into bytes (`None` on odd length or a non-hex
/// character). The crate's single hex decoder — a byte-identical duplicate
/// in `compat/handshake.rs` was removed (Occam) in favour of this one.
///
/// **Used-by:** [`HiveState::resolve_tg_hash`], [`parse_bench_group_hmacs`],
/// and `compat/handshake.rs` (nonce/TG/device-id parsing).
pub(crate) fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Encode bytes as a lowercase hex string. Counterpart of [`hex_decode`],
/// same single-copy rule.
///
/// **Used-by:** the TG log lines in this file and `compat/handshake.rs`
/// (join responses, word-code flow).
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// §7.5.4 deliver-gate group-key load. PRODUCTION custody = a sealed keystore
/// (R2-KEYSTORE §4; the HK arrives at provision/join, sealed at rest) — that's a
/// FOLLOW-UP. For the FR-2b BENCH ONLY: an explicit `R2_GROUP_KEYS_BENCH` env var
/// pointing at composer's plaintext json `{ "keys": { "<tg_u32>": "<64-hex HK>" } }`
/// (keyed by the WIRE target_group). This is the C3 at-rest exposure, so it is
/// gated behind an env var — production NEVER auto-loads plaintext. Unset/empty
/// => empty map => gate INACTIVE (migration mode: deliver + warn).
///
/// **Used-by:** [`HiveState::new`] only (populates `group_hmacs` at boot).
/// DEV BUILDS ONLY (R2-BUILDMODE §5.1): the C3 plaintext-key path does not
/// exist in a prod binary.
#[cfg(feature = "dev")]
fn load_bench_group_hmacs() -> HashMap<u32, GroupHmac> {
    let map = HashMap::new();
    let path = match std::env::var("R2_GROUP_KEYS_BENCH") {
        Ok(p) if !p.is_empty() => p,
        _ => return map, // no bench keys configured -> gate inactive (migration)
    };
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("§7.5.4: R2_GROUP_KEYS_BENCH={path} unreadable ({e}) — deliver-gate INACTIVE");
            return map;
        }
    };
    let map = parse_bench_group_hmacs(&data);
    log::warn!(
        "§7.5.4: loaded {} group key(s) from BENCH PLAINTEXT {path} — C3 at-rest exposure, DEV-ONLY \
         (production MUST use a sealed keystore, R2-KEYSTORE §4)",
        map.len()
    );
    map
}

/// Parse composer's bench group-keys json `{ "keys": { "<tg_u32>": "<64-hex HK>" } }`
/// into GroupHmacs keyed by the WIRE target_group (u32). Pure (no env/file) so the
/// parsing is unit-testable; `load_bench_group_hmacs` handles the env + file read.
///
/// **Used-by:** [`load_bench_group_hmacs`] and the `group_key_tests` module.
/// Compiled for dev builds + tests (the pure parser stays test-covered in
/// every mode; only the env/file INPUT path is dev-gated).
#[cfg(any(test, feature = "dev"))]
fn parse_bench_group_hmacs(data: &str) -> HashMap<u32, GroupHmac> {
    let mut map = HashMap::new();
    let json: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("§7.5.4: bench group-keys json parse error ({e}) — deliver-gate INACTIVE");
            return map;
        }
    };
    if let Some(keys) = json.get("keys").and_then(|k| k.as_object()) {
        for (tg_str, hk_val) in keys {
            let tg: u32 = match tg_str.parse() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let hk_hex = match hk_val.as_str() {
                Some(s) => s,
                None => continue,
            };
            match hex_decode(hk_hex) {
                Some(b) if b.len() == 32 => {
                    let mut hk = [0u8; 32];
                    hk.copy_from_slice(&b);
                    map.insert(tg, GroupHmac::new(hk));
                }
                _ => log::warn!("§7.5.4: tg {tg} HK is not 32 bytes — skipped"),
            }
        }
    }
    map
}

#[cfg(test)]
mod denied_frame_tests {
    use super::*;
    use r2_cbor::{Decoder, Item};

    /// Pin the ratified R2-HOST-API §3.2.1 `event.delivery.denied` key map:
    /// keys 0/2/3/5/7 reused verbatim from `event.delivery`; 8=target_group,
    /// 9=reason; 1 (sub_id), 4 (payload), 6 (from_tg) omitted.
    #[test]
    fn denied_frame_matches_ratified_key_map() {
        let frame = build_denied_frame(0x00AB_CD12, 42, 0xDEAD_BEEF, "forgery");
        let msg = r2_wire::decode_extended(&frame).expect("decodes as extended");
        let denied_hash = r2_fnv::r2_hash("r2.api.event.delivery.denied").unwrap();
        assert_eq!(msg.header.event_hash, denied_hash, "outer wire class hash");

        let mut dec = Decoder::new(msg.payload);
        let entries = match dec.next().expect("map header") {
            Item::Map(n) => n,
            _ => panic!("payload is not a CBOR map"),
        };
        assert_eq!(entries, 7, "exactly the 7 ratified entries (0,2,3,5,7,8,9)");

        let mut seen: std::collections::BTreeMap<u64, String> = std::collections::BTreeMap::new();
        for _ in 0..entries {
            let key = dec.next().expect("cbor key");
            let val = dec.next().expect("cbor val");
            let k = match key {
                Item::UInt(k) => k,
                _ => panic!("non-uint key"),
            };
            let rendered = match (k, val) {
                (0 | 3 | 5 | 7 | 8, Item::UInt(v)) => v.to_string(),
                (2 | 9, Item::Text(bytes)) => String::from_utf8(bytes.to_vec()).unwrap(),
                _ => panic!("unexpected key {k} or wrong value type"),
            };
            seen.insert(k, rendered);
        }
        assert_eq!(seen[&0], "0", "cid=0 (unsolicited notification)");
        assert_eq!(seen[&2], "r2.api.event.delivery.denied");
        assert_eq!(seen[&3], u64::from(denied_hash).to_string(), "event_hash");
        assert_eq!(seen[&5], 0x00AB_CD12u64.to_string(), "from_hive");
        assert_eq!(seen[&7], "42", "msg_id");
        assert_eq!(seen[&8], 0xDEAD_BEEFu64.to_string(), "claimed target_group");
        assert_eq!(seen[&9], "forgery", "reason (ratified text taxonomy)");
        for omitted in [1u64, 4, 6] {
            assert!(!seen.contains_key(&omitted), "key {omitted} must be omitted");
        }
    }

    /// FLOW proof (the acceptance check this event exists for — composer
    /// renders forge-reject RED from this notification, Roy's real-red bar):
    /// a deliver-gate reject actually ARRIVES on a subscriber channel, and
    /// the §3.2.1 match rules hold — a denied-class-filtered subscription
    /// receives it, a `from_tg`-filtered subscription NEVER does (the TG
    /// claim is exactly what's untrusted), and a broadcast subscription
    /// receives it distinguishably (class hash on the outer wire / key 2).
    #[tokio::test]
    async fn deny_inbound_flows_to_subscribers_per_ratified_match_rules() {
        use crate::mgmt::subscriptions::SubscriptionFilter;

        let state = HiveState::new(0x0000_0001, 64, 16);

        // A: filtered on the denied class — MUST receive.
        let (tx_a, mut rx_a) = mpsc::channel::<Vec<u8>>(4);
        let (_id_a, subs_a) = state.register_subscriber(tx_a).await;
        subs_a.lock().await.add(SubscriptionFilter {
            event_class: Some("r2.api.event.delivery.denied".to_string()),
            ..Default::default()
        });

        // B: from_tg-filtered — MUST NOT receive (§3.2.1 omits key 6; the
        // frame's TG claim is unverified by definition).
        let (tx_b, mut rx_b) = mpsc::channel::<Vec<u8>>(4);
        let (_id_b, subs_b) = state.register_subscriber(tx_b).await;
        subs_b.lock().await.add(SubscriptionFilter {
            from_tg: Some([0xAB; 8]),
            ..Default::default()
        });

        // C: broadcast (no filter) — receives denies too, distinguishable.
        let (tx_c, mut rx_c) = mpsc::channel::<Vec<u8>>(4);
        let (_id_c, subs_c) = state.register_subscriber(tx_c).await;
        subs_c.lock().await.add(SubscriptionFilter::default());

        state
            .deny_inbound(7, 0x0402_1CBD, "unauthenticated", 0x00AB_CD12)
            .await;

        let frame_a = rx_a.try_recv().expect("denied-class subscriber receives");
        let denied_hash = r2_fnv::r2_hash("r2.api.event.delivery.denied").unwrap();
        assert_eq!(
            r2_wire::decode_extended(&frame_a).expect("decodes").header.event_hash,
            denied_hash
        );

        assert!(
            rx_b.try_recv().is_err(),
            "from_tg-filtered subscription must never match a deny"
        );

        let frame_c = rx_c.try_recv().expect("broadcast subscriber receives");
        assert_eq!(
            r2_wire::decode_extended(&frame_c).unwrap().header.event_hash,
            denied_hash,
            "broadcast consumer distinguishes a deny by class hash"
        );
    }
}

#[cfg(test)]
mod group_key_tests {
    use super::*;

    #[test]
    fn parses_bench_json_keyed_by_target_group() {
        let hk_hex = "27755ad8866f5633b5001002cf0ae581f8395d5c415ef59a70b5d7179bc1b23d";
        let json = format!(r#"{{"keys":{{"177560432":"{hk_hex}","99":"deadbeef"}}}}"#);
        let map = parse_bench_group_hmacs(&json);
        // valid 32-byte HK -> loaded; the short "deadbeef" -> skipped.
        assert_eq!(map.len(), 1);
        let gh = map.get(&177560432).expect("tg 177560432 present");
        assert_eq!(&gh.key()[..], &hex_decode(hk_hex).unwrap()[..]);
        assert!(!map.contains_key(&99));
    }

    #[test]
    fn empty_or_invalid_json_yields_empty_map_migration() {
        assert!(parse_bench_group_hmacs("not json").is_empty());
        assert!(parse_bench_group_hmacs(r#"{"keys":{}}"#).is_empty());
        assert!(parse_bench_group_hmacs(r#"{}"#).is_empty());
    }
}

/// Build an `r2.api.event.delivery` notification frame for one subscription
/// match. Encoded per R2-HOST-API §3.2 (event.delivery payload keys 0–7).
fn build_delivery_frame(
    sub_id: u32,
    event_class: &str,
    event_hash: u32,
    payload: &[u8],
    from_hive: u64,
    from_tg: Option<TrustGroupHash>,
    msg_id: u64,
) -> Vec<u8> {
    use r2_cbor::{Encoder, Value};
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let event_class_hash = r2_fnv::r2_hash("r2.api.event.delivery").expect("known event");

    let mut payload_buf = vec![0u8; 96 + event_class.len() + payload.len()];
    let used = {
        let mut enc = Encoder::new(&mut payload_buf);
        let entries: usize = if from_tg.is_some() { 8 } else { 7 };
        enc.map(entries).expect("map header");
        enc.kv(0, &Value::UInt(0)).expect("cid=0 (notification)");
        enc.kv(1, &Value::UInt(sub_id as u64)).expect("sub_id");
        enc.kv(2, &Value::Text(event_class)).expect("event_class");
        enc.kv(3, &Value::UInt(event_hash as u64)).expect("event_hash");
        enc.kv(4, &Value::Bytes(payload)).expect("payload");
        enc.kv(5, &Value::UInt(from_hive)).expect("from_hive");
        if let Some(tg) = from_tg {
            enc.kv(6, &Value::Bytes(&tg)).expect("from_tg");
        }
        enc.kv(7, &Value::UInt(msg_id)).expect("msg_id");
        enc.len()
    };

    let outbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0,
            k: 0,
            msg_id: 0,
            event_hash: event_class_hash,
            payload_len: used as u32,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &payload_buf[..used],
        hmac_tag: None,
    };
    let mut wire = vec![0u8; used + 64];
    let n = encode_extended(&outbound, &mut wire).expect("encode_extended fits");
    wire.truncate(n);
    wire
}

/// Build an `r2.api.event.delivery.denied` notification frame per the ratified
/// R2-HOST-API §3.2.1 key map: keys 0/2/3/5/7 reused verbatim from
/// `event.delivery`; keys 1 (`sub_id`), 4 (`payload`) and 6 (`from_tg`)
/// deliberately omitted (a denial is not tied to a subscription, and a denied
/// frame's payload/TG claim is unverified by definition); new keys
/// 8 = `target_group` (uint32, the denied frame's *claimed* target group) and
/// 9 = `reason` (text: `"forgery"` | `"unauthenticated"` | `"fail_closed"`).
fn build_denied_frame(from_hive: u64, msg_id: u64, target_group: u32, reason: &str) -> Vec<u8> {
    use r2_cbor::{Encoder, Value};
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let event_class = crate::mgmt::api::EV_EVENT_DELIVERY_DENIED;
    let event_class_hash = r2_fnv::r2_hash(event_class).expect("known event");

    let mut payload_buf = vec![0u8; 96 + event_class.len() + reason.len()];
    let used = {
        let mut enc = Encoder::new(&mut payload_buf);
        enc.map(7).expect("map header");
        enc.kv(0, &Value::UInt(0)).expect("cid=0 (notification)");
        enc.kv(2, &Value::Text(event_class)).expect("event_class");
        enc.kv(3, &Value::UInt(event_class_hash as u64)).expect("event_hash");
        enc.kv(5, &Value::UInt(from_hive)).expect("from_hive");
        enc.kv(7, &Value::UInt(msg_id)).expect("msg_id");
        enc.kv(8, &Value::UInt(target_group as u64)).expect("target_group");
        enc.kv(9, &Value::Text(reason)).expect("reason");
        enc.len()
    };

    let outbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0,
            k: 0,
            msg_id: 0,
            event_hash: event_class_hash,
            payload_len: used as u32,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &payload_buf[..used],
        hmac_tag: None,
    };
    let mut wire = vec![0u8; used + 64];
    let n = encode_extended(&outbound, &mut wire).expect("encode_extended fits");
    wire.truncate(n);
    wire
}

/// Build a `r2.mgmt.event.error` frame with code `backpressure` and the
/// affected sub_id at key 3. Per R2-HOST-API §6.2.
fn build_backpressure_error(sub_id: u32) -> Vec<u8> {
    use r2_cbor::{Encoder, Value};
    use r2_wire::{encode_extended, ExtendedHeader, ExtendedMessage, Flags, MsgType};

    let event_hash = r2_fnv::r2_hash("r2.mgmt.event.error").expect("known event");
    let mut payload_buf = [0u8; 64];
    let used = {
        let mut enc = Encoder::new(&mut payload_buf);
        enc.map(3).expect("map header");
        enc.kv(0, &Value::UInt(0)).expect("cid=0 (notification)");
        enc.kv(1, &Value::Text("backpressure")).expect("code");
        enc.kv(3, &Value::UInt(sub_id as u64)).expect("sub_id ctx");
        enc.len()
    };

    let outbound = ExtendedMessage {
        header: ExtendedHeader {
            version: 0,
            msg_type: MsgType::Event,
            flags: Flags::default(),
            ttl: 0,
            k: 0,
            msg_id: 0,
            event_hash,
            payload_len: used as u32,
            target_group: 0,
            target_hive: 0,
        },
        route: None,
        payload: &payload_buf[..used],
        hmac_tag: None,
    };
    let mut wire = vec![0u8; used + 64];
    let n = encode_extended(&outbound, &mut wire).expect("encode_extended fits");
    wire.truncate(n);
    wire
}

/// Map a route-engine [`r2_route::transport::Transport`] back to the
/// CAPS-side enumerated kind from R2-USB Appendix A. Used by
/// [`HiveState::try_send_via_dongle`] (Phase USB-5) to look up which
/// dongle (if any) advertises a transport of the requested kind.
#[cfg(target_os = "linux")]
fn transport_to_caps_kind(
    transport: r2_route::transport::Transport,
) -> crate::usb::TransportKind {
    use crate::usb::TransportKind;
    use r2_route::transport::Transport;
    let id = match transport {
        Transport::Lora => 1,
        Transport::Ble => 2,
        Transport::Wifi => 3,
        Transport::Internet => 4,
        Transport::Usb => 5,
        Transport::WifiMesh => 6,
        Transport::Udp => 7,
    };
    TransportKind::Enumerated(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport_seam::HiveTransports;

    /// The platform host state satisfies the hive-core `HiveTransports` seam
    /// through a trait object — i.e. hive-core forwarding code can hold
    /// `&dyn HiveTransports` and call it. With no transports registered every
    /// send fails, but the seam is object-safe and callable, which is what this
    /// asserts (the trait itself now lives in `r2-hive-core`).
    #[tokio::test]
    async fn hive_state_is_a_hive_transports_trait_object() {
        let state = HiveState::new(0x0000_0001, 64, 16);
        let seam: &dyn HiveTransports = &state;
        assert!(!seam.send_to_hive(0x0000_0002, b"frame").await);
        assert!(seam
            .send_to_hive_via(0x0000_0002, None, b"frame")
            .await
            .is_none());
    }
}
