# XIAO edge-bridge power-standby — firmware plan (the heat fix)

> **Status: ✅ PATH-1 FLASHABLE + STAGED on alfred — Roy-OPERATOR-gated (not scope-gated).** All gates
> cleared: spec ratified (R2-RUNTIME §3.2.6 @4072063), Roy scope CONFIRMED, core landed both driver diffs
> (@8508309), surgically re-vendored (dfr1195-fw bd67669), both `cargo +esp check` green. **Staged:**
> `alfred:~/xiao-standby-04ce0049.elf` (+ standby-off fallback `xiao-fieldfallback-a6114724.elf`).
> **SCOPE NOTE: the staged image is RADIO RX-duty-cycle ONLY — MCU light-sleep (1c) is DEFERRED**
> (documented TODO; radio duty-cycle alone is the fastest heat win). **SF7 miss-rate gate (metal):**
> a high SF7 3ms/5ms miss-rate → ask CORE for a canonical `preamble_len` profile/KAT bump (core-owned,
> §2 path-(b)), NOT a local firmware tweak. Remaining = Roy flashes (XIAO not on alfred yet) + bench.

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
The primitive path 2 builds on. **ARCHITECTURAL FINDING (2026-07-10, code-verified):** path-1 needs
**TWO core-owned diffs**, not one — because `LoRaTransport` OWNS the radio + RX arming
(`service()` re-issues continuous `listen()` at `new:61`/TxDone`:154`/RxTimeout`:166`), so firmware
cannot duty-cycle RX by itself. Both authored as review-ready diffs in
**docs/SX1262-SETRXDUTYCYCLE-DIFF-PROPOSAL.md** (handed core): Diff 1/2 = the `listen_duty_cycle`
primitive; **Diff 3 = a duty-cycle MODE on `LoRaTransport`** (`rx_duty` policy None=continuous default
+ `set_rx_standby`/`set_rx_continuous`). Three firmware parts on top:
- **(1a) Driver — add `SetRxDutyCycle` (0x94) to `r2-sx1262`** *(core-owned crate → author + hand
  core / core commits; flag spec-first N/A, it is an impl capability).* The SX1262 HW duty-cycle:
  `RX for rxPeriod → Sleep(warm) for sleepPeriod → auto-repeat`, DIO1 fires on **RxDone**
  (PreambleDetected is NOT armed — the core impl arms RxDone only, and RxDone-wake is sufficient); the
  chip stays in warm-sleep between windows (µA-class vs mA continuous). Sizing invariant:
  `sleepPeriod + rxPeriod ≤ D4 preamble airtime` so the periodic RX window is guaranteed to catch a
  D4 frame's preamble (SF7/BW125 preamble ≈ a few ms; the SX1262 datasheet duty-cycle sizing math).
- **(1b) Firmware — `lora_route_task` uses RX-duty-cycle instead of continuous `listen()`**, behind
  an OFF-BY-DEFAULT `standby` feature (no per-target fork; the STEP-4 enforcement path). The
  **local** `wake_cadence`/`wake_window` (§3.2.2 *config* knobs — LOCAL power policy, **NOT** wire
  fields; specs ruled `dc` is class-only, R2-ROUTE §3B.1) become the *real* rxPeriod/sleepPeriod.
- **(1c) MCU light-sleep between DIO1 wakes** — esp-hal `light_sleep` with DIO1 GPIO wake + USB-resume
  wake. WiFi is already off on the pure edge bridge (LoRa + USB only), so light-sleep gates the CPU +
  peripherals between DIO1/USB events. (Embassy already idles the CPU on `await`; light-sleep is the
  deeper win — clock-gates peripherals.)

**Effect:** radio warm-sleeps between windows + MCU light-sleeps between DIO1/USB events → the
always-hot idle draw drops with **no phone-coupling**.

### Bench sizing — core-verified (correctness-#2 lane), READ BEFORE the metal bench
**Detection rule (governs every SetRxDutyCycle window):** `(rxPeriod + sleepPeriod) ≤ TX preamble
length` — so the preamble always spans ≥1 full RX window (else it can fall entirely in a sleep gap =
a HARD miss). Current firmware = **3 ms rx / 5 ms sleep = 8 ms cycle**; SF7/BW125 8-symbol preamble =
**8.19 ms** ⇒ cycle < preamble ✓.
- **⚠ SF7 marginal knob:** rxPeriod 3 ms ≈ 2.9 SF7 symbols of worst-case detect overlap. If the
  SX1262 preamble-detect threshold needs **>3 symbols**, that is exactly where a non-trivial SF7
  miss-rate comes from. **Bench 3/5 + 8-sym preamble FIRST** (the aggressive low-power config — the
  heat-fix goal); measure miss-rate, then adjust on data (don't preemptively soften).
- **On a high miss-rate, two knobs (core's lean = (b) FIRST):** (a) bump rxPeriod → more detect
  margin but costs idle power (erodes the heat-fix); **(b) lengthen TX-side preamble to 12–16 symbols**
  → costs a little airtime, widens the window for the `cycle ≤ preamble` rule, and helps EVERY
  receiver + SF12. Prefer (b) before touching rxPeriod.
  - **⚠ (b) is a CORE-OWNED canonical profile change, NOT a firmware-local edit** (core clarified).
    `LoRaConfig.preamble_len: u16` is a clean config field applied to BOTH TX (SetPacketParams @
    configure) and RX-rearm, so TX/RX stay consistent per-node by construction — but it's part of the
    KAT-locked profile (`as923_nz` / benchsf7), the anti-mutual-deafness single-source-of-truth. A
    per-node override would risk ASYMMETRIC mesh deafness (short-preamble TX vs long-preamble RX
    detector = the exact drift the profile-lock prevents). **Workflow:** bench miss-rate high → I ping
    core → **core** lands the `preamble_len` bump on the canonical profile + EXTENDS the profile-lock
    KAT to also assert `preamble_len` (it does NOT today ⇒ a preamble drift would be silent; core is
    closing that gap) → **I re-vendor** the bumped profile. Rollout MUST be uniform (mixed longer-RX /
    shorter-TX is asymmetric), so it's one coordinated core land re-vendored everywhere.
- **★ SF12 re-size (do NOT carry 3/5 to SF12):** SF12/BW125 8-symbol preamble = **262.144 ms** — that
  is the *detection budget* (`rxPeriod + sleepPeriod ≤ 262 ms` so the periodic window always spans a
  preamble). **⚠ CORRECTED 2026-07-11 (supervisor-codex, math-verified):** this 262 ms is the PREAMBLE
  airtime, NOT the hardware sleep cap. The SX1262 `SetRxDutyCycle` 24-bit `sleepPeriod` hardware max =
  `0xFFFFFF × 15.625 µs` = **262.144 SECONDS** — 1000× the SF12 preamble, so it is **NOT the binding
  constraint**; the prior text conflated the two (262 ms preamble vs 262 s cap share the digits, differ
  ×1000). **Consequence:** SF12 ALLOWS a much LONGER duty cycle than SF7 (≈262 ms `rx+sleep` budget vs
  SF7's tight 8.19 ms), so SF12 is *more* power-favorable — carrying SF7's 3ms/5ms to SF12 WASTES that
  headroom (an ~8 ms cycle where ~250 ms is allowed), it is not blocked by any cap. **Re-size SF12 from
  the preamble/detection-margin + SCF/power policy** (e.g. rx≈8 ms / sleep≈250 ms), not from the
  hardware cap. SF12 is field canon; benchsf7 SF7 is bench-only. See [[setrxdutycycle-preamble-sizing]],
  [[extended-frames-dont-fit-sf12]].

## 3. PATH 2 — phone-coupled standby (builds on path 1, separable)
- **Phone-presence hook:** phone-gone detected via (a) USB suspend (USB-Serial-JTAG suspend), (b) BLE
  disconnect (if the BLE leg is up), or (c) app-closed (app-heartbeat absence over the USB pipe) →
  drop the SX1262 duty-cycle to **Intermittent** (longer `sleepPeriod`) + deeper MCU light-sleep.
- **Wake on:** phone-reconnect (USB-resume) / its own beacon cadence (periodic wake to beacon
  presence so the phone can re-discover it) / DIO1 (a frame still arrives in the sparse window).

## 4. SPEC CONTRACT — RATIFIED (specs, R2-RUNTIME v0.25 §3.2.6 @4072063, merged main)
When the edge bridge sleeps, D4's frames must not be lost. **RESOLVED — no new wire:**
- **dc-advertise (§12.6):** the duty-cycled edge bridge advertises **only its `duty_class`
  (Intermittent, key `dc`=1 value 2)** — `dc` is **class-only** (specs held the ratified §3B.1 ruling
  `R2-ROUTE.md:693`: "`dc` is class-only, no cadence/period; §12.6 needs no `dc` period field").
  `wake_cadence`/`wake_window` are **LOCAL config, NOT wire fields** (my draft's "on the dc field"
  wording was rejected + is corrected here).
- **SCF (reuse §3B.1, UNCHANGED):** D4 is XIAO's **direct neighbour**, so §3B.1 hop-by-hop custody
  ("cannot currently forward toward destination → buffer", push-on-wake flush) already covers holding
  destined-**through** frames for a sleeping next-hop. specs confirmed §12.6/§3B.1 UNCHANGED (no new
  §3B.x route rule needed — the carve-out lives entirely in R2-RUNTIME §3.2.6).
- **Sizing invariant (specs-refined):** the standby `wake_cadence` MUST be **shorter than the UPSTREAM
  buffering node's `scf_ttl_s`** (§3.2.2 policy, F2-default **120 s**) — **NOT the literal 120 s**.
  Reason: `scf_ttl_s` is a deployment-tunable knob on the *upstream* node; a bridge sleeping 90 s
  satisfies "<120 s" but silently drops if an operator set upstream `scf_ttl_s`=60 s. The *relationship*
  is always correct; 120 s is only the field-proven default.
- **Carve-out landed (§3.2.6):** bridge=AlwaysOn default; PURE EDGE BRIDGE (sole downstream = a
  presence-driven sink) MAY standby → advertise Intermittent. **Discriminator invariant:** standby
  legal IFF no dependent downstream expects it awake. **Transit bridge** (LoRa island) MUST stay
  AlwaysOn — Intermittent-transit = conformance violation (discriminator = *topology*, not hardware).

## 5. Sequencing (report-first, then spec, then impl)
1. ✅ **This doc → supervisor/Roy** (scope-eyeball). — reported.
2. ✅ **Hand specs the §4 contract** → **RATIFIED** R2-RUNTIME v0.25 §3.2.6 @4072063 (merged main),
   no new wire, sizing invariant refined (cadence < upstream `scf_ttl_s`).
3. ✅ **Path 1 (driver 1a + firmware 1b) LANDED + STAGED** — core landed all 3 duty-cycle diffs
   (`SetRxDutyCycle` 0x94 + `listen_duty_cycle` seam + `LoRaTransport` RX-arming mode) @core `8508309`;
   hive surgically re-vendored r2-sx1262 + r2-transport into dfr1195-fw (`bd67669`+`4af9b97`, byte-
   identical) + committed the off-by-default `standby` firmware arm (`810573e`); both
   `cargo +esp check --features xiaobridge` (off) AND `xiaobridge,standby` (on) green; standby ELF
   BUILT + STAGED on alfred (stage-only, NEVER flash — Roy-only). **REMAINING: Roy-gated bench flash**
   → idle-draw/heat + duty-engages measurement + keep the D4→XIAO→phone delivered-path green
   (SCF-hold honoured) → path-1 closes. 1c (MCU light-sleep) = documented TODO (untestable off-metal).
4. **Path 2** on top, separable.

*Trail: dfr1195-fw RESUME; r2-hive RESUME. Driver crate: r2-sx1262 (core-owned).*
