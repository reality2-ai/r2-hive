//! Linux / cloud **platform layer** — the std impl of the hive-core [`Platform`]
//! seam (R2-HIVE north-star: the trait lives in `r2-hive-core`; each target
//! supplies its own impl). The ESP32-S3/DFR1195, Uno-Q, and wasm layers will add
//! their own impls in their own crates.
//!
//! Re-exports the trait so `crate::platform::Platform` keeps resolving across the
//! bin while the convergence migrates modules into `r2-hive-core`.

use std::sync::Arc;

pub use r2_hive_core::platform::Platform;

/// Linux / cloud platform: std `SystemTime` / `Instant` + the OS CSPRNG.
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
