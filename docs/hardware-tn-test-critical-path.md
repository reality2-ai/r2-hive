# Hardware TN Test — critical path

**Goal (Roy, #1 priority):** prove transient networking on **physical radios**, not just sim.
**Milestone (definition of done):** **two DFR1195 boards exchange ONE routed R2-WIRE frame over a real
radio**, with the route engine making the forward decision *on-device*.
**Owner:** hive (lead track; BOS-on-R2 paused). **Co-leads:** core (D3b radio drivers — its new top
priority), workshop (no_std build/flash path), composer (OTA/flash, radio trait).

---

## 0. SoC RESOLVED — and a correction to flag

**DFR1195 = DFRobot FireBeetle 2 ESP32-C6 (RISC-V).** Authoritative: core's D3b skeleton
`r2-core/platforms/dfr1195` (Cargo `esp32c6`, README "FireBeetle 2 ESP32-C6 (RISC-V)", target
`riscv32imac-unknown-none-elf`, `espflash --chip esp32c6`).

- **Two stale docs to fix** (carried a wrong "ESP32-S3-WROOM-1-N4" from the devkit): hive's
  `docs/esp32-hive-firmware-architecture.md` (says S3/xtensa) and **composer's OTA-REPLY-STATUS-CONTRACT
  flash bound** (assumed S3-WROOM-1-N4 4 MB → ~1.5 MB OTA slot). **C6 flash size + partition layout must be
  re-confirmed** for the TOO_BIG bound (FireBeetle 2 C6 is typically 4 MB, so the bound likely holds — but
  verify against the actual part before OTA).
- **Good news on toolchain:** C6 is RISC-V → **stock Rust nightly + `build-std`**, NOT the espup Xtensa
  channel. Simpler/faster to stand up than an S3 would have been.

---

## 1. Current state (have)

- **hive-core: 5 no_std seams** — `sync_host` (incl. `route_inbound_sync`: parse R2-WIRE → ingest neighbour
  → plan_forward → execute over a `SyncTransport`), `platform` (clock/RNG), `transport_seam`, `identity`,
  `ota`. The portable routing + identity + OTA logic, Linux-verified (31 core tests).
- **core D3b skeleton** (`r2-core/platforms/dfr1195`, `f9c9fde`): Path B no_std esp-hal/embassy for C6.
  `wifi.rs` implements the sync `Transport`; `peers.rs` host-centralised resolution (4/4 unit tests);
  esp-hal/esp-wifi/embassy version-pinned; **`HIVE:` markers** at every board-confirmation point
  (esp-hal::init shape, WiFi STA assoc, embassy-net Stack build, BLE bringup, SX1262 SPI pins). `send()`
  on BLE/LoRa intentionally **stubbed** (sync→async bridge left for hive). LoRa generic over composer's
  chip-agnostic radio trait (`lora.rs`, updated 2026-06-16: physical-unit `LoRaConfig`, `poll()`+`read()`).
- **Transports Linux-verified** — UDP-LAN round-trip + router parse/dedup integration tests (this session).
- **OTA receiver** byte-confirmed with composer (not on the first-light path; needed for field updates).

## 2. Shortest path to the milestone

Deliberately **WiFi-UDP first** (core's `wifi.rs` sync Transport exists; stock toolchain) — it proves the
*on-device routing stack + a real radio transmit* with the least new code. LoRa (the true
infrastructure-less TN proof) is the immediate follow-on.

**Stage A — first light (validation, laptop ↔ 1 board):**
1. Confirm board in hand + install toolchain: `rustup target add riscv32imac-unknown-none-elf` (nightly),
   `espflash`. *(Roy: board + USB; me: toolchain.)*
2. Build core's `platforms/dfr1195` skeleton standalone on my toolchain; resolve the `HIVE:` init points
   against the metal — esp-hal::init, WiFi STA assoc, embassy-net Stack. *(me + core.)*
3. Wire the **hive host loop**: `route_inbound_sync` (RouteEngine) + `wifi.rs` sync Transport + the
   writer-task spawn + RX poll (the sync→async bridge core left for me). *(me.)*
4. Flash one board; it joins Roy's WiFi; **laptop (my tested UDP-LAN) ↔ board** exchange a routed
   R2-WIRE frame. Proves the whole on-device stack against a known-good peer.

**Stage B — THE milestone (board ↔ board over real radio):**
5. Flash a **second** board; both join the same WiFi (or AP-mode).
6. Inject one R2-WIRE frame on board A via a firmware test hook (IO18 button / serial), addressed to B →
   A's route engine plans forward → `wifi.rs` send → B receives → B's engine delivers (LCD or serial). ✅

**Stage C — true transient-networking proof (LoRa, follow-on):** repeat Stage B over **SX1262 LoRa**
(infrastructure-less, the real TN medium) — needs core's LoRa driver un-stubbed + composer's radio trait +
SPI wiring + antennas/region. This is the longest pole (greenfield radio, supervisor on record).

## 3. Gaps (need) — with owner

| # | Gap | Owner | Blocks |
|---|---|---|---|
| 1 | esp-hal/embassy **board crate** = host loop wrapping RouteEngine + sync Transport; board init (`HIVE:` points); pin map (SX1262 SPI/CS/DIO1/BUSY/RST, LCD, IO18) | **hive** (on core's skeleton) | A,B |
| 2 | sync→async **bridge**: writer-task spawn + RX poll feeding `route_inbound_sync` | **hive** | A,B |
| 3 | core's **wifi.rs** HW-validated (sync Transport on real C6) | core + hive (validate) | A,B |
| 4 | sync seam **landed in r2-transport** (`poll_recv` default-None + `TransportAddr`/`InboundFrame`) — today it's hive's transitional mirror in `sync_host`; mirror works for the milestone, but co-define/land cleanly | core + hive | clean-up (not blocking) |
| 5 | core's **LoRa/BLE `send()`** un-stubbed (sync→async over real radio tasks) | core + hive | C only |
| 6 | **Toolchain** riscv32imac nightly + build-std + espflash on the dev machine | hive (Roy: install perm) | A |
| 7 | **Flash + monitor** path (`espflash --chip esp32c6`); per-board identity (NVS) | hive; workshop (patterns) | A,B |
| 8 | **OTA** receiver on-device (field updates) | hive + composer | deferred (not milestone) |

## 4. What ROY must physically provide (the crux)

- **2× DFRobot DFR1195 (FireBeetle 2 ESP32-C6) boards** + USB cables. *(One unblocks Stage A; two for the milestone.)*
- **Confirm the exact part / flash size** (resolves the OTA TOO_BIG bound; C6 FireBeetle 2 ≈ 4 MB — verify).
- **A WiFi network** (2.4 GHz; SSID + password) the boards + laptop can all join — OR confirm AP-mode on a board is acceptable.
- **Permission to install** the riscv32 toolchain on the dev machine (stock nightly, no xtensa needed).
- **(Stage C / LoRa)** SX1262 antennas + the region (AS923 for NZ / US915 / EU868) for legal LoRa TX.
- **(Optional)** confirm the 0.96" LCD + IO18 button are populated, or accept **serial-only** for the milestone display/inject hooks.

## 5. Coordination

- **core** — D3b real radio drivers (its new top priority): HW-validate `wifi.rs` for the milestone;
  un-stub LoRa/BLE for Stage C; **EXTEND r2-transport with the sync seam** so hive drops the mirror.
- **workshop** — no_std build/flash path + the **tuxedo** dev machine; reuse their
  `setup-firmware.sh`/`build-firmware.sh` + partition patterns (their path is std esp-idf S3 = reference,
  but the build/flash mechanics transfer). Confirm whether they have a C6 build path or only S3.
- **composer** — OTA push + flash/carrier profile (Stage C+, not the first milestone) and the chip-agnostic
  radio trait (already in `lora.rs`); re-confirm the C6 flash/OTA-slot bound.

## 6. Risks / unknowns

- **Hardware in hand** — nothing flashes until boards + WiFi exist (Roy). Stage A needs 1 board; milestone needs 2.
- **Path B greenfield radio** (on record) — WiFi-UDP sidesteps it for first light; LoRa (Stage C) is the longest pole.
- **`HIVE:` init points** — esp-hal/esp-wifi/embassy version pins vs my installed toolchain may need
  reconciliation on first build (expected; that's what the markers are for).
- **WiFi-UDP ≠ infrastructure-less TN** — Stages A/B prove the on-device routing stack + real radio TX, but
  the *transient* (mobile, lossy, no-infra) property is only truly proven at Stage C (LoRa). Be explicit
  with Roy that the milestone is "routed frame over real radio"; full TN-on-radio = Stage C.
- **Bench time** — this is hardware-in-the-loop; iteration is slower than Linux CI.

---

**Bottom line:** the software is ready to meet the metal — hive-core's routing/identity seams are
Linux-verified and core's C6 D3b skeleton + `wifi.rs` sync Transport exist. The critical path to the
milestone is **(a) Roy provides 2× DFR1195 + WiFi, (b) I install the riscv32 toolchain, build core's
skeleton, resolve the `HIVE:` init points, and wire the host loop + sync→async bridge, (c) flash two
boards and route one frame board↔board over WiFi-UDP.** LoRa (true infrastructure-less TN) is the
follow-on. **The single hard blocker is physical: boards + network in hand.**
