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

## Harness facts (composer bearer-map, 2026-07-12)
- **USB console = LoRa TX/RX BRIDGE only — NO per-transport inject-as-received.** Egress = LoRa RX→USB
  (verbatim). Ingress = USB→**real LoRa TX** (`DATA_TX_LORA`→`lora_route_task` transmits on-air). There is
  NO inject-as-RECEIVED (BLE/LoRa) + NO R2RX tap of the DUT radio TX. ⇒ a no-RF SIM leg for BLE/WiFi needs a
  **firmware change** (add a USB control-frame inject-per-transport-origin + a TX tap); LoRa is exercisable
  only via real RF through the bridge. (Composer flagged wanting a no-firmware-change sim leg — that path
  doesn't exist yet; scope a fw inject-harness feature if the bench needs it.)
- **Identity is in the BOOT BANNER** (`DEV … hive={my_hive:08x} TG={tg_label} persona={bool} role`), catchable
  on the next reset. Defaults if UNPROVISIONED: TG = `r2tg-demo-0000-0000-0000-000000000001`, hive_id =
  mac_low3 fallback from `75:C3:3C`. persona.bin@0x12000 (if present) overrides with hk/tg_hash/hive_id/label.

## Golden decode reference (for android's offline decode-proof, 2026-07-12)
android's live `dd bs=1` capture byte-DROPS the USB egress (measured 27–30B vs the true fixed 31B), so a
naive `decode_compact` hit `InvalidRouteLen` — a capture-tooling artifact, NOT a decoder bug. Proof of the
artifact: byte0=`0x06` sets `has_route`, so a single dropped byte makes `data[12]` read as `rlen=0x00` →
exactly `InvalidRouteLen`. Byte-exact golden frame from the CANONICAL `r2_wire::encode_compact`
(`crates/r2-hive-bin/examples/gen_golden_compact_frame.rs`, round-trips `decode_compact`):
- **compact frame (31B):** `0653000164cedbf305fe0701011234a10018eaa101182a0102030405060708`
  - `06`=ver0|Event(type0)|route+hmac · `53`=ttl5/k3 · `0001`=msg_id · `64cedbf3`=event_hash ·
    `05fe0701`=target · `01`=route_len · `1234`=route[0] · 8B payload · 8B hmac (arbitrary; `decode_compact`
    only slices the tag — HMAC *verify* is separate `verify_compact`).
- **R2-USB DATA record (33B):** `1f000653000164cedbf305fe0701011234a10018eaa101182a0102030405060708`
  (`1f00` = payload_len 31 LE).
- **0xA1 sighting golden** already in `dfr1195 platforms/dfr1195/USB-BEACON-SIGHTING-FORMAT.md` KAT:
  `1700 A1 01 02 11 B201007FCE111165325A9ABAFE8AC11402 D6 09` (rssi/snr trailing 2B are SAMPLE-only).

## Board-health note — the QUIET is peer-driven, not a wedge (2026-07-12)
The `xiao_bridge_task` egress is PURE forwarded LoRa RX with NO local keepalive — if the LoRa peer stops
TX'ing, egress goes to 0 bytes (benign). android confirmed the XIAO is STILL enumerated at the same MAC
(D8:3B:DA:75:C3:3C) → it did NOT reset into ROM download mode (that re-enumerates). A single DTR toggle on
a Python close does NOT force S3 download mode (that needs the esptool RTS/DTR *sequence*) and could not
have; the forward-task also can't wedge on it (USB-Serial-JTAG egress drops bytes when unread, never
blocks). ⇒ safe confirm = check whether the LoRa PEER is still transmitting, NOT poke the XIAO tty. Board
untouched (download-mode-reset risk + android's live capture port).

## Still open
1. **hive_id / persona / TG / build_id** — exact values need the boot banner (android can catch on a reset) or
   a safe serial read (no pyserial; not risked). Defaults above apply if unprovisioned.
2. Whether to reflash `xiaobridge,ble` now (BLE beacon) vs keep the current LoRa/USB bridge for android's capture.
3. Whether the bench needs a fw USB inject-per-transport harness (composer) — a fw feature, not present today.
