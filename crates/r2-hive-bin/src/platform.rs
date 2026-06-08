//! Platform abstraction — the seam for the one-codebase, multi-target hive
//! (R2-HIVE north-star).
//!
//! The hive is **one codebase** that runs everywhere: Linux/cloud (tokio/axum),
//! ESP32-S3/DFR1195 (esp-hal/embassy, no_std), Uno-Q, and the wasm browser hive.
//! Platform-coupled capabilities the host loop needs are expressed through this
//! trait so the hive-core logic stays platform-agnostic; each target provides its
//! own [`Platform`] impl.
//!
//! This is the first convergence increment: it establishes the **clock** and
//! **RNG** seams (the smallest, most pervasive, no_std-friendly ones). Transports
//! (the R2-TRANSPORT *sync* interface on no_std; async `r2-discovery` on host),
//! storage (identity / OTA), and display/input follow as the convergence proceeds
//! — see `docs/esp32-hive-firmware-architecture.md`.

use std::sync::Arc;

/// Platform-provided capabilities, abstracted so hive-core logic is
/// platform-agnostic. `Send + Sync + 'static` so it can live in an [`Arc`] inside
/// shared state across async tasks / threads.
pub trait Platform: Send + Sync + 'static {
    /// Wall-clock time, whole seconds since the Unix epoch. Used for protocol
    /// timestamp windows (e.g. the relay handshake ±60s, R2-TRANSPORT-RELAY §3.2).
    fn now_unix(&self) -> u64;

    /// Monotonic milliseconds since an arbitrary fixed start (process boot).
    /// Used for timeouts/liveness, where wall-clock jumps must not matter.
    fn monotonic_ms(&self) -> u64;

    /// Fill `buf` with cryptographically-secure random bytes (challenge nonces,
    /// key material). Implementations MUST use a CSPRNG.
    fn fill_random(&self, buf: &mut [u8]);
}

/// Linux / cloud platform: std `SystemTime` / `Instant` + the OS CSPRNG.
///
/// The ESP32-S3, Uno-Q, and wasm impls will live in their own platform layers;
/// this is the first impl and the one exercised by the current std hive.
pub struct LinuxPlatform;

impl Platform for LinuxPlatform {
    fn now_unix(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn monotonic_ms(&self) -> u64 {
        use std::sync::OnceLock;
        use std::time::Instant;
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_millis() as u64
    }

    fn fill_random(&self, buf: &mut [u8]) {
        getrandom::getrandom(buf).expect("OS CSPRNG available");
    }
}

/// Convenience constructor for the default (Linux) platform as a trait object.
pub fn linux() -> Arc<dyn Platform> {
    Arc::new(LinuxPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_unix_is_after_2020() {
        // 2020-01-01 UTC — a sanity floor that the wall clock is real.
        assert!(LinuxPlatform.now_unix() > 1_577_836_800);
    }

    #[test]
    fn monotonic_is_nondecreasing() {
        let p = LinuxPlatform;
        let a = p.monotonic_ms();
        let b = p.monotonic_ms();
        assert!(b >= a);
    }

    #[test]
    fn fill_random_fills() {
        // Two draws of a non-trivial buffer must differ (overwhelmingly).
        let p = LinuxPlatform;
        let (mut x, mut y) = ([0u8; 32], [0u8; 32]);
        p.fill_random(&mut x);
        p.fill_random(&mut y);
        assert_ne!(x, [0u8; 32]);
        assert_ne!(x, y);
    }

    #[test]
    fn usable_as_trait_object() {
        let p = linux();
        assert!(p.now_unix() > 0);
    }
}
