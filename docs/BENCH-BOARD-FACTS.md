# Bench board facts (Alfred) — MAC ↔ hive_id ↔ TG ↔ port ↔ fw

Persisted per supervisor's XIAO bench bring-up directive (2026-07-12). Living record; update as
provisioning facts (hive_id / TG) are read from each board.

## XIAO — the pairing PERIPHERAL + transport peer (Roy directive 2026-07-12)
| fact | value | source |
|---|---|---|
| chip | **ESP32-S3** (Seeed XIAO ESP32-S3; the off-the-shelf stand-in for the custom-sensor MCU stage) | MAC OUI D8:3B:DA = Espressif; [[custom-sensor-3stage-architecture]] |
| WiFi/base MAC | **D8:3B:DA:75:C3:3C** | `/dev/serial/by-id/usb-Espressif_USB_JTAG_serial_debug_unit_D8:3B:DA:75:C3:3C`, udevadm ID_SERIAL_SHORT |
| USB | VID **303a** (Espressif), USB-Serial/JTAG native, **/dev/ttyACM1** | udevadm |
| firmware | **dfr1195 platform (esp-hal/embassy, ESP32-S3) + `xiaobridge` feature** | dfr1195-fw platforms/dfr1195; xiaobridge = the pure edge-bridge build |
| bearers (HW) | **BLE real** (esp-radio/ble+coex+esp-now → trouble-host), **WiFi real** (esp-radio/wifi), **LoRa** (r2-sx1262) **only if a SX1262 module is physically attached** | Cargo features `ble`/`lora`; LoRa presence = a BENCH-PHYSICAL fact (unconfirmed from here) |
| beacon | build_class=2 BLE advert (opaque static-random address, mfr-data 0xFFFF + `b2` magic) + 17-byte LoRa beacon | dfr1195 Cargo.toml notes |
| SAS-SF | XIAO build carries `benchsf7` (SF7) — MUST match its LoRa peers (mixed-SF can't demod) | [[sf12-airtime-cant-carry-sensor-stream]] |
| **hive_id** | **TBD** — derive from the board's provisioning (read from serial banner or the flashed KS1) | pending safe serial read / provisioning |
| **TG** | **TBD** — the throwaway bench TG it's commissioned into | pending |
| Arduino Leonardo | VID 2341, /dev/ttyACM0 — **DO NOT TARGET** (not an R2 board) | udevadm |
| RAK4630 | physically DISCONNECTED (no VID 1209/239a; by-label RAK4631 dangling) as of 2026-07-12 | — |

## Real-vs-sim bearer plan (per supervisor's radio-sim-over-UDP directive)
- **BLE** = REAL (esp-radio BLE controller on the S3).
- **WiFi** = REAL (esp-radio WiFi STA/AP on the S3).
- **LoRa** = REAL iff a SX1262 module is attached to this XIAO; **else UDP sim-profile** (radio-sim-over-UDP) for
  topological isomorphism with the real LoRa mesh. Which one applies is a bench-physical fact — CONFIRM the
  module before claiming real-LoRa.

## Open (blockers to full confirmation — need a safe read or a bench fact)
1. Is the XIAO CURRENTLY flashed with the `xiaobridge` fw + running/beaconing? (BLE scan on Alfred saw ~10 BLE
   devices in a home RF environment but could NOT positively ID the R2-BEACON: opaque advert address, and
   raw mfr-data needs `btmon` privileges / a serial banner.)
2. Serial banner read is BLOCKED: no pip/pyserial for a DTR-low read, and a plain console-open risks resetting
   the S3 into ROM download mode (the DFR1195 §USB-1 #14 lesson) — NOT attempted against the live board.
3. LoRa module physically attached? (decides real-LoRa vs UDP-sim.)
