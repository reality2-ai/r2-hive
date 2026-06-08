# RESUME — r2-hive (hive-worker)

Fleet checkpoint 2026-06-09. Master save: `r2-specifications/fleet-context/FLEET-CONTEXT-SAVE.md`.

**Role:** the hive runtime. North-star: **ONE hive codebase usable everywhere**, built on **core's no_std crates**
+ thin per-platform layers. "Bring hive up to a general tool" = converge r2-hive (today Linux/std) onto that one
codebase — do NOT fork per-target firmwares. Chain specs → core → hive.

**Done + pushed:** v0.2-relay-handshake migration + 4 vector fixtures, full suite green.

**In flight (resume here) — D2: DFR1195 (ESP32-S3) firmware, Path B pure no_std (esp-hal/embassy):**
- Build ON core: firmware = core's no_std stack (engine/trust/route/wire) + core's D3b bindings, wrapped in an
  esp-hal/embassy host loop. Do NOT reimplement core logic.
- Consume the **R2-TRANSPORT SYNC** interface (per R2-DISCOVERY §5), NOT the async r2-discovery §4 bindings
  (those are host/std only).
- You own: board layer (SX1262/LCD[ST7735-class SPI TFT]/IO18 drivers), the on-device host event loop, and a
  **no_std OTA receiver** (embassy-net; the existing `ota_tcp.rs` is std → reference only).
- **Validation handoff:** core authors the D3b no_std radio drivers but CANNOT flash/verify them — **you validate
  on a real DFR1195** (hardware-in-the-loop) and feed defects back to core. Coordinate the sync binding surface closely.
- References (std, port patterns NOT code): core `platforms/esp32` (ESP-IDF demo: real BLE/WiFi/OTA/trust),
  workshop ESP32 firmware. Later: Uno-Q/Linux 2nd board class (`r2-core/platforms/unoq/`).
- OTA: composer owns the push wire (F5/F5b); you own the no_std device receiver — agree the wire contract.

**WIP checkpointed:** `docs/esp32-hive-firmware-architecture.md`. **Branch:** `v0.2-relay-handshake`.
