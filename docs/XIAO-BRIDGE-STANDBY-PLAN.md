# XIAO edge-bridge power-standby — firmware plan (the heat fix)

> **Status:** PLAN for Roy scope-eyeball (Roy GO via supervisor 2026-07-10). SPEC-FIRST: the
> dc-advertise/SCF contract (§4) lands in `r2-specifications` **before** any path-1 flash.
> Report-plan-before-deep-implementation per supervisor. Verified against the LIVE `dfr1195-fw`
> worktree `8022c2e` (not just the committed patch).

## 0. Root cause (ground-truthed, live worktree)
The XIAO edge bridge (`xiaobridge` feature: ESP32-S3 + Wio-SX1262; `lora_route_task` RXes, drains
`DATA_RX` → `xiao_bridge_task` forwards to the phone over USB-Serial-JTAG) runs the SX1262 in
**continuous RX**: `listen()` issues `SetRx(RX_CONTINUOUS = 0x00FFFFFF)` (`r2-sx1262/src/lib.rs:800`).
The radio never sleeps → always-hot idle draw. **STEP-4 duty-cycle enforcement is NOT landed** —
duty-cycle is *advertised, not enforced* (`main.rs:388`, `:1288` "ENFORCED duty-cycle is STEP 4",
`:2816-18`). This is the deferred STEP 4.

## 1. Scope
The **XIAO as the pure EDGE bridge only** (D4 → phone). Mid-mesh-transit (a bridge that also relays
*between* mesh nodes) is **out of scope** for now — an edge bridge has exactly one upstream sensor
(D4) and one downstream sink (the phone), which is what makes SCF sizing tractable.

## 2. PATH 1 — SX1262 RX-duty-cycle + MCU light-sleep (fastest heat win, no phone-coupling)
The primitive path 2 builds on. Three parts:
- **(1a) Driver — add `SetRxDutyCycle` (0x94) to `r2-sx1262`** *(core-owned crate → author + hand
  core / core commits; flag spec-first N/A, it is an impl capability).* The SX1262 HW duty-cycle:
  `RX for rxPeriod → Sleep(warm) for sleepPeriod → auto-repeat`, DIO1 fires on preamble-detect /
  RxDone, the chip stays in warm-sleep between windows (µA-class vs mA continuous). Sizing invariant:
  `sleepPeriod + rxPeriod ≤ D4 preamble airtime` so the periodic RX window is guaranteed to catch a
  D4 frame's preamble (SF7/BW125 preamble ≈ a few ms; the SX1262 datasheet duty-cycle sizing math).
- **(1b) Firmware — `lora_route_task` uses RX-duty-cycle instead of continuous `listen()`**, behind
  an OFF-BY-DEFAULT `standby` feature (no per-target fork; the STEP-4 enforcement path). The
  advertised `wake_cadence`/`wake_window` (§3.2.2 knobs, already plumbed) become the *real* rxPeriod/
  sleepPeriod.
- **(1c) MCU light-sleep between DIO1 wakes** — esp-hal `light_sleep` with DIO1 GPIO wake + USB-resume
  wake. WiFi is already off on the pure edge bridge (LoRa + USB only), so light-sleep gates the CPU +
  peripherals between DIO1/USB events. (Embassy already idles the CPU on `await`; light-sleep is the
  deeper win — clock-gates peripherals.)

**Effect:** radio warm-sleeps between windows + MCU light-sleeps between DIO1/USB events → the
always-hot idle draw drops with **no phone-coupling**.

## 3. PATH 2 — phone-coupled standby (builds on path 1, separable)
- **Phone-presence hook:** phone-gone detected via (a) USB suspend (USB-Serial-JTAG suspend), (b) BLE
  disconnect (if the BLE leg is up), or (c) app-closed (app-heartbeat absence over the USB pipe) →
  drop the SX1262 duty-cycle to **Intermittent** (longer `sleepPeriod`) + deeper MCU light-sleep.
- **Wake on:** phone-reconnect (USB-resume) / its own beacon cadence (periodic wake to beacon
  presence so the phone can re-discover it) / DIO1 (a frame still arrives in the sparse window).

## 4. SPEC CONTRACT — lands FIRST (specs ratify, BLOCKS path-1 flash)
When the edge bridge sleeps, D4's frames must not be lost. The contract to pin:
- **dc-advertise (§12.6):** the duty-cycled edge bridge advertises its `duty_class` (Intermittent) +
  `wake_cadence`/`wake_window` on the HB `dc` field (already *advertised*, now *load-bearing*).
- **SCF-1 store-carry-forward (§3B.2):** the upstream sensor (D4) with frames destined **through** a
  duty-cycled next-hop bridge **MUST SCF-hold** them until the bridge's next advertised wake window;
  the SCF-hold TTL is sized by the advertised cadence (§3B.2 TTL invariant).
- **The question for specs (the net-new bit):** §3B.2 SCF today covers a sleeping *originator*
  (neighbours hold inbound for a sleeping sensor). Here the sleeping node is a **transit edge bridge**
  — D4 holds *outbound* frames for a sleeping *next hop*. Does §12.6 + §3B.2 already cover the
  duty-cycled-next-hop-bridge case, or does it need a normative statement (bridge advertises RX window
  → upstream SCF-holds destined-through frames to that window; TX-outside-window or sleep-without-
  advertise = conformance violation)? **Ratify first.**

## 5. Sequencing (report-first, then spec, then impl)
1. **This doc → supervisor/Roy** (scope-eyeball). ← now.
2. **Hand specs the §4 contract** → specs ratifies the dc-advertise/SCF-for-duty-cycled-bridge piece.
3. **Path 1** (driver 1a with core → 1b/1c firmware) once the contract is ratified; verify heat drop
   on the bench; keep the D4→XIAO→phone delivered-path green (SCF-hold honoured).
4. **Path 2** on top, separable.

*Trail: dfr1195-fw RESUME; r2-hive RESUME. Driver crate: r2-sx1262 (core-owned).*
