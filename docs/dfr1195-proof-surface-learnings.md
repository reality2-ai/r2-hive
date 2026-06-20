# DFR1195 proof-surface — validated hardware learnings → plugin + sentant

Consolidated from real-hardware bring-up (first-light + LCD + LED working on the DFR1195 S3). Roy: push
these to composer (via supervisor) to **create the plugin(s) + the associated sentant**. Split per pattern:
**hive owns the device-specific output drivers/plugins** (validated below); **composer owns the StatusDisplay
sentant + catalogue**; **specs/core own the general capability traits/descriptors**.

## 1. Display — ST7735S (WORKING on metal)

- **Controller:** ST7735S, **160×80 RGB565**, 0.96" color IPS.
- **Pins:** MOSI GPIO11 · SCK GPIO12 · CS GPIO17 · DC GPIO14 · RST GPIO15 · BL GPIO16 · PWR GPIO48.
- **⚠ KEY FINDING — controller power GPIO48 is ACTIVE-LOW** (drive LOW = on). Backlight GPIO16 is a separate
  active-high rail. With GPIO48 HIGH the panel is backlit but the controller is **dead/blank** — this cost a
  debug cycle; bake it into the board profile so no one else hits it.
- **SPI:** 20 MHz, Mode 0 (40 MHz was too fast; 20 is reliable). esp-hal 1.x `Spi::new(SPI2,…).with_sck(12).with_mosi(11)`.
- **Driver stack:** `mipidsi 0.9` (model `ST7735s`) + `embedded-graphics 0.8` + `embedded-hal-bus 0.3`
  (ExclusiveDevice). Config: `display_size(80,160)` + `display_offset(26,1)` + `Rotation::Deg90` +
  `ColorInversion::Inverted` (offset/inversion still being dialled to perfection, but text renders + positions).
- **Proven layout (the proof surface):** a **status line on top** + an **event log scrolling up** below
  (newest at bottom, oldest scrolls off) — fits ~1 status + 6 event lines at FONT_6X10 on 160×80. This is the
  calm-tech glanceable surface; matches composer's locked CMD_RENDER view-model.

## 2. LED — single mono LED on GPIO21 (heartbeat working)

- **Onboard LED: GPIO21, MONO** (no RGB/WS2812 — unlike workshop's WS2812 boards). So status is encoded as
  **blink PATTERNS, not colour.** Buttons: Key1 GPIO18 (inject/user), Key2 GPIO0 (boot).
- **Validated:** a gentle **heartbeat "lub-dub"** (two ~45 ms pulses then rest, ≈1.5 s cycle) = "all well" —
  glanceable even when the screen is off.
- **Proposed pattern vocabulary** (mirrors workshop's LedState intent, mono-adapted):
  - `all-well` → slow heartbeat (current)
  - `ota` → fast even blink (~5 Hz) **while the screen may be down** (Roy's key use case)
  - `joining/provisioning` → slow blink
  - `error/fault` → rapid burst / SOS
  - `identify` → solid for N seconds (locate-this-node)

## 3. First-light substrate (validated)

esp-hal **1.1.1** boots no_std on the real S3; matrix esp-hal 1.1.1 / esp-alloc 0.10 / esp-backtrace 0.17 /
esp-println 0.15 / **esp-bootloader-esp-idf 0.5.0** (`esp_app_desc!()` — the descriptor espflash 4.4.0 needs).
Flashed with the **2-slot OTA partition table**; bootloader booted from ota_0 = OTA-laid-out from first flash.
(embassy/esp-wifi tier — needed for the routing + remote-driven display + OTA receiver — pending the
esp-hal-embassy↔esp-hal 1.1.1 reconcile.)

## 4. What composer should create (the ask)

Two device-output **capabilities**, each = a general capability (specs/core trait+descriptor) + a hive device
driver/plugin + composer's sentant driving it:

1. **Display plugin** (`r2.hw.display`): hive's ST7735S driver implements the general display render capability
   (CMD_RENDER + CMD_CLEAR; descriptor: st7735s/160×80/rgb565/backlight=dimmable/power_cut=yes). Already
   contracted with composer (`ed50505`).
2. **LED status plugin** (`r2.hw.led` — NEW): hive's GPIO21 mono-LED driver implements a general **indicator**
   capability — a small pattern set (`all-well`/`ota`/`joining`/`error`/`identify`) the sentant selects;
   descriptor: `{kind: mono, patterns: [...]}` (RGB boards expose `{kind: rgb}` + colours later).
3. **StatusDisplay sentant** (composer, already started, `e4c7ae3`): computes node status (id/role/TX/RX/
   delivered/dropped/rssi/link + OTA progress) and drives **both** outputs — render fields to the display,
   select a pattern on the LED. One sentant, two glanceable surfaces; device-agnostic + repackaged per board.

**Telemetry events** feeding the sentant: strawman sent (r2.route.tx/rx/delivered/dropped/neighbour/link,
r2.update.started/progress/applied/failed) — names firm up + get specs/core-ratified as the routing/OTA tier
is wired.

hive implements both device drivers (display done, LED heartbeat done; pattern set + plugin-ization next);
composer authors the LED capability into the contract + the StatusDisplay sentant to drive both surfaces.
