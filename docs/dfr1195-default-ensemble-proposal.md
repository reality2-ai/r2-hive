# DFR1195 default setup — board profile + default ensemble (proposal)

**Why:** a freshly-provisioned DFR1195 should come up with a sensible **default setup** (Roy). Split per the
established pattern: **hive owns the device-specific board profile** (capabilities/drivers, like the
LoRaRadio per-chip impl); **composer owns the ensemble + catalogue** (what sentants run). This doc proposes
both; composer authors the canonical ensemble in its catalogue.

## 1. DFR1195 board profile (hive-owned — content for composer's `board.toml`)

The device capability descriptor the default ensemble + drivers select against. Grounded in validated
hardware (silicon-confirmed + first-light):

```toml
[board.dfr1195]
sku          = "DFR1195"            # DFRobot LoRaWAN ESP32-S3 Dev Board
soc          = "esp32s3"            # Xtensa LX7, rev v0.1/v0.2 seen
flash_mb     = 4
psram        = false
usb          = "serial-jtag"        # native USB, /dev/ttyACM*, no programmer

[board.dfr1195.ota]                 # validated: bootloader read it, booted ota_0
scheme       = "2-slot-ab"
ota_0        = 0x20000
ota_1        = 0x200000
slot_bytes   = 0x1E0000             # 1.875 MB — FirmwareSink::slot_capacity / TOO_BIG bound
otadata      = 0xf000

[board.dfr1195.display]             # display capability descriptor (LCD contract)
driver       = "st7735s"
width        = 160
height       = 80
color        = "rgb565"
backlight    = "dimmable"           # GPIO16 PWM
power_cut    = true                 # GPIO48
# pins: mosi=11 sck=12 cs=17 dc=14 rst=15 bl=16 pwr=48

[board.dfr1195.transports]
wifi         = true                 # esp-wifi STA (WiFi-UDP) — bringup in progress
ble          = true                 # esp-wifi BLE
lora         = "sx1262"             # SPI; LoRaRadio seam, lora-phy driver

[board.dfr1195.identity]
scheme       = "hkdf-master-secret" # shared r2-esp/hive_id: usb_link_id + mesh_hive_id (NVS)
```

## 2. Default ensemble (composer-owned — proposed composition)

What a provisioned DFR1195 runs by default. **Tier reality (agreed):** the MCU hive is **routing+transport
(+ OTA + display plugin)** — on-device *sentants/ensembles* are std-tier (deferred). So the default ensemble
is split across where each piece runs:

| Component | Runs on | Owner | Role |
|---|---|---|---|
| **R2 routing + transports** (WiFi-UDP, BLE, SX1262 LoRa) | DFR1195 firmware | hive/core | the mesh hive itself — relay/dedup/TTL/spray |
| **OTA receiver** | DFR1195 firmware | hive | wireless updates into ota_0/ota_1 (after WiFi tier) |
| **display output plugin** (ST7735S) | DFR1195 firmware | hive | renders CMD_RENDER frames (calm-tech proof surface) |
| **proof-surface display SENTANT** (calm-tech view model) | full hive / composer | composer | computes WHAT to show → drives the device's display plugin over the mesh |
| **identity / provisioning** | DFR1195 (NVS) | workshop/core | usb_link_id + TG membership |

So the **default ensemble** (composer's catalogue entry, e.g. `dfr1195-default`) ≈ the proof-surface display
sentant targeting the board profile's `display` capability + the routing defaults, provisioned onto any
device whose board.toml matches `[board.dfr1195]`. The device contributes the routing/transport/OTA/display-
plugin capabilities; the ensemble supplies the sentant that uses them.

## 3. Coordination

- **hive (me):** maintain the board profile above (device truth); implement the ST7735S display plugin +
  OTA receiver + transports in firmware (sequenced after the WiFi/embassy tier).
- **composer:** author the canonical `dfr1195-default` ensemble in the catalogue (proof-surface sentant +
  defaults), targeting the board profile; provision it onto DFR1195 devices.
- **specs/core:** the general display capability trait + descriptor (already the converged ask) is what the
  board profile's `display` block instantiates.

Forward-looking: the firmware is at first-light (routing/display/OTA not yet wired); this default setup
becomes live as those land. Pinning it now so the device profile + the catalogue entry are ready.
