# RESUME — r2-hive (hive-worker)

Updated 2026-06-09 (owned by hive). Master save (read-only ref):
`claude-fleet/fleet-context/FLEET-CONTEXT-SAVE.md`.

**Role:** the hive runtime. North-star: **ONE hive codebase usable everywhere**, built on
**core's no_std crates** + thin per-platform layers (Linux/cloud, ESP32-S3/DFR1195, Uno-Q, wasm).
"Bring hive up to a general tool" = converge r2-hive (today Linux/std) onto that one codebase —
do NOT fork per-target firmwares. Chain: specs → core → hive. composer orchestrates hives, isn't one.

**Current branch:** `platform-trait` (local + pushed). Built atop the v0.2 work (`0aa6ab7`).

## Done + green
- **v0.2 migration + relay handshake + 4 vector fixtures** — full r2-hive suite GREEN; on
  `v0.2-relay-handshake` (pushed). Fixtures all specs-verified + landing: host-api (28),
  usb (specs), usb-pair (12 → canonical home **R2-PROVISION §5.3.4**), plugin-web (11, Ed25519).
  Generators: `crates/r2-hive-bin/examples/gen_{host_api,usb_pair,plugin_web}_vectors.rs`.

## In flight — Platform-trait extraction (north-star convergence step 1)
Split today's std hive → `r2-hive-core` (no_std+alloc host loop) behind a `Platform` trait +
thin platform layers (linux first). Verifiable on Linux now; foundation for esp32/wasm/unoq.
- DONE: `src/platform.rs` (`Platform` trait + `LinuxPlatform`); `HiveState.platform` (default
  LinuxPlatform, no `new()` sig change). Seam 1 = clock (`69ab8fb`, handshake timestamp),
  seam 2 = RNG (`04d19cc`, handshake nonce via `fill_random`). 99 lib tests + full suite green.
- NEXT seam (big one): **transports** — introduce the transport seam so the host loop is
  runtime-agnostic (R2-TRANSPORT **sync** for no_std; async r2-discovery for host). Then
  storage (identity/OTA), then the real `r2-hive-core` no_std crate split.

## Next major phase — D2: DFR1195 (ESP32-S3) firmware, Path B pure no_std (esp-hal/embassy)
Gated on the convergence above + core's D3b. Sketch: `docs/esp32-hive-firmware-architecture.md`.
- Firmware = core's no_std stack + core's **D3b** no_std SYNC radio bindings, wrapped in an
  esp-hal/embassy host loop. Consume **R2-TRANSPORT SYNC** (R2-DISCOVERY §5), not async §4.
- hive owns: board layer (SX1262 LoRa / LCD / IO18 button), on-device host loop, **no_std OTA
  receiver** (embassy-net; std `ota_tcp.rs` is reference only). **Validation handoff:** core
  authors D3b but can't flash — **hive validates on real DFR1195**, feeds defects back.
- Near-term scope flag: r2-def/ensemble/dispatch are std-tier → initial MCU hive is
  ROUTING+TRANSPORT only (no on-device ensembles) until those are re-tiered no_std.
- References (std, patterns not code): core `platforms/esp32`, workshop `firmware/esp32-s3`.

## Pending Roy / cross-repo
- **Roy:** greenlight specs to COMMIT **R2-PROVISION §5.3.4** (usb-pair home; uncommitted WIP).
- **hive TODO (small, on v0.2 line):** update `usb_pair.rs` doc citations R2-HIVE §6.4 →
  **R2-PROVISION §5.3.4**. Also parked: R2-USB v2/§3.5 → v0.1/§3.3 refs in usb_serial.rs/main.rs.
- **Deps:** core **D3b** (no_std sync BLE/WiFi/LoRa) = hard blocker for radios; composer = OTA
  push + carrier + ensemble; specs = hw test defs.
- Also: Phase-3 adversarial-refuter role (deployment reality) — stand by for specs' conjectures.

## Resume hygiene
Keep this current. WIP-checkpoint + push `platform-trait` periodically. Safe git only:
named `git add` / `git add -u` — never `git add -A`/`.`; never stage secrets.
