# DFR1195 first-light — findings (2026-06-19)

Board connected (on **tuxedo-os** `/dev/ttyACM0`); hive on **Alfred** (esp/Xtensa toolchain), passwordless
SSH → build-on-Alfred / flash-on-tuxedo. This is the metal-validation half of the core↔hive D3b split:
**core authors the skeleton, hive builds+flashes on real hardware and feeds defects back.** Here's the loop's first pass.

## ✅ Confirmed from silicon (`espflash board-info`)
- **Chip: esp32s3 (rev v0.1), Flash: 4MB, WiFi+BLE**, MAC f4:12:fa:50:26:98, Secure Boot + Flash Encryption **disabled**.
- Definitively **ESP32-S3** — closes the C6/S3 question with the actual chip ID. (core's skeleton's `esp32c6` was a DFR1117-Beetle-C6 conflation.)

## ✅ Pipeline established
- Alfred: `esp` toolchain (espup) + Xtensa GCC (`~/.rustup/toolchains/esp/xtensa-esp-elf/.../xtensa-esp32s3-elf-gcc`); `export-esp.sh` at `~/Development/homelab/export-esp.sh` — **must be sourced before building** (sets PATH/LIBCLANG).
- tuxedo-os: cargo 1.95 + espflash 4.4.0 + cargo-espflash, board at `/dev/ttyACM0`, `roycdavies` in `dialout`. (workshop's sensor rig + :21042 dashboard also on tuxedo — flashing `/dev/ttyACM0` only, no service restarts.)

## ✅ Build SUCCEEDS for S3 — 3 skeleton defects found + fixed (patch: `docs/dfr1195-s3-validation.patch`)
The esp-hal/esp-wifi/embassy **version matrix compiles clean** (87 deps built, no footgun). Three fixes to core's skeleton, all validated to a 126K Xtensa ELF:
1. **C6→S3 re-target** — `Cargo.toml` `esp32c6`→`esp32s3` (esp-hal/esp-hal-embassy/esp-wifi/esp-backtrace/esp-println); `.cargo/config.toml` target `riscv32imac-unknown-none-elf`→`xtensa-esp32s3-none-elf` + `--chip esp32s3`; `rust-toolchain.toml` `channel = "esp"` (Xtensa), drop the riscv target.
2. **`wifi.rs:139` embassy-net API** — `UdpSocket::send_to` wants its own `IpEndpoint`, not `core::net::SocketAddr`. `From<SocketAddr>` needs smoltcp `proto-ipv4`+`proto-ipv6`, but only `proto-ipv4` is enabled → use `From<SocketAddrV4>` (mesh is IPv4-only). Matched on `dg.dest`.
3. (build then linked: needed `export-esp.sh` sourced for `xtensa-esp32s3-elf-gcc`.)

## ⛔ FLASH BLOCKED — espflash 4.4.0 ↔ esp-hal 0.23 version skew
`espflash flash` (and `--ram`) **reject the image: "ESP-IDF App Descriptor missing"**. espflash 4.4.0 (2026) requires the app descriptor in the image; **esp-hal 0.23 (2024, the skeleton's pin) does not emit it** (the `esp-bootloader-esp-idf` crate + `esp_app_desc!()` macro, standard in the esp-hal 1.0 era, aren't present). No espflash bypass flag (`--ram` enforces it too).

**Resolution (core's skeleton call + workshop's matrix knowledge):**
- **Preferred:** bump the skeleton to the current matrix — esp-hal **1.0** + `esp-hal-embassy`/`esp-wifi` current + **`esp-bootloader-esp-idf`** (provides the descriptor) — which pairs with espflash 4.4.0. This is a real API migration (esp_hal::init/clocks/esp-wifi changed 0.23→1.0) = core's skeleton work; hive re-validates on metal.
- **Alternative:** pin an older espflash (pre-descriptor-enforcement) just for this build — but workshop's rig uses 4.4.0 on tuxedo; don't disturb. Prefer the version bump.

## State
Everything up to a **bootable image format** is validated; first-light (chip executes our firmware) is **one version-matrix bump away**. core owns the bump; hive flashes the moment it lands. Patch with the 3 validated S3 fixes: `docs/dfr1195-s3-validation.patch`.
