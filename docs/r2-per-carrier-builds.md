# Per-carrier platform builds — one ensemble, two platforms (unified-hive-architecture)

Roy DIRECTIVE (REQUIRED, not optional): the next firmware deliverable produces **separate
DFR1195 + XIAO binaries** (per-carrier PLATFORM layer) that run the **SAME ENSEMBLE** (identical
plugins+sentants/logic; only the platform layer differs). Proves **logical = portable, platform =
per-carrier**. Fold into the NEXT deliverable, ideally TOGETHER WITH the #24 BLE stack. composer
composes the ONE ensemble + the two board.toml CarrierProfiles; hive builds the two binaries.

## The split (what's shared vs per-carrier)
**SHARED (the ensemble — identical, one source):** io_task logic = conductor-PLL heartbeat +
RouteEngine + trust deliver-gate + persona read + health telemetry + IDENTIFY + the lub-DUB
envelope shape + the negotiation engine (r2-discovery, #24). The LEDC, WiFi, the R2 core crates.

**PER-CARRIER (the platform layer — Cargo-feature-gated):**
| Knob | DFR1195 (`carrier-dfr1195`) | XIAO-S3 (`carrier-xiao`) |
|---|---|---|
| Flash / partition | 4MB / `dfr1195-partitions.csv` | 8MB / `dfr1195-partitions-8mb.csv` |
| PSRAM | none | **octal PSRAM** (esp-hal `psram` init + feature) |
| Screen | ST7735S (LCD init) | none (skip) |
| LED | GPIO21 active-high | GPIO21 active-high (external) |

## Build shape
ONE Cargo crate (platforms/dfr1195), two mutually-exclusive features: `carrier-dfr1195` (default) /
`carrier-xiao`. Platform init in main is `#[cfg(feature=...)]`-gated (PSRAM init, LCD init,
LED/screen config); the ensemble/logic is common (no cfg). Partition table is FLASH-TIME (the right
CSV per carrier via `espflash --partition-table`), not in the binary. Build twice:
`cargo build --release --no-default-features --features carrier-xiao` etc. → 2 binaries.
hive drives the 2 builds (needs the esp toolchain) with composer's composed config; composer flashes
per MAC-reservation (the 4 XIAO = ACM2/3/4/5).

## Carrier-detection (boot guard, hive deliverable)
Probe MAC-OUI + esp-hal PSRAM-present at boot → log the detected carrier; if it mismatches the
COMPILED carrier (e.g., the no-PSRAM DFR1195 build booted on a XIAO), WARN/reject (don't run the
degraded binary — that's the boot-flakiness/replug cause). The compiled carrier is a `#[cfg]` const.

## Status
REQUIRED; folds into the next deliverable with the #24 BLE stack. The current single binary
(4MB/no-PSRAM/has_screen-byte) is the DFR1195 carrier; the XIAO carrier adds the PSRAM init +
8MB partition + no-screen. The has_screen byte / LED byte become `#[cfg]` carrier constants
(no runtime profile-byte needed once per-carrier builds land — removes that fragility entirely).
