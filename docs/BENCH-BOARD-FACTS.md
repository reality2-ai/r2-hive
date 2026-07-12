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

## ✅ RESOLVED via android's passive serial capture + fw-source decode (2026-07-12)
- **CURRENTLY FLASHED + RUNNING** the `xiaobridge` build (android receives its USB stream). NOT dead.
- **USB stream format** (`xiao_bridge_task`, dfr1195 main.rs:4923): one-shot opening SYNC `04 00 32 52 02 00`
  = `[len 4 LE][magic 0x3252][v2][flags 0x00 = COMPACT]` at STARTUP (missed on a late connect), then each
  message = `[u16 LE payload_len][payload VERBATIM]`. Payloads = **LoRa RX frames forwarded verbatim** —
  compact R2-WIRE DATA (byte0>>6==0) that passed the target-group prefilter, or 0xA1 beacon-sightings. It is
  the compact-on-LoRa bridge stream (was extended `…02 01` pre-2026-07-10), NOT full R2-USB v2 §3.5 type-demux.
- **EGRESS free-runs** (LoRa→USB forwarded regardless of host). **INGRESS** (host→XIAO→LoRa TX) requires the
  host to send its opening SYNC first, then framed messages (XIAO consumes the 1st framed msg as SYNC).
- **Bearers, current xiaobridge build:** WiFi = **REAL + UP** (net_task+wifi_task unconditional). LoRa = **REAL
  + UP** (loraroute+loratcxo; a LoRa peer is TX'ing ~2s compact frames the XIAO RX's + forwards). BLE = **HW
  present but NOT advertising** — `xiaobridge` does NOT pull the `ble` feature, so `ble_task` is not spawned.
- **⚠ GAP for the multi-bearer BEACONING task:** the current build beacons on LoRa + serves WiFi but does NOT
  advertise BLE R2-BEACON. To meet "beaconing on every bearer" → rebuild `--features xiaobridge,ble` (adds
  ble_task = R2-BEACON advertise + L2CAP CoC). **Reflash HELD** — android is live-capturing the current LoRa
  stream; coordinate before reflashing so we don't disrupt its bench capture.

## Still open
1. **hive_id / TG** — read from a safe serial banner or the flashed KS1 (still no pyserial; not risked).
2. Whether to reflash `xiaobridge,ble` now (BLE beacon) vs keep the current LoRa/USB bridge for android's capture.
