//! Transport seam — the outbound multi-transport contract hive-core forwarding
//! depends on, abstracted from the concrete platform transports (R2-HIVE
//! north-star: one hive-core, thin per-platform layers).
//!
//! - **Host (Linux/cloud):** satisfied over the async `r2-discovery` transports
//!   (WS / UDP-LAN / BLE / LoRa) — the current [`crate::hive::HiveState`] impl.
//! - **MCU (ESP32/no_std):** will be satisfied over core's no_std **R2-TRANSPORT
//!   sync** drivers (D3b). A sync `Transport::send` is fire-and-forget ("accepted,
//!   not delivered", R2-TRANSPORT) so it adapts to this async contract trivially
//!   (wrap in a ready future under embassy). See
//!   `docs/esp32-hive-firmware-architecture.md`.
//!
//! hive-core forwarding logic targets this trait (`&dyn HiveTransports`) rather
//! than a concrete transport set, so the same routing code runs on every platform.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hive::HiveState;

    /// The platform host state is usable as the `HiveTransports` seam through a
    /// trait object — i.e. hive-core code can hold `&dyn HiveTransports`. With no
    /// transports registered every send fails, but the seam is object-safe and
    /// callable, which is what this asserts.
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
