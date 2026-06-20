# Hardware TN Test — critical path

**Goal (Roy, #1 priority):** prove transient networking on **physical radios**, not just sim.
**Milestone (definition of done):** **two DFR1195 boards exchange ONE routed R2-WIRE frame over a real
radio**, *and* the first-flashed image already carries a working **OTA receiver + 2-slot partition table**
so every subsequent update is wireless (Roy standing requirement — see §7).
**Owner:** hive (lead track; BOS-on-R2 paused). **Co-leads:** core (D3b radio drivers), workshop (S3
build/flash path + reference firmware), composer (OTA push / reply-status / board profile).

---

## 0. SoC — CONFIRMED ESP32-S3 (a prior C6 error, now corrected)

**DFR1195 = DFRobot "LoRaWAN ESP32-S3 Dev Board" — ESP32-S3-WROOM-1-N4 (Xtensa LX7), 4 MB flash, SX1262
LoRa.** Verified against the DFRobot wiki (`wiki.dfrobot.com/dfr1195`), the product page
(`dfrobot.com/product-2933.html`), and the SKU name `SKU_DFR1195_LoRaWAN_ESP32_S3`.

- **Correction:** an earlier version of this doc said ESP32-C6/RISC-V. That came from **core's D3b
  skeleton `r2-core/platforms/dfr1195`** (its Cargo/README/.cargo all say "FireBeetle 2 ESP32-C6") — core
  appears to have **conflated DFR1195 (S3) with the DFR1117 Beetle ESP32-C6** (workshop's C6 board). The
  DFRobot docs are unambiguous: DFR1195 is **S3**. **core's skeleton targets the wrong chip and must be
  redone for esp32s3/xtensa** (flagged — §5).
- **Consequences (vs the wrong C6 assumption):**
  - **Toolchain is the HARDER espup/Xtensa path**, not stock riscv: `espup` installs the Xtensa Rust
    fork (`channel = "esp"`), target `xtensa-esp32s3-none-elf` (Path B no_std esp-hal), `espflash --chip
    esp32s3`. (C6 would have been stock-nightly riscv32 — DFR1195 is not C6.)
  - **composer's original ESP32-S3 `board.toml` + the S3-WROOM 4 MB OTA bound were RIGHT, not stale** —
    restore them. The ~1.5–1.9 MB OTA-slot budget holds on this 4 MB part.

---

## 1. Current state (have)

- **hive-core: 5 no_std seams** — `sync_host` (incl. `route_inbound_sync`), `platform`, `transport_seam`,
  `identity`, `ota` (the OTA receiver state machine, `OtaReceiver`, byte-confirmed vs composer's contract).
  Portable routing + identity + OTA logic, Linux-verified (31 core tests).
- **core D3b skeleton** (`r2-core/platforms/dfr1195`, `f9c9fde`): Path B no_std esp-hal/embassy, `wifi.rs`
  sync `Transport`, `peers.rs` host-centralised resolution (4/4 tests), `HIVE:` board-confirm markers,
  BLE/LoRa `send()` stubbed, LoRa over composer's radio trait. **BUT built for esp32c6/riscv32 — wrong
  SoC; core must re-target esp32s3/xtensa (§5).** The *structure* (host loop, sync Transport, peers
  resolution) is reusable; the chip/target/esp-hal-features layer must change.
- **workshop reference (now directly relevant — DFR1195 IS S3):** `firmware/esp32-s3` (std esp-idf,
  ESP-IDF v5.2.5) = board-level reference (GPIO map, USB-JTAG console, partition numbers); `r2-esp` OTA
  `mark_app_valid()` **self-proof principle** (validate on local self-proof, never on back-end reach; no
  rollback timer) — carry the *principle* into the no_std OTA wiring.
- **Transports Linux-verified** (UDP-LAN round-trip + router parse/dedup tests).

## 2. Shortest path to the milestone

WiFi-UDP first (core's `wifi.rs` sync Transport; proves on-device routing + real-radio TX with least new
code); OTA receiver wired from the first flash (Roy req); LoRa (true infra-less TN) is the follow-on.

**Stage A — first light (laptop ↔ 1 board):**
1. Install toolchain: `espup install` (Xtensa fork), target `xtensa-esp32s3-none-elf`, `espflash` 4.4.0.
   *(Roy: 1 board + USB + toolchain-install perm.)*
2. **Re-target core's skeleton to esp32s3** (core leads; me validate) + resolve the `HIVE:` init points
   on the metal (esp-hal::init, WiFi STA assoc, embassy-net Stack). **Budget time for the esp-hal /
   esp-wifi / esp-hal-embassy / embassy-* version-matrix pinning** — workshop's flagged footgun, where the
   time actually goes.
3. Wire the **hive host loop**: `route_inbound_sync` (RouteEngine) + `wifi.rs` sync Transport + the
   writer-task spawn + RX poll (the sync→async bridge core left for me).
4. **Flash a FULL first image** (bootloader + 2-slot partition table + app + OTA receiver) over USB; board
   joins Roy's WiFi; **laptop (my tested UDP-LAN) ↔ board** route one R2-WIRE frame.

**Stage B — THE routed-frame milestone (board ↔ board):**
5. Flash a second board (full image); both on the same WiFi (or AP-mode).
6. Inject one R2-WIRE frame on A (firmware hook / button / serial) addressed to B → A's engine plans
   forward → `wifi.rs` send → B receives → B's engine delivers (LCD/serial). ✅

**Stage B′ — OTA round-trip (first-class, per Roy — in/just-after the milestone):**
7. From the laptop, push a new image over WiFi (composer F5/F5b `ota_push` → my `OtaReceiver` over
   embassy-net) → board accepts (sha256 + size-bound + write inactive slot + set-boot) → reboots into the
   new image. ✅ Proves every post-#1 update is wireless.

**Stage C — true transient-networking proof (LoRa, follow-on):** repeat Stage B over **SX1262 LoRa**
(infra-less, the real TN medium) — core's LoRa driver un-stubbed + composer's radio trait + SPI wiring +
antennas/region. Longest pole (greenfield radio).

## 3. Gaps (need) — with owner

| # | Gap | Owner | Blocks |
|---|---|---|---|
| 1 | **Re-target core's D3b skeleton esp32c6→esp32s3** (esp-hal `esp32s3` feature, `xtensa-esp32s3-none-elf`, espup) | **core** (hive validates) | A,B |
| 2 | esp-hal/embassy **board crate** = host loop + board init (`HIVE:` points) + pin map (SX1262 SPI/CS/DIO1/BUSY/RST, LCD, IO18) — on core's re-targeted skeleton | **hive** | A,B |
| 3 | sync→async **bridge** (writer-task spawn + RX poll → `route_inbound_sync`) | **hive** | A,B |
| 4 | core `wifi.rs` HW-validated on real S3 | core + hive | A,B |
| 5 | **2-slot OTA partition table** (4 MB: nvs/otadata/phy + ota_0/ota_1 ≈1.875 MB each) handed to espflash `--partition-table` | hive (workshop layout) | A (full flash), B′ |
| 6 | **no_std OTA wiring**: `OtaReceiver` ↔ embassy-net (WiFi) + esp-storage flash writes + A/B set-boot/mark-valid (`esp-bootloader-esp-idf`/`esp-hal-ota`-style) — NOT esp-idf `esp_ota_ops` | hive + core | B′ |
| 7 | sync seam landed in r2-transport (`poll_recv` + types) — today hive's transitional mirror; works for milestone | core + hive | cleanup |
| 8 | espup/Xtensa toolchain + espflash 4.4.0 on the dev box | hive (Roy: install perm) | A |
| 9 | core LoRa/BLE `send()` un-stubbed | core + hive | C only |

## 4. What ROY must physically provide

- **2× DFRobot DFR1195 (LoRaWAN ESP32-S3 Dev Board)** + USB cables. (1 unblocks Stage A; 2 for the milestone.)
- **A 2.4 GHz WiFi network** (SSID + password) boards + laptop join — or confirm AP-mode is OK.
- **Permission to install the espup/Xtensa toolchain** on the dev box (S3 needs the Xtensa Rust fork — not stock).
- **(Stage C / LoRa)** SX1262 antennas + region (AS923 for NZ / US915 / EU868) for legal TX.
- **Ground-truth check:** confirm the chip silk says **ESP32-S3** + the module marking / flash size at the bench (logged) — docs say S3-WROOM-1-N4 4 MB; verify the unit.
- **(Optional)** confirm LCD + IO18 button populated, or accept serial-only for the display/inject hooks.

## 5. Coordination

- **core** — **(URGENT) re-target the D3b skeleton esp32c6→esp32s3** (it built for the wrong SoC — likely
  DFR1117/DFR1195 mix-up); then HW-validate `wifi.rs`, un-stub LoRa/BLE (Stage C), EXTEND r2-transport
  with the sync seam. The skeleton's *structure* is reusable; the chip layer changes.
- **workshop** — `firmware/esp32-s3` is now the **on-point board reference** (GPIO/partitions/USB-JTAG) for
  the actual S3 board; reuse the partition layout + espflash 4.4.0 mechanics (`--chip esp32s3`,
  `--partition-table`, `save-image` for OTA slots) + `build-firmware.sh` versioning/naming logic + the OTA
  self-proof principle. (Their `r2-esp`/`setup-firmware.sh` are esp-idf-only — don't port to no_std.)
  Their C6 path is the *DFR1117*, not ours. Build is portable (any Linux box w/ Xtensa target); flashing
  needs the box the board is plugged into (don't block on tuxedo).
- **composer** — OTA push (F5/F5b `ota_push`) + reply-status contract (mine matches) + the S3 `board.toml`
  and 4 MB OTA bound (**RIGHT — restore, my earlier "stale" flag was based on the wrong C6 SoC**); radio trait for Stage C.

## 6. Risks / unknowns

- **SoC error caught before building** — would have wasted effort on the wrong toolchain/target. Now
  S3-confirmed; ground-truth = Roy at the bench. **core's skeleton must be re-targeted first.**
- **espup/Xtensa is the harder toolchain** (vs the mistaken C6/stock-riscv) — more setup; esp-hal version
  pinning is the real time sink (workshop).
- **4 MB is tight for 2× ~1.875 MB OTA slots** + nvs/otadata/phy — layout fits (≈3.9 MB) but leaves little
  headroom; confirm image size stays under slot size (the `OtaReceiver` TOO_BIG bound enforces it).
- **Path B greenfield radio** — WiFi-UDP sidesteps for first light; LoRa (Stage C) is the longest pole.
- **WiFi-UDP ≠ infra-less TN** — Stages A/B prove on-device routing + real-radio TX; the *transient*
  property is only truly proven at Stage C (LoRa). Be explicit with Roy.
- **Bench time** — hardware-in-the-loop iteration is slower than Linux CI.

---

**Bottom line:** software is ready to meet the metal (5 hive-core seams Linux-verified; OTA receiver
state-machine done). The path: **(a) Roy provides 2× DFR1195 (S3) + WiFi + espup-toolchain perm; (b) core
re-targets its D3b skeleton esp32c6→esp32s3; (c) I install espup, build the skeleton, resolve the `HIVE:`
init points, wire the host loop + sync→async bridge + the `OtaReceiver`↔embassy-net path with a 2-slot
partition table; (d) flash two boards (full images), route one frame board↔board over WiFi-UDP, then
demonstrate a wireless OTA round-trip.** LoRa is the follow-on. **Hard blockers: physical hardware (Roy) +
core re-targeting the skeleton to S3.**

---

## GROUNDED UPDATE (2026-06-20, post first-light + full core read)

Roy: "the hive core code should already have all of that present." **Verified — it is.** A full read of the
core crates + the `platforms/dfr1195` skeleton confirms the transient-networking brain is **present, real
(not stubbed), and compiles for xtensa-esp32s3** (it built during first-light):

- **`r2-route`** `RouteEngine::plan_forward()` → `Drop | DeliverOnly | Directed | Flood`, with **TTL decrement,
  dedup seen-cache, spray-K, relay probability, congestion**; `NeighbourTable` (decay + per-transport quality),
  `PathTable` (reinforcement), `DedupCache`. **Zero firmware changes needed** to the route logic.
- **`r2-wire`** compact(12B)/extended(22B) encode/decode + route stack + HMAC; **`MsgType::Heartbeat` is
  first-class** (= the TG-maintenance heartbeat on the wire — grounds the heartbeat-sync design).
- **`r2-transport`** `Transport` trait + UDP (port 21042) + `LoRaRadio` HAL seam. **`r2-discovery`** peer types.
- **`r2-harness`** already proves the routing in sim (broadcast+dedup / spray-directed / store-carry-forward).
- Platform: **`WifiTransport` sync-half + `udp_writer_task` async drain are COMPLETE**; `PeerTable` complete;
  BLE/LoRa sync-halves are skeletons; `main.rs` marks the HIVE wiring entry points.

**The ONLY gap is the async radio binding** — esp-wifi controller + embassy-net Stack init — i.e. the
**`esp-hal-embassy 0.9.1 ↔ esp-hal 1.1.1` matrix conflict** parked for first-light. **That conflict is now the
single thing between us and a TN test.** Minimal WiFi-UDP path: resolve the matrix → esp-wifi STA + embassy-net
Stack + `udp_writer_task` → static `note_peer` bootstrap → `RouteEngine` over `WifiTransport` → RX
(UDP→validate→`plan_forward`→drive→`send`) + TX (originate `ForwardRequest`→`plan_forward`→`send`) → **proof:
board A→B receive+relay, dedup drops the re-send, TTL decrements, spray-K splits.** Show it on the LCD log/LED.

**Net:** the existing core makes the TN test a **wiring job, not a build job**. Critical path = the WiFi-UDP
bring-up (resolve the embassy matrix; core owns `wifi.rs`, hive owns the platform Cargo deps + `main.rs`
bring-up + metal validation). See `docs/dfr1195-first-light-findings.md` for the embassy-matrix specifics.
