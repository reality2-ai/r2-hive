//! Platform abstraction — the seam for the one-codebase, multi-target hive
//! (R2-HIVE north-star).
//!
//! Platform-coupled capabilities the hive-core logic needs are expressed through
//! this trait so the core stays platform-agnostic; each target supplies its own
//! impl in its platform layer (e.g. `LinuxPlatform` in `r2-hive-bin`, an
//! esp-hal/embassy impl for the DFR1195 firmware, a wasm impl for the browser
//! hive). The trait itself is `no_std` — only the impls pull in a platform.
//!
//! Establishes the **clock** and **RNG** seams; storage (identity/OTA) and
//! display/input follow as the convergence proceeds.

/// Platform-provided capabilities, abstracted so hive-core logic is
/// platform-agnostic. `Send + Sync + 'static` so it can live in an `Arc` inside
/// shared state across async tasks / threads.
pub trait Platform: Send + Sync + 'static {
    /// Wall-clock time, whole seconds since the Unix epoch. Used for protocol
    /// timestamp windows (e.g. the relay handshake ±60s, R2-TRANSPORT-RELAY §3.2).
    fn now_unix(&self) -> u64;

    /// Monotonic milliseconds since an arbitrary fixed start (process/boot).
    /// Used for timeouts/liveness, where wall-clock jumps must not matter.
    fn monotonic_ms(&self) -> u64;

    /// Fill `buf` with cryptographically-secure random bytes (challenge nonces,
    /// key material). Implementations MUST use a CSPRNG.
    fn fill_random(&self, buf: &mut [u8]);
}
