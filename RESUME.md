# RESUME — r2-hive (hive-worker)

Updated 2026-06-18 (owned by hive). Master save (read-only ref):
`r2-fleet/fleet-context/FLEET-CONTEXT-SAVE.md` (moved from claude-fleet, now tooling-code-only).

**Role:** the hive runtime. North-star: **ONE hive codebase usable everywhere**, built on
**core's no_std crates** + thin per-platform layers (Linux/cloud, ESP32-S3/DFR1195, Uno-Q, wasm).
"Bring hive up to a general tool" = converge r2-hive (today Linux/std) onto that one codebase —
do NOT fork per-target firmwares. Chain: specs → core → hive. composer orchestrates hives, isn't one.

**Current branch:** `platform-trait` (local + pushed). Built atop the v0.2 work (`0aa6ab7`).

## Active (besides the branch) — priorities per Roy (2026-06-16)
- **#1 LEAD TRACK: first real-hardware TN test on the DFR1195 rig.** Critical-path doc DELIVERED +
  CORRECTED (`45a7194`, `docs/hardware-tn-test-critical-path.md`). **TWO boards now live on tuxedo-os:
  ttyACM0 (S3 rev v0.1, MAC …26:98) + ttyACM1 (S3 rev v0.2, MAC …90:10)** — enough for hive-to-hive
  (field.lab milestone). Confirm port before flashing each. Milestone = two DFR1195s exchange one
  routed R2-WIRE frame over real radio, AND the first USB image already ships a working OTA receiver +
  2-slot partition table (Roy standing req — every later update wireless). Shortest path = WiFi-UDP first
  (core wifi.rs) → board↔board (Stage B) → wireless OTA round-trip (Stage B', composer F5 ota_push ↔ my
  OtaReceiver) → LoRa (Stage C, true infra-less TN). **SoC CONFIRMED ESP32-S3** (DFRobot wiki + SKU
  SKU_DFR1195_LoRaWAN_ESP32_S3 = ESP32-S3-WROOM-1-N4 Xtensa, 4MB, SX1262). Target xtensa-esp32s3-none-elf
  (espup Xtensa fork — the HARDER path), espflash --chip esp32s3. **I briefly mis-ID'd it as C6 from
  core's skeleton (which conflated DFR1195 with DFR1117 Beetle C6) — corrected; lesson: verify SoC vs the
  primary source, not a downstream artifact.** **BLOCKERS: (1) physical — Roy provides 2× DFR1195 (S3) +
  2.4GHz WiFi + espup-toolchain perm (+ LoRa antennas/region for C); (2) core must RE-TARGET its
  platforms/dfr1195 skeleton esp32c6→esp32s3 (flagged — its structure reuses, chip layer changes).**
  workshop's firmware/esp32-s3 is now the on-point board reference (GPIO/partitions/USB-JTAG/espflash
  mechanics/OTA self-proof). composer's S3 board.toml + 4MB OTA bound = RIGHT (un-flagged my churn).
  - **D3b division of labor AGREED with core** (Roy made the radio drivers core's top priority):
    **core OWNS** r2_transport::Transport bindings (wifi/ble/lora seam), peers.rs resolution, the SX1262
    LoRaRadio impl, and authors a first-draft esp-wifi/embassy-net bringup against the S3 pins. **hive
    OWNS** esp-hal chip/clock/heap init, esp-wifi controller + STA assoc, embassy-net Stack, flash/monitor
    loop, host-loop wiring (route_inbound_sync + sync→async bridge), the **esp-storage FirmwareSink** impl
    (OTA flash A/B + set-boot for my OtaReceiver), and metal validation + defect loop (core can't
    compile/flash — author→hive-flash→defect). **Pins:** core's matrix (esp-hal 0.23/esp-hal-embassy 0.6/
    esp-wifi 0.12/embassy-net 0.6/esp-alloc) with chip feature **esp32s3** + target xtensa-esp32s3-none-elf;
    reconcile on first metal build. **Authoring order:** WiFi-UDP → OTA → SX1262 LoRa; BLE deprioritized.
    **SX1262 = wrap a mature crate (lora-phy/sx126x) behind the LoRaRadio trait** (robustness > 'fully
    ours' for the greenfield longest-pole radio).
  - **⚡ FIRST LIGHT ACHIEVED** (`599f11b`, `docs/dfr1195-first-light-findings.md` + `dfr1195-firstlight.patch`).
    esp-hal **1.x** no_std firmware BUILDS (Alfred) → FLASHES (tuxedo ttyACM0 via SSH) → BOOTS → serial:
    "r2-dfr1195: FIRST LIGHT" + alive loop, booted from **OTA ota_0** (flashed WITH the 2-slot partition
    table → OTA-laid-out from first flash, Roy's req). **Descriptor blocker SOLVED:** esp-bootloader-esp-idf
    **0.5.0** (not 0.2.0) + esp_app_desc!(). Validated bare-metal matrix: esp-hal 1.1.1 / esp-alloc 0.10.0 /
    esp-backtrace 0.17.0 / esp-println 0.15.0 / esp-bootloader-esp-idf 0.5.0. Done in a git **worktree**
    (`~/Development/R2/dfr1195-fw-wt`); patch handed to core. **NEXT (WiFi-UDP + OTA-receiver tier):** resolve
    the **embassy conflict** (esp-hal-embassy 0.9.1 ↔ esp-hal 1.1.1 `__esp_hal_embassy`) → re-enable seam
    modules (wifi/ble/lora/peers) + esp-wifi 0.15.1 + embassy-net, wire core's WifiTransport/STA/Stack. OTA
    *receiver* (makes flash #2+ wireless) needs this WiFi tier — until then updates are USB.
  - **⚡⚡ PROOF SURFACE WORKING on BOTH boards** (`876bb98`, `docs/dfr1195-proof-surface-learnings.md`).
    LCD + LED running on ttyACM0 (rev v0.1) AND ttyACM1 (rev v0.2). **LCD (ST7735S):** status line on top +
    event log scrolling up; 20MHz SPI, mipidsi 0.9, offset(26,1)/Deg90/inverted. **KEY find: GPIO48
    controller power is ACTIVE-LOW** (HIGH = backlit-but-dead; cost a debug cycle — in the board profile).
    **LED (mono GPIO21):** gentle heartbeat "lub-dub" = all-well (visible even when screen off). Pins:
    MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48(active-low); LED21; btn18/btn0. **PUSHED to composer via
    supervisor** to create TWO general device-SPANNING capabilities + StatusDisplay sentant: display plugin
    (ST7735S driver, contracted ed50505) + **LED indicator plugin (NEW** — mono/rgb/canvas per-board, pattern
    vocab all-well/ota/joining/error/identify; Roy: LED signals status when screen down). hive owns device
    drivers (display+LED heartbeat done; pattern-set + plugin-ization next); composer the sentant+catalogue;
    specs/core the general capability traits.
  - **r2.hw.led capability DRAFTED for specs/core** (`4a9f0dd`, `docs/r2-hw-led-capability-proposal.md`) —
    semantic CMD_SET_STATUS{status} vocab (ok/joining/ota/error/identify/idle — meanings not blink-codes);
    descriptor kind:mono|rgb + statuses + dimmable + (rgb) colour slots; device driver maps status→rendering.
    **CRITICAL (Roy): LED INDEPENDENT of display** — firmware-direct base statuses (boot/ota/error) signal
    when the screen is down → don't route LED via the render plugin. **Firmware TODO:** init the LED
    before/around the display + a panic→error pattern, so a display fault never silences the LED. Sent specs.
  - **PROJECT: LoRa heartbeat-SYNC ("fireflies")** (`33eac83`, `docs/lora-heartbeat-sync-design.md`) — Roy's
    next showcase: synchronise the LED heartbeats via sentants exchanging r2.sync.fire events over LoRa
    (pulse-coupled oscillators). **PREREQUISITE (Roy): both nodes on the SAME TG** (events are TG-scoped) →
    needs identity (workshop hive_id/NVS) + **r2-trust no_std verify** (group-HMAC on MCU, currently std) +
    R2-PROVISION join on MCU. Deployment-reality catch (refuter): synced firing = simultaneous half-duplex
    TX = collisions → TX jitter/desync so LEDs sync tight while radio announces spread. Gated on LoRa + TG
    tiers (both downstream). **Algorithm is host-prototypable NOW** (offered to supervisor: r2-harness-style
    convergence sim + tune ε/jitter/T + partition/heal; + a TN-sync conjecture for specs). composer owns the
    HeartbeatSync sentant.
  - **FIRST-LIGHT PASS DONE (board live!)** (`db33289`, `docs/dfr1195-first-light-findings.md`). Board on
    **tuxedo-os /dev/ttyACM0**; hive on **Alfred** (esp/Xtensa toolchain); passwordless SSH = build-on-Alfred
    /flash-on-tuxedo. **SILICON-confirmed esp32s3 rev v0.1 / 4MB** (espflash board-info — settles SoC for
    good). core's skeleton **BUILDS for xtensa-esp32s3** with 3 hive fixes (patch `docs/dfr1195-s3-validation.patch`):
    C6→S3 re-target; wifi.rs:139 embassy-net SocketAddrV4→IpEndpoint; source export-esp.sh
    (`~/Development/homelab/export-esp.sh`) for the Xtensa linker. esp-hal/esp-wifi/embassy matrix compiles
    clean (no footgun). **FLASH BLOCKED:** espflash 4.4.0 requires the ESP-IDF app descriptor; esp-hal 0.23
    doesn't emit it (no bypass). **FIX = core bumps skeleton to esp-hal 1.0 + esp-bootloader-esp-idf matrix**
    (API migration; core's call — flagged + patch handed). I re-validate on metal the moment core pushes.
    Coexistence on tuxedo OK (only /dev/ttyACM0, no service restarts; workshop's :21042 untouched).
    **MATRIX DISCOVERED (cargo search):** esp-hal **1.1.1**, esp-hal-embassy **0.9.1**, esp-wifi **0.15.1**
    (restructured around NEW **esp-rtos 0.3** scheduler), esp-bootloader-esp-idf **0.5.0**, esp-alloc 0.10,
    esp-backtrace 0.19, esp-println 0.17, + embassy-* bumps. esp-wifi 0.12→0.15 = near-rewrite of the
    controller/init bringup = **core's authored domain** → handed core the migration + matrix; **hive =
    fast metal-validator** (isolated git worktree `~/Development/R2/dfr1195-fw-wt` + board + esp toolchain
    ready; core pushes → I build+flash+report in minutes). core is ACTIVELY on the skeleton (4d15812 S3
    re-target + c4927bb LoRaRadio) — do NOT touch its live working tree; validate via the worktree.
  - DONE (unblocked prep): **2-slot OTA partition table** (`3ad44e1`, `docs/dfr1195-ota-partitions.md`) —
    critical-path gap #5, hive-owned. 4MB S3: ota_0/ota_1 @ 0x1E0000 (1.875MB) + nvs/otadata/phy, fits +
    128KB headroom. FirmwareSink::slot_capacity()=0x1E0000 → OtaReceiver TOO_BIG bound. Handed to core for
    integration into platforms/dfr1195 once S3-re-targeted.
  - **Part D4: LCD display PLUGIN** (Roy directive; post-first-light, NOT blocking). DFR1195 LCD =
    **0.96in color 160×80 = ST7735S** (DFRobot wiki); pins MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48.
    Roy's split: **hive = device-specific no_std ST7735S output plugin** implementing a **GENERAL display
    capability** (render trait + descriptor: res/color-format/has-backlight/has-power-cut) that **specs
    defines + core implements** (LoRaRadio-pattern); **composer = display SENTANT + view-model** (the WHAT,
    calm-tech glanceable). General/reusable for composer's catalogue, not test-specific. Contract Qs
    answered to composer (now the GENERAL `b32d47d` DISPLAY-PLUGIN-CONTRACT-PROPOSAL, supersedes LCD-only):
    one general 'display' capability + per-board driver selected by board.toml (LoRa-carrier pattern).
    **LOCKED contract (composer `ed50505`, confirmed — final):** MANDATORY device-agnostic core = **CMD_RENDER
    (r2_cbor int-keyed view-model) + CMD_CLEAR**. OPTIONAL + descriptor-gated **CMD_BACKLIGHT(level u8 0..255,
    0=off → GPIO16 PWM)** — sentant sends it only when descriptor.backlight != 0; my ST7735S driver implements
    it; driver MAY self-manage a calm-tech default (idle-dim/wake) when none sent. **power_cut (GPIO48) =
    driver-local via descriptor flag, no command.** DFR1195 descriptor: **ST7735S / 160×80 / RGB565 /
    backlight=dimmable / power_cut=yes**. General capability TRAIT + descriptor = specs/core to define +
    ratify (LoRaRadio pattern; converged ask from composer + me); composer view-model rides on top.
    **Driver impl sequences after esp-hal-1.1 first-light.**
- **PAUSED (Roy, pending UX feedback): storing-backend / BOS-on-R2.** Branch `storing-backend` —
  RecordStore seam skeleton landed + shelved-ready (`docs/storing-backend-hive-scoping.md`). Do NOT
  build further until Roy resumes. Resume point: SQLite-behind-the-seam + persistence ensemble.
- ~~TN refutation re-run~~ DONE (`2642263`) — core `da89050` wired the knobs; re-ran both vs r2-harness:
  TN-L2-XT-BL-001 (OOM guard, set_scf_buffer_cap+tail-drop) and TN-L2-XT-AB-001 (entanglement epoch) now
  DECIDABLE → CONFIRMED. Filed to specs+core with 2 deployment-lens refinements (tail-drop vs TTL-aware
  eviction; epoch/buffer RAM-volatility). Resolution addendum in docs/phase3-tn-refutation-batch3.md.
  Standing refuter duty otherwise idle (remaining L0/L1/L3 functional cells sweepable on request).
- ~~CONVERGENCE BLOCKER: R2-WEB v0.6 CSP drift~~ **RESOLVED** (`827295b`) — Roy ratified R2-WEB v0.6 csp;
  synced hive web.rs to `WebPluginManifest.csp = Option<CspPolicy>`: `MountedBundle.csp` → `CspPolicy`,
  `build_csp`→`render_csp` (renders the directive BTreeMap), `restrictive_default` defensive fallback,
  `DEFAULT_CSP` removed, tests + integration manifests updated. BIN builds vs core's current tree; full
  workspace green (17 blocks). SECURITY FLAG to specs: §3.4.1 restrictive_default dropped
  `frame-ancestors 'none'` (+base-uri/form-action) vs the pre-v0.6 hive default → unframed web UIs now
  clickjackable unless they author csp; suggested specs re-add it. **→ RATIFIED as R2-WEB v0.7**
  (specs 5553f80): restrictive_default restores frame-ancestors 'none'+base-uri 'self'+form-action 'self'
  + adds script-src 'wasm-unsafe-eval'. `restrictive_default()` is **r2-def's (core)** — hive web.rs only
  CALLS it, so hive INHERITS the fix automatically once core updates r2-def (flagged core; no hive code
  change for the default). **hive v0.7 follow-ups (low pri, behind firmware lead):** (a) re-add the
  `frame-ancestors 'none'` assertion to web_plugin_integration test once core's restrictive_default emits
  it; (b) connect-src `+ws` serve-time append (render_csp adds hive's live WS origin when serving).

## Done + green
- **v0.2 migration + relay handshake + 4 vector fixtures** — full r2-hive suite GREEN; on
  `v0.2-relay-handshake` (pushed). Fixtures all specs-verified + landing: host-api (28),
  usb (specs), usb-pair (12 → canonical home **R2-PROVISION §5.3.4**), plugin-web (11, Ed25519).
  Generators: `crates/r2-hive-bin/examples/gen_{host_api,usb_pair,plugin_web}_vectors.rs`.
- **core D3a synced + relay driver CONFIRMED** (`3c5ba9c`) — core's WebSocketTransport §4.4.1 fan-out +
  UDP-LAN are now REAL (core `52b0e4e`). hive's relay driver (`compat/handshake.rs`: v0.1/v0.2 Ed25519
  handshake → `peers().connect()`→OutboundRx, `push_inbound` on recv, drain `outbound_rx.next()`→ws.send,
  `remove_peer` on cleanup) builds + runs GREEN against the real machinery (was scaffold). One core
  API-drift fix: `WebPluginManifest.subscriptions` added to 3 test manifest builders. Full suite green.
- **Transport + router integration tests** (`11443cf`,`828b419`) — filled a zero-coverage gap now that
  core D3a transports are real. `tests/transport_integration.rs` (3): HiveState send path round-trips
  over REAL loopback UDP-LAN sockets (set_udp_transport + send_to_hive_via → Wifi slot), no-transport→None,
  Wifi-hint routing. `tests/router_integration.rs` (5): route_frame NotR2Wire rejection, the 32-byte
  HMAC-tag trim fallback, valid-frame routing, and engine dedup (seeded neighbour → flood then dup-drop).
  Transport layer now VERIFIED working against core's real machinery, not just compile-green.
- **USB spec citations resolved** (`4c70d2c`,`8f31231`) — usb_pair/usb/main/usb_serial/usb_hotplug/api.rs
  all R2-HIVE §6.4.x → R2-PROVISION §5.3.4 (specs ruled it the canonical pairing home); R2-USB v2→v0.1.
  Type-byte divergence: specs RULED **ratify** as R2-USB §3.2.1 (don't drop; collision-free). Both
  wire extracts (type-byte table + CAPS + legacy detection; PAIR_* msg vocab + CBOR layout) committed
  `docs/r2-usb-wire-extract-for-specs.md` (`5232e61`) + sent to specs. Spec authoring is Roy-gated.

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
- DONE: **Platform + transport seams migrated into r2-hive-core** (`234fd60`) — `Platform` trait
  (clock+RNG) → `core/src/platform.rs` (no_std), `LinuxPlatform` impl stays in bin + re-exports trait;
  `HiveTransports` outbound seam → `core/src/transport_seam.rs` (async-trait, no_std+alloc, needs
  `alloc::boxed::Box`), `HiveState` impl + `&dyn` trait-object test stay in bin (`hive.rs`).
  r2-hive-core builds no_std; full workspace green (100 bin lib + 6 core tests). Pushed.
- DONE: **storage seam migrated into r2-hive-core** (`b42658c`) — `core/src/identity.rs` (no_std+alloc):
  `MasterSecret` derivation (HKDF-SHA256 → hive_id/DEV_PK/DEV_SK), `DerivedIdentity`, fingerprint, UUIDv4,
  web-auth-key + the seam itself (`IdentityStore` trait, `StoreBackend`, platform-neutral `StoreError`
  replacing `io::Error` at the trait boundary). bin keeps std stores (`FileStore`/`KeyringStore`/
  `auto_store` + permissions/XDG/getuid), impls the core trait (io→StoreError), re-exports core types
  (mgmt::identity::* unchanged). RNG stays platform-side (getrandom→`from_bytes`); `bytes()` →
  documented storage-only `expose_secret_bytes()`. ed25519-dalek/hkdf/sha2/zeroize added to core
  default-features=false. r2-hive-core no_std; full workspace green (94 bin lib + 13 core tests).
- DONE: **OTA-receiver seam in r2-hive-core** (`354f395`) — `core/src/ota.rs` (no_std), the portable
  half of the firmware receiver: constants (OTA_PORT 21043/CMD_*/STATUS_*/PREAMBLE_LEN),
  `OtaPreamble::parse` (image_len u32 LE + sha256[32]), `OtaError` CODEs (PREAMBLE/TOO_BIG/BAD_MAGIC/
  SHA_MISMATCH/WRITE_FAIL/NO_SLOT/SHORT) + alloc-free `encode_reply/ok/error`, `FirmwareSink` trait
  (storage seam = flash I/O), `OtaReceiver` state machine (TOO_BIG bound-check BEFORE begin, streaming
  SHA-256, verify→finalize, abort-on-error). NOT a migration (no OTA code existed in bin) — built from
  core's `platforms/esp32/src/ota_tcp.rs` reference + composer's OTA-REPLY-STATUS-CONTRACT. 11 tests.
  Heads-up sent to composer to confirm CODE set / push-side framing. **Platform supplies:** embassy-net
  byte reads + esp-storage `FirmwareSink` impl (device); host uses a RAM mock. CMD_QUERY handled by
  platform layer (build info), not core.
- NEXT: with routing/identity/OTA cores all no_std + **5 seams** in place (sync_host, platform,
  transports, identity, ota), the convergence's host-side factoring is largely done. Remaining is
  firmware-tier (gated): swap `sync_host` seam mirror → `r2_transport::` when core EXTENDs r2-transport
  (poll_recv default-None + TransportAddr/InboundFrame); esp-hal/embassy board crate (P0) + esp-storage
  FirmwareSink + embassy-net OTA host loop (needs xtensa toolchain + hardware + core D3b).

## Next major phase — D2: DFR1195 (ESP32-S3) firmware, Path B pure no_std (esp-hal/embassy)
Gated on the convergence above + core's D3b. Sketch: `docs/esp32-hive-firmware-architecture.md`.
- Firmware = core's no_std stack + core's **D3b** no_std SYNC radio bindings, wrapped in an
  esp-hal/embassy host loop. Consume **R2-TRANSPORT SYNC** (R2-DISCOVERY §5), not async §4.
- hive owns: board layer (SX1262 LoRa / LCD / IO18 button), on-device host loop, **no_std OTA
  receiver** (embassy-net; std `ota_tcp.rs` is reference only). **Validation handoff:** core
  authors D3b but can't flash — **hive validates on real DFR1195**, feeds defects back.
- **Identity:** my firmware CONSUMES the shared `r2-esp/hive_id` module (workshop-owned, one impl per
  north-star) — incl. the agreed `usb_link_id = HKDF(master_secret,"r2-usb-link-v1")` (stable USB-link
  id) / `mesh_hive_id = HKDF(master_secret,info=tg_id)` split. Do NOT fork a parallel derivation. Gated
  on specs ratifying R2-USB §3.6 (workshop holds the change until then).
- Near-term scope flag: r2-def/ensemble/dispatch are std-tier → initial MCU hive is
  ROUTING+TRANSPORT only (no on-device ensembles) until those are re-tiered no_std.
- References (std, patterns not code): core `platforms/esp32`, workshop `firmware/esp32-s3`.

## Pending Roy / cross-repo
- **OPEN — CAPS device-identity gap: CONFIRMED REAL, fix agreed, spec-first** (awaiting specs §3.6
  authoring, Roy-gated). ROOT CAUSE (workshop firmware answer): ESP32 derives `hive_id_bytes =
  HKDF(master_secret, info=tg_id)` = TG-SCOPED, and the SAME 16 bytes feed CAPS §3.6 + my link-key store
  key + reconnect HMAC + mesh hive_id (§6.2.1). Cross-TG provisioning → different value → my LinkKeyStore
  (keyed solely on CAPS hive_id_bytes) misses → silent forced re-pair. AGREED FIX (workshop owns,
  r2-esp/hive_id.rs): split — `usb_link_id = HKDF(master_secret,"r2-usb-link-v1")` STABLE/TG-indep → CAPS
  + link-key store; `mesh_hive_id = HKDF(master_secret,info=tg_id)` → mesh. **My host needs ZERO change**
  (store keys on whatever stable CAPS id arrives). PROPOSED NORMATIVE RULE relayed to specs: CAPS
  hive_id_bytes MUST be stable for device life + TG-independent; mesh hive_id (§6.2.1) is separate →
  R2-USB §3.6 + R2-WIRE §6.2.1 cross-ref; composer also a consumer (provisioning/OTA). workshop HOLDS
  firmware change until specs ratifies §3.6 wording. Minor: dev devices paired pre-fix do a 1-time
  re-pair (harmless pre-launch). eFuse-MAC comment already marked impl-defined-pending-spec (`b33547f`).
- ~~Roy: greenlight R2-PROVISION §5.3.4~~ DONE — specs confirms COMMITTED (`4b74b20`, v0.6, Roy
  green-lit) on `spec-conformance-v0.2`. Cite by paragraph name (no §5.3.4.y sub-numbers).
- ~~hive TODO: usb_pair.rs citation fix~~ DONE (`4c70d2c`) — usb_pair.rs §6.4.x → R2-PROVISION
  §5.3.4 (SAS verification/Link key/Reconnect/Key agreement); main.rs+usb_serial.rs "R2-USB v2" →
  "R2-USB v0.1", SYNC frame → §3.3. Doc-only; builds clean.
- ~~OPEN: type-byte divergence + usb.rs frame-vocab mapping~~ **CLOSED — RATIFIED + VERIFIED.** specs
  authored all three (`71ee053` spec-conformance-v0.2, Roy-authorized): **R2-USB v0.2** §3.3 version
  negotiation / §3.5 type byte / §3.6 CAPS / §3.7 control + Appendix A transport kinds; **R2-PROVISION
  v0.7 §5.3.4** message vocabulary (PAIR_* 4-11). I VERIFIED both against usb.rs — all bytes match (CAPS
  keys, msg fields, nonce_rc/tag b16, abort vocab exact 8-match). **Both normative tightenings specs
  added were ALREADY honoured by the impl:** (a) failed reconnect does NOT fall back to first-attach
  (`usb.rs:846-848` → fail_pairing→Closed); (b) AutoPairUnsafe NOT default (Strict default; dev-only
  ctor used only in tests; prod watcher `usb_hotplug.rs:590` = Strict). usb.rs cites finalized
  (`12c6a43`): 'pending ratification' dropped, framing→§3.5-3.7, pairing→§5.3.4. Impl is now CANON.
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
  → 0.05*(1-c) underflow (TN-L2-IT-BL-101). DYN batch ADJUDICATED by specs (`a9c28b1`): 3 new
  R2-ROUTE issues (8→11) — additive-vs-multiplicative BLOCKED+Roy-gated, RSSI-sigma re-tiered
  HARDWARE, expf/fixed-point forward-flagged.
  **BATCH 3 FILED** (`d161054`, docs/phase3-tn-refutation-batch3.md) — un-refuted SCF + XT/entanglement
  cells, grounded in real r2-route + r2-harness code. Key: RouteEngine has NO buffer/queue/entanglement
  (ForwardAction lacks a Queue variant; no-path → Drop(NoViableNeighbour) = silent drop); entanglement
  is SIM-ONLY (r2-harness live:bool, honesty #6; r2-trust §7 = no keep-alive/@entangled routing).
  Verdicts: TN-L2-IT-BL-002 FALSIFIED (no queue); TN-L2-IT-AB-000 FALSIFIED for carry>60s dedup;
  TN-L2-XT-BL-001 OOM-guard not sim-decidable (re-tier hw); all XT-AB cells test sim gate not
  authenticated crossing (passes-while-violating-spirit); BL-101 CONFIRM / BL-100 FALSIFY (no
  heartbeat → entangled-but-unreachable on duty-cycled links); XT-AB-001 undecidable (no instance id);
  XT-BL-100 'kept' conflicts w/ 30min route eviction.
  **BATCH 3 ADJUDICATED** (supervisor, verdict-of-record; catalogue write pending perm): IT-BL-002
  ACCEPT-FALSIFIED → R2-ROUTE #7 (MUST → named SCF layer, DUAL bound RAM×TTL; engine silent-Drop OK at
  routing layer); IT-AB-000 ACCEPT-FALSIFIED → operative rule = IT-AB-001 (idempotency at dispatch);
  IT-BL-000/XT-BL-000 = PRODUCTION-UNREALIZED (sim tests logic only, lifts no impl signal); XT-BL-001
  ACCEPT not-decisive → experiment revised (inject buffer cap; true OOM=hardware); XT-AB cells honesty-#6
  (authenticated-crossing MUSTs deferred to r2-trust §7 production); **XT-BL-100 entangled-but-unreachable
  = HEADLINE** → BLOCKED impl-missing (§7.3 keep-alive DEFINED-unimplemented); 3 Roy options, supervisor
  recommends implement §7.3 minimal keep-alive (decay-exemption REJECTED-leaning — contradicts BL-101);
  XT-AB-001 ACCEPT sim-undecidable → instance/epoch id (harness + R2-TRUST §7.6, Roy-gated); XT-BL-100
  NOT-falsified CLARIFIED (record-retention §7.3 vs route-eviction R2-ROUTE 2.5 both defined, no conflict).
  Remaining open cells: IT/XT main-path L0/L1/L3 functional cells (lower deployment-lens value) on request.

## Resume hygiene
Keep this current. WIP-checkpoint + push `platform-trait` periodically. Safe git only:
named `git add` / `git add -u` — never `git add -A`/`.`; never stage secrets.
