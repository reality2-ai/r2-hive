//! Optional systemd integration (Phase 3e).
//!
//! When built with `--features systemd`, the daemon:
//!
//! 1. Calls `sd_notify(READY=1)` after the HTTP listener is bound and
//!    the management socket is up — so a `Type=notify` unit can
//!    transition to `active` without timing out.
//! 2. Spawns a watchdog task that posts `WATCHDOG=1` every
//!    `WatchdogSec/2`, derived from `WATCHDOG_USEC` set by systemd.
//!    If systemd is not the parent (env var unset), the watchdog
//!    task exits silently.
//!
//! On non-Linux platforms or with the feature off, every function in
//! this module is a no-op.

#[cfg(feature = "systemd")]
pub fn notify_ready() {
    if let Err(e) = sd_notify::notify(false, &[sd_notify::NotifyState::Ready]) {
        log::debug!("sd_notify(READY=1) failed: {e} (not a systemd-activated process?)");
    } else {
        log::info!("sd_notify(READY=1) sent");
    }
}

#[cfg(not(feature = "systemd"))]
pub fn notify_ready() {}

/// Spawn the watchdog ping loop. Reads `WATCHDOG_USEC` from systemd
/// and pings at half that interval. If the env var isn't set, returns
/// immediately — non-systemd parents see no behaviour change.
#[cfg(feature = "systemd")]
pub fn spawn_watchdog() {
    let mut usec: u64 = 0;
    let enabled = sd_notify::watchdog_enabled(false, &mut usec);
    if !enabled || usec == 0 {
        log::debug!("watchdog disabled (WATCHDOG_USEC unset or zero)");
        return;
    }
    let half = std::time::Duration::from_micros(usec / 2);
    log::info!(
        "systemd watchdog enabled — pinging every {} ms",
        half.as_millis()
    );
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(half).await;
            if let Err(e) = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]) {
                log::warn!("watchdog ping failed: {e}");
                // Stop pinging — systemd will notice and act per the
                // unit's WatchdogSignal / Restart policy.
                break;
            }
        }
    });
}

#[cfg(not(feature = "systemd"))]
pub fn spawn_watchdog() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_ready_is_safe_outside_systemd() {
        // Without NOTIFY_SOCKET set, sd_notify is a no-op (with feature on)
        // or a real no-op (with feature off). Either way, this MUST NOT
        // panic, because the binary calls it unconditionally at startup.
        std::env::remove_var("NOTIFY_SOCKET");
        notify_ready();
    }

    #[cfg(feature = "systemd")]
    #[tokio::test]
    async fn spawn_watchdog_no_systemd_returns_immediately() {
        // No WATCHDOG_USEC ⇒ the function returns without spawning.
        // Verifying it doesn't hang or panic is enough.
        std::env::remove_var("WATCHDOG_USEC");
        std::env::remove_var("WATCHDOG_PID");
        spawn_watchdog();
    }
}
