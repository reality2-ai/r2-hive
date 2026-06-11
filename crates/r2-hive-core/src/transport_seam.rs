//! Transport seam — the outbound multi-transport contract hive-core forwarding
//! depends on, abstracted from the concrete platform transports (R2-HIVE
//! north-star: one hive-core, thin per-platform layers).
//!
//! - **Host (Linux/cloud):** satisfied over the async `r2-discovery` transports
//!   (WS / UDP-LAN / BLE / LoRa) — the `HiveState` impl in `r2-hive-bin`.
//! - **MCU (ESP32/no_std):** will be satisfied over core's no_std **R2-TRANSPORT
//!   sync** drivers (D3b). A sync `Transport::send` is fire-and-forget ("accepted,
//!   not delivered", R2-TRANSPORT) so it adapts to this async contract trivially
//!   (wrap in a ready future under embassy). See [`crate::sync_host`].
//!
//! hive-core forwarding targets this trait (`&dyn HiveTransports`) rather than a
//! concrete transport set, so the same routing code runs on every platform.
//! `no_std` + `alloc` (async-trait boxes the futures).

use alloc::boxed::Box; // async-trait boxes the returned futures (no_std).
use async_trait::async_trait;
use r2_route::transport::Transport;

/// Outbound frame delivery to a hive over the multi-transport fallback chain.
#[async_trait]
pub trait HiveTransports: Send + Sync {
    /// Send `frame` to `hive_id` over the best available transport. Returns
    /// `true` if some transport accepted it for transmission.
    async fn send_to_hive(&self, hive_id: u32, frame: &[u8]) -> bool;

    /// Send preferring `hint` (the route engine's recommended transport), falling
    /// back through the priority order. Returns the transport used, or `None` if
    /// every transport failed.
    async fn send_to_hive_via(
        &self,
        hive_id: u32,
        hint: Option<Transport>,
        frame: &[u8],
    ) -> Option<Transport>;
}
