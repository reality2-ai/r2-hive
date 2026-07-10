# r2-sx1262 `SetRxDutyCycle` (0x94) — driver diff PROPOSAL (hive → core review-accept)

> **Status:** PROPOSAL for **core** (r2-sx1262 + r2-transport are CORE-OWNED — hive authors, core
> accepts/adjusts + commits). Enables the XIAO edge-bridge heat-fix path-1 (R2-RUNTIME §3.2.6
> ratified @4072063; Roy scope-eyeball CONFIRMED; supervisor GREEN). Matches core's pre-flagged
> driver shape (fleet, this turn) and folds its **two correctness points**. Physical-µs params keep
> the chip-agnostic seam intact and let firmware size per-SF.
>
> **TWO core-owned diffs (both required for path-1):** Diff 1/2 = the `listen_duty_cycle` **primitive**
> (r2-transport seam + r2-sx1262 `0x94`); Diff 3 = the **LoRaTransport RX-arming MODE** (because
> `LoRaTransport::service` OWNS RX arming — the firmware can't duty-cycle by itself). See Diff 3's
> architectural-finding note.

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
**RULED (core, agreed):** do NOT arm `PreambleDetected` — the SX126x intermediate-RX mode receives a
detected packet to completion before resuming the duty-cycle loop, so RxDone is the correct + sufficient
wake-and-drain signal; preamble-wake would wake the host pre-packet for zero drain benefit + burn
wake-power (defeats the heat-fix). Minimal diff wins.
**⚠ CORRECTION (core, for the record — no code impact, not armed):** my prose said `1<<5`; that is WRONG
— on the SX1262 `PreambleDetected` is **BIT 2 (`1<<2` = 0x04)**; `1<<5` (0x20) is `HeaderErr`. The driver
has no `PreambleDetected` const today (`irq::` = TX_DONE `1<<0` / RX_DONE `1<<1` / CRC_ERR `1<<6` /
TIMEOUT `1<<9`). A future arming would add `PREAMBLE_DETECTED = 1<<2`, not `1<<5`.

## Diff 3 — `crates/r2-transport/src/lora_transport.rs` (LoRaTransport RX-arming MODE) — REQUIRED, ⚠ NOT YET LANDED
> **STATUS (2026-07-10):** Diff 1/2 **LANDED @1bbb32b** (core; verified in tree: `listen_duty_cycle`
> default in lora.rs:67, `SET_RX_DUTY_CYCLE=0x94` + `us_to_steps` + Sx1262 override in r2-sx1262).
> **Diff 3 is NOT in 1bbb32b** — `lora_transport.rs` still re-issues plain `radio.listen()`; no
> `set_rx_standby`/`rx_duty`/`arm_rx`. The firmware (dfr1195-fw 810573e) calls `lora.set_rx_standby(..)`,
> so it will NOT compile until Diff 3 lands. **Re-vendor HELD on Diff 3.** (Core landed from the first
> handoff, 733d82d — the Diff 3 follow-up c7fc7a8 was queued while core was busy; re-flagged.)

**Architectural finding (evidence):** `LoRaTransport` OWNS the radio (moved in at `new`) and OWNS RX
arming — it re-issues continuous `radio.listen()` at THREE sites: `new:61`, TxDone re-listen `:154`,
RxTimeout/CrcErr re-listen `:166`. So the firmware **cannot** duty-cycle RX by itself once the radio
is inside the transport — path-1 needs a duty-cycle MODE here. This is the clean seam (keeps TX
airtime-gating intact); the alternative (firmware bypasses `service` for RX) splits radio ownership
and is worse. **This is a SECOND core-owned change, on top of Diff 1/2.**

**(3a)** add a field to `struct LoRaTransport` + init in `new`:
```rust
    /// RX arming policy: `None` = continuous `listen()` (default = today's behaviour);
    /// `Some((rx_us, sl_us))` = SetRxDutyCycle standby (R2-RUNTIME §3.2.6 pure-edge-bridge
    /// standby). Set by the firmware on sink-ABSENT; cleared on sink-PRESENT.
    rx_duty: Cell<Option<(u32, u32)>>,
```
`new`: add `rx_duty: Cell::new(None),` to the struct literal (keeps `new`'s `radio.listen()` as-is —
a fresh node starts continuous; the firmware opts into standby after).

**(3b)** one private helper + route all re-arm sites through it:
```rust
    /// Arm RX per the current policy — continuous or duty-cycled. Every re-listen site
    /// routes through here so the mode is honoured uniformly. Takes `&mut R` (the caller
    /// already holds the radio borrow) to avoid a double-borrow.
    fn arm_rx(&self, radio: &mut R) -> Result<(), R::Error> {
        match self.rx_duty.get() {
            Some((rx, sl)) => radio.listen_duty_cycle(rx, sl),
            None => radio.listen(),
        }
    }
```
Replace the two in-`service` re-arm calls (`:154`, `:166`) `let _ = radio.listen();` →
`let _ = self.arm_rx(&mut radio);`. (Leave `new:61` continuous.)

**(3c)** two public setters the firmware calls on sink-presence transitions (re-arm immediately):
```rust
    /// Switch RX to duty-cycled standby (R2-RUNTIME §3.2.6): the radio autonomously cycles
    /// `rx_period_us` ↔ warm-sleep `sleep_period_us`, waking on DIO1/RxDone. Re-arms now.
    /// Call on sink-ABSENT. `rx_period_us` MUST be ≥ the per-SF preamble window; note the
    /// SX1262 `sleepPeriod` caps at 24-bit × 15.625 µs ≈ 262 ms, so radio-duty-cycle alone
    /// gives a sub-second wake cadence (the R2-RUNTIME §3.2.6 `scf_ttl_s` sizing bound binds
    /// path-2's DEEP MCU light-sleep, not this radio primitive).
    pub fn set_rx_standby(&self, rx_period_us: u32, sleep_period_us: u32) -> Result<(), R::Error> {
        self.rx_duty.set(Some((rx_period_us, sleep_period_us)));
        self.arm_rx(&mut self.radio.borrow_mut())
    }
    /// Switch RX back to continuous listen (sink-PRESENT / AlwaysOn). Re-arms now.
    pub fn set_rx_continuous(&self) -> Result<(), R::Error> {
        self.rx_duty.set(None);
        self.arm_rx(&mut self.radio.borrow_mut())
    }
```
Non-breaking: default `None` = exactly today's continuous behaviour; TX still `standby()`+`transmit()`
then TxDone re-arms via `arm_rx` back into duty-cycle. (`Cell`/`RefCell` already imported.)

## Firmware side (hive-owned, dfr1195-fw — authored after core accepts + re-vendor)
- **`RxenRadio` MUST override `listen_duty_cycle`** (rxen HIGH + `inner.listen_duty_cycle`) — else it
  inherits the trait default (`self.listen()` = continuous) and the mode is a no-op. (main.rs)
- **`lora_route_task`**, off-by-default `standby` feature: after `LoRaTransport::new`, call
  `lora.set_rx_standby(rx_us, sl_us)` (path-1: unconditional; path-2: on sink-absent, `set_rx_continuous`
  on sink-present) + MCU `light_sleep` between DIO1/USB wakes.
- Sizing (SF7 bench first): `rx_us` ≥ SF7 preamble window (SF7 symbol ≈ 1.02 ms); `sl_us` up to the
  ~262 ms cap. Re-size for SF12 before any field use.

## Notes
- No change to `Lr2021` or the `lora_transport.rs` mock — the Diff-1 trait default covers them.
- Two core-owned diffs total: Diff 1/2 (the `listen_duty_cycle` primitive) + Diff 3 (the LoRaTransport
  mode). Firmware becomes flashable once core accepts BOTH and hive re-vendors r2-sx1262 + r2-transport.
