# r2-sx1262 `SetRxDutyCycle` (0x94) — driver diff PROPOSAL (hive → core review-accept)

> **Status:** PROPOSAL for **core** (r2-sx1262 + r2-transport are CORE-OWNED — hive authors, core
> accepts/adjusts + commits). Enables the XIAO edge-bridge heat-fix path-1 (R2-RUNTIME §3.2.6
> ratified @4072063; Roy scope-eyeball CONFIRMED; supervisor GREEN). Matches core's pre-flagged
> driver shape (fleet, this turn) and folds its **two correctness points**. Physical-µs params keep
> the chip-agnostic seam intact and let firmware size per-SF.

## Design: one new trait method + one Sx1262 override (NON-BREAKING)
`listen_duty_cycle(rx_period_us, sleep_period_us)` on `LoRaRadio`, **with a default that falls back
to continuous `listen()`** — so LR2021, the `lora_transport.rs` test mock, and any other impl keep
compiling unchanged (a radio without HW duty-cycle still receives correctly, just no power saving).
Only the SX1262 overrides it with the real `0x94`. Physical-µs units (not chip steps) honour the
seam's "PHYSICAL UNITS only" rule; the driver converts µs → 24-bit 15.625 µs steps internally.

## Diff 1 — `crates/r2-transport/src/lora.rs` (the seam: default method)
Add to `trait LoRaRadio` (after `standby`, before the closing brace):

```rust
    /// Enter **duty-cycled** receive (SX126x SetRxDutyCycle, datasheet §13.1.4): the radio
    /// autonomously cycles an RX window (`rx_period_us`) ↔ warm-sleep (`sleep_period_us`),
    /// waking the host via DIO1 only on `RxDone`. This is the STEP-4 duty-cycle enforcement
    /// for a pure edge-bridge in standby (R2-RUNTIME §3.2.6) — it trades a bounded
    /// miss-probability for a large idle-power (heat) saving.
    ///
    /// ★ SIZING (caller's responsibility, PER-SF): `rx_period_us` MUST be ≥ the LoRa
    /// preamble-detection window for the active SF, or the receiver sleeps through a packet's
    /// preamble. The intermediate-sleep RX restarts preamble detection each `rx_period_us`, so
    /// long-SF needs a long window: SF12/BW125 symbol ≈ 32.8 ms (a few-symbol preamble ≈ 100+ ms)
    /// vs SF7/BW125 symbol ≈ 1 ms. **Bench at SF7 first** (benchsf7); SF12 needs the long window
    /// or it regresses RX. `sleep_period_us`/wake cadence is bounded by the UPSTREAM buffering
    /// node's `scf_ttl_s` (R2-RUNTIME §3.2.6 sizing invariant), NOT a fixed constant.
    ///
    /// **Default:** falls back to continuous [`listen`](Self::listen) — non-breaking for radios
    /// without a hardware duty-cycle primitive (correct RX, no power saving).
    fn listen_duty_cycle(
        &mut self,
        _rx_period_us: u32,
        _sleep_period_us: u32,
    ) -> Result<(), Self::Error> {
        self.listen()
    }
```

## Diff 2 — `crates/r2-sx1262/src/lib.rs` (the SX1262 override + opcode + helper)
**(2a)** add to `mod op` (after `SET_RX = 0x82`):

```rust
    pub const SET_RX_DUTY_CYCLE: u8 = 0x94;
```

**(2b)** free helper near `RX_CONTINUOUS` (top-of-file):

```rust
/// Convert a physical-unit microsecond duration to the SX126x 24-bit big-endian step count.
/// The chip's timing grid is 15.625 µs/step (= 1/64 ms), so `steps = us * 64 / 1000` is exact
/// on-grid; saturates at the 24-bit max (~262 ms) so an over-long request clamps, never wraps.
fn us_to_steps(us: u32) -> [u8; 3] {
    let steps = ((us as u64) * 64 / 1000).min(0x00FF_FFFF) as u32;
    let b = steps.to_be_bytes();
    [b[1], b[2], b[3]]
}
```

**(2c)** override in `impl LoRaRadio for Sx1262` (after `listen`, mirrors it):

```rust
    fn listen_duty_cycle(
        &mut self,
        rx_period_us: u32,
        sleep_period_us: u32,
    ) -> Result<(), Self::Error> {
        self.clear_irq_status(irq::ALL)?;
        // CORRECTNESS #1 (core-flagged): the chip auto-cycles RX↔warm-sleep INTERNALLY — the host
        // cannot toggle the RF-switch per-window. So hold the RX path (RXEN HIGH) for the WHOLE
        // duty-cycle session — set it ONCE here, never per-window. (A NoRxen no-ops; DIO2 drives the
        // in-chip antenna switch regardless.) The caller MUST NOT transmit() mid-session — that path
        // drives RXEN LOW and ends the RX front-end enable.
        let _ = self.rxen.set_high();
        let rx = us_to_steps(rx_period_us);
        let sl = us_to_steps(sleep_period_us);
        // SetRxDutyCycle(rxPeriod[3B BE], sleepPeriod[3B BE]) — 24-bit each, 15.625 µs steps (§13.1.4).
        self.cmd(op::SET_RX_DUTY_CYCLE, &[rx[0], rx[1], rx[2], sl[0], sl[1], sl[2]])
    }
```

## Correctness points (core-flagged) — how each is handled
1. **RF-switch over the auto-loop** — folded into 2c: `rxen.set_high()` ONCE before `0x94`, never
   per-window (the loop is autonomous), plus a doc-warning that `transmit()` mid-session drops RXEN.
2. **SF12-preamble vs rxPeriod sizing (the real trap)** — the primitive is SF-agnostic; sizing is a
   **caller** decision, exposed via the physical-µs `rx_period_us` param + the ★SIZING doc. Bench at
   SF7 (short preamble) first; SF12 needs a long `rx_period_us` or it sleeps through packets. The
   driver does NOT hardcode a window.

## DIO1 wake — already wired, ONE open decision for core
`configure()` already arms `irq::ALL` (incl. `RX_DONE = 1<<1`) on the DIO1 mask (lib.rs:501-506), so
**RxDone → DIO1 wake works with no change** — that is exactly the bridge's wake-and-drain signal.
**Open (core's call):** whether to ALSO arm `PreambleDetected` (`1<<5`, currently unarmed) on DIO1.
Not needed for RxDone-driven drain (we wake on a *completed* packet); only useful for waking earlier
on a preamble. My lean: **do NOT add it** for path-1 (RxDone-wake suffices, keeps the diff minimal);
revisit if bench shows we need earlier wake. Flag if you disagree.

## Notes
- No change to `Lr2021` or the `lora_transport.rs` mock — the trait default covers them (non-breaking).
- Firmware side (dfr1195-fw `lora_route_task`, off-by-default `standby` feature + MCU light-sleep) is
  authored separately in the dfr1195-fw worktree; it calls `listen_duty_cycle` + waits on DIO1. It
  becomes flashable once core accepts this diff and hive re-vendors r2-sx1262/r2-transport.
