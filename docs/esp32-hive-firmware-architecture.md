# DFR1195 (ESP32-S3) hive firmware — host-loop / board architecture sketch

> **SoC CONFIRMED ESP32-S3 (2026-06-16):** DFR1195 = DFRobot "LoRaWAN ESP32-S3 Dev Board",
> **ESP32-S3-WROOM-1-N4** (Xtensa LX7), 4 MB flash, SX1262 — verified vs `wiki.dfrobot.com/dfr1195` +
> SKU `SKU_DFR1195_LoRaWAN_ESP32_S3`. (A brief mis-ID as "ESP32-C6" — from core's D3b skeleton conflating
> DFR1195 with the DFR1117 Beetle C6 — has been corrected.) Target = `xtensa-esp32s3-none-elf` (espup),
> `espflash --chip esp32s3`. See `docs/hardware-tn-test-critical-path.md`.

Status: **sketch for alignment** (Phase 3 Part D). Architecture ratified by Roy:
**Path B — pure no_std (esp-hal/embassy)**, under the north-star below. This is a
sketch to confirm the shape before building; not yet implemented.

## North-star (Roy)
**ONE hive codebase, everywhere.** The cloud/Linux hive, the DFR1195 firmware, the
Uno-Q hive, and the browser wasm-hive are the **same codebase** — core's no_std
crates (which implement the specs) + a **thin per-platform host/board layer**. Do
**not** fork separate hive codebases. Pure no_std is what lets one codebase span
MCU → cloud. Chain: **specs → core → hive** (specs canonical; core = no_std
spec-impl crates; hive = those crates + platform layers). composer orchestrates
fleets of hives (plugins, sentants/ensembles, OTA, proof UX) — it is **not** the hive.

## The convergence refactor (precondition, incremental)
r2-hive today is a std/Linux monolith (tokio + axum + libc). To realize the
north-star, split it:

```
r2-hive-core   (no_std + alloc; platform-agnostic)
  - the host event loop, routing glue over RouteEngine, dispatch wiring,
    connection/peer lifecycle — expressed against a Platform trait, no tokio/axum.
Platform trait (the seam)
  - async transports (send/recv frames), timer/clock (now), storage (OTA/identity),
    rng, optional display + input. Each platform implements it.
platform layers (thin):
  - r2-hive-linux   : tokio + axum + std sockets  (today's hive, refactored onto the trait)
  - r2-hive-esp32   : esp-hal + embassy + board drivers  (THIS firmware)
  - r2-hive-unoq    : Linux-capable MPU (reuses r2-hive-linux mostly)
  - r2-hive-wasm    : browser runtime + WebSocket
```
First concrete step (unblocked, no radios): extract the **Platform trait** and move
the host loop behind it, with the existing Linux code as the first impl. That alone
makes the codebase multi-target-shaped and is verifiable on Linux.

## DFR1195 board layer (hive owns)
Target: ESP32-S3 (xtensa) + SX1262 LoRa + 0.96" LCD + button IO18.
- **Runtime:** `esp-hal` (no_std HAL) + `embassy` (async executor) + `esp-alloc`
  (heap for the alloc tier core crates need).
- **Radios:** WiFi via `esp-wifi` + `embassy-net` (smoltcp); BLE via **`trouble`**
  (no_std BLE — aligns with core's `platforms/trouble-test`); LoRa via an
  `embedded-hal` SX1262 driver (`lora-rs`/`sx126x`) over SPI.
- **Peripherals:** LCD via `embedded-graphics` (SSD1306/ST77xx over I2C/SPI) —
  shows delivery state (human-visible pass/fail, Part D4); button IO18 via GPIO
  interrupt — triggers a test event.
- **Storage/OTA:** `esp-storage` + esp-hal OTA partition API; no_std OTA receiver
  over `embassy-net` TCP (protocol reference = workshop/core `ota_tcp.rs`, which is
  std — reference only).

## On-device host loop (hive owns, built ON core — no core logic reimplemented)

**Transport interface = R2-TRANSPORT *sync* (NOT async r2-discovery §4).** Per
R2-DISCOVERY §5 the no_std tier uses the R2-TRANSPORT **synchronous** transport
interface directly; the async `r2-discovery` §4 bindings are alloc/std (laptop/wasm
host) only. So the firmware host loop is a **sync** poll/dispatch loop over
RouteEngine, consuming core's no_std **sync** radio drivers — not async transport tasks.

embassy provides the executor + timers for the **board/peripheral** layer (LCD,
button, SPI, OTA socket); the transport/route path is the R2-TRANSPORT sync call path:
1. **Poll inbound** — for each radio (WiFi-UDP, BLE5, SX1262-LoRa), call the core
   no_std **sync** transport `recv` (D3b drivers) → feed `RouteEngine`.
2. **Route/forward** — `RouteEngine` (core, already no_std) decides forwarding over the
   neighbour table; outbound via the sync transport `send` (multi-transport fallback).
3. **LCD task** — render delivery/route state.
4. **Button task** — inject a test event on IO18.

## Dependencies / coordination
- **core D3 — two tracks:**
  - **D3a** (async `udp_lan`/`tcp` r2-discovery §4 bindings) — alloc/std, core's
    **Linux-verifiable** track for the laptop/wasm host. NOT used by this firmware.
  - **D3b** (no_std **sync** radio drivers: WiFi-UDP / BLE5 / SX1262-LoRa, on
    `platforms/esp32`, against the R2-TRANSPORT sync interface) — **HARD blocker for
    radios.** core **authors** D3b; **I own hardware-in-the-loop VALIDATION** — flash
    on a real DFR1195, verify, feed defects back to core (the spec→core→hive radio
    chain ends at my bench). I coordinate the **sync binding surface** closely with core;
    host loop + board layer scaffold against it now with stub transports.
- **composer:** owns the OTA **push** wire (POST /api/ota/<addr> pattern) + carrier-board
  profile + test sentants/ensemble; I own the no_std OTA **receiver**.
- **specs:** hardware test definitions (separate tier).

## Reference: workshop's firmware (std, NOT portable under Path B)
workshop's `firmware/esp32-s3` is **esp-idf-svc 0.51 / STD+alloc** (ESP-IDF v5.2.5,
rust channel=esp, OS threads — **not** no_std/esp-hal/embassy). Its `r2-esp` crate
(beacon, l2cap PSM 0xD2, wifi_sta/wifi_prov, **device-side `ota_tcp`**, data/log/reset
TCP servers, NVS Ed25519 identity, LED) is a **working ESP32-S3 WiFi+BLE+OTA substrate**
— but **std, so reference-only under Path B no_std** (learn the BLE/WiFi/OTA *flow* +
the build/partition/OTA *patterns*, reimplement on esp-hal/esp-wifi/`trouble`/
embassy-net). Reusable as **patterns**: `tools/{setup,build}-firmware.sh` (espflash
`save-image`, partitions.csv, `.meta.json`, 2-slot OTA), the per-carrier dir layout
`firmware/<soc>/<carrier>/`, and the ADR-001/003 carrier-add process. workshop nodes
are **leaf emitters** (no on-device routing): RouteEngine + neighbour table +
multi-transport forwarding is **net-new** for a hive. No LoRa anywhere in the fleet —
**SX1262 is greenfield.**

## Reality flags (must resolve)
1. **Ensemble hosting is not MCU-ready.** `r2-def`/`r2-ensemble`/`r2-dispatch` are
   **std-tier** in r2-core (excluded from the no_std build matrix during the
   consolidation). So the initial DFR1195 hive is **routing + transport** (RouteEngine
   over real radios) — *not* full sentant/ensemble hosting on-device. Running ensembles
   on the MCU needs those crates no_std/alloc-tiered (core + spec work) — flag for the
   roadmap.
2. **std references only.** workshop's `firmware/esp32-s3` (xtensa-esp32s3-espidf,
   espflash, per-carrier devkitc/xiao, device-side `r2-esp::ota_tcp`, `/api/ota` push)
   and core's `platforms/esp32` demo are **ESP-IDF/std** — learn the BLE/WiFi/OTA flow
   + build/partition/OTA patterns, but the portable asset is core's no_std crates.
3. **Refactor scope.** Converging the existing std hive onto the Platform trait is
   real work; do it incrementally (extract the trait first, Linux as first impl).

## Cross-repo contracts (Phase 3 Part D — agreed)
- **Scope:** routing-only near-term MCU hive ACK'd (supervisor-provisional, pending Roy) —
  RouteEngine + transport (relay/dedup/TTL/spray/partition/heal); events originate/terminate
  at full hives (laptop/wasm) or firmware test hooks (IO18 inject / LCD display). On-device
  ensemble hosting is a later roadmap item (needs r2-def/ensemble/dispatch re-tiered no_std).
- **OTA reply-status contract** (composer, `r2-composer/specifications/OTA-REPLY-STATUS-CONTRACT.md`)
  — my embassy-net no_std receiver MUST emit: reply = `[status:u8][msg_len:u16 LE][msg:utf8]`;
  `0x00`=SUCCESS(`OK`) only after sha256-match + write-inactive-slot + set-boot ok, then reboot
  (~2s); `0x01`=ERROR, msg = `<CODE>[ detail]`, CODE ∈ {PREAMBLE, TOO_BIG, BAD_MAGIC,
  SHA_MISMATCH, WRITE_FAIL, NO_SLOT, SHORT}. Status bytes stay 0x00/0x01 (R2-UPDATE RESP_OK/ERR);
  CODE rides in msg. **HW: DFR1195 = ESP32-S3-WROOM-1-N4 = 4 MB flash** (NOT 8 MB like devkitc) →
  ~1.5 MB OTA slots → TOO_BIG bound-checks against that BEFORE writing.
- **SX1262 LoRa sync driver:** composer drafts the sync trait **against core's R2-TRANSPORT sync
  interface** and sends to core (not a parallel composer trait); **I own the board SPI wiring /
  bus-share** (watch LCD+SX1262 shared-SPI contention).
- **Sync host-loop↔driver seam:** being co-defined with core now (R2-TRANSPORT sync `send` exists;
  inbound `poll_recv` is the gap) — scaffold the host loop against the agreed seam; core authors
  D3b drivers; I hardware-validate.

## Phasing
- **P0 (now, unblocked):** extract the Platform trait + host-loop split (Linux first);
  scaffold `r2-hive-esp32` (esp-hal + embassy boot); LCD + button drivers (no core dep);
  host-loop skeleton with RouteEngine wired + stubbed transports.
- **P1 (gated on core D3):** wire real BLE/WiFi/LoRa via core's no_std bindings.
- **P2:** no_std OTA receiver; LCD delivery display; button test trigger.
- **P3:** integrate composer ensemble + DFR1195 carrier profile + specs hw tests.
