# Network-OTA receiver (#17) — implementation plan + go/no-go risks

Source-grounded plan (esp-bootloader-esp-idf 0.5.0 + esp-storage 0.6 + embassy-net 0.9). The receiver code is
straightforward; **two metal prerequisites gate whether it can work end-to-end** — resolve these first.

## ⚠ Go/no-go prerequisites (verify BEFORE investing in the full receiver)

1. **Bootloader OTA support (BLOCKER candidate).** esp-bootloader-esp-idf's own docs warn: *"the prebuilt
   bootloaders provided by espflash might not include OTA support."* We flash espflash's **default** 2nd-stage
   bootloader (runner = `espflash flash --chip esp32s3`; `prebuilt/` is just host binaries, not a bootloader).
   It boots ota_0 today, but **must honor `otadata` to switch to ota_1** for OTA to work. If it doesn't, we
   need a custom OTA-capable bootloader first (a separate task — coordinate core/workshop; esp-idf bootloader
   or `esp-bootloader-esp-idf`'s own). **Cheap test:** flash a build to ota_1 by offset, write otadata to
   select slot 1, reboot, check serial for `Loaded app from partition at offset 0x200000`. Until this passes,
   OTA cannot complete.
2. **Flash-write-while-WiFi hang (dual-core S3).** esp-storage wraps each flash op in a critical-section; on
   S3 flash ops suspend cache, and with esp-radio + esp-rtos running there are documented hangs/brownouts
   (esp-storage#31, esp-hal#3102). **Mitigation:** quiesce the radio (stop the WiFi controller / pause the
   net+heartbeat tasks) around the erase+write burst; erase per-sector (short CS) not the whole 1.875 MB at
   once; keep CPU at max clock. Measure whether a per-sector `write` returns promptly with WiFi up.

## The receiver (once prerequisites pass)

- **No flash module in esp-hal 1.1** → add `esp-storage = { version="0.6", features=["esp32s3"] }` (+ `embedded-storage 0.3.1`; sha2 already via r2-trust). `FlashStorage::new()` is the backend.
- **OTA API** (esp-bootloader-esp-idf 0.5.0): `OtaUpdater::new(&mut flash, &mut [u8; PARTITION_TABLE_MAX_LEN])` (reads the flashed partition table off flash — no csv needed). `next_partition() -> (FlashRegion, slot)` = the inactive slot (avoids the booted one via `booted_partition()`). Write via `FlashRegion` (`NorFlash::erase` 4096-aligned, then `Storage::write` 4-byte-aligned, partition-relative). `activate_next_partition()` → sets otadata. `set_current_ota_state(OtaImageState::New/Valid)`. Reboot: `esp_hal::system::software_reset()`.
- **Streaming** (slot=1.875 MB, can't buffer): erase sector → write chunk → feed sha2 → next. Verify the streamed SHA-256 against the announced hash at COMMIT; only then `activate_next_partition()` + reboot.
- **Transfer protocol** (new UDP port 21043, stop-and-wait): `START{total_len,sha256}` → `DATA{seq,len,payload}` (4096B chunks, seq=sector idx) ACK each → `COMMIT` → verify → activate → reboot. Host sender = a ~25-line python script on tuxedo (reads the new .bin, sends to 192.168.4.x:21043). Hash UNPADDED bytes (host hashes the raw .bin; pad only the flash write).
- **Cargo:** esp-storage 0.6 (esp32s3), embedded-storage 0.3.1, sha2 0.10. RAM: ~3KB table buf + ≤4KB sector buf + fixed Sha256 — no 1.875MB alloc.
- **App descriptor:** already emitted (`esp_app_desc!()`); the new .bin carries its own. esp-bootloader 0.5.0 does NOT validate the image at write time (the 2nd-stage bootloader checks the image SHA at boot) — our wire SHA-256 is our integrity gate.
- **Rollback:** if the bootloader has auto-rollback, the NEW build must `set_current_ota_state(Valid)` early in main, else it reverts. Decide per the bootloader behavior (prereq #1).

## Visible proof
The LCD already prints `b:{BUILD_ID}`. Build the new image with a different `R2_BUILD_ID`, OTA-send it; after
reboot the LCD `b:` field changes = no-USB OTA proven. Also feeds health telemetry `ota_status` + `fw_sha` (#18).

## Wiring
Add a 4th UDP socket/task on 21043 (current io_task binds 21042 only; bump StackResources if needed). The OTA
task is mostly idle (waits for START); the radio-quiesce (prereq #2) only kicks in during a transfer.
