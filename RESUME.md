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
- DONE seams: 1 = clock (`69ab8fb`), 2 = RNG (`04d19cc`), 3 = **transports** (`1e24da8`):
  `src/platform.rs` (`Platform` trait + `LinuxPlatform`); `HiveState.platform` (default,
  no `new()` sig change); `src/transport_seam.rs` (`HiveTransports` trait = outbound
  multi-transport contract, `HiveState` impls it, `&dyn` proven). 100 lib tests + full suite green.
- DONE: **sync host-loop seam** (`sync_host.rs`, `683241f`) — `SyncTransport` trait
  (`kind`/`send`/`poll_recv`) + `TransportAddr`/`InboundFrame` + `provisional_hive_id` +
  `poll_inbound` tick primitive; Linux-verified via sync-stub. **TRANSITIONAL local mirror** of
  the seam core+hive AGREED (R2-DISCOVERY §5 sync). Core will EXTEND r2-transport
  (`Transport::poll_recv` default-None + TransportAddr/InboundFrame) → then delete the mirror,
  import `r2_transport::`. Host resolves source_addr→hive_id; driver-owned RX buffer.
- DONE: **RouteEngine wired into the sync host loop** (`route_inbound_sync`, `3ebdb61`) — parse
  R2-WIRE → ingest neighbour → `plan_forward` → execute Drop/DeliverOnly/Directed/Flood over
  `SyncTransport`; routing-only (no ensemble/TG/WS host bits); host-centralised resolution
  (specs-confirmed conformant, R2-DISCOVERY §5). Linux-verified end-to-end (real RouteEngine +
  sync-stub relay). 106 lib tests, full suite green.
- DONE: **`r2-hive-core` crate split started** (`a05b108`) — new `#![no_std]`+alloc crate (deps
  r2-wire/route/fnv only, no tokio/axum/std-net); **`sync_host` moved into it and compiles no_std**
  = PROOF the routing host-loop is MCU-portable. bin depends on it + re-exports `sync_host`
  (zero churn). Full workspace green (r2-hive-core 6 tests + bin suite).
- NEXT: migrate more seams into r2-hive-core as their std-coupling is factored — the **Platform**
  trait (trait→core, `LinuxPlatform` stays in bin) and the **transport seam**; then **storage seam**
  (identity: `MasterSecretStore` already a trait, just std-`io`-flavored). The MCU/wasm platform
  layers depend on r2-hive-core. Swap `sync_host` seam mirror → `r2_transport::` when core EXTENDs it.
  esp-hal/embassy board crate (P0) = firmware tier (needs xtensa toolchain + hardware); radio = core D3b.

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
- Phase-3 adversarial-refuter role (deployment reality): FILED first batch to specs (the 5
  high-value TN conjectures). Two systemic findings — (A) must_text bounds by TTL/time, never
  MEMORY (MCU RAM = fixed tables+eviction; fixed-size dedup evicts before window W); (B) hop-TTL
  ≠ wall-clock (a carried frame's hop-TTL never decrements while carried). Verdicts:
  TN-L2-IT-BL-001 + TN-L2-IT-AB-001 FALSIFIED-as-stated; BL-002/XT-BL-001/L1-IT-BL-004 REFINE.
  + sim-tier-decidability flag (sim needs bounded-mem + carry-time model, else mark tier=hardware).
  Awaiting specs adjudication; more conjectures can be reviewed on request.
  DYN-family batch (v0.3, 13 conjectures) ALSO filed: grounded vs real r2-route (f32 + libm::expf,
  multiplicative c+0.2*(1-c), mobility is an engine INPUT not RSSI-classified). Findings: (A)
  TN-L0-IT-BL-100 spec-vs-impl — must_text additive +0.1 vs impl multiplicative +0.2*(1-c) [core
  reconcile]; (B) TN-L2-IT-BL-100 RSSI-sigma classifier UNREALIZED + fragile under real RSSI noise
  → tier=hardware [strongest]; (C) soft-float expf cost on no-FPU (ESP32-C6); (D) fixed-point future
  → 0.05*(1-c) underflow (TN-L2-IT-BL-101). Awaiting specs adjudication.

## Resume hygiene
Keep this current. WIP-checkpoint + push `platform-trait` periodically. Safe git only:
named `git add` / `git add -u` — never `git add -A`/`.`; never stage secrets.
