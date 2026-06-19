# DFR1195 OTA partition table (ESP32-S3-WROOM-1-N4, 4 MB)

Owned by hive (critical-path gap #5). The **first USB flash must already carry this 2-slot A/B layout** so
every subsequent update is wireless (Roy standing requirement — OTA-from-first-flash). Hand this to
core's `r2-core/platforms/dfr1195` once it is re-targeted to S3; in the no_std/esp-hal path the table is
passed to espflash explicitly (`espflash flash --partition-table partitions.csv …`), not via CMake
(workshop: `setup-firmware.sh` is esp-idf-only, dropped).

## partitions.csv

```csv
# Name,    Type, SubType, Offset,    Size
# ESP32-S3-WROOM-1-N4 = 4 MB (0x400000). 2-slot A/B OTA.
nvs,       data, nvs,     0x9000,    0x6000
otadata,   data, ota,     0xf000,    0x2000
phy_init,  data, phy,     0x11000,   0x1000
ota_0,     app,  ota_0,   0x20000,   0x1E0000
ota_1,     app,  ota_1,   0x200000,  0x1E0000
```

(Bootloader occupies `0x0`, the partition table `0x8000`; `nvs` begins at `0x9000` — standard ESP layout.)

## Budget check (fits 4 MB)

| Region | Offset | Size | End |
|---|---|---|---|
| (bootloader + part-table) | `0x0` | `0x9000` | `0x9000` |
| nvs | `0x9000` | `0x6000` (24 KB) | `0xf000` |
| otadata | `0xf000` | `0x2000` (8 KB) | `0x11000` |
| phy_init | `0x11000` | `0x1000` (4 KB) | `0x12000` |
| ota_0 | `0x20000` | `0x1E0000` (1.875 MB) | `0x200000` |
| ota_1 | `0x200000` | `0x1E0000` (1.875 MB) | `0x3E0000` |
| (free headroom) | `0x3E0000` | `0x20000` (128 KB) | `0x400000` |

Top of flash = `0x400000` (4 MB). **Fits**, with 128 KB headroom. `nvs` (24 KB) holds the per-device
identity (`usb_link_id` / mesh `master_secret`) + provisioning state across OTA, per the workshop
identity-split agreement.

## Ties to the `OtaReceiver` (r2-hive-core `ota.rs`)

- **`FirmwareSink::slot_capacity()` MUST return `0x1E0000` (1,966,080 bytes)** = one OTA slot. The
  `OtaReceiver` TOO_BIG check rejects any image larger than this **before** any flash write — so a
  firmware image must stay under 1.875 MB. (Routing+transport+WiFi+OTA no_std image should be well under;
  watch it as on-device ensembles are added later.)
  - **PINNED cross-repo (single source of truth):** `0x1E0000` is the one number — composer's
    OTA-REPLY-STATUS-CONTRACT.md + board.toml now state push-side `TOO_BIG == FirmwareSink::slot_capacity()
    == 0x1E0000`, citing this doc (composer commit `552536a`). composer's push has no hard-coded size; the
    device enforces TOO_BIG. So when I implement the esp-storage `FirmwareSink`, `slot_capacity()` returns
    exactly `0x1E0000`.
- The esp-storage `FirmwareSink` impl (hive-owned board layer) writes the **inactive** slot (the one
  `otadata` says isn't booted), then sets boot + marks-valid on the self-proof principle (workshop's
  `mark_app_valid`: validate on local self-proof, never on back-end reach; no rollback timer).
- `otadata` (8 KB / two sectors) is the A/B boot-selection record the bootloader reads — written last,
  after sha256-match + slot write succeed (the contract's SUCCESS precondition).

## First-flash vs OTA-flash artifacts (workshop mechanics)

- **First (USB) flash** = a **merged** image: bootloader + partition table + app into ota_0, via
  `espflash flash --chip esp32s3 --partition-table partitions.csv` (full image). After this, ota_0 is the
  running app and ota_1 is the empty inactive slot.
- **OTA-flash image** = **app-only** (`espflash save-image --chip esp32s3`, not `--merge`) — the bytes
  composer's F5/F5b `ota_push` streams to the `OtaReceiver`, which writes them into the inactive slot.

## Open / verify-at-bench

- Confirm the real module is N4 (4 MB) at the bench (silk + flash id) — docs say S3-WROOM-1-N4 4 MB.
- If a future build needs > 1.875 MB per slot, either drop to a single-slot (no OTA — rejected, breaks
  the standing requirement) or move to an 8 MB module (not this part). The 1.875 MB ceiling is the real
  on-device image budget for the DFR1195.
