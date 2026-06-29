# RESUME — r2-hive (hive-worker)

## ► 2026-06-30 — PER-HOP RX TRANSPORT TELEMETRY (supervisor-elevated, core test dep) — DONE+GREEN
Firmware `dfr1195-fw` at `a2f1718`; r2-hive recovery patch refreshed at `2108576`. Supervisor elevated per-hop
transport-tagged telemetry from path-animation polish to a CORE TEST DEPENDENCY (the bench must visualise REAL
link-strength-through-usage, which only real observed per-hop traffic can drive). Observability only — no spec gate.
- **What landed (Phase A):** `msg.rx` now emits `{0:id,1:at,2:from_hop,3:origin,4:transport}`. New key `4` =
  the `r2_route::Transport` ordinal of the carrier the frame was RECEIVED on. Numbering is the canonical 7-bit
  space (`transport.rs`: Ble0/Wifi1/Lora2/Internet3/Usb4/EspNow5/Udp6 == `transport_allow_mask` bits), so bench
  per-link counts map 1:1 to host mask semantics.
- **Tap (core-confirmed):** all inbound radios coalesce through one `DATA_RX` channel — so the RX carrier was
  being lost there. Added a 4th `MeshRxFrame` field stamped per-feeder (espnow_task=EspNow, lora_task=Lora,
  blemesh CoC=Ble) + the io_task UDP select-arm=Udp; threaded to `emit_msg` k4. This is core's flagged
  handle_rx/DATA_RX site. NO wire/on-air change; the tag never re-enters the air.
- **Why rx-side is sufficient:** every received frame = one real `(from_hop, transport)` link traversal, so
  rx counting fully measures traffic crossing each link (Roy's link-strength-through-usage signal) with no
  multi-carrier ambiguity. `emit_msg` change is ADDITIVE (keys 0-3 unchanged) → composer's `/r2` parser keeps
  working and adopts k4 when ready.
- **Build-verify:** `cargo build --release` GREEN (xtensa esp32s3) across `routetest` / `loraroute` / `blemesh`
  / `nobt` / default — covers all three feeders + both sides of the routetest gate.
- **Caveat:** `msg.*` telemetry is `routetest`-gated (the regime composer's bench runs in). Broadening to ALL
  traffic is a separate, more invasive scope call — flag before doing it.
- **Phase B (scoped, NOT built):** egress-carrier tag on `msg.tx`/`msg.relay` (per-carrier emit in
  `mesh_broadcast`, since a bridge fans out ESP-NOW+LoRa). Only needed if the bench wants the SEND-side carrier;
  rx-side already counts every link. Also pending: composer's item (2) per-device transport-mask ENFORCEMENT hook
  at the DATA_RX/handle_rx seam (waits on core's runtime mask shape + composer ping).
- **Coordination:** notified supervisor (done), composer (the exact k4 shape for the /r2 parser), core (tap +
  numbering confirm; offered BIT vs ordinal). Do not assume composer has adopted k4 yet.

## ► 2026-06-30T06:26:56+12:00 — DOCTOR-ONLY FINAL IDLE REFRESH
Objective: doctor-only durable handoff refresh after stopped-lane fleet activity. No code/content edits; update
`RESUME.md` only if ground truth shows stale current state, then commit/push and idle.
- **Branch/HEAD/worktree:** r2-hive is on `platform-trait`, clean and in sync with `origin/platform-trait`.
  The authoritative current HEAD is whatever `git rev-parse HEAD` / `origin/platform-trait` shows — do NOT
  trust any frozen hash written in this file, since each RESUME refresh is itself a doc-only commit that
  advances HEAD. The recent chain of doc-only hygiene commits is
  `a10d63f`→`18e3b1c`→`e422250`→(this refresh); none of them touched repo source. The substantive firmware
  work lives in the sibling `dfr1195-fw` worktree, not here.
- **Firmware worktree state:** `/home/roycdavies/Development/R2/dfr1195-fw-wt` is on `dfr1195-fw` at
  `54973b9ba17a` (`feat(dfr-ota): R2/R3/R4 OTA-receiver hardening (specs-sanctioned)`), matching
  `origin/dfr1195-fw`, with exactly one dirty file: `M docs/dfr1195-firstlight.patch` inside that sibling
  worktree. No platform source diff was observed there this turn. Do not "clean" that core-owned worktree from
  r2-hive.
- **Transport allow-mask status:** implemented in r2-hive host/sync/local-mgmt and currently verified. Tracked-file
  check shows `crates/r2-hive-bin/src/mgmt/transport_policy.rs`, `mgmt/api.rs`, `mgmt/mod.rs`,
  `crates/r2-hive-bin/src/hive.rs`, `crates/r2-hive-core/src/sync_host.rs`, and the focused integration tests are
  all tracked. `rg` confirms `mgmt/mod.rs` exports `transport_policy`, `mgmt/api.rs` dispatches
  `r2.mgmt.transport.allow_mask.{state,set,clear}`, `HiveState` delegates the effective mask to
  `route_engine.transport_allow_mask()`, and host sends check the mask before physical egress. Targeted gates run
  at current HEAD all PASS:
  `cargo test -p r2-hive-core route_respects_transport_allow_mask_before_sync_send -- --nocapture`;
  `cargo test -p r2-hive-core route_drops_when_mask_removes_only_sync_candidate -- --nocapture`;
  `cargo test -p r2-hive --test transport_integration transport_allow_mask_filters_host_send_before_physical_egress -- --nocapture`;
  `cargo test -p r2-hive --test mgmt_integration transport_allow_mask_mgmt -- --nocapture`. Only observed warning:
  pre-existing `r2-wire` dead-code warning for `EXT_AUTH_MAX`.
- **DFR/ESP32 patch + partition status:** r2-hive `docs/dfr1195-firstlight.patch` still byte-matches
  `git -C /home/roycdavies/Development/R2/dfr1195-fw-wt diff c46383e..HEAD -- platforms/dfr1195/Cargo.lock
  platforms/dfr1195/Cargo.toml platforms/dfr1195/build.rs platforms/dfr1195/src/main.rs
  platforms/esp32/sdkconfig.defaults`, and reverse-apply check in the firmware worktree PASSes. Source config
  remains custom-partition canonical: `platforms/esp32/sdkconfig.defaults` has
  `CONFIG_PARTITION_TABLE_CUSTOM=y`, `CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"`, and
  `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`; `platforms/esp32/partitions.csv` has `otadata@0xf000`,
  `ota_0@0x20000 size 0x1E0000`, and `ota_1@0x200000 size 0x1E0000`. Generated ESP-IDF `out/sdkconfig` also
  shows rollback enabled, anti-rollback not set, `TWO_OTA` not set, and custom table enabled. The prior ESP32
  build artifact still exists:
  `platforms/esp32/target/riscv32imac-esp-espidf/release/r2-esp32` = 3,698,964 bytes, mtime
  `2026-06-28 07:50:37 +1200`. I did NOT rerun the ESP32 build this turn; current `esp-idf-sys` output has no
  copied `out/partitions.csv`, so the known custom-partition copy race/workaround is still a real build caveat.
- **Known external-gated items / no local code-only action:** ESP32/DFR OTA confirmed-boot and rollback still need
  metal/network validation; radarprobe remains blocked on Roy-side physical/model facts (continuity RO->GPIO44,
  DI->GPIO43, DE-RE->GPIO6, MAX485 5V/GND, radar model/datasheet); CCR1 remains composer-contract/emitter gated;
  ESP-IDF custom partition handling still needs a portable fix or documented repeatable workaround; transport
  allow-mask firmware role-profile ingestion, per-hop telemetry tags, and bench metal validation were not added by
  the host/sync/mgmt patch and remain scoped to later contract/bench work. Do not re-adopt ESP-IDF
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` unless the image shrinks below 1 MiB or a different built-in table is proven.
- **Paused-branch note:** `crates/r2-hive-core/src/record_store.rs` is not part of current `platform-trait`; it
  belongs to the paused `storing-backend` branch at `478203a`. Treat any RecordStore seam notes as branch-scoped
  unless that branch is explicitly resumed.
- **Verification this turn:** `git status --short --branch`; `git log -5 --oneline --decorate`; `date -Iseconds`;
  focused `git ls-files`/`rg` wiring checks; the four targeted cargo tests above; sibling firmware
  `git status`/`git log`; patch `cmp` byte-match and reverse-apply check; ESP32 sdkconfig/partition/artifact
  inspection; `fleet inbox | tail -80` confirming the doctor-only refresh request. No full workspace test or fresh
  ESP32 build was run because this is a RESUME-only doctor refresh.

## ► 2026-06-30 — DOCTOR HYGIENE / MARKER WORDING CLEARED
Objective: resolve fleet-doctor handoff hygiene only: inspect stale marker wording in `RESUME.md`, verify the
old `transport_policy.rs` untracked/unwired blocker against disk, and avoid code changes. Result: **DOC HYGIENE
ONLY** on branch `platform-trait`; pre-edit worktree was clean at `41eed45`.
- **Transport-policy blocker status:** resolved. Ground truth: `git ls-files --stage` shows
  `crates/r2-hive-bin/src/mgmt/transport_policy.rs`, `crates/r2-hive-bin/src/mgmt/api.rs`, and
  `crates/r2-hive-bin/src/mgmt/mod.rs` are all tracked. `rg -n "transport_policy|TransportPolicy|transport policy" .`
  shows `mgmt/mod.rs` exports `pub mod transport_policy;`, `mgmt/api.rs` imports it and dispatches
  state/set/clear event classes to it, and integration tests reference the same module.
- **Marker cleanup:** replaced remaining stale marker wording in old handoff notes with concrete
  `follow-up`, `remains open`, or completed-task language. No active technical follow-up was removed; the FR-2
  firmware work, AP-failover WiFi layer, and LED-init work remain recorded as open where they were already
  described.
- **Changed files:** `RESUME.md` only.
- **Verification:** narrow doc checks only: `git status --short --branch`; fleet-doctor marker scan of
  `RESUME.md`; `rg --files | rg 'transport_policy\.rs$|transport_policy'`; tracked-file check via
  `git ls-files --stage`; wiring check via `rg -n "transport_policy|TransportPolicy|transport policy" .`. No
  cargo tests are needed for this docs-only hygiene change.
- **Do not assume:** this entry does not re-verify the previously green transport-policy cargo gates, metal bench,
  or firmware patch application; it only records current tracked/wired handoff state and removes stale marker
  wording.

## ► 2026-06-29 — BENCH PHASE-2 TRANSPORT-DISABLE WIRING / IMPLEMENTED+GREEN
Objective: wire the now-unblocked Phase-2 node-wide egress transport software-disable policy in r2-hive without
inventing hive-local routing semantics, then verify and push. Result: **IMPLEMENTED** against core's canonical
`r2_route` API on branch `platform-trait` (pre-work HEAD `852e03b`; this RESUME entry is in the transport-policy
implementation commit).
- **Verified authority before coding:** r2-specifications clean on `spec-conformance-v0.2` at
  `45b8a507e731aeeaae124f263f0809c4116502c5`; R2-TRANSPORT §2.3A says `transport_allow_mask` is `0x7F`
  default all-on, node-wide, egress-only, disable-only, leased/acknowledged/clearable, local-authority-only by
  default, not gossiped/mesh-written; R2-ROUTE §5.2 applies it as a hard candidate filter before scoring;
  R2-RUNTIME §3.2.2 lists it as an optional role-profile knob. r2-core clean on `r2-core-consolidation` at
  `7c0320eaa9ca49e26dcb2d4ae4fb27fd6af405cb`; `c2737b9` exposes
  `RouteEngine::{transport_allow_mask,set_transport_allow_mask_bits,clear_transport_allow_mask,set_transport_allowed,transport_allowed}`
  over the canonical 7-bit `TransportSet`, and `DataPlane` delegates to the same surface. No r2-core files were
  edited.
- **Host/state wiring:** `HiveState` now keeps only local ACK/state lease metadata; the effective policy remains
  single-sourced in `route_engine.transport_allow_mask()`. Added `transport_policy_snapshot`,
  `set_transport_policy_lease`, and `clear_transport_policy`. `send_to_hive_via` now snapshots the core allow mask
  and skips disabled transports before any physical WS/UDP/BLE/LoRa/USB-dongle send attempt. This covers local
  sends that do not pass through `RouteEngine::plan_forward` first; route-engine planned egress already gets the
  core hard filter before scoring.
- **Mgmt surface (local only, no mesh mutation):** new UDS/loopback mgmt event classes:
  `r2.mgmt.transport.allow_mask.state`, `.set`, `.clear`. Requests are R2-WIRE extended frames with CBOR payloads:
  `state {0:cid}`; `set {0:cid,1:mask_uint8,2:lease_id_uint,3:source_text}`; `clear {0:cid,1:lease_id_uint?}`.
  Set ACK returns `{0:cid,1:requested_mask,2:accepted_mask,3:effective_mask,4:all_mask,5:lease_id,6:source,7:true}`.
  State/clear return `{0:cid,3:effective_mask,4:all_mask,7:active_bool}` plus lease fields `{1,2,5,6}` when active.
  Unknown bits are acknowledged via core truncation (e.g. requested `0x82` → accepted/effective `0x02`). A second
  different lease gets `r2.mgmt.event.error` code `lease_conflict`; clearing without a lease id is the local
  force-clear. Mgmt-only daemon state returns `unsupported` rather than silently unknown.
- **Sync/no_std proof:** `r2-hive-core::sync_host::route_inbound_sync` still delegates to the caller's
  `RouteEngine`; focused tests set the core mask directly and prove (a) masked higher-scoring LoRa is not sent
  while WiFi remains viable, and (b) a masked only-candidate drops without egress. No firmware source or
  `docs/dfr1195-firstlight.patch` changed; firmware/host boundaries preserved.
- **Changed files:** `crates/r2-hive-bin/src/hive.rs`,
  `crates/r2-hive-bin/src/mgmt/{api.rs,mod.rs,transport_policy.rs}`,
  `crates/r2-hive-bin/tests/{mgmt_integration.rs,transport_integration.rs}`,
  `crates/r2-hive-core/src/sync_host.rs`, and `RESUME.md`.
- **Verification:** targeted tests PASS:
  `cargo test -p r2-hive-core route_respects_transport_allow_mask_before_sync_send -- --nocapture`;
  `cargo test -p r2-hive-core route_drops_when_mask_removes_only_sync_candidate -- --nocapture`;
  `cargo test -p r2-hive --test transport_integration transport_allow_mask_filters_host_send_before_physical_egress -- --nocapture`;
  `cargo test -p r2-hive --test mgmt_integration transport_allow_mask_mgmt -- --nocapture`.
  Full gate PASS: `cargo test --workspace` (105 r2-hive lib tests, 20 mgmt integration tests, 4 transport
  integration tests, all other workspace tests/doc-tests green; one pre-existing ignored router authenticated-dedup
  fixture remains ignored). `git diff --check` PASS. `cargo fmt --all --check` is NOT a valid repo-local gate today
  because it tries to format/check the sibling `r2-core` path dependency and reports pre-existing r2-core rustfmt
  drift; the new `transport_policy.rs` was rustfmt'd directly and unrelated rustfmt churn was reverted.
- **Refutation / peer challenge:** asked core for an adversarial API/semantics check. The direct off-thread answer
  hit the provider spend-limit message, but supervisor relayed the peer-review result: specs-codex found no spec
  gaps; core-codex found one concrete WIP blocker, to ensure `transport_policy.rs` is tracked and that `mgmt/mod.rs`
  + `mgmt/api.rs` dispatch it. That blocker is resolved by the final staged file set before commit.
- **Composer/bench next endpoint:** composer should drive the local UDS management socket (default
  `r2_hive::default_socket_path()`, usually `$XDG_RUNTIME_DIR/r2-hive.sock` or `/tmp/r2-hive-<uid>.sock`) with
  `r2.mgmt.transport.allow_mask.set {0:cid,1:mask,2:lease_id,3:"composer:bench-phase2"}`. For "disable LoRa only",
  send mask `0x7B` (`0x7F & !Transport::Lora.bit()`). Clear with
  `r2.mgmt.transport.allow_mask.clear {0:cid,1:lease_id}` or omit key `1` for local force-clear. Do not send this
  as a mesh `r2.api.event.send`; mesh-received frames intentionally do not mutate the policy.
- **Do not assume:** this is host/sync/mgmt enforcement only. No firmware role-profile ingestion of
  `transport_allow_mask` was added in this patch, no per-hop telemetry tags were added, and no metal bench was run
  because no core-crate pin/bump or firmware artifact changed in r2-hive.

## ► 2026-06-29 — BENCH PHASE-2 TRANSPORT-DISABLE RECHECK / BLOCKED-ON-HIVE-CALLABLE CANONICAL API
Objective: re-check the stale transport-disable hold after specs/core landed the Phase-2 policy commits, then either
wire the smallest hive integration or record the precise blocker. Result: **NO HIVE CODE WIRING YET**; the spec is
now ratified locally, and core has a lower-level `r2-dataplane` `PhyMask` setter, but current hive code has no
callable canonical 7-transport policy surface without inventing a hive-local clone.
- **Verified local ground truth:** r2-hive `platform-trait` was at `eeee933` with only this `RESUME.md` dirty;
  r2-specifications was clean on `spec-conformance-v0.2` at `45b8a507e731aeeaae124f263f0809c4116502c5`;
  r2-core was clean on `r2-core-consolidation` at `c5d0be8df05e99c2fa9f9540400752f29890e7f6`. The DFR firmware
  worktree remains `dfr1195-fw` at `54973b9` with only its nested `docs/dfr1195-firstlight.patch` dirty, so do not
  assume that worktree already tracks core `c5d0be8`.
- **Spec surface now landed:** `d55577c` adds R2-TRANSPORT §2.3A `transport_allow_mask` over the canonical §2.2
  7-transport bitmask (`0x7F` all-on), node-wide, egress-only, disable-only, leased/acknowledged/clearable, local
  authority by default, and not advertised/gossiped. R2-ROUTE §5.2 now says the mask is a hard filter before
  scoring. R2-RUNTIME §3.2.2 adds optional role-profile `transport_allow_mask`.
- **Core surface now landed:** `4ca1364` adds `r2_dataplane::{PHY_FLRC, PHY_LORA, PHY_ALL}` and
  `DataPlane::{egress_enabled_mask,set_egress_enabled_mask,set_egress_phy_enabled,egress_phy_enabled}`. The mask
  is applied inside `r2-dataplane` to `handle_rx_frame` relay output and `poll_keepalive` output, and it strips
  unknown bits. This is lower-level physical-carrier policy (`PHY_ALL == PHY_FLRC|PHY_LORA`), not the canonical
  `Transport` `0x7F` mask by itself.
- **Blocker verified in code:** `rg` over current core found no `transport_allow_mask`, route-engine policy setter,
  or `select_transport`/`RouteEngine::plan_forward` parameter for the 7 canonical `Transport` bits. `r2-route`
  still selects from `NeighbourEntry.transports`, MTU, link quality, and strategy only. Current r2-hive does not
  depend on `r2-dataplane` in its host crates; `rg r2_dataplane` in r2-hive hits only a process-hygiene comment and
  the firmware patch artifact. The DFR firmware source imports only `encode_dc_seq_cbor`, `frame_fingerprint`,
  `parse_dc`, and `parse_seq` from `r2_dataplane`; it does not instantiate `DataPlane`, `handle_rx_frame`, or
  `poll_keepalive`, so there is no existing object to call the new setter on.
- **Why no hive patch this turn:** wiring Linux/cloud `HiveState::send_to_hive_via` or
  `r2-hive-core::sync_host::route_inbound_sync` would require a new hive-owned 7-bit mask/lease manager and a
  mapping to `Transport::{Ble,Wifi,Lora,Internet,Usb,EspNow,Udp}` outside core's landed API. Wiring the DFR patch
  directly would require either migrating the firmware io loop onto `r2_dataplane::DataPlane` or fabricating a
  local `Transport`→`PhyMask` policy adapter. Both would create semantics the user explicitly barred.
- **Smallest unblocked path once core/supervisor picks it:** either (A) core exposes the canonical
  `transport_allow_mask` as a shared policy type/manager and route/host filter API over `r2_route::Transport`
  bits, then hive wires `HiveState`, `sync_host`, UDS/loopback mgmt ACKs, tests, and role-profile ingestion; or
  (B) firmware first migrates the DFR io path to the landed `r2-dataplane` two-entry-point contract, then hive can
  set `DataPlane::set_egress_enabled_mask()` at the physical-carrier boundary and separately reconcile the
  spec-level `Transport` mask mapping. Until then, keep the policy local-only; mesh-received frames MUST NOT
  mutate it.
- **Peer/refutation:** asked core whether a host-wide `Transport` policy API exists or whether only the
  `DataPlane` `PhyMask` setter landed; the off-thread answer was the provider spend-limit message, so no peer
  challenge was available. Confidence is from local disk inspection only.
- **Verification this turn:** `git status --short --branch` in specs/core/hive; `git show --stat` for
  `d55577c`, `45b8a50`, `4ca1364`, `c5d0be8`; spec reads of R2-TRANSPORT §2.3A, R2-ROUTE §5.2, and R2-RUNTIME
  §3.2.2; targeted `rg`/`sed` inspections of `r2-dataplane`, `r2-route`, hive `HiveState`, hive `sync_host`, and
  the DFR firmware worktree. No cargo tests were run because this turn intentionally makes a docs/handoff-only
  blocker update.
- **Changed files:** `RESUME.md` only. Do not add hive-local transport-mask semantics or mesh-remote control
  frames to bypass the missing shared API.

## ► 2026-06-28 — DFR FIRMWARE PRE-METAL HARDENING (refutation-review items, supervisor GO) — DONE+GREEN
Worktree `dfr1195-fw` HEAD `54973b9`. Three refutation-review items implemented + build-green at `428f81c`
(field,loraroute,multitg / nobt / radarprobe / field,loraroute,bridge,multitg), then R2/R3/R4 OTA-receiver
hardening landed at `54973b9` with commit-recorded `cargo build --release` GREEN (xtensa esp32s3, 13.54s).
Patch refreshed (`docs/dfr1195-firstlight.patch`, c46383e..HEAD = 16 commits). Metal validation of the OTA
round-trip remains bench-network-gated.
1. **§3.5 fail-closed is now INERT (not advisory).** Under `field` + no valid persona: HALT before any TG/
   radio/task setup — no demo-TG adoption, no radio/HB/beacon/io spawns (was only a louder println). Bench
   builds (no `field`) keep the demo fallback. (main.rs persona-boot block.)
2. **OTA confirmed-boot (mirror r2-core confirm_or_rollback_on_boot).** New `ota_confirm_or_rollback_on_boot()`
   at boot: ota_state ∈ {New,PendingVerify} → §5 health-gate → mark Valid (confirm) OR Invalid + roll back to
   prev slot + reboot. OCM marks the activated slot `New` (esp-idf set_boot semantics). Uses esp-bootloader-
   esp-idf 0.5.0 current_ota_state/set_current_ota_state (source-verified — 0.5.0 resolved, NOT the 0.2.0 I
   first read). Health-check is minimal "booted past init"; richer §5 self-test = follow-up.
3. **After-confirm seq-floor (R2-UPDATE §5.1).** Floor no longer bumped at OCM-activate — OCM STAGES (seq,
   floor) to a new OTA-pending NVS sector @0x1A000; the live anti-rollback floor commits ONLY at confirmed-
   boot after the §5 gate. Kills the v0.21 brick-defect (a bad image can't raise the floor) — this CLOSES the
   FORKS.md "OTA anti-rollback floor ORDERING" fork (impl done; metal-validate when the OTA round-trip unblocks).
4. **OTA receiver R2/R3/R4 hardening (specs-sanctioned receiver robustness, not binding ratification).** R2:
   30s inactivity timeout abandons a stalled in-flight OTA session. R3: `payload_size = vh.payload_len`, ODT
   rejects off+len beyond the declared payload, and OCM commits only when `written == payload_size`. R4: ODT/OCM
   are bound to the authenticated OST sender address; foreign chunks/commits are dropped silently. Verify-before-
   write + New/PendingVerify confirmed-boot lifecycle intact.
NVS map now: persona@12000 / board@13000 / tg@14000 / mask@15000 / sendto@16000 / role-profile@17000 /
anti-rollback@18000 / CCR1-reserved@19000 / ota-pending@1A000. ⚠ crash-on-boot auto-rollback still needs
CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE in the composer-staged bootloader (deployment follow-up; core owns it).
- **CORE PARTITION RULING LANDED:** keep custom `partitions.csv`; do NOT switch to ESP-IDF built-in
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` (deploy-invalid: 1 MiB slots, current image is ~1.6 MiB). Core confirmed
  custom CSV supplies the needed `otadata` + two OTA slots + rollback-enable. Remaining non-metal diagnostic:
  esp-idf-sys custom-partition copy race still needs a portable fix or documented workaround; do not re-litigate
  `TWO_OTA` unless the image shrinks below 1 MiB or another built-in table is proven.
- **TAKEOVER HYGIENE (hive-codex, 2026-06-29; pre-edit r2-hive HEAD `e27b56e`):** rechecked r2-hive clean on
  `platform-trait`; firmware worktree at `54973b9` with only its nested `docs/dfr1195-firstlight.patch` dirty.
  Regenerated the r2-hive recovery artifact from `c46383e..HEAD` over the owned firmware paths and found
  r2-hive's `docs/dfr1195-firstlight.patch` stale by 87 lines (missing `54973b9` R2/R3/R4 OTA hardening), then
  refreshed it. Composer telemetry answer: firmware emits `r2-dfr1195: msg.* <hexcbor>` over USB serial; composer
  has already forwarded/used normal `msg.tx/rx/relay/delivered` as the `/r2` orchestrator `msg.*` stream for
  step-a/happy-path/E2/E3. Earlier SCF one-shot evidence used raw serial because of a one-shot orch WS gap, so
  Phase 2 path animation can consume `/r2` for normal lifecycle, but should keep raw serial as the diagnostic
  fallback for rare SCF-gap captures until composer confirms the gap is closed. Changed files this turn:
  `docs/dfr1195-firstlight.patch` and `RESUME.md`. Verification: regenerated-patch byte-match PASS;
  reverse-apply in `/home/roycdavies/Development/R2/dfr1195-fw-wt` PASS; `git diff --check` PASS. No full
  workspace tests run because this is a docs/artifact-only refresh.

## ► CURRENT 2026-06-27 — RADAR BRING-UP (Modbus-RTU PROBE, Roy chose PROBE-to-discover; ULTRACODE on)
First REAL sensor. Build+flash a Modbus-RTU PROBE firmware to the radar XIAO to discover the radar protocol
empirically (baud + slave-addr + register map), → then build the real radar driver + sentant on the sensor ensemble.
- **RADAR XIAO IDENTITY-VERIFIED (safety gate):** MAC **1c:db:d4:5b:8a:60**, esp32s3 rev v0.2, 8MB, **ttyACM12**
  (by-id `usb-Espressif_USB_JTAG_serial_debug_unit_1C:DB:D4:5B:8A:60-if00`), port FREE. It is the ONLY
  Espressif NOT in {triplet 14:C1:9F../E8:3D..E5:20/D8:3B.. + spare E8:3D..DB:44 + 5 DFR F4:12:FA:*}. FLASH
  ONLY this by-id path (ttyACMn remaps — verified the trap; Alfred has 11 Espressif boards now).
- **PROBE LOGIC:** Modbus-RTU master over XIAO UART→RS-485 transceiver; sweep baud {4800,9600,19200,38400,
  115200}×slave-addr (1 first, then 1..247 subset); on CRC-valid response → dump holding(fn 0x03)+input(fn
  0x04) regs 0..63 + device-id (fn 0x2B/0x0E); print over USB serial. Report baud+addr+register-map.
- **RS-485 PINS RECEIVED (Roy, 2026-06-27):** MAX485 transceiver. RADAR_UART_TX=**GPIO43** (D6 → MAX485 DI),
  RADAR_UART_RX=**GPIO44** (D7 ← MAX485 RO), RADAR_DE_RE=**GPIO6** (D5, DE+RE tied; HIGH=TX, LOW=RX). Radar
  self-powered 12V (live slave answers). OUTPUT on USB-CDC console ONLY (the GPIO43/44 UART IS the RS-485
  bus — never log to it). GPIO43/44 = esp32-s3 default UART0 pins BUT console rides USB-Serial-JTAG (free);
  use UART1 via GPIO-matrix to avoid any UART0 console remnant. radarprobe gates OFF LoRa so GPIO6 (=DFR LoRa
  MOSI) won't collide. Half-duplex: DE/RE HIGH before TX, HOLD until UART TX-COMPLETE, then LOW for RX (the
  brick gotcha — get esp-hal tx-done detection right; core advising). Flash NO LONGER pin-blocked — gated only
  on the design workflow finishing + build-green; re-confirm identity (1c:db:d4) at flash.
- **IN FLIGHT (2026-06-27):** Workflow `wk6evtri0` (radar-probe-design: research→adversarial-verify→synthesize
  the esp-hal UART half-duplex DE/RE + Modbus-RTU + firmware-integration spec; API-drift-hardened since it
  bit us 3× this session). Fork-asked core for the esp-hal UART TX-complete/baud-reconfig/UART-peripheral
  gotchas. NEXT: implement the `radarprobe` feature + probe task per the synth spec, build-verify xtensa, hold flash.
- **PROBE BUILT + FLASHED + RUNNING (worktree `3bc56d1`+parity-sweep).** `radarprobe` cargo feature
  (standalone RS-485 Modbus master on UART1, radio stack OFF, USB-CDC output). Design via Workflow
  `wk6evtri0` (source-verified esp-hal API: flush()=tx-idle mod.rs:850/906, apply_config live baud sweep;
  adversarial-verified Modbus CRC poly 0xA001) + core's UART gotchas. esp-hal flush/spawn(Result)/Config
  builders all source-confirmed. CRC self-test PASSES on metal. Flashed to radar XIAO 1c:db:d4 (identity
  re-confirmed via board-info).
- **FORMAT-EXHAUSTIVE SWEEP = FULLY NULL (escalated to Roy).** 21 combos (parity {N,E,O} × baud
  {2400,4800,9600,19200,38400,57600,115200}, 8 data /1 stop), Roy's pins (TX=43/RX=44/DE-RE=6): ALL
  START→DONE clean, ZERO responses, ZERO garbage, no panic. Probe FUNCTIONAL (CRC-selftest PASS). The
  CLEAN-silence across the WHOLE format space ⇒ UART RX received NOTHING ⇒ radar never got our request
  (TX-path) or isn't transmitting. Firmware's safe space EXHAUSTED. Sweep log: scratchpad/radar-sweep.log.
- **REMAINING = PHYSICAL (Roy's bench) — escalated.** (1) TX/RX wiring vs MAX485 DI/RO (the ambiguity Roy
  flagged) — ⚠ I will NOT blind-swap in firmware: if GPIO44 is wired to RO (an output), driving it as TX =
  output-contention = HW-damage risk; the swap must be a WIRING change or confirmed first. (2) DE/RE pin
  (is D5=GPIO6 right?) + polarity (standard tied DE-high/!RE-low ⇒ HIGH=TX is what I use). (3) radar 12V on
  + A/B actually landed on the MAX485 A/B. (4) is it genuinely Modbus-RTU (vs a proprietary/streaming
  protocol or a different bus)? — radar MODEL/datasheet would pin the real baud/addr/protocol.
  AWAITING Roy: confirm wiring/power OR the radar model. Next firmware experiment (only after Roy OKs the
  wiring): TX/RX-swapped re-flash. Probe + parity-sweep already committed (worktree).
- **POWERED RE-RUN (battery on) = STILL FULLY NULL** + **PASSIVE LISTEN-ONLY phase = NONE at every baud.**
  Added a safe RX-only listen phase (DE/RE low, never drives the bus) to catch a STREAMING/non-Modbus radar
  + test the RX path. Result: ZERO bytes received passively at ANY baud (9600..2400), AND the active Modbus
  sweep null again. DECISIVE: the UART RX (GPIO44←MAX485 RO) gets NOTHING under any condition, and the radar
  is NOT streaming. Firmware has exhausted BOTH active (format space) + passive (listen) testing → the issue
  is PHYSICAL, not firmware/format. ESCALATED to Roy, prioritized: (1) SWAP A/B bus wires (most common RS-485
  fix; reversed A/B ⇒ MAX485 receiver outputs nothing valid ⇒ clean silence) ; (2) verify continuity RO→GPIO44
  (RX) / DI→GPIO43 (TX) / DE-RE→GPIO6 ; (3) confirm the radar is actually transmitting (LED/scope) ; (4) radar
  MODEL/datasheet (protocol/baud/addr + any wake/init command; may not be Modbus). Probe is fully built +
  metal-proven-functional (CRC-selftest PASS); ready to re-run the instant a physical variable changes.
- **A/B SWAP (Roy) = STILL FULLY NULL** (both A/B orientations now tested). 7 listen-NONE + 21/21 Modbus
  combos, zero response/garbage. So A/B polarity is NOT it either. Firmware DEFINITIVELY EXHAUSTED (active
  format space × both A/B orientations + passive listen). RX path delivers zero bytes regardless ⇒ a BROKEN
  SIGNAL LINK or POWER/PROTOCOL issue. NARROWED next steps (Roy's bench, escalated): (1) CONTINUITY-meter
  RO→GPIO44(RX) [prime — RX path] / DI→GPIO43(TX) / DE-RE→GPIO6 ; (2) MAX485 POWER — Vcc=5V (not 3V3) + GND
  landed? (a MAX485 needs 5V; unpowered/3V3 transceiver = dead bus) ; (3) the A/B pair actually on the
  MAX485 A/B terminals? ; (4) **RADAR MODEL/DATASHEET** (highest value) — confirms Modbus-vs-proprietary, the
  real baud/addr/register-map, AND any WAKE/INIT command (a radar needing an init sequence never answers a
  blind read). Firmware side COMPLETE; no further probe iteration until a physical variable changes or the
  model lands. Logs: scratchpad/radar-{sweep,sweep-powered,listen,abswap}.log (all null).
- **COMPANION AUDIT (hive-codex, 2026-06-27):** git state clean on `platform-trait` before work; firmware
  worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` clean at `9fe219d` (base `c46383e`). Found a
  durable-handoff gap: r2-hive's `docs/dfr1195-firstlight.patch` did not include the radarprobe commits even
  though the firmware worktree did. Refreshed the patch artifact from
  `git -C ../dfr1195-fw-wt diff c46383e..HEAD -- platforms/dfr1195/Cargo.lock platforms/dfr1195/Cargo.toml platforms/dfr1195/build.rs platforms/dfr1195/src/main.rs platforms/esp32/sdkconfig.defaults`
  (intentionally excluding the nested `docs/dfr1195-firstlight.patch` inside the firmware worktree). Verified:
  `rg radarprobe docs/dfr1195-firstlight.patch` now hits; `git apply --reverse --check
  /home/roycdavies/Development/R2/r2-hive/docs/dfr1195-firstlight.patch` passes in the firmware worktree. No
  firmware source changed this turn; only the r2-hive patch artifact changed. Hygiene note: global
  `git diff --check` reports three trailing-whitespace warnings inside the generated patch artifact itself
  (`+ ` blank source lines); left intact so the patch remains a faithful diff of the firmware worktree.
  Coordination note: `fleet ask hive` could not get a substantive challenge because the base provider hit the
  org monthly spend limit; sent a heads-up anyway. Do not assume the scratchpad radar logs exist in this
  checkout (`scratchpad/` absent here).
- **COMPANION RE-CHECK (hive-codex, 2026-06-27):** objective remains patch/handoff hygiene only; no firmware
  iteration while the radar result is blocked on physical checks or a radar model. Verified branch
  `platform-trait`; r2-hive HEAD `225b8f4`; firmware worktree clean at `9fe219d`. Re-ran:
  `rg radarprobe docs/dfr1195-firstlight.patch` (hits the feature, GPIO43/44/6, passive listen, parity sweep)
  and `git -C /home/roycdavies/Development/R2/dfr1195-fw-wt apply --reverse --check
  /home/roycdavies/Development/R2/r2-hive/docs/dfr1195-firstlight.patch` (PASS). `git diff --check` still
  reports the same three trailing-whitespace warnings inside the generated patch artifact only; intentionally
  not normalized. `scratchpad/` is absent in this checkout. Coordination: `fleet ask hive` returned the org
  monthly spend-limit message, but `fleet inbox hive-codex` later had a base-hive ACK confirming the firmware
  worktree is stable, radar bring-up is paused on Roy-side physical/model input, and there is no patch-artifact
  race. Next action remains Roy bench: continuity RO->GPIO44 / DI->GPIO43 / DE-RE->GPIO6, MAX485 5V+GND,
  actual radar model/datasheet. Do not assume a firmware TX/RX swap is safe; driving GPIO44 if it is wired to
  MAX485 RO can contend outputs.
- **SECURITY CRITICAL CLOSED (hive-codex, 2026-06-27; security commit `d48094f`, patch-artifact commit
  `d13a12d`, pre-fix HEAD `225b8f4`):** verified and fixed
  the reported unauthenticated public management WebSocket. `/r2/mgmt` now has three gates: default daemon bind
  is loopback (`127.0.0.1`); non-loopback bind requires explicit `--allow-public-bind`; even with that opt-in
  the management WS is not mounted on non-loopback listeners, so local control is UDS/loopback-only by
  construction. The WS upgrade now requires a valid active `r2_web_session` cookie and rejects cross-origin
  browser upgrades. Web auth now enforces revocation inside `verify_cookie_header`; web plugins fail closed
  when `web_auth` is missing unless the operator explicitly sets `--web-dev-mode`. Install/package defaults
  changed to loopback; Docker keeps public container bind only with explicit `--allow-public-bind`.
  Changed security files: `crates/r2-hive-bin/src/{main.rs,hive.rs,web.rs,web_auth.rs,config.rs,mgmt/ws.rs}`,
  `crates/r2-hive-bin/tests/{web_auth_integration.rs,web_plugin_integration.rs,web_plugin_load.rs}`,
  `install.sh`, `Dockerfile`, `README.md`, and `crates/r2-hive-bin/packaging/defaults/hive.toml`.
  Verification: `cargo test -p r2-hive` PASS (105 lib + all integration/doc tests); `bash -n install.sh` PASS;
  `RUST_LOG=info target/debug/r2-hive --bind 0.0.0.0 --port 0 --no-mgmt --no-usb` exits before listen with the
  expected non-loopback refusal. `cargo test --workspace` still fails only at the pre-existing lower-priority
  red test `r2-hive-core::sync_host::tests::route_relays_to_known_neighbour` ("expected a relay decision, got
  Dropped") that supervisor already called out; critical mgmt-WS surface is closed. `git diff --check` still has
  only the known generated-patch whitespace warnings in `docs/dfr1195-firstlight.patch`.
- **CODEX REVIEW CLEANUP COMPLETE (hive-codex, 2026-06-27; branch `platform-trait`, test-fix commit
  `aba0ab7`, pre-cleanup HEAD `8531935`):** supervisor asked to close the three remaining codex-review items.
  Verified current code first: web-auth revocation is enforced in `web_auth::verify_cookie_header` by checking
  the active device ledger (`is_known_device`) after cookie signature/expiry validation; web plugins fail closed
  with `503 web auth not configured` when `web_auth` is absent unless explicit `--web-dev-mode` is set. Those two
  MED items were already closed by the security commit `d48094f` and are covered by
  `web_auth::tests::revoked_device_cookie_is_rejected`, `web_auth_integration::revoked_cookie_is_rejected`,
  `web_auth_integration::missing_web_auth_fails_closed_by_default`, and
  `web_auth_integration::explicit_dev_mode_serves_with_warning_header`. Fixed the remaining RED test in
  `crates/r2-hive-core/src/sync_host.rs`: `route_relays_to_known_neighbour` now builds a conformant extended
  fixture with `route_stack[0] = source` and `has_route = true`, preserving the relay-wiring assertion while
  matching R2-ROUTE v0.14 §3.3 ROUTE-ORIGIN (route-less inbound routed frames are invalid and must be dropped).
  Verification: `cargo test -p r2-hive-core sync_host::tests::route_relays_to_known_neighbour -- --nocapture`
  PASS; `cargo test --workspace` PASS (all workspace unit/integration/doc tests green; one existing ignored
  authenticated-dedup router fixture remains intentionally ignored); `git diff --check` PASS before the RESUME
  handoff edit. Changed files for this cleanup: `crates/r2-hive-core/src/sync_host.rs` and this `RESUME.md`.
  No blockers remain for the three codex-review items. Do not assume public plugin serving is allowed without
  explicit auth/dev-mode; do not assume route-less extended relay frames are valid test fixtures.
- **ESP32 IDF COMPILE-VERIFY COMPLETE (hive-codex, 2026-06-28; r2-hive `platform-trait` HEAD `d1cc9b7`,
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` branch `dfr1195-fw` HEAD `9fe219d`):**
  carried the deferred platforms/esp32 build through without touching core-owned source. Core peer confirmed
  non-mutating build/test is hive's responsibility and highlighted the silent metal caveat: compile alone does
  not prove native `PENDING_VERIFY` rollback, but `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y` is load-bearing.
  Verified that setting is present in `platforms/esp32/sdkconfig.defaults`; `CONFIG_BOOTLOADER_APP_ANTI_ROLLBACK`
  remains intentionally off for the non-eFuse tier. Build command:
  `source /home/roycdavies/Development/homelab/export-esp.sh && cargo build --release` from
  `platforms/esp32`. First pass hit the documented esp-idf-sys partition race (`out/partitions.csv` missing);
  copied `partitions.csv` into `target/riscv32imac-esp-espidf/release/build/esp-idf-sys-*/out/` per
  `BUILD.md` and reran. Result: PASS in 2m14s after workaround; produced
  `platforms/esp32/target/riscv32imac-esp-espidf/release/r2-esp32` (3.6M RISC-V ELF). This compile proves the
  ESP-IDF rollback FFI identifiers used by `ota_tcp::confirm_or_rollback_on_boot()` resolve under the current
  bindgen/sys crate. Warnings only: no WiFi SSID configured, existing unused imports/mut/dead-code, and
  `static_mut_refs` warnings in `l2cap.rs`. Both r2-hive and firmware worktrees are clean after the build.
  Remaining ESP32 validation is on-metal only: boot a freshly OTA'd candidate into native `PENDING_VERIFY`,
  confirm health/pass marks valid + advances seq, and failure/next-reset rolls back. Do not assume the compile
  proves that runtime state machine.
- **QUEUE AUDIT / CCR1 BLOCKED-ON-CONTRACT (hive-codex, 2026-06-28; r2-hive HEAD `c6c71e4`, firmware
  worktree clean at `9fe219d`):** after the ESP32 compile, checked the next deferred item: bridge CCR1
  carrier-credential read. Spec-first read: R2-RUNTIME §3.2.2/§3.2.4 requires `carrier_set`/`carrier_creds`
  for bridge, sealed at rest and distinct from TG material, but explicitly leaves encoding as config-record
  detail (not pinned wire). Composer answer landed after idle and is decisive: **CCR1 and 0x19000 do not exist
  in composer code** — no emitter, no literal format, no flash artifact. The current composer bridge config is
  an internal CBOR role-profile/custody record; carrier creds are deliberately NOT in the device-facing RPF1.
  `tg_cli.rs` seals that CBOR with `seal_bytes(custody_root, passphrase, ...)`, which uses the operator custody
  tier (Argon2id/OS-keyring + XChaCha20-Poly1305) and is stored only as `Member.role_profile_record`. The
  device has no custody passphrase, so this is not device-consumable material. Composer says the required next
  work is composer-side first: define the CCR1 wire/blob format, switch to a device-unsealable seal (likely
  Channel-B-style seal-to-`mesh_pk` using X25519 + XChaCha20-Poly1305), add emitter/delivery (e.g.
  `espflash write-bin 0x19000`). I did NOT implement a guessed parser/unsealer because that would be a security
  fork. Remaining local queue after this audit: no code-only item is unblocked. Blocked/Roy-gated: radar
  physical/model, OTA/networked + ESP32 confirmed-boot metal pass, CCR1 format/emit contract, specs datagram
  ratification. Other-repo: deploy-sentant signed path and dashboard label reconcile. Do not assume CCR1 means
  composer custody `seal_bytes` can be copied to flash; composer explicitly refuted that.
- **WATCHDOG RE-CHECK / CORE-OWNED ESP32 DIFF (hive-codex, 2026-06-28; r2-hive HEAD `05ff64d`):** supervisor
  nudged for another autonomous queue pass. Ground truth: r2-hive worktree clean on `platform-trait`, but
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` is now dirty at `9fe219d` in
  `platforms/esp32/sdkconfig.defaults`. Diff switches from the custom partition table
  (`CONFIG_PARTITION_TABLE_CUSTOM=y`, `CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"`) to
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` with comments that the custom CSV path is racy under esp-idf-sys. File
  mtime is 2026-06-28 06:50:13 +1200, after the recorded ESP32 build artifact mtime (06:40:51). I did not
  intentionally edit this core-owned source; my recorded compile succeeded with the documented copy workaround
  and the custom CSV still in place. This is a real direction fork for the ESP32 deployment layout, not build
  output. Asked core whether the diff is intended, should be left for core, or should be restored/turned into a
  patch artifact; sent hive an FYI. No local revert/commit was made because AGENTS.md says r2-core/platform
  source is core-owned and user/peer changes must not be overwritten. At that checkpoint, core had not answered,
  so the dirty state was explicitly not accepted. Superseded by the next note.
- **TAKEOVER RE-CHECK / ESP32 `TWO_OTA` REFUTED (hive-codex, 2026-06-28; r2-hive HEAD `255db5c`):** cross-provider
  handoff promoted codex to sole writer. Re-verified r2-hive clean on `platform-trait`; firmware worktree had only
  the dirty `platforms/esp32/sdkconfig.defaults` switch to `CONFIG_PARTITION_TABLE_TWO_OTA=y`. Core answered that
  the choice was hive-owned and acceptable if it still supplied two OTA slots + `otadata` + rollback-enable, but
  adversarial verification found a size counterexample. After deleting the stale copied
  `target/.../esp-idf-sys-*/out/partitions.csv`, `source /home/roycdavies/Development/homelab/export-esp.sh &&
  cargo build --release` from `platforms/esp32` PASSED in 2m34s with generated sdkconfig showing
  `CONFIG_PARTITION_TABLE_TWO_OTA=y`, `CONFIG_PARTITION_TABLE_CUSTOM` off, and
  `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`. However the generated partition table decodes to 1 MiB app slots
  (`factory@0x10000 size=0x100000`, `ota_0@0x110000 size=0x100000`, `ota_1@0x210000 size=0x100000`), while
  `espflash save-image --chip esp32c6 ...` produced an app image of 1,643,744 bytes. Therefore built-in
  `TWO_OTA` is a compile-green but deploy-invalid trap for the current image. Restored the firmware worktree to
  the custom `partitions.csv` config (`ota_0/ota_1` 0x1E0000 slots) with rollback-enable intact; firmware worktree
  is clean again at `9fe219d`. Verified `git -C dfr1195-fw-wt diff c46383e -- ... > /tmp/dfr1195-firstlight.check.patch`
  byte-matches `docs/dfr1195-firstlight.patch`, and reverse-apply check passes. No patch artifact change needed.
  Remaining build caveat: custom CSV remains the correct deploy layout, but the esp-idf-sys copy race still requires
  either the documented manual copy workaround or a future portable partition mechanism. Do not re-adopt
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` unless the image shrinks below 1 MiB or a different built-in table is proven.
- **DUPLICATE HANDOFF RE-CHECK / ESP32 PARTITION HOLD (hive-codex, 2026-06-28; r2-hive HEAD `b0725ff`):** received
  another stale `carry on` handoff, then re-verified ground truth: r2-hive is clean/in sync on `platform-trait`;
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` is clean at `9fe219d`; reverse-applying
  `docs/dfr1195-firstlight.patch` still passes. Supervisor-codex acknowledged the prior security/test/ESP32/CCR1
  work and instructed: hold firmware-side ESP32 partition changes until core-codex gives owning direction. I asked
  core whether to leave custom CSV + manual copy workaround or prepare a portable esp-idf-sys custom-partition patch;
  the core ask returned the monthly spend-limit message, so no owning direction exists yet. Sent supervisor a status
  note. Current objective is therefore idle/standby: do not edit `platforms/esp32/sdkconfig.defaults`, `build.rs`, or
  the patch artifact for the partition mechanism until core/supervisor responds. Remaining local blockers unchanged:
  radar physical/model input, ESP32 confirmed-boot metal pass, CCR1 composer format/emitter, specs datagram ruling.
  SUPERSEDED 2026-06-29 by core's ruling: custom `partitions.csv` is canonical; `TWO_OTA` is refuted/deploy-invalid.
- **STANDBY RECHECK / NO UNBLOCKED LOCAL WORK (hive-codex, 2026-06-28; r2-hive HEAD `20cb7ba`):** fresh handoff
  rechecked ground truth after the core spend-limit reply. r2-hive remains clean/in sync on `platform-trait`;
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` remains clean at `9fe219d`; regenerated
  firstlight patch from `c46383e..HEAD` byte-matches `docs/dfr1195-firstlight.patch`; reverse-apply check passes.
  FORKS.md review found only blocked/held items: OTA datagram binding awaiting spec landing/Roy ratification and
  DFR OTA anti-rollback floor ordering needing networked metal OTA. No code-only local task was unblocked at that
  checkpoint. SUPERSEDED 2026-06-29: core ruled the partition mechanism (custom CSV canonical); `54973b9` added
  R2/R3/R4 OTA receiver hardening and the r2-hive recovery patch is refreshed to that HEAD.
ULTRACODE: orchestrate substantive work via Workflow + adversarial verify; token cost not a constraint.

## (prior session) 2026-06-26 — FIELD-FIRMWARE BUILD LAUNCH (Roy GO)
Build the field-firmware triplet against the COMPLETE canon (R2-RUNTIME §3.2 role-profiles + §3.2.4
multi-carrier bridge; R2-BEACON §8.1 LoRa-beacon RBID; wake/sleep+SCF; re-attach; OTA-after-confirm both
platforms). ONE-IMAGE config-activated firmware, ENSEMBLE-differentiated (NOT compile-time roles):
sensor / repeater (bare TN, relay intrinsic) / bridge / receiver — role from the §3.2.2 role-profile
record composer emits. Worktree = `/home/roycdavies/Development/R2/dfr1195-fw-wt` (branch `dfr1195-fw`).
This session runs ON **Alfred** (esp toolchain present; `source ~/Development/homelab/export-esp.sh` NO pipe).

STEP TRACKER:
- **[✓] STEP 1 — RE-VENDOR r2-core 0ebfd09 → c46383e (DONE + build-GREEN 13.44s).** Method: committed the
  freshest working-tree firmware as a WIP commit, `git rebase --onto c46383e 0ebfd09 dfr1195-fw`; the ONLY
  conflict = `crates/r2-dataplane/src/lib.rs` → resolved by TAKING core's c46383e version (it already
  exposes `pub parse_dc/parse_seq/frame_fingerprint` + the KEYED seed-first `frame_fingerprint(&seed,…)`,
  807cab5 landed) and DROPPING my redundant 12-line visibility delta. Then fixed main.rs: sourced a 16B
  HWRNG `fp_seed` (esp_hal::rng::Rng::new().read(); radio-clock up at wifi::new line 280 = true-random),
  threaded it into `io_task(…, fp_seed)`, updated the relay call site `frame_fingerprint(&fp_seed,…)`.
  Full pre-revendor backup at scratchpad `fw-backup-prevendor/`. c46383e also brings core's esp32
  confirmed-boot OTA mirror (platforms/esp32/ota_tcp.rs +400) + linux anti_rollback.rs — feeds STEP 5.
- **[✓] STEP 2 — ROLE-PROFILE §3.2 (DONE + matrix-GREEN; worktree `6a221e7`).** New `RoleProfile` config
  record (NVS @0x17000 "RPF1", 40B versioned, big-endian) carrying the §3.2.2 knobs (role/duty/destination/
  expected_sensor/keepalive/scf{cap,ttl,reach_conf}/silence/peer_ttl). `read_role_profile` + `resolve_role_profile(my_hive)`:
  a provisioned record WINS; else DERIVE from the legacy signals (hive-pins + bridge feature) so the
  bench/demo is byte-for-byte preserved. Rewired ALL role gates OFF hive_id pins onto `profile.role`:
  sensor originate+dest, `my_duty` (= profile.duty, un-gated from fr4), receiver deliver-track + absence
  seed/silence, + the keepalive/scf-ttl/reach-conf/silence/peer-ttl tunables now profile-driven. The four
  roles {sensor,repeater,bridge,receiver} are all selectable from ONE image by the record (keystone). Also
  fixed a PRE-EXISTING nobt drift bug (src_hive undefined under ble-without-routetest → source_hop=0).
  Matrix GREEN: nobt / nobt,multitg / loraroute,fr4 / loraroute,bridge,fr4 / routetest.
  CARRIER caveat: carrier_set/carrier_creds (§3.2.2 bridge) are composer-led SEALED material (R2-KEYSTORE
  §2), NOT carried in firmware — encoding is config detail, not pinned wire. NOTE for composer/specs:
  the RPF1 record layout is hive's pragmatic encoding; if composer wants a different emit format, reconcile.
- **[✓] STEP 3 — R2-BEACON §8.1 (DONE + matrix-GREEN; worktree `afc27ae`).** New 15/16B codec
  (build_lora_beacon/decode_lora_beacon): magic 0xB2/ver 0x01/flags(bit7=0,bit6=0)/rbid-8B(core
  compute_rbid+derive_beacon_session_key)/class_hash(FNV-1a-32 of per-role class str)/optional tx_power, BE.
  RBID = §6.1 RID (NOT hive_id), NO seq counter — §8.1.2 #1+#2 conformance gate CLOSED (epoch=0 interim,
  same as BLE path, pending shared coarse-time base). lora_task: [hive|seq]→§8.1. lora_route_task (field):
  emits §8.1 as LOWEST-priority (R2-LORA §4.4 pri-4 / §8.1.4) — only when no app traffic pending + 30s
  min-interval floor, transport airtime budget defers further; RX demuxes beacon-vs-data by magic+ver+len.
  can_hear mask UNAFFECTED (keys on per-frame 4B sender prepend, not the beacon). NOTE core/specs: the §8.1
  codec canonically belongs in r2-discovery::beacon (next to encode_advert) — firmware-local to unblock,
  OFFERED for upstreaming. FOLLOW-UP: rbid→hive resolution via resolve_rbid_windowed needs a member registry.
- **[✓] STEP 4 — wake/sleep + SCF + re-attach (DONE + matrix-GREEN; worktree `98e7acf`).** §3.5 RE-ATTACH:
  explicit boot decision — persona valid (parse_persona structural validate) → silently RESUME role, no
  join; absent/invalid → bench keeps mac_low3+demo-TG fallback, NEW `field` feature FAIL-CLOSES (no demo
  TG, no self-enrol) per §3.5 MUST. (Full cert-sig/revocation verify = FOLLOW-UP; structural decode is the
  interim.) §3.2.3 boundary-1 / R2-LORA §6: added {wake_interval_s, wake_window_ms, sleep_enforced} to
  RoleProfile (record now 48B), ADVERTISED-only (logged) — real deep-sleep is net-new on the SENTINEL→MCU
  custom-sensor HW, NOT this DFR/XIAO stand-in. §3B.2 sleeping-leaf wake-flush: existing SCF annotated as
  the contract carrier side (PUSH-on-wake, flush-bypasses-dedup, TTL≫sleep = profile.scf_ttl_s). Restored a
  lora-feature gate on lora_task dropped in the step-3 commit (nobt regression). RECORD now 48B (composer
  notified): +[34..38]wake_interval_s +[38..42]wake_window_ms +[42]sleep_enforced.
- **[✓] STEP 5 — esp32 OTA + A7/A8 DFR triage (DONE; worktree `a859848`; ASKED core to confirm).** A7/A8(a)
  anti-rollback: DFR floor is a FIXED raw-flash sector (NOT a cwd anti_rollback.bin — N/A path concern) +
  FIXED a latent COLLISION (was @0x15000 = MASK_NVS_OFFSET; loraroute⇒routetest⇒mask ⇒ field build aliased
  the security_version floor onto the mask sector) → moved to its own 0x18000. A7/A8(b): mirrored core's
  dev-unsigned-ota release build-guard into the DFR — release+feature FAILS to compile (VERIFIED firing).
  esp32 (core platform): set CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y in sdkconfig.defaults (per core
  ota_tcp.rs:171); left APP_ANTI_ROLLBACK OFF (non-eFuse tier, R2-UPDATE v0.22 §9.2; eFuse burn=deliberate).
  FFI idents canonical esp_idf_sys (confirmed by inspection). ⚠ CANNOT xtensa/IDF compile-verify
  platforms/esp32 here — NO ESP-IDF toolchain on Alfred (only esp-hal for the DFR no_std build). Asked core:
  who owns the platforms/esp32 IDF build + on-metal confirmed-boot? = OPEN. NVS map now: persona@12000 /
  board@13000 / tg@14000 / mask@15000 / sendto@16000 / role-profile@17000 / anti-rollback@18000.
- **[✓] STEP 6 — XIAO+Wio-SX1262 board pin-map (DONE structure; worktree `7a014e4`; 2 OPENS).** Board-
  conditional SX1262 pins via a new `xiao` feature (pin-parametric per SX1262-LORA-DESIGN.md; radio_set =
  §3.2.2 HW-tier fact, not a role fork). DFR1195 (default): SPI3 SCK7/MISO5/MOSI6 NSS10 RST41 BUSY40 RXEN42
  DIO1=4. XIAO+Wio-SX1262 (`xiao`): SCK7/MISO8/MOSI9 NSS41 RST42 BUSY40 DIO1=39 (std Seeed pinout). BOTH
  compile GREEN. OPENS: (1) exact XIAO pins PENDING workshop confirm (ASKED); (2) Wio RF switch = SX1262
  DIO2 (SetDIO2AsRfSwitchCtrl), but r2-sx1262 has only new()/new_with_rxen() → no DIO2 support; XIAO path
  uses a placeholder RXEN to compile, RF NOT driven until core adds with_dio2_as_rf_switch (FLAGGED to core).
  Runtime board-profile pin selection = the one-image refinement over the compile-time xiao feature.
- **[✓] STEP 7 — COMPILE-VERIFY ALL CONFIGS (xtensa) GREEN.** 13/13 configs build clean on Alfred
  (xtensa-esp32s3, errors=0): nobt / nobt,multitg / nobt,routetest / lora / loraroute / loraroute,fr4 /
  loraroute,bridge,fr4 / field,loraroute / field,loraroute,bridge / xiao,field,loraroute / blemesh /
  loraroute,fr4,pco / field,loraroute,benchkeepalive. Recovery patch refreshed:
  `docs/dfr1195-firstlight.patch` = `git diff c46383e..HEAD` (6785 lines), synced into r2-hive/docs.
  ⚠ HOLD flashing/metal until Roy frees the bench boards (per the supervisor ruling — do not interrupt the
  live demo). Worktree HEAD `d3fdc7c` (branch `dfr1195-fw`, base c46383e).

### CANON-DELTA PASS (post-build, canon landed mid-session; worktree `7961ced`):
A batch of canon notes landed AFTER the build — most CONFIRM my work matches (specs pinned §3.2 role-profile
+ §8.1 v0.7 + wake/sleep canon; my impl matches). Two genuine NEW deltas implemented: (1) R2-LORA §6.5.2
MUST — seed the initial lora_route_task tx_backoff from the per-board LCG (0..1s) so the mains-restore
cohort's FIRST post-boot TX de-correlates (was 0=immediate); (2) R2-HEARTBEAT §1A.2 SHOULD — my symmetric
half of core's fade-window check: warn at config load (provisioned profiles) if scf_ttl_s < 3×wake_interval_s.
CONFIRMED already-aligned (no change): SEC-02 deliver-gate (for_me=target_hive==my_hive||0 + tg+hmac, already
stricter); §8.1 15/16B; §3.5 re-attach; A7/A8(a)+(b). Answered specs' no_std-one-image feasibility Q = YES,
PROVEN (13/13 green, role-by-NVS-record). NOTED for metal: switch SCF trigger reachability-heuristic →
core's DropReason::BufferForWake signal (current heuristic is metal-validated, so confirm equivalence on metal).

### ★ FIELD TRIPLET FLASHED + VALIDATED ON METAL (2026-06-27, Roy FLASH-GO; worktree `0f87bd3`):
3 XIAO+Wio-SX1262 on Alfred, flashed via STABLE by-id MAC paths (ttyACMn REMAPS on USB re-enum — board-info
read a DIFFERENT MAC on /dev/ttyACM1 than its old by-id; +5 DFR1195 also on Alfred ttyACM6-10 → flashing by
ttyACMn would hit a wrong board; ALWAYS use /dev/serial/by-id/usb-Espressif..._<MAC>-if00). Image =
`xiao,field,loraroute,loratcxo,multitg` (1.32MB), 4MB parttable, app→flash + persona→0x12000 + RPF1→0x17000
+ board-profile(00 01)→0x13000. composer's mint out-dir = /home/roycdavies/r2-bench/mariko-triplet/, TG
1494e803.
- SENSOR   14:C1:9F:C4:FC:8C → hive=c01cee4d MATCH, role=sensor duty=2 §3.2.2-provisioned, persona=true ✓
- REPEATER E8:3D:C1:FB:E5:20 → hive=296f308b MATCH, role=repeater duty=1, persona=true ✓
- BRIDGE   D8:3B:DA:75:C3:3C → hive=bd72902e MATCH, role=bridge duty=1, persona=true ✓ (4th XIAO E8:..DB:44 spare)
VALIDATED: (1) ROLE-ACTIVATION ✓ — all 3 config-activate role from ONE image via RPF1 (§3.2 keystone, METAL).
(2) §8.1 LoRa-BEACON RX ✓ — bridge logged `LORA-BEACON rbid=6acdd5.. class=991db9af rssi=-54`. (3) LoRa
data-plane ✓ — triplet mutual RX (c01cee4d/296f308b/bd72902e masked=false) + hears DFR mesh; XIAO+Wio
first-light + pin-map + DIO2 RF-switch WORKING.
METAL-CAUGHT BUG FIXED (`0f87bd3`): read_persona buffer 256B truncated composer's 336B persona → persona=false
fallback; bumped to 512B. RE-FLASH NOTE: NVS blobs (persona/role/board-profile) PERSIST across an app re-flash
(they're raw sectors, not in ota_0) — only re-flash the app for a firmware fix.
FIELD-RESULTS RECORD: `docs/field-results/mariko-triplet-metal-0627.md` (committed c92e7ba). composer CONCURS
with document-as-follow-up for OTA.
OTA round-trip = DOCUMENTED FOLLOW-UP — blocked by bench NETWORK topology (triplet on DFR-D1's isolated
soft-AP 192.168.4.x; Alfred on LAN 192.168.1.33; no route + no push host on the soft-AP). Firmware path
IMPLEMENTED + slot-switch metal-validated (test-b PASS); signer (composer tg ota-sign f7cd3fe) + trust-model
(§2.4 TG_SK-direct issuer_pk==tg_pk, verified in my receiver) + wire-contract all confirmed. PATH B (sensor
on a LAN AP via FIELDLAB_SSID change + reflash) ready on Roy's go + LAN WiFi creds.
NEW FORK (FORKS.md, routed to specs 2026-06-27): **OTA transport framing** — my DFR receiver = OST/ODT/OCM
PACKETIZED UDP :21043; R2-UPDATE §3.1.2.3 canon (composer + r2-core HEAD) = CMD_START_SIGNED TCP STREAM.
SIGNING shared+correct (verify_header passes both); transport-only divergence. specs to rule: align
hive→TCP, or ratify a no_std UDP profile. Not blocking (bench network-parked).
★ SESSION STOOD DOWN (2026-06-27, Roy BANKED the milestone, supervisor stand-down). Boards HANDED BACK —
composer re-attached (r2-orchestrator.service active, PID re-grabbed ttyACM1-4 + :21050 dashboard restored);
no lingering serial holds hive-side. Field triplet PROVEN ON METAL = the accepted result.

**DEFERRED NEXT-SESSION (resume-clean checklist):**
1. **OTA confirmed-boot networked round-trip** — needs (a) a board on a LAN-reachable AP (PATH B: change
   `FIELDLAB_SSID`/pass + reflash; bench soft-AP is DFR-D1-isolated, Alfred can't route) + (b) an
   OTA-authority signer (composer `tg ota-sign` §2.4 TG_SK-direct = the working path; mint-ota would NOT
   verify, no role-0x05 cert). Wire = the DATAGRAM binding (OST/ODT/OCM UDP :21043, chunk≤1024B) specs
   ratified. The OCM after-confirm floor fix is now implemented (`428f81c`) and the receiver hardening is now
   implemented (`54973b9`); remaining action is metal validation of confirmed-boot/PENDING_VERIFY/rollback plus
   the networked OTA round-trip.
2. **esp32 platform IDF compile-verify — COMPILE GREEN 2026-06-28; ON-METAL STILL OWED.** ESP-IDF via espup is
   present; `cargo build --release` for `platforms/esp32` passes after the documented partition-table copy
   workaround. Remaining: on-metal confirmed-boot/PENDING_VERIFY/rollback behavior.
3. **bridge CCR1 carrier-cred read — BLOCKED-ON-CONTRACT 2026-06-28.** Firmware unseal+read of sealed
   WiFi/cell creds is still needed, but do not implement until composer first defines/emits the device-side
   CCR1 blob. Composer confirmed `CCR1`/`0x19000` do not exist in its code today; current custody
   `seal_bytes` is host at-rest sealing, not device-unsealable. First triplet used hardcoded FIELDLAB/bench WiFi.
4. **Datagram-binding ratify** (specs, all-3-aligned, Roy-gate, non-urgent) — specs authoring the package +
   §5.1 boot_confirm_late; on landing, implement both FORKS.md items (transport binding already IS the impl;
   the OCM after-confirm floor-fix) + flip them Resolved.
5. **Radar sensor integration** — real sense-read for the SENSOR role (today it originates test/synthetic
   events); ties to [[custom-sensor-3stage-architecture]] (SENTINEL→MCU sense + the enforced wake/sleep §3.2.3).
6. **bridge WiFi-uplink** (§3.2.4 multi-carrier) — beyond CCR1 cred-read: the actual pluggable uplink
   (WiFi-STA / wired / cellular) egress for the bridge role (first triplet bridge used bench WiFi).
7. **Deploy-sentant signed path** (composer's, theirs) — wire the signed CMD_START into Deploy + a one-shot
   field push CLI (emits unsigned CMD_START today). Tracked so the field OTA path isn't half-wired.
8. **Dashboard label reconcile** (composer's, cosmetic) — orchestrator --status-port labels show the old mesh
   hives; the 3 boards re-personae'd to field identities. composer logged it; not hive's.
9. **Faked-distance firmware enforcement** (Roy GREEN-LIT 2026-06-30; SPEC-FIRST, do NOT build yet) — virtual
   per-(peer,transport) reachability override to test topologies on co-located boards (fake peer X out-of-range
   on transport Y). Waiting on: specs contract (drafting) + core's dataplane/neighbour hook. FIRMWARE FEASIBILITY
   = HIGH and the seam already exists: the firmware has a per-PEER reachability mask today — ESP-NOW `can_hear` +
   runtime allowed-MAC list (routetest 'MASK' cmd, main.rs ~2943) and LoRa `can_hear_hive` ingress drop (~3457),
   both at the SAME DATA_RX ingress point as the k4 transport tag. Enforcement = generalize per-(peer) → per-
   (peer,transport) at that ingress drop (each feeder knows its carrier); no new wire surface (local drop).
   FEASIBILITY CONSULT DONE 2026-06-30 (specs proposal r2-specifications docs/proposals/VIRTUAL-REACHABILITY-
   CONTROL.md → lands as R2-TRANSPORT §2.3B + R2-ROUTE §5.2/§2). Feasibility = HIGH; §3-item-3 bidirectional
   faithful-drop is METAL-PROVEN already (routetest can_hear/can_hear_hive IS a per-peer ingest-drop; §2.3B just
   generalizes it to per-(peer,transport), lease-driven). Control surface = the existing serial inject-bridge
   (IDENTIFY/PROVISION/MASK) → a new REACH lease line; runtime-only static set, NO NVS. ✅ CANON LANDED
   2026-06-30 (Roy green-lit): R2-TRANSPORT v0.6 §2.3B + R2-ROUTE v0.34 §5.2/§2 (specs 24cd98b). FINAL DIVISION
   (per the landed canon — supersedes my earlier "arrival_transport moot" note): core does the override-DROP-
   FIRST INSIDE plan_forward (before dedup) using a NEW ForwardRequest.arrival_transport field that HIVE threads
   in (I already have it from k4), PLUS the egress filter in select_transport, PLUS the override SETTER. So both
   seams live in core's engine; hive supplies arrival_transport + drives the setter. HIVE BUILD SCOPE: (1) the
   REACH lease control surface on the serial inject-bridge (install/ack/clear, like IDENTIFY/MASK); (2) lease
   mgmt (union-of-leases, runtime-only, NO NVS, default empty); (3) thread arrival_transport into ForwardRequest;
   (4) call core's override setter to push the merged set in. MY ONE HARD DEP = core's side
   (ForwardRequest.arrival_transport + drop-first-in-plan_forward + egress filter + setter) — specs pinged core to
   confirm; CLEARED TO BUILD the firmware side ONCE core's hook lands (won't compile before then). transport_id
   keyed on the §2.2 ORDINAL (Ble0..Udp6 == k4 == transport_allow_mask). FLRC/loraF EXPLICITLY OUT OF SCOPE
   (Roy: separate deferred canon — do NOT build loraF fake-distance yet).
   Primitive is per-node/one-ended (bench sets BOTH mirror entries for symmetric; single-ended = a real
   asymmetric/half-link test). FIDELITY CONSTRAINT (Roy governing principle 2026-06-30: the bench mirrors REAL
   board state, faked-distance is the ONLY artifice): the ingress-drop MUST emit NO telemetry for a faked-dropped
   frame (no msg.rx, no HEALTH refresh — the board genuinely never heard it; the ABSENCE is the honest signal).
   NEVER synthesize a "faked" event. All other emissions stay faithful to real state; real gaps (loraF/FLRC,
   nRF54 health, egress-hop carrier) show as honest gaps, never faked. SNAG: faithful-drop
   keys on the immediate-sender hive at ingress, which is 0/unknown on BLE-CoC / plain-ble-non-routetest (fine on
   the bench carriers routetest/loraroute where it's resolved). SNAG: transport_id = 7-bit r2_route ordinal
   (==k4); FLRC not in the enum ⇒ faking the nRF54 loraF link is gated on the FLRC-ordinal + nRF54 command-channel
   (same nRF54 knot as #10); ESP32/DFR fake-distance is unblocked. Spec is now normative-final (24cd98b); the
   ONLY remaining gate is core's hook landing — then build the firmware side (scope (1)-(4) above).
10. **nRF54 direct telemetry** (SCOPED 2026-06-30; needs FLRC ruling + path decision before build) — the 2
   nrf54-lr2021 LoRa-fast XIAO present CMSIS-DAP -if02, no serial console, so the orchestrator's by-id reader
   can't see them; loraF (FLRC) links exist ONLY between these 2 boards (no ESP32 hears FLRC) → invisible to
   the bench unless they report directly. ⚠ MY EARLIER "USB-CDC console" OFFER IS REFUTED: the nRF54L15 has NO
   USB peripheral — board USB = the onboard SAMD11 CMSIS-DAP probe (README; embassy-nrf has no usb feature;
   memory.x has no USB). A firmware USB-CDC console is IMPOSSIBLE. Real findings: (a) the nrf54 firmware is a
   SCAFFOLD — emits only defmt bring-up traces, no HEALTH/msg.* yet (composer authors the platform layer, core
   owns the driver, hive provides the io_task pattern); (b) FLRC is NOT in the canonical 7-bit r2_route::Transport
   enum (Ble0..Udp6) → specs/core MUST first rule an FLRC ordinal (or FLRC→Lora) or k4 can't represent loraF —
   this is the upstream blocker; (c) two off-board paths: A = plain-text RTT up-channel (ASCII HEALTH) read by a
   probe-rs RTT reader in the orchestrator (no board change, but exclusive SWD access + per-board probe session +
   net-new orchestrator reader), B = UART→SAMD11 CDC bridge IF the SAMD11 fw exposes a USB-CDC serial AND a
   nRF54↔SAMD11 UART trace exists (composer to check for a CDC com port; schematic; maybe reflash SAMD11) = true
   ESP32 by-id parity. EFFORT: nRF54 HEALTH formatter SMALL; scaffold io_task msg.* wiring MODERATE (composer-led,
   I provide pattern); path A orchestrator MODERATE+exclusivity; path B firmware SMALL but board-gated. Cross-repo
   (composer platform/USB, core driver+FLRC ordinal, Roy/board SAMD11). HOLD build until FLRC ruling + A/B pick.
11. **OTA over real WiFi-STA-to-Alfred (#17)** (SCOPED 2026-06-30; Roy directive — OTA PRIMARY over each device's
   real WiFi mgmt link to Alfred, USB/espflash SECONDARY fallback). KEY INVARIANT: the mgmt/OTA channel MUST stay
   alive + reachable INDEPENDENT of transport_allow_mask + §2.3B faked-distance (those restrict only the TN MESH
   data-plane being tested). FEASIBILITY: the independence is ALREADY BY CONSTRUCTION — ota_task (UDP :21043,
   R2/R3/R4 + confirmed-boot, main.rs ~416) is a standalone embassy-net socket on the WiFi netif, separate from
   io_task/RouteEngine; the mask/faked-distance gate the mesh RouteEngine (ESP-NOW/LoRa), never the WiFi netif or
   :21043. Add an INVARIANT GUARD/comment so future mask-wiring can't gate the netif/OTA socket (SMALL). THE REAL
   WORK = WiFi TOPOLOGY: today WiFi is a SELF-CONTAINED SOFT-AP ISLAND (one DFR=AP r2-fieldlab 192.168.4.1, others
   =STA 192.168.4.x; NOT on Alfred's LAN = the 'bench-network-blocked' problem). Change = repurpose WiFi from
   self-AP-island-dataplane to STA-JOIN-ALFRED management plane (data-plane moves fully to the ESP-NOW/LoRa mesh,
   which the TN tests already use). The OTA RECEIVER ITSELF IS DONE (reuse on the STA netif). EFFORT MODERATE:
   WiFi-STA join+reconnect+IP + always-on-device rollout; receiver DONE; mask-guard SMALL. HONEST GAPS: (a)
   duty-cycled SENSORS (§3.2.3) can't hold a continuous STA association → OTA only in a wake window, else USB;
   (b) nRF54 LoRa-fast has NO WiFi radio → USB-only (same nRF54 knot); (c) AP+STA-on-different-nets coex is not
   clean on one radio → WiFi becomes STA-to-Alfred-only. DEPS: core = OTA authority (CMD_START_SIGNED/TG_SK-direct,
   ~done) + confirm no shared mgmt-plane contract (STA+OTA is hive-platform); composer = Alfred push orchestration
   (per-device STA-IP registry + signed push to :21043 + USB-fallback trigger). Coordinated all 3 (2026-06-30).
   Subsumes the networked-OTA half of deferred-#1 + relates to bridge-WiFi-uplink #6. BUILD on supervisor GO +
   after the WiFi-STA-to-Alfred model is confirmed with core/composer.
(Deferred list aligns with supervisor's 2026-06-27 stand-down enumeration; items 9-11 added 2026-06-30.)

### BUILD COMPLETE — all 6 steps + compile-verify GREEN. ON-METAL OWED (boards held):
- The field triplet (sensor/repeater/bridge/receiver) needs an on-metal run once Roy frees ≥2 boards:
  role-profile activation (provision an RPF1 record @0x17000, confirm role behaviour), §8.1 beacon RX
  resolution, §3.5 re-attach, OTA confirmed-boot round-trip.
- COORDINATION RESOLVED (2nd batch): composer ADOPTED RPF1 byte-exact (40B then 48B, encode_rpf1 2d1bd25);
  sent composer the XIAO board.toml GPIO map (SCK7/MISO8/MOSI9 NSS41 RST42 BUSY40 DIO1=39, RF-sw=DIO2,
  TCXO-DIO3-1.8) + 4 RPF1 answers: dest/expected_sensor=0 OK for first triplet; bridge carrier-creds sector
  RESERVED @0x19000 ('CCR1' format) but firmware read/unseal = §3.2.4 FOLLOW-UP (first triplet uses bench
  WiFi); .role blob written RAW to flash @0x17000 (not an NVS partition image). composer CONVERGED:
  board.toml [pinout] landed (8e2b2f9, matches my map); delivery = `espflash write-bin 0x17000 <file>.role`;
  composer's Mariko orchestrator side COMPLETE+green (RPF1 v2 48B emit + §3.2.4 carrier+seal + deploy-set).
  Remaining XIAO check = Seeed schematic-PDF confirm = METAL-BRING-UP item (verify MISO/MOSI on first
  XIAO LoRa light; not blocking). **core CONFIRMED the XIAO
  RF-switch WORKS with Sx1262::new()** (DIO2 keyed unconditionally in configure(); 88f549f added
  with_dio2_as_rf_switch alias) — dropped the false "RF not driven" caveat (worktree HEAD updated). **specs
  landed R2-RUNTIME v0.12 §3.2** stating one-image config-activated PROVEN, citing this build. NVS map now
  ends: role-profile@17000 / anti-rollback@18000 / (reserved) carrier-creds@19000.
- Cross-fleet OPENS (replies in): **core RULED** sdkconfig+FFI correct, NVS-collision N/A for esp32
  (namespaced API), and **platforms/esp32 IDF build + on-metal confirmed-boot is HIVE's** → I must install
  ESP-IDF (espup) to compile-verify platforms/esp32 (Alfred has only esp-hal/xtensa) = OWED. core's
  r2-sx1262 DIO2-RF-switch support = still open (flagged). **workshop CONFIRMED** the XIAO pins vs
  meshtastic seeed_xiao_s3 variant.h (my map was right) — confirm vs Seeed schematic before canon. composer
  = RPF1 emit (48B) + board.toml = queued. §8.1 codec OFFERED to core for r2-discovery::beacon upstreaming.
- SEPARATE TRACK (not firmware): repoint r2-hive-bin/Cargo.toml path-deps at r2-core's now-landed
  r2-def/r2-dispatch/r2-ensemble/r2-transport/r2-discovery (core msg 21:27) — awaiting core 'build green' go.
Canon refs read + pinned: R2-RUNTIME §3.2.1–3.2.4, R2-BEACON §8.1.1–8.1.4. Gap-analysis input doc =
`docs/field-firmware-role-prep.md`. Shorter cycles; update this tracker each step.

---

## (PRIOR) 2026-06-26 — LoRa PHASE 0 (does LoRa survive #20?)
**Re-oriented after a /clear (context-saturation stall).** #20 hardening CLOSED; my DFR signed-OTA
receiver DONE+committed (r2-hive `434132e` + `5c93026`). **TASK NOW = LoRa PHASE 0** (supervisor-directed,
I LEAD): the one test telling us what survived #20 — does LoRa still work on CURRENT firmware (HEAD,
post-#20/hardening)?
1. Build CURRENT unified firmware with `loraroute` feature (full = `nobt,loraroute,loratcxo,multitg`).
   Firmware worktree = `/home/roycdavies/Development/R2/dfr1195-fw-wt` (branch `dfr1195-fw`, was `0ebfd09`).
   Build on Alfred: `source ~/Development/homelab/export-esp.sh` first (xtensa linker).
2. Flash 2 DFR1195s AS923-NZ wairoa (R2-LORA §2.1/§3.1 = TN-FR-1 config). DFR boards are on **tuxedo**
   (`ssh tuxedo`); by-id ports from composer at flash-time. XIAO can't run LoRa (no SX1262).
3. Re-run heartbeat-sync + TN-FR-1 neighbour-discovery/`directed_via`; confirm mutual-RX + HB-sync hold.
**REPORT:** PASS = LoRa survived #20 → restore → Phase 1 parity. FAIL = regression to localise. Framing:
conjecture/refutation, TN-FR-1 re-asserted on current firmware.

### ☑ CHECKPOINT (2026-06-26 ~02:30 NZ) — Phase 0 metal HELD by supervisor; build-PASS = the accepted result.
**SUPERVISOR FINAL CALL:** stand down on Phase 0 metal. BUILD-PASS IS the Phase 0 result that matters —
*LoRa survived #20, confirmed.* Metal mutual-RX + HB-sync is a CONFIRMATION that waits for a clean window
(Roy/composer freeing a 2nd board, or the demo ending) — do NOT interrupt Roy's live demo, do NOT grab the
1 free port, STOP queuing composer. Everything staged at `tuxedo-os:~/phase0/` for an instant run when a
window opens. **Two follow-ups queued (both no-rush, both confirmed to core):**
1. **frame_fingerprint seed-first sig (core 807cab5):** my call-site is main.rs:1403 (A1 option-c
   FingerprintCache). Worktree base (0ebfd09) still has the OLD 4-arg sig → NO break now. When core advances
   the worktree base to include 807cab5: update :1403 to `frame_fingerprint(&seed, fr_origin, msg_id,
   payload, hmac_tag)` + source a 16B secret seed from the ESP32-S3 HWRNG (esp_hal Rng/Trng) for
   DataPlane::new + the call (NOT derived — guessable). Interim [0;16] = sound.
2. **Field-firmware prep (supervisor-offered) — GAP ANALYSIS DELIVERED.** The supervisor (NOT specs) owns
   the field-firmware canon, and it's NOT yet authored (only `docs/planning/FIELD-SENSOR-FIRMWARE.md` plan
   exists) → my current-firmware ground truth is its authoring input. Wrote the full answer in
   `docs/field-firmware-role-prep.md` + sent the supervisor (a)-(e): roles=FOUR (receiver=terminal
   sink/display+absence-track, distinct from bridge=transit); NO config-struct today (role = hive_id-match ×
   features, all hardcoded consts — listed the knobs+values); 8B beacon = my_hive(u32 BE)++seq(u32 BE),
   separate from §12.6 HB (keep distinct, evolve beacon into R2-BEACON §8); per-role deltas; join = persona
   @0x12000 persists, re-attach silently resumes, **NO self-enrol**. TWO new-behaviour flags for canon:
   (i) sensor duty-cycle ADVERTISED not ENFORCED (no real wake/sleep yet); (ii) no autonomous enrol.
   **OWNERSHIP CLARIFIED:** **specs** is the actual canon AUTHOR (it owns R2-LORA/R2-BEACON/R2-ROUTE;
   already landed R2-ROUTE §13.4 + R2-LORA §9.1 LoRa-no-sender-quota; will author R2-BEACON §8 + the
   role-profile) and was EXPLICITLY blocked on hive's gap analysis. Sent the full analysis to BOTH supervisor
   AND specs (specs' earlier fork-ask predated the analysis). **NEXT GATE:** specs pins R2-BEACON §8 + the
   role-profile struct/enum → THEN I implement against the pinned canon (NOT a guessed struct). Both replies
   pending. (Attribution quirk post-account-B: specs↔supervisor msgs sometimes mislabel sender — content is fine.)

### PROGRESS (2026-06-26 ~01:50 NZ):
- **BUILD-LEVEL VERDICT = PASS.** Built current firmware `nobt,loraroute,loratcxo,multitg` on Alfred —
  13.4s, ZERO errors, 24 dead-code warnings only, fresh ELF
  `dfr1195-fw-wt/platforms/dfr1195/target/xtensa-esp32s3-none-elf/release/r2-dfr1195` (1065112B, 01:44).
  LoRa firmware survives #20 at source level (no API-drift from r2-dataplane/route/wire consolidation,
  dc re-emit, H9-secure HB-rx, A1 reconcile). **GOTCHA:** must `source ~/Development/homelab/export-esp.sh`
  WITHOUT a pipe (piping source = subshell = PATH lost → "linker xtensa-esp32s3-elf-gcc not found").
- **BENCH IS LIVE — not a hardware gap.** The `tuxedo` ssh alias is a DEAD tailnet node (7d offline) =
  my timeout. Rig moved to **`tuxedo-os`** (100.90.50.112). All 5 DFR1195 enumerate; TN-FR-1 rig present
  + provisioned Jun22: D1 50:26:98=ttyACM0 (480e900e orig), D2 b7:90:10=ttyACM1 (2cab5f69),
  D3 b6:0a:a0=ttyACM4 (f91c8911), D4 52:99:28=ttyACM3 (06ae082b), D5 50:23:E4=ttyACM2 (0dcadbf8).
- **FLASH PAYLOAD PRE-STAGED** to `tuxedo-os:~/phase0/` = {espflash 4.4.0 (tuxedo-os has none), ELF
  `r2-dfr1195-loraroute`, `dfr1195-partitions.csv`}. espflash runs natively there.
- **GATE = port-release (REFINED ~02:1x NZ).** Orchestrator RESTARTED → PID 3197; now holds
  ttyACM0/2/3/4, leaves **ttyACM1 (D2 2cab5f69) FREE**. Only ONE of two needed ports free → can't run
  mutual-RX yet (needs 2 boards that hear each other; originator role NOT required — any pair works).
  Queued composer TWICE for a 2nd port (unanswered, busy/offline). ESCALATED to supervisor →
  **SUPERVISOR RULING (resolved): hive = STAND BY.** The metal-run is gated on Roy's live demo holding the
  ttys; do NOT interrupt it. Hold until composer/Roy frees ≥2 boards (then run instantly). (Overnight freeze
  was account A's weekly cap; now on account B, fresh budget.) Run script
  is staged at `tuxedo-os:~/phase0/phase0-run.sh` (hardcoded D1 ACM0 + D2 ACM1 — EDIT ports if a different
  pair is freed). **NEXT when 2 ports free:** ssh tuxedo-os, flash both with
  `~/phase0/espflash flash --chip esp32s3 --partition-table ~/phase0/dfr1195-partitions.csv --port <by-id>
  -a hard-reset --non-interactive ~/phase0/r2-dfr1195-loraroute` (partition-table = persona@0x12000 survives),
  monitor both for boot `DEV <maclow3> hive=` + mutual-RX + heartbeat-sync + neighbour-discovery, then
  RESTORE baseline + tell composer to re-attach.
Refs: [[lora-message-passing-metal]], [[dfr1195-firmware-bench-workflow]]. Shorter cycles + /clear when prompted.
(Everything below this block is PRIOR state — kept for recovery.)

---

Updated 2026-06-24 (owned by hive). Master save (read-only ref):
`r2-fleet/fleet-context/FLEET-CONTEXT-SAVE.md` (moved from claude-fleet, now tooling-code-only).

**Role + normative policy** (do-NOT-fork-per-target, authority chain specs→core→hive, before-editing,
stop conditions, no-go): **→ [AGENTS.md](AGENTS.md)**. Live spec-vs-impl forks: **→ [FORKS.md](FORKS.md)**.
This file is **STATE-ONLY** — running state, in-flight work, the session arc. (Policy moved to AGENTS.md per
the F8 process-hygiene split, 2026-06-25.)

**Current branch:** `platform-trait` (local + pushed, HEAD `ce80733`). Built atop the v0.2 work (`0aa6ab7`).

## PCO FIRMWARE MIGRATION SESSION (2026-06-24) — bundle built-green, AT THE FLASH-WINDOW
Spec-first migration of the DFR1195 firmware to **R2-HEARTBEAT v0.5** + an **Occam mesh-retire**, plus the nRF54
data-plane seam. Firmware lives in the **dfr1195-fw-wt WORKTREE** (`r2-core/platforms/dfr1195`); r2-hive holds only
the PATCH (`docs/dfr1195-firstlight.patch`) — the commits below are r2-hive patch-snapshot commits.

**THE BUNDLE (built-green PRE-FLASH, all pushed):**
- `0ad8566` §1A phase-lock -> OPTIONAL: leaderless-PCO (coupling-nudge + rate-consensus + period-jitter-off) goes
  behind an OPTIONAL `pco` feature; DEFAULT = free-run + loose period-jitter + β=0 = the §1A loose-jittered
  keepalive (the FR-1-REL POS-arm, already metal-tested -> a default-flip of TESTED code). Retired loosehb+rateoff.
- `d7507cd` §3B.1 power_state advertise (emit): HB byte 8 = self-asserted availability class, tier-aware (AlwaysOn
  DFR / Intermittent fr4-SENSOR-D1). **FORMAT SUPERSEDED:** specs caught byte-8 FORKS R2-WIRE §12.6 (HB payload is
  a CBOR MAP). Unified pass = re-emit as CBOR key `dc` (RENAMED duty_class — avoids the R2-BEACON §7.2.1 battery
  power_state collision), DROP the redundant 4B origin + fw_ver. The CBOR re-emit + byte-8 REVERT is HELD until
  specs lands §12.6 (see NEXT #2).
- `20703ab` §1A.1 RATE-DECOUPLE (the delicate one): the ~2s phase oscillator still drives fire_seq (the originate
  cadence + LED beat) UNCHANGED, but the keepalive HB-EMIT is throttled to KEEPALIVE_PERIOD_MS=30_000 (the §1A.1
  tunable knob; supervisor-confirmed 30s = "tens of s", DG-1 silence ~90s) — un-conflates liveness from the
  demo/proof signal. pco = every-beat (phase-lock); blackout test arm = every-beat (throttle cfg-gated out).
- `3095804` + `cef7516` Occam MESH-RETIRE (NOT a deletion — HELD+flagged as a compound-gate refactor): step 1 =
  excise the lora_mesh_task fn+spawn (the safe sliver, mutually-exclusive with loraroute); step 2 = ATOMIC
  compound-gate refactor dropping the loramesh/lorareach features — loramesh lived in the FR-2-bridge/ESP-NOW SPAWN
  SELECTORS (main.rs:346/:412/:2893), and since loramesh was NEVER set in any flashed config, not(loramesh)≡true
  everywhere -> each gate-simplification is a VERIFIED NO-OP. lorareach (§4.2 PCO reachback) retired -> simple
  phase-error.
- `ce80733` benchkeepalive feature (OFF by default): KEEPALIVE_PERIOD_MS 8s under the feature else 30s ship —
  ship-safe + reproducible + format-agnostic (dominates the uncommitted-binary option) for bench watchability.
- `7b3cfe3` chore: gitignore `prebuilt/` (14MB binaries out of git history).

**NO-OP INVARIANT (the load-bearing safety claim):** every FLASHED config spawns IDENTICAL tasks after the
gate-refactor — verified per-config (nobt/routetest->espnow; loraroute->LoRa leaf no espnow; loraroute+bridge->
espnow re-enabled; blemesh->neither). The bench is the EMPIRICAL test of this conjecture; if the demo regresses it
REFUTES "the migration preserves the demo" -> spec-first fix, no papering.

**BUILD MATRIX = 7 configs GREEN (errors=0):** fr4 / loraroute+bridge / loraroute / nobt+routetest / nobt /
blemesh / fr4+pco.

**BENCH/SHIP BINARIES STAGED** (supervisor: "you build both"): 6 release ELFs + app-.bin (OTA) + a merged sample at
`prebuilt/bench-bundle-0624/` (GITIGNORED, local-only — the committed artifact is the SOURCE/benchkeepalive feature,
NEVER the binaries) = {leaf (D1/D2 loraroute) / bridge (D3 loraroute+bridge = FR-2) / recv (D4 routetest)} x
{ship 30s / bench 8s}.

**FLASH-WINDOW: OPEN (Roy GO, boards free).** composer flashes/OTAs + monitors the ttys; I (firmware owner)
INTERPRET the 3 verdicts: (a) FR-2 bridge survives, (b) LED-sync + FR-4 NO-REGRESS [the critical one], (c) keepalive
fires + silence-detectable (8s bench). AWAITING composer's serial output to interpret per-item; then SHIP (30s)
binaries onto demo-correct boards.

**SESSION-RESTART RECOVERY:** a post-/compact degradation was cleared by a mid-session restart; the clean 7-config
matrix build (errors=0) + the bundle proved the recovery (supervisor: "welcome back").

**REMAINING / NEXT (priority order):**
1. BENCH-VERIFY (in progress with composer) — interpret (a)/(b)/(c), confirm the ship binaries go on demo boards.
2. duty_class CBOR re-emit — parse §12.6 `dc` on receive + call core's `set_neighbour_duty_class` + REVERT byte-8
   (`d7507cd`); GATED on specs landing the unified §12.6/§1A/§3B.1 pass.
3. r2-dataplane module (POST-bench) — NEW crate `r2-core/crates/r2-dataplane` (no_std; deps r2-route+r2-wire+
   r2-trust; core's location call). hive-OWNED: types `DataPlane`/`RxDisposition`/`PhyMask` + `handle_rx_frame` +
   `poll_keepalive`, factoring the bench-VALIDATED dfr RX logic; UNBLOCKS core's nrf54 gateway `handle_rx` body.
   PhyMask = u8 platform-agnostic egress bitmask (the plan_forward-egress->bit map is the PLATFORM adapter);
   deliver_out = RAW channel push (NOT through r2-dispatch — std/above-boundary). core registers + wires.
4. LED-flash-out (gate the FIRE-driven LED behind pco; coordinate with composer's bench LED-sync check) +
   sensor-piggyback (§1A.1, the SENSOR tier piggybacks liveness on sense-wake).

**KEY DECISIONS this session:** spec-first throughout (read §1A/§3B.1 before coding); HELD-and-flagged TWICE
(mesh-retire = compound-gate refactor not a deletion; power_state byte-8 forks §12.6) rather than blind-executing;
committed-feature > uncommitted-binary for bench (dominates both options); push-per-green-step (standing order).
Deep context in the memory files: occam-hb-simplification, r2-hive-multi-target-goal, lora-message-passing-metal,
linux-hive-deliver-gate-gap.

## OVERNIGHT AUTONOMOUS CAMPAIGN (2026-06-22, supervisor grant; Roy winding down)
Per supervisor: continue the TN metal refutation campaign autonomously — SPEC-FIRST on any weakness
(route to specs, queue for Roy, NO canon mandate overnight), RESTORE the 2-TG baseline after each run
(protect the live demo), commit auditable field.* records, tick off survived refutations, keep this file
current, don't wait per-conjecture.
- **TN-FR-2 (LoRa<->ESP-NOW gateway / DG-2 #16) = PASS / metal-green (2026-06-23).** field.* =
  `docs/field-results/lora-fr2-0623/TN-FR-2.json` (+ raw serial). 4 DFR, ONE TG 'wairoa' (3932969629,
  composer-prov2'd): D1=origin(480e900e) ->LoRa-> D2=router(2cab5f69) ->LoRa-> D3=BRIDGE(f91c8911, dual-radio
  SX1262+ESP-NOW) ->ESP-NOW-> D4=receiver(06ae082b). PROVEN: **D4 (ESP-NOW-only) DELIVERED 12 distinct Events
  that originated at D1 over LoRa (dlv=11) = the Event CROSSED LoRa->ESP-NOW**; the engine AUTO-BRIDGES — D3
  directed_via next_hop=06ae082b x11 (transport-aware best_transport picks the ESP-NOW egress, NO bridge
  routing code); dedup-once-across (D3 DROP-Duplicate x36, each msg_id delivered once = DG-2, dedup keys on
  frame-carried origin, transport-agnostic); bidirectional (D4 replies retrace ESP-NOW->D3->D2->LoRa->D1,
  D1 reply-DELIVERED x12); forced multihop (D1 masks D3-direct x48 via hardcoded can_hear_hive -> D1->D2->D3).
  Delivery ~63% (vs FR-1's 11% — the fast ESP-NOW leg). Firmware (eed35f9): `bridge` feature + PER-TRANSPORT TX
  channels (DATA_TX_LORA vs DATA_TX) + mesh_broadcast (bridge pushes BOTH carriers). Baseline restore in
  progress (composer reflash+reprovision+reattach-5). NEXT: FR-2b = TRUE LoRa<->WiFi/UDP gateway w/ PI5 (Linux
  r2-hive RECEIVER over real WiFi, composer pre-provisioned pi5 keystore); then FR-4 capstone (role sim +
  TN-FR-1-REL loose-jittered-HB two-arm). See [[lora-message-passing-metal]].
- **TN-FR-2 (LoRa<->WiFi gateway / DG-2 #16) = UNBLOCKED + DESIGNED, build pending composer's board map (2026-06-23).**
  core CONFIRMED (DG-2/BL-300/BL-301 sim-validated): (1) **dedup is transport-agnostic** — DedupCache keys on
  (frame-carried origin, msg_id) ONLY, so a LoRa-received frame re-forwarded on WiFi is NOT re-delivered/looped
  (dedup on RECEIVE; engine excludes the inbound source_hop from the flood set). (2) **MTU = handle-the-reject**:
  engine select_transport uses the FLAT LoRa MTU (222) but the DRIVER transmit() rejects > the actual lora_mtu(SF,BW)
  (e.g. 51@SF12) — so the bridge MUST check lora.send()/transmit() result and DROP that egress on reject (BL-301;
  never truncate/fragment, R2-TRANSPORT §2.2). (3) **the engine AUTO-BRIDGES**: NeighbourEntry.transports is a
  bitmask; plan_forward returns Hop{neighbour,TRANSPORT} and picks egress per hop — NO bridge routing code.
  FIRMWARE DESIGN (the bridge node = composer's D3, on both LoRa + the WiFi-island carrier):
  - Run BOTH carriers (lora_route_task + the WiFi-island carrier) feeding the SHARED DATA_RX; pass frame-carried
    origin (TN-FR-1 proved). Airtime-gate the LoRa egress via service(now_ms)+set_neighbour_count (WiFi->LoRa
    Events DEFER under load, not drop). Drop-on-LoRa-MTU-reject.
  - **KEY ARCH CHANGE**: DATA_TX is a CONSUMING channel (each frame -> ONE carrier), so it does NOT broadcast on
    both. Need PER-TRANSPORT TX routing: either split into DATA_TX_LORA + DATA_TX_WIFI (each carrier drains its
    own) with io_task pushing per advice's egress transport (Hop.transport for Directed; BOTH for Flood), OR a
    transport selector on DATA_TX. This honors core's Hop{transport} auto-bridge. Leaf nodes (LoRa-only, WiFi-only)
    use just their one channel.
  - **TRANSPORT-TAGGED INGEST**: the HB ingest_observation currently HARDCODES transport=EspNow (main.rs ~954);
    thread the ingress transport through DATA_RX (add a tag to MeshRxFrame) so the bridge's neighbour table tags
    LoRa-neighbours vs WiFi-neighbours correctly = what makes plan_forward's auto-bridge work (directed). Flood
    bridging works WITHOUT it (broadcast both + dedup), so a flood-first proof is the lower-risk first run.
  composer's FR-2 DEFS (RECEIVED, locked; full defs catalogue/topologies/wairoa-fr4/, this = fr4 minus the
  WiFi-router): **D1=origin (480e900e), D2=LoRa-router (2cab5f69), D3=BRIDGE (f91c8911, SX1262 LoRa + onboard
  WiFi), RECEIVER=PI5 (ssh pi5, Linux r2-hive over WiFi/Internet = the marae hub).** PATH: D1 ->(LoRa)-> D2
  ->(LoRa)-> D3[bridge] ->(WiFi)-> PI5. MASK: D1->[D2]; D2->[D1,D3]; D3->[D2(LoRa),PI5(WiFi)]; PI5->[D3]. ONE
  TG 'wairoa' spanning both islands (gateway test, not isolation — the bridge carries the GroupHmac across;
  keys ~/.r2/group-keys.json#wairoa, composer provisions/hands over). composer PROVISIONS + builds the gateway
  dashboard view; hive builds bridge/leaf fw + flashes + runs via ssh. **SCOPE NOTE: the WiFi side is a REAL
  WiFi link to a LINUX r2-hive (PI5), NOT ESP-NOW — so D3's 2nd carrier = onboard WiFi/UDP to PI5, and PI5 runs
  the r2-hive Linux/std build as a 'wairoa' routing RECEIVER (its RouteEngine delivers + the receive-flash
  logs). Bigger integration than DFR-only FR-1.**
  OPEN PREREQ (asked composer, queued): how D3 reaches PI5 over WiFi in r2-hive's model — UDP broadcast on a
  shared LAN (D3 STA + PI5 on one router/AP)? D3 joins a PI5 AP? which port / the existing wifi.rs UDP path? +
  confirm PI5 runs r2-hive Linux as the wairoa routing peer. Don't build D3's WiFi carrier blind = spec-first.
  FIRMWARE FOLLOW-UP (board-map-independent, do in the FR-2 build): (a) transport-tagged DATA_RX ingest — construct
  Observation with the REAL ingress transport (Transport::Lora vs Wifi) instead of hardcoded EspNow (main.rs
  ~954); core confirmed engine auto-populates NeighbourEntry.transports + plan_forward picks egress (dual-homed
  D3 = both bits on one entry, best_transport per-MTU). (b) msg.* telemetry over /r2 — PINNED schema (R2-CBOR,
  event NAME discriminator, compact-int body): msg.tx{0:id,1:from,2:to} / msg.rx{0:id,1:at,2:from_hop} /
  msg.relay{0:id,1:at,2:next_hop(0=flood)} / msg.delivered{0:id,1:at,2:dup}; id=loraroute msg_id stable across
  the 4 (routed to specs to pin). (c) LED on_received receive-flash + relay-flash (composer 👍). PROOF target:
  Event D1 -> D2 -> D3 -> PI5 delivered EXACTLY-ONCE across the bridge (DG-2 dedup-once, transport-agnostic).
  Reliability (loose-jittered-HB + retransmit) = TN-FR-4 capstone two-arm (specs TN-FR-1-REL). See [[lora-message-passing-metal]].
- **TN-FR-1 (BL-200-over-LoRa MESSAGE-PASSING) = PASS / metal-green (2026-06-23).** field.* =
  `docs/field-results/lora-fr1-0623/TN-FR-1.json` (+ raw serial). Routed Events A->B->C over LoRa on 3 DFR
  (A=480e900e, B=2cab5f69, C=f91c8911 — all TG-A), MASK-forced multi-hop: **C DELIVERED A's REQUESTs via B
  (dlv=2), directed_via B (next_hop=C for A->C, next_hop=A for the replies), exactly_once (B DROP-Duplicate
  x4), reply retraced C->B->A and DELIVERED at A, LED fires on receipt.** Baseline (2-TG demo) restored
  (reattach-5, health 200). KEY METAL LESSONS: (1) the released D1/D2/D3 originator is **480e900e** (MAC
  50:26:98), NOT 0dcadbf8 (that board, MAC 50:23:E4, stays in the demo) — re-keyed the MASK + auto-origin.
  (2) build needs **multitg** so all 3 use the NVS-provisioned TG-A key (else C can't HMAC-verify A's Event).
  (3) **synchronized-fire collisions** on the half-duplex air dropped most frames (B's TX reached A/C ~1/100s
  under lockstep); an **ALOHA TX-jitter (0-300ms) in lora_route_task** decorrelated TX starts enough to prove
  the path. RELIABILITY FINDING: per-msg delivery ~2/19 at SF7 w/ always-on tight PCO -> the reliability
  fix = Roy's refinement (HB as LOOSE jittered BACKGROUND path-maintenance, lower rate) + retransmit; feeds
  TN-FR-4. CORRECTNESS proven; the data-plane (core's LoRaTransport::service + frame-carried origin) holds.
  Firmware below ⬇ (loraroute) was the staged build; this run added the jitter + 480e900e re-key + multitg.
- **TN-FR-1 firmware (loraroute) — built atop the staged work below (2026-06-23).**
  Roy's #1: route an Event A->B->C over LoRa on 3 DFR1195, MASK-forced multi-hop (A can't hear C), validate
  directed_via B + exactly_once@C + LED-flash on RECEIPT (not heartbeat). The DEFERRED CSMA/heartbeat-mesh
  redesign is NOT this. Built a new **`loraroute`** feature (= `lora` + `routetest` + `r2-transport/alloc`):
  - Uses core's READY `LoRaTransport::service(now_ms)` data-plane (continuous-RX + TX-pacing + §4.2/§4.3
    airtime-gating, defer-not-drop) instead of the naive half-duplex `lora_mesh_task`. New `lora_route_task`
    drains DATA_TX -> LoRa, feeds RX -> DATA_RX; carries ALL frames (Events, not HB-only like loramesh).
  - Thin **`RxenRadio`** newtype impls `LoRaRadio` to toggle the DFR1195 RF switch (GPIO42 HIGH-RX/LOW-TX)
    around transmit/listen/standby — keeps the RXEN concern in the per-platform layer (LoRaTransport is
    chip-agnostic). The one-codebase seam.
  - **4-byte immediate-sender hive PREPEND** per LoRa frame = the LoRa analogue of ESP-NOW's L2 src MAC on
    a MAC-less broadcast medium: feeds the hive-based `can_hear_hive` MASK (hardcoded A={B} B={A,C} C={B},
    no fragile tty provisioning) forcing A->B->C, and threads the true RELAYER as src_hive into DATA_RX for
    the §4.3.4 TrailReinforcer.
  - **ForwardRequest.origin = frame-carried originator** (was hardcoded `0`) — the BL-200/M-ESPNOW-3 fix,
    core-confirmed: per-(origin,msg_id) dedup is what makes exactly_once + directed_via hold multi-hop.
  - **LED flashes on DELIVERED receipt** (RECEIPT_SIGNAL; heartbeat envelope suppressed under loraroute).
  - Board A auto-originates REQUEST->C at boot (loraroute default SENDTO) = self-contained 3-board run.
  BUILD GREEN: `cargo build --release --features nobt,loraroute,loratcxo` -> ELF staged (983KB) on alfred,
  ready to flash. NOTE: the `dfr1195-fw-wt` worktree is a SEPARATE stale clone of r2-core — I synced its
  `crates/r2-transport/src/{lora_transport,lora,lib}.rs` to canonical core (commit 027a912, airtime-gating)
  to get `service(now_ms)`/`set_neighbour_count`/`lora_mtu`. Patch regenerated: `docs/dfr1195-firstlight.patch`.
  BLOCKER (NOT idle): composer can't release the DFR ttys on tuxedo — the `reattach-dfr-45.sh` ssh is
  approval-gated, needs the operator or Roy's morning. composer pings `dfr-fr1-off` when 0 holders. THEN:
  flash 3 DFR (A=0dcadbf8, B=2cab5f69, C=f91c8911), watch C's LED flash on each routed message, capture
  directed_via/exactly_once serial -> commit `field.*` TN-FR-1, restore baseline. Ladder after: TN-FR-2
  (LoRa<->WiFi gateway, DG-2), TN-FR-4 (role-based sensor/router/receiver Wairoa sim).
- **DONE: BL-200 RESOLVED + PASS/metal-green** (one-line reply-msgid u16-dedup collision; fix=shared
  `r2_route::trail::reply_msg_id`, commits up to `9fe9068`; §4.3.4 vindicated, §4.6-MUST refuted; baseline
  restored-clean 5/5 DFR multitg). Metal field.* count: BL-100 survived, BL-200 resolved-pass.
- **DONE: BL-103 SURVIVED** (`3a32856`). §2.5 neighbour eviction+rediscovery holds on real ESP-NOW: silent
  board EVICTED from the route-engine nbr table (conf->0.01), ACTIVE neighbour RETAINED (selective, not a
  flush), returning board REDISCOVERED fresh. Method: fastevict route-clock x20 (1800s horizon->seconds) +
  blackout[60,150)s + NBR-TBL telemetry, 3 isolated XIAO (MASK->NVS). TUNING: x120/x40 amplified conf-
  variance (evicted active too); x20 = clean contrast. Reused real engine decay_neighbours. No spec weakness.
  field.* = TN-L1-IT-BL-103.json. Baseline restored. **3 metal field.*: BL-100 survived, BL-200 resolved-pass,
  BL-103 survived.**
- **DONE: WiFi HB-sync SURVIVED** (`c4082c0`, TN-L0-IT-HBSYNC-WIFI). Leaderless PCO converges over WiFi/UDP
  (3 XIAO SoftAP star, X1=AP via AP_MAC_MATCH flip, spread_ms->0-4ms, synced=true) = engine is TRANSPORT-
  AGNOSTIC (ESP-NOW + WiFi). **4 metal field.*: BL-100, BL-200, BL-103, HBSYNC-WIFI.**
- **BLE 2-board sync BLOCKED** (finding): blemesh M8b negotiation hardcodes M7_PROVIDER_HIVE=0x0dcadbf8 (a
  fixed test peer) -> elects an absent provider for arbitrary pairs -> no CoC. Needs generalizing; BLE is
  L0-2-node-only regardless -> BLE-mesh = WAIROA-7 queued for Roy.
- **⚠️ X4 (2c81b4a3) NEEDS A POWER-CYCLE (Roy, morning):** its USB-JTAG de-enumerated during the WiFi run
  (port vanished from /dev/serial/by-id); X1/X2/X3 restored fine to multitg (one-off X4 USB casualty, not a
  defect). X4 is OFFLINE / stuck on the WiFi build until physically re-plugged. The 5 DFR + 3 XIAO are clean.
- **🔦 LoRa FIRST LIGHT ACHIEVED (`7387686`) — TOP priority, the Wairoa rung is ALIVE.** Bidirectional
  LoRa between 2 DFR1195 SX1262 radios: D2 RX from=480e900e (rssi-44 snr12), D1 RX from=2cab5f69 (rssi-45
  snr13), clean 8B payload every cycle. Wired core's r2-sx1262 onto the DFR1195 via esp-hal (SPI3 SCK7/
  MISO5/MOSI6 + NSS10-CS + BUSY40/RST41 + RXEN42 + Delay; Sx1262::new().with_tcxo(V1_8)) + a concrete-typed
  lora_task (configure->listen->loop{poll RX; TX beacon}, RXEN HIGH-RX/LOW-TX). VALIDATED on RF: TCXO DIO3
  1.8V PLL-lock, RXEN42 polarity, DIO1, full driver API, wire (sync0x21/916.8MHz), RSSI/SNR. BENCH config
  (overrides, NOT defects): SF7 (SF12 ~2s ToA vs ~3s windows = partial-catch CRC-err at 30cm = timing
  artifact; SF7 ~40ms clean) + tx_power -9dBm (30cm; deployment +20/+22). field.* = LORA-FIRSTLIGHT.json.
  Baseline restored (D1+D2 multitg). **5 metal results: BL-100, BL-200, BL-103, HBSYNC-02/wifi, LoRa-first-light.**
  NEXT (supervisor ladder): (1) core's RXEN driver param (drop manual toggle); (2) LoRa MESH = bridge
  io_task (PCO + r2-route) to the LoRa carrier (like espnow/blemesh) = multi-board LoRa heartbeat+routing;
  (3) SF12 real-distance range test; (4) cross-transport LoRa<->WiFi gateway (DG-2 #16 = HBSYNC-07 coherence).
- **LoRa MESH = PARTIAL-FINDING (`b872008`, HBSYNC-02 transport=lora).** Built loramesh (io_task PCO+routing
  bridged onto the LoRa carrier via half-duplex lora_mesh_task, ESP-NOW gated off). PCO syncs TIGHT pairwise
  over LoRa (D2 e=0.001 spread=2ms = engine+bridge WORK) but the 3-board mesh doesn't SUSTAIN (nbrs->0):
  (1) LoRa airtime (130ms+ SF7) uncompensated in the PCO phase = §4.2 reachback the interop spec flagged
  for LoRa, METAL-CONFIRMED (D1 spread 245ms desync) -> routed SPEC-FIRST to specs/core; (2) naive bridge
  floods all traffic over the slow half-duplex link -> HBs starved. NEXT: §4.2 airtime-comp (specs/core +
  lora_airtime::time_on_air_ms — asked core if landed) + hive carrier traffic-shaping (HBs-prioritized,
  ToA-aware) -> clean LoRa mesh -> SF12 range -> LoRa<->WiFi gateway (DG-2 #16). **6 metal results: BL-100,
  BL-200, BL-103, HBSYNC-02/wifi, LoRa-first-light, LoRa-HBSYNC-partial.**
- **HBSYNC-03 sustain re-run (§4.2+shaping) = NOT green yet — deeper finding (`4700c0a` has §4.2+shaping+
  lorareach).** Ran specs' 2x2: arm2 (shaping+§4.2) + arm1 (shaping-only) BOTH = no 3-board reception
  (nbrs=0). Debug PROVED HBs TX'd fine (b0=0x29 mt=5 txd=true), Events dropped -> NOT shaping/§4.2. ROOT
  CAUSE = my naive half-duplex lora_mesh_task poll-loop (drain DATA_TX + poll RX + 10ms yield) has an
  RX/TX listen-window timing flaw -> radio misses peers multi-board. NEXT BUILD = redesign lora_mesh_task
  per core's CONTINUOUS-RX / event-driven / ToA-aware pattern (DIO1-IRQ RX + listen-before-talk/CSMA for
  the synchronized-fire collision; asked core for a reference shape). HB on metal = 30B unsigned (nobt),
  §4.2 ToA used 62B -> use actual frame_len. §4.2+shaping are correct components (kept). Baseline restored.
- THEN (per supervisor): cross-transport LoRa<->WiFi gateway (DG-2, #16); BLE-mesh 'perhaps' (WAIROA-7);
  LR2021 (composer leads). SECONDARY: WiFi MASKED routing (IP-MASK port; specs queued BL-203/200-over-wifi/
  BL-000/AB-000/BL-001) + BL-100 demote sweep (#13). M-ESPNOW-3 (carry frame-origin->ForwardRequest.origin,
  core contract confirmed engine.rs:56-64; + H1 authenticate route_stack[0]) = canonical BL-200-class kill.
  SIM-ONLY (specs): BL-204 idle-fade, L2-XT-BL-200, silence-is-signal (~40000s idle).
- **M-ESPNOW-3 follow-up:** carry frame-origin in the relay frame -> ForwardRequest.origin -> r2_route
  (origin,msg_id) dedup = the canonical fix that kills the origin-degraded class (beyond BL-200).


## Active (besides the branch) — priorities per Roy (2026-06-16)
- **NEXT TRACK — TN REFUTATION MATRIX (hive = METAL runner).** Roy's big campaign: every
  routing+message-passing edge case across ALL transports, conjecture/refutation, coverage dashboard.
  Axes: topology(L0 full/L1 multihop/L2 SCF-beyond-radio/L3 partition+heal) × scope(intra/inter-TG) ×
  trust-plane(above/below-TG) × payload(events/data) × transport(BLE/WiFi/ESP-NOW/LoRa/UDP) + edge cases.
  Flow: specs authors matrix+schema (IN PROGRESS) → core sim-tier harness → **hive runs the METAL tier on
  the 9 co-located boards spanning all radios** (`field.*` = metal only). **SPEC-FIRST INVIOLABLE:** weakness
  found → note + route to specs BEFORE any code. CLEAR until the matrix lands; supervisor points me at the
  first tranche. Prereq proven: 9-board co-located 2-TG ESP-NOW mesh LIVE. See memory
  [[tn-refutation-matrix-campaign]].
- **METAL TIER LIVE — FIRST field.* RESULT LANDED (`34aef54`).** TN-L2-IT-BL-100 (RSSI-σ mobility
  classifier, tier=hardware-exclusive) **SURVIVED on real ESP-NOW.** Built the `rssicls` firmware (real
  per-recv RSSI from r.info.rx_control.rssi → rolling per-neighbour σ → §2.4 classify σ<5dBm=Infra →
  feed obs.mobility; r2-route does the differential decay). 3 XIAO captured ~11min hearing the live
  9-board mesh: 49/49 settled (n≥20) readings σ<5dBm (min 0.19 / max 3.94 / mean 0.94) → classifier
  holds; the prior FINDING B refutation did NOT reproduce. ROBUSTNESS finding flagged to specs (worst
  link 3.94dBm ≈1dBm headroom = thin margin → metal evidence for the §2.4 hysteresis/stationary-margin
  fix). Auditable record + raw serial: `docs/field-results/TN-L2-IT-BL-100.json` (specs' capture schema).
  **NEXT: BL-200 wiring** (the first ROUTING field.*) — TrailReinforcer (`r2_route::trail`, core 7201d02)
  synced into the worktree + compiling; wire topology-mask + reply-send (normal routing + reply_marker) +
  the 3 reinforcer call-sites + decay_paths-from-tick + directed_via/exactly_once telemetry → run on 5
  ESP-NOW boards. Then BL-103 (eviction+rediscovery, reuses directed_via telemetry + blackout arm).
  KEY: metal REUSES r2-route::RouteEngine + r2_route::trail = field.* validates the REAL engine+policy.
- **BL-200 (first ROUTING field.*) DONE — PARTIAL / sim-vs-metal DIVERGENCE (`8480089`).** 5 DFR ESP-NOW,
  routetest build (full BL-200 firmware: topology MASK + §4.3.4 TrailReinforcer + A->D origin + reply
  emitter; commits 71f4f82/34efe11/141e6ad/d98fc64). PROVEN on metal: directed_via converges adjacent-to-dest
  (R2->D 20/20, flood->directed over time) + exactly_once@D (20x1) + alt-X no-steal. REFUTED: end-to-end —
  upstream A->R1, R1->R2 STAY FLOODING after 20 clean reply round-trips. The §4.3.4 reply-confirmed trail
  forms at the hop adjacent to dest (unambiguous reverse link D->R2) but NOT upstream where the reply floods
  back over un-converged paths (strong-reinforce sees varying senders -> path-to-D never concentrates).
  Routed SPEC-FIRST to specs + core (spec refinement: pin reverse next-hop? / refutation / hive wiring).
  Record: `docs/field-results/TN-L1-IT-BL-200.json` + raw serial. PROCESS: first run contaminated (demo
  lowest-hive emitted 49 Events) -> gated demo off under routetest + dropped <64,64,64> workaround (core
  9497a60 made trail generic) -> clean re-run. Baseline RESTORED (5 DFR -> multitg, rejoined TGs).
  **2 metal field.* results: BL-100 survived, BL-200 partial-divergence.** NEXT: BL-103 (eviction+rediscovery,
  reuses directed_via telemetry + blackout arm); re-run BL-200 if specs/core refine §4.3.4. LESSON: the
  metal tier earns its keep — it found a real sim-vs-metal divergence the sim 8/8 could not.
- **BL-200 RESOLVED (`bdc4d3b` fw + `bc6e029` field.*=resolved-pass).** The divergence was a ONE-LINE
  FIRMWARE BUG, not a spec gap. Root cause (metal-pinpointed via instrumented RT-DBG of core's 3 bits
  contains/sender/path-conf): the reply REUSED the request's msg_id + dedup keys on `(msg_id as u16)` ->
  reply collided with the already-forwarded request -> DROP Duplicate at every relay -> reply died at the
  hop ADJACENT to dest (still reinforced via on_received-BEFORE-dedup) -> never propagated upstream = the
  exact "adjacent-converges, upstream-floods" signature. Ruled OUT: spec gap, CAP (256>>~28), broadcast-
  overhearing (MASK isolates; core's sim silent/converged-everywhere). FIX = distinct reply msg_id
  `h.msg_id | 0x8000` (LOW-16 since dedup truncates — a first 0x8000_0000 attempt still dropped, caught on
  metal). VERIFIED isolated 5-DFR: R1->R2 directed_via, path-conf 0.66->0.96 (was flooding); R2->D 0.984;
  D exactly_once. §4.3.4 ADEQUATE (specs+core agreed). ENABLERS: MASK-NVS @0x15000 + SENDTO-NVS @0x16000
  (defeat the capture serial-open DTR-reset that wedged earlier runs) + a tight composer handshake (zero
  race). LESSON: metal found+pinpointed+FIXED a bug the SIM STRUCTURALLY COULD NOT (no u16-dedup-truncation
  nor on_received-before-dedup model). Instrument-first + spec-first prevented a canon change for a wiring
  bug. **3 metal field.*: BL-100 survived, BL-200 resolved-pass.**
- **🎉 9-BOARD CO-LOCATED CROSS-HOST MESH LIVE (0622.1517, serial-verified).** Roy directive: bring the
  4 XIAO ESP32-S3 on **alfred** into the leaderless mesh with tuxedo's 5 DFR1195. DONE. Built the SAME
  `nobt` leaderless-0.4 firmware ON alfred (esp toolchain; `source ~/Development/homelab/export-esp.sh`
  for the xtensa-esp-elf gcc — NOT `~/export-esp.sh`), flashed all 4 XIAO via espflash + the 4MB OTA
  partition table (`r2-hive/docs/dfr1195-partitions.csv`) + board-profile `0x00 0x00 @0x13000`
  (has_screen=false, led_active_low=false). Per board: ttyACM1 14:C1:9F:C4:FC:8C→af1464f4 · ttyACM2
  E8:3D:C1:FB:DB:44 · ttyACM3 D8:3B:DA:75:C3:3C→2c81b4a3 · ttyACM4 E8:3D:C1:FB:E5:20→998de7fc.
  RESULT: all 4 XIAO `synced=true nbrs=8` — each hears the other 8; peer maps include ALL 5 tuxedo DFR
  hive_ids (50:23:E4=0dcadbf8, 52:99:28=06ae082b, B6:0A:A0=f91c8911, B7:90:10=2cab5f69, 50:26:98=480e900e).
  spread 749ms→0-3ms cross-host (alfred+tuxedo, SAME ROOM) + cross-arch (XIAO+DFR1195) — RF is board-to-board,
  host-agnostic, exactly as Roy predicted. **XIAO LED = NO code change:** GPIO21 is hardcoded for BOTH
  carriers + polarity DEFAULTS active-HIGH (read_board_profile) = exactly what the XIAO external LEDs need;
  a per-target LED change would have DIVERGED the build and split the mesh. **8MB vs 4MB:** XIAO flash=8MB,
  DFR=4MB; used the 4MB table for production-parity (meshing unaffected by unused upper flash) — revisit an
  8MB layout (`docs/dfr1195-partitions-8mb.csv`) at the OTA phase.
- **STEP 3 — 2-TG per-TG keying firmware: IMPLEMENTED + COMPILES (committed; metal proof pending composer).**
  Behind a new `multitg` feature (live `nobt` demo byte-for-byte unaffected; BOTH `nobt` and `nobt,multitg`
  build green on alfred/xtensa). **Inc1 (`6e2eeca`) runtime PROVISION receive:** uart_rx_task reads the board's
  OWN USB-serial RX (composer SECURITY correction — the secret GroupHmac key must NOT go on the air like the
  IDENTIFY mesh-frame; point-to-point USB only) → `r2_trust::provision::parse_provision(line, my_wire=my_hive)`
  (core `0b44e56`, USED not re-implemented) → `write_provisioned_tg` persists {magic,tg_id,32B key} raw @0x14000
  (own 4KB sector; read-back verified) → `PENDING_PROVISION` hands the key to io_task → swaps live GroupHmac +
  target_group (no reboot); boot restores from NVS (overrides persona/demo). `tg_id`==`my_tg_hash` (fnv1a_32(UUID)
  decimal = frame target_group). ACK on serial: `PROVISION-APPLIED wire=<8hex> tg_id=<dec>` / `PROVISION-ERR`.
  **Inc2 (`5678837`) HB-signed + verify-gated coupling:** the heartbeat pulse is now `sign_extended(group_hmac)`'d
  and the io_task couple-gate flips from plaintext `target_group==my_tg_hash` to `verify_extended(&m,&group_hmac)`
  (specs §6.3 — coupling REQUIRES a GroupHmac-verified pulse). A TG-A node fails-verify a TG-B pulse → no couple
  → 2 independent sync clusters on shared RF = the logical-partition proof. **HB wire change → all-9 coordinated**
  (a multitg node won't couple to an unsigned nobt pulse → a 2-board multitg pair SELF-ISOLATES from the nobt
  mesh = a clean self-contained test). **Board→TG split (composer-confirmed):** TG-A=177560432 {D1 480e900e, D2
  2cab5f69, D3 f91c8911, X1 998de7fc/ACM4, X2 c2106bd5/ACM2}; TG-B=1584099016 {D4 06ae082b, D5 0dcadbf8, X3
  af1464f4/ACM1, X4 2c81b4a3/ACM3}. **NEXT (coordinated w/ composer):** flash a 2-board multitg pair (proposed
  ACM2=TG-A + ACM1=TG-B alfred XIAO) → composer provisions direct-to-tty → confirm NO cross-TG coupling, then
  re-provision same-TG → confirm coupling (minimal refutation), then all-9 rollout. BLOCKER: composer's
  orchestrator holds all 4 alfred XIAO ttys (the alfred dashboard feed) — it must release ports before I flash.
- **STEP 3 — METAL-VALIDATED (`4614a7a`, alfred XIAO pair, test keys over direct USB).** **Inc1 PROVEN
  end-to-end:** PROVISION-APPLIED with the correct 32B key (fingerprint key0=cc key31=cc xor=00), live
  GroupHmac+target_group install w/o reboot, NVS persist + boot-restore (`PROVISIONED TG restored from NVS
  — tg_id=1584099016`). **Inc2 verify-gate PROVEN by two controls:** POSITIVE (same key → couple) via the
  persona key (nbrs=1 when both multitg+unprovisioned); NEGATIVE (TG-A vs TG-B provisioned → HB-DBG
  `verify=false` → nbrs=0, no coupling, self-isolated from the 7 nobt boards too) = the cross-TG isolation.
  The provisioned-same-key positive is logically identical to the persona positive; composer's reliable
  provision_bridge completes it for the record. **METAL-FOUND BUG FIXED:** IDENTIFY-era uart_rx line buffer
  was `[u8;64]` → truncated the ~94B PROVISION line (key cut → BadKeyLength) → bumped to `[u8;128]`.
  **HARNESS LESSON:** my raw-tty `printf` PROVISION writes are UNRELIABLE (USB-CDC, no flow control —
  identical write = APPLIED on one board, BadKeyLength on another via byte-drop); the clean positive-control
  + all-9 rollout go through composer's reliable provision_bridge (hive flashes, composer provisions). Use
  `/dev/serial/by-id/` paths (ttyACMn renumbers on reset). **Restored ACM1+ACM2 → nobt + erased provision
  NVS → 9-board mesh WHOLE again (ACM1 nbrs=8 synced=true verified).** Commits: `6e2eeca` Inc1, `5678837`
  Inc2, `4614a7a` buffer-fix. See memory [[dfr1195-firmware-bench-workflow]].
- **CLEAN 2-TG PROOF (composer-driven) + ALL-9 ROLLOUT DONE.** composer drove the clean cross-TG proof via
  its reliable writer (prov2.py: OPOST-clean + my 128B buffer): PHASE A (X2=TG-A, X3=TG-B → both nbrs=0,
  isolated) + PHASE B (re-provision X2=TG-B same as X3 → both nbrs=1, COUPLE) = isolate↔couple driven
  purely by the GroupHmac key. Then on Roy's direct GO, the ALL-9 ROLLOUT: handshake = composer releases
  ports → hive foreground-flashes → composer provisions. hive flashed ALL 9 to the uniform multitg build
  `0622.1624mt9` (4 alfred XIAO local; 5 tuxedo DFR via `ssh tuxedo-os` with espflash binary + ELF + csv
  pre-staged in /tmp — tuxedo has no toolchain). composer provisions per fleet.json (TG-A 5 / TG-B 4) +
  renders. **HOST FACT:** this session runs ON alfred; tuxedo-os is remote (DFR-5 host, no espflash).
- **🎉 CROSS-HOST 2-TG HEARTBEAT LIVE (goal #14, metal) — directive→plan→canon→sim 10/10→metal→LIVE.**
  composer provisioned all 9 + reattached; live /r2 verdict: TG-A(177560432)={X1,X2,D1,D2,D3} all nbrs=4
  (fully coupled, cross-host alfred+tuxedo); TG-B(1584099016)={X3,X4,D4,D5} coupled (2 full + 2 marginal-RF).
  CROSS-ISOLATION CLEAN: TG-A sees 0 TG-B, TG-B sees 0 TG-A — the GroupHmac partition holds on ONE shared
  9-board ESP-NOW mesh, cross-arch (XIAO+DFR). Residual = bench RF (TG-B's 2 marginal members want the
  powered hub for tight convergence; the partition is clean). **XIAO LED FIX (Roy ground truth):** the 4
  XIAO LEDs are ACTIVE-LOW (roster said active-HIGH = WRONG) → wrote board-profile [0x00 0x01] @0x13000 on
  all 4 (byte1=0x01=active-low firmware convention; verified X3 read-flash=00 01 + boot led_active_low=true
  + TG key survived @0x14000). hive writes the polarity byte (composer's board.toml byte1 convention is
  OPPOSITE). DFR-5 = active-high (untouched). See memory [[dfr1195-firmware-bench-workflow]].

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
    (`~/Development/R2/dfr1195-fw-wt`); patch handed to core.
  - **⚡ WiFi/embassy MATRIX RESOLVED + COMPILES** (worktree Cargo.toml; memory [esp32-wifi-embassy-matrix]).
    The blocker was NOT a version bump: esp-wifi→**esp-radio** rename (esp-wifi 0.15.x links-collides on
    xtensa-lx-rt ^0.20 vs esp-hal 1.1.x ^0.22), scheduler esp-hal-embassy→**esp-rtos** (superseded, wanted a
    private esp-hal feature). VERIFIED set (resolves + compiles xtensa, 58s, 241K ELF): esp-hal **1.1.1**
    (unchanged) / esp-rtos 0.3.0 (esp32s3,embassy,esp-radio) / esp-radio 0.18 (default-features=false,
    esp32s3,wifi) / esp-alloc 0.10 / esp-bootloader-esp-idf 0.5.0 / embassy-net **0.9.1** / embassy-sync 0.7 /
    embassy-executor 0.10 (default-features=false) / embassy-time 0.5 / xtensa-lx-rt 0.22. **DRIFT flagged to
    core:** wifi.rs targets embassy-net 0.6 → needs same-day turn to **0.9** (IpEndpoint::from + UdpSocket::new
    /Stack lifetime). **NEXT (field.lab):** migrate main.rs bare-metal→esp-rtos/embassy async + esp-radio STA +
    embassy-net Stack, re-enable mod wifi (once core's wifi.rs@0.9), spawn udp_writer_task, wire RouteEngine →
    board A originates → board B receives+relays (dedup/TTL/spray). network-OTA receiver rides the same tier.
  - **🎯🎯 FIELD.LAB DONE — first routed R2-WIRE frame board↔board on REAL HARDWARE** (`a99313b`). WiFi-up
    smoke PASSED (soft-AP r2-fieldlab 192.168.4.1 ↔ STA .2, role auto-by-MAC), then the routed frame: board A
    (hive 502698) originates an R2-WIRE *extended* Event over real WiFi radio → board B (b79010) decodes +
    `r2_route::RouteEngine::plan_forward` + **DELIVERED msg_id=7..13 ttl=4 'hello-TN'** + **DEDUP** the
    duplicate. Stack: esp-radio 0.18/esp-rtos 0.3/embassy-0.9, one combined recv/send UDP socket task (port
    21042), static IPs. **HW finding (confirms core's B1):** RELAY ≠ DELIVERY — first cut let plan_forward's
    relay verdict (Drop NoViableNeighbour on a 2-board leaf) mask delivery; separated → delivers. Boards: my
    field.lab pair = ttyACM0(AP 502698)/ttyACM1(STA b79010), by MAC via /dev/serial/by-id; workshop's 3
    DFR1195s = ACM9/10/11.
  - **🎯 THE FLEET WORKS — synced LED heartbeats over TN** (`cb8fa14`). Both boards run a leaderless
    Mirollo-Strogatz pulse-coupled oscillator: fire = LED beat + broadcast R2-WIRE `Heartbeat` frame;
    receiving the peer's fire = advance-only phase nudge. Initialized 1.1s apart → phase-lock ~60ms apart
    (proven coupling: crystal drift <1ms/26s). Serial: AP `HB phase 0.97->1.00` then `FIRE` (pulse triggers
    fire); STA convergence `0.70->0.82->0.97->lock`, `synced false->true`. Clock = embassy_time (esp-rtos
    time-driver). composer's HeartbeatSync sentant = CONDUCTOR-PLL (std tier); mine = leaderless PCO (MCU) —
    flagged the mixed-TG model-alignment Q.
  - **LCD status surface RESTORED** (`988f0ac`) — ST7735S in the async render loop (GPIO48 active-low,
    offset 26,1, Deg90, 20MHz), shows role/ip/TG/build/beats/dlv/`fleet: IN SYNC` from atomics io_task
    updates. WiFi + routed frames + PCO heartbeat + LCD all coexist, no panic.
  - **🎯 GOAL #2 — intra-TG TRUST DELIVER-GATE working on hardware** (`045048b`). Real HMAC-SHA256
    (r2-trust `GroupHmac`, which BUILDS for xtensa — 38s, no getrandom issue) gates delivery at the B1
    deliver branch ONLY; relay stays trust-agnostic. AP originates signed intra-TG Events alternating
    good/bad HMAC; STA: `DELIVERED msg_id=6 'in-TG' (tg+hmac ok)` / `DELIVER-BLOCKED msg_id=7 hmac_ok=false
    (relay unaffected)`, consistent. Canon (core 5f8798b): `target_group = FNV-1a-32(TG_UUID string)` via
    r2_fnv const; `sign_extended`/`verify_extended` (target_group+event_hash inside the MAC). Both boards
    share TG_UUID + hk (demo stand-in for the join). LCD shows dlv/blk.
  - **TONIGHT'S ARC (all on metal, 2 boards):** WiFi ✅ · routed R2-WIRE frame (deliver+dedup) ✅ · synced
    heartbeat ✅ · LCD ✅ · intra-TG trust deliver-gate ✅ · conductor-PLL heartbeat (TG-scoped + version
    telemetry) ✅. **Both headline goals — TN + trust groups — proven + canon-aligned on real hardware.**
  - **CONTINUED-SESSION metal wins (all committed):** N-board broadcast (fire/Event → subnet 192.168.4.255,
    verified) ✅ · **unique per-board STA IP** from low MAC byte (the real N-board fix; .2 would collide) ✅ ·
    **organic lub-DUB LED heartbeat** via LEDC PWM hardware duty-fades (Roy: "heartbeat not flash"; io_task
    FIRE_SIGNAL → main renders the envelope) ✅ · **OTA bootloader CONFIRMED (test a)**: my no_std app boots
    under the ESP-IDF BL (extract first 0x8000 of /tmp/dfr1195-merged.bin → espflash --bootloader; "Loaded app
    from 0x20000" + app runs) — the OTA BL blocker is closed ✅ · esp-storage builds for xtensa ✅. STA
    (ttyACM1) now runs the ESP-IDF BL. Conductor-PLL note: locks but ~0.1-period steady-state OFFSET (tighten
    with β freq term / higher gain — refinement).
  - **MORE continued-session metal wins:** **conductor-only beaconing (NO-FLOOD)** — only the conductor beacons
    the fire, followers PLL-listen silently ✅ · **2nd-order conductor-PLL (β/freq term)** — kills the ~200ms
    offset, e→±0.005–0.025 (<50ms), 5 LEDs as ONE ✅ · **5-board mesh** (my 2 + composer's 3, ESP-IDF BL) ✅ ·
    **real-TG persona reader (#20)** — read bundle raw @0x12000, r2_cbor-decode, run on PROVISIONED hk/tg/derived-
    hive; **TG=4b3df45d OFF DEMO** on both my boards (persona=true), cond=3e0d688f, synced=true, DELIVERED good /
    BLOCKED bad on the real hk ✅. Hand-rolled derive_hive_id (HKDF→v4-UUID-string→FNV; r2_trust::derive_hive_id
    not in pinned r2-trust). **KS1-CANONICAL derive_hive_id** — re-synced r2-trust to **abde165** (the no-v4-forcing
    fix; 256489b + my hand-roll BOTH v4-forced = matched each other but DIVERGED from KS1). ids now byte-exact to
    composer: **502698→480e900e, b79010→2cab5f69** (were the wrong v4-forced 3e0d688f/cce44b60). Conductor re-elects
    to lowest (STA 2cab5f69); AP follows+locks (STA→AP broadcast direction also confirmed). r2-trust pinned abde165 ✅. **OTA test (b) PASS** —
    wrote valid image to ota_1, firmware activate_next_partition() + reboot, ESP-IDF BL booted ota_1 @0x200000;
    both OTA prereqs CLOSED; converted to report-only (production-safe). Op-note: espflash flash does NOT reset
    otadata — erase 0xf000/0x2000 to recover a board to ota_0 ✅.
  - **EVEN MORE wins (this session):** **health #18** — r2.hb.health CBOR (13-key), every-5th-beat, followers
    DIRECT to the collector AP, AP logs `HEALTH <hex>` for composer's orchestrator serial-reader; verified e2e
    (AP collects own 480e900e + STA 2cab5f69) ✅ · **shared parse_persona** — adopted r2_trust::parse_persona
    (core 1b93108), dropped my decode glue; one codebase with workshop ✅ · **carrier-aware has_screen** — LCD
    init+render gated on board-profile byte @0x13000 (0x00=XIAO no-screen, else=DFR1195); ONE binary runs on
    screenless XIAO-S3 (9-board) ✅ · **perfect sync** — 2nd-order PLL now locks to e=-0.000 (zero offset) ✅.
    r2-trust pinned 1b93108. 9-board = 5 DFR1195 + 4 XIAO-S3 (all-S3, true PLL, GPIO21 LED); role-by-MAC →
    only 502698=AP, XIAO=STA; composer flashes my binary + provisions XIAO (persona@0x12000 + 0x00@0x13000).
  - **9-BOARD MESH CONFIRMED (metal) 🎉** — composer flashed all 4 XIAO + 3 DFR1195; ALL on tuxedo USB
    (my ACM0=AP/ACM1=STA, XIAO ACM2-5, DFR1195 ACM9-11). Verified synced=true + dlv climbing (trust delivering)
    across composer's DFR1195 (ACM9/10/11 dlv~1692) AND a XIAO (ACM2) = cross-arch (S3 DFR1195 + XIAO)
    beat-as-one on real TG 4b3df45d, conductor = lowest canon id 06ae082b. AP serial held by r2-compos
    (composer orchestrator) = the health #18 dashboard feed working by design; do NOT re-flash the live AP.
  - **OTA network receiver (#17)** — DE-RISK PASSED (flash-write-while-WiFi: 20ms/sector, heartbeat-safe, no
    quiesce). Receiver built (UDP 21043 START/DATA/COMMIT stream → sector-write → SHA-256 → activate+reboot) +
    otadata anchor (Factory→ota_0 so activate→ota_1 seq=2). PROVEN: 512KB stream+write+sha_ok+valid 0xE9 image+
    activate ok + test-b slot-switch. NOT yet cleanly e2e (board-to-board boot-INTO-ota_1 snagged on test-
    corrupted otadata + can't test on the live AP). Test sender gated OFF (OTA_SELFTEST=false). Next clean
    verify: a fresh-otadata board, NOT the live soft-AP. LESSON: never re-flash the live soft-AP mid-demo.
  - **LATEST (0621.1227):** **per-carrier LED polarity** — XIAO-S3 GPIO21 is ACTIVE-LOW (inverse of DFR1195);
    profile byte1 @0x13001 (0x01=active-low; erased→active-low iff no-screen, so XIAO byte0=0x00 already works);
    LEDC idle + lub-DUB envelope polarity-mapped ✅. **#23a conductor-timeout re-elect** — forget a SILENT
    conductor after 4 beats → re-elect next-lowest; healthy conductor = no churn (replaced the churny every-3
    forget) ✅. **AP-SPOF live (#23b):** the soft-AP (502698) went dark (my live re-flash wedged it) → STAs
    stranded (no network → no app-layer election can help; my STA came up alone/CONDUCTOR). FIX = revive 502698
    (Roy physical RST; port held by composer's health reader so no remote reset). **#23b AP-FAILOVER = the real
    fix, NOT YET built:** pre-designated backup (lowest AP-capable hive from the heartbeat roster) detects
    esp-radio disassociation + promotes STA→AP at runtime @192.168.4.1; others re-scan/associate. Substantial +
    risky (runtime WiFi mode switch) — implement on a test pairing, not the live mesh.
  - **CONVERGENCE BUG FOUND + FIXED (serial-verified, 0621.1227):** the 9-board "not converged" root was a
    VERSION MISMATCH — 3 DFR1195 (ACM9/10/11) were on a STALE pre-KS1 build (0621.0858) computing WRONG hive_ids
    (a0dce700/63f798ea/b658276e) → SPLIT-BRAIN conductor election (boards disagreed on the lowest id). XIAO were
    on 0621.1148 (pre-LED-polarity → dark). FIX: re-flashed all 7 accessible boards to 0621.1227 (KS1 ids + LED
    polarity + conductor-timeout). RESULT (direct serial): 8/9 lock to cond=06ae082b (=529928/ACM10), e≈0.000,
    synced=true, cross-arch (DFR1195 + XIAO). 9th = AP 502698/ACM0 still dark on old build (port held by
    composer's health reader) → revive via Roy RST (beats+follows) or composer port-release + re-flash to canon.
    LESSON: a mixed-build fleet WILL split — keep ALL nodes on one build; verify by SERIAL not telemetry.
  - **9/9 CONVERGED + UNIFIED + AP REVIVED (0621.1244, serial-verified) 🎉** — all 9 on ONE build/span;
    single conductor = ACM10 (529928→06ae082b); all 8 others (incl the AP) lock cond=6ae082b synced=true
    e≈0.000 cross-arch (5 DFR1195 + 4 XIAO). AP 502698 revived via composer port-release re-flash → canon id
    480e900e, role=AP, beats as follower. **AP later re-wedged → composer un-wedged it (espflash-reset,
    firmware intact) → all 9 back to sync_state=1; composer fixed the dashboard feed (their plugin poll bug,
    NOT my HEALTH format — parsed all 9 byte-exact). Health dashboard LIVE.**
  - **XIAO LED FIXED + ROBUST (Roy confirmed correct).** The XIAO GPIO21 LEDs are EXTERNAL active-HIGH (not
    the built-in active-low user LED). The byte-toggle (0x13001) was FRAGILE (composer's 1-byte re-provisioning
    leaves byte1 erased → the old !has_screen inference re-inverted on every re-flash). FIX (committed, 0621.1314,
    re-flashed the 4 XIAO): read_board_profile DEFAULTS active-high — led_active_low only on byte1==0x01 explicit
    override; NEVER infer from has_screen (polarity is hardware/wiring-specific, not SoC-derivable). Robust across
    re-flash + re-provisioning. **R2-WIRE v0.6**
    (msg_id-in-HMAC-span) = deferred: SEPARATE all-9-coordinated update; current bench all on the same span.
  - **#24 BLE↔WiFi TWO-PLANE — STARTED (Roy: now the focus; AP wedged again = the motivating need).**
    Architecture settled (workshop+core, r2-route pattern): pure no_std S0–S4 negotiation ENGINE in
    **r2-discovery** (core lands it from my interface) behind a **NegotiationRadio trait**; radio glue
    per-platform (hive=esp-radio, workshop=esp-idf); protocol primitives reused (r2-wire/trust/beacon);
    reuse `lowest_live_id` (conductor election). DELIVERED: the engine interface (S0–S4 table + trait
    surface) → core, who **LANDED THE ENGINE** (r2-discovery::negotiation, 03648fb — pure no_std heap-free
    S0–S4, 4 tests green, conforms my §4A table). core's answers: engine carries its own thin roster
    (NegotiationEngine<16>); `lowest_live_id` exported; trait = poll_scan→NegObservation{hive_id,caps} /
    send_control+poll_control(HiveId) / bring_up_provider+join_provider(DataPlaneParams fixed-buf) /
    data_plane_state→TransportState / now_ms; drive eng.poll(&mut radio) each tick + request_data_plane()
    + set_power_state(); new(my_hive,my_caps,5000,10000). Eligibility source: R2-BEACON §7.2 flags — power_state
    bits 1-0 readable NOW, provider_capable bit 2 PENDING Roy's authorization (I model both). **MY NEXT = the
    esp-radio NegotiationRadio impl** (THE focus): control plane (ble HCI + trouble-host: advertise RBID+flags
    / scan / L2CAP CoC) + data plane (existing SoftAP/UDP → Available/Failed). BLE foundation scouted
    (esp-radio `ble` HCI + trouble-host/bt-hci). Big lift: deps+coex → HCI↔trouble wiring → advertise → scan
    → L2CAP, on a TEST PAIRING first. Subsumes #23/#23b (wedged AP → auto-renegotiate over BLE). §4A Profile-A.
    (AP-WEDGE cause diagnosed: esptool-flash on the LIVE AP wedges it — NOT the read-only health-reader; use
    `systemctl --user stop/start r2-orchestrator` around any AP re-flash; the durable fix is this BLE-failover.)
  - **NAMED REQUIREMENTS (roadmap, careful test-pairing — NOT on the live mesh):** #23b **AP-FAILOVER** (Roy:
    "TN should renegotiate the hotspot if it goes away") — pre-designated backup (lowest AP-capable hive from
    the roster) detects disassociation → promotes STA→AP (same SSID/IP) → others re-associate; conductor-timeout
    app-half DONE, WiFi-layer half remains open. **BLE-BEACON discovery** (R2-DISCOVERY) = the out-of-band substrate
    that solves the no-network-to-elect chicken-and-egg (beacon presence/hive_id/TG/AP-capability/roster over
    BLE, independent of the WiFi-AP) — #23 negotiation rides it. **IDENTIFY** cmd (LED solid on /r2 identify).
    **PER-CARRIER PLATFORM BUILDS — REQUIRED (Roy, reverses the earlier deprioritization).** Next firmware
    deliverable = SEPARATE DFR1195 (4MB/no-PSRAM) + XIAO (8MB/octal-PSRAM) binaries running the SAME ENSEMBLE
    (identical logic; only the platform layer differs) = unified-hive proof (logical=portable, platform=
    per-carrier). Architecture in docs/r2-per-carrier-builds.md: ONE crate, features carrier-dfr1195(default)/
    carrier-xiao; ensemble shared (no cfg) — io_task heartbeat+route+trust+persona+health+IDENTIFY+#24 engine;
    platform #[cfg]-gated — PSRAM init (xiao), LCD init (dfr1195), LED/screen. Partition flash-time (4MB/8MB
    CSVs both pushed). hive builds the 2 binaries (esp toolchain) from composer's ONE ensemble + 2 board.tomls;
    composer flashes per MAC-reservation. **The has_screen/LED bytes become #[cfg] carrier CONSTS → RETIRES
    the fragile profile-byte.** Carrier-detection boot-guard (MAC-OUI + PSRAM-probe → reject wrong-build) =
    hive's. composer leads composition (CARRIER-COMPOSITION.md, sdkconfig=Path-A/std only; my Path-B uses Cargo
    features). FOLD into the SAME next deliverable as the #24 BLE stack. (composer driving both S3 targets now.)
  - **IDENTIFY (Roy locate-a-board) — DONE + VALIDATED.** Device-side: r2.hb.identify Directed frame →
    target LED SOLID ~5s override (polarity-aware), refresh/clear. INJECT-BRIDGE (uart_rx_task): reads
    "IDENTIFY <wire_hex> <1|0>" off the USB-Serial-JTAG RX half + broadcasts the frame; runs on every board,
    composer points --identify-port at b79010. VALIDATED on b79010: RX-sharing OK (esp-println TX intact)
    + inject works. composer flipping --identify-port now (composer-side done, 7ec3706). NOTE: the device-
    side override needs the IDENTIFY build on each TARGET board (only b79010 has it now → rides the next
    fleet re-flash). sync_state→0/1/2 (composer dashboard now treats 1=locked; resolved). LED byte DROPPED
    by composer (byte1 reserved; polarity = my active-high default + a Cargo feature) — fragility gone for good.
  - **#24 BLE→WiFi — ACTIVE, 3 METAL MILESTONES HIT (Roy: push now, not parked).** Off-by-default `ble`
    Cargo feature (live fleet still builds). On b79010 (--features ble), all metal-verified:
    (1) **deps resolve+compile** — esp-radio ble+coex + bt-hci 0.8.1 + trouble-host 0.6.0;
    (2) **BLE controller inits + WiFi+BLE COEX holds** (BleConnector + WiFi mesh stays synced);
    (3) **trouble-host ADVERTISE up + EXTERNALLY SCAN-CONFIRMED** — bluetoothctl on tuxedo sees
    `Device C0:52:2C:AB:5F:69` (= my random addr, hive 2cab5f69), while the board stays WiFi-synced.
    (4) **REAL R2-BEACON codec wired + advertising** — `ble_task` uses `r2_discovery::beacon::{compute_rbid,
    encode_advert, LegacyBeacon, BeaconFlags, PowerState}` (core, byte-exact) → 24-byte canonical payload in
    the 0xFF manufacturer AD; metal: `BLE advertising R2-BEACON rbid=471a93a8.. (24 B)`; external scan
    confirms `ManufacturerData 0x01b2` (the encode_advert output, vs the old 0x3252 placeholder).
    **VERSION-COMPAT (the #1 risk) SOLVED: trouble 0.6.0 = bt-hci 0.8** (esp-radio 0.18; 0.2=bt-hci0.3 /
    0.7=bt-hci0.9 both mismatch). Built against core's **r2-discovery @9996fa3** (beacon+negotiation;
    default + --features ble both build clean). **Advertise CANON-CORRECT**: `my_key =
    derive_beacon_session_key(&hk, my_hive)` (PER-MEMBER, HKDF(hk, salt=r2-beacon-rbid-v1, info=hive_be32)[..16]
    — core fb5b189; a TG-wide key would make all RBIDs identical) → compute_rbid; metal-verified rbid changed
    per-member key, Expand-only construction @9996fa3, metal rbid=baf64d9d. epoch=0 still placeholder until a shared coarse-time base.
    (5) **SCAN + RESOLVE on metal — S0 DISCOVER COMPLETE.** ble_task ADVERTISES + SCANS concurrently
    (join3: run_with_handler + advertise + scan). R2ScanHandler.on_adv_reports → ble_find_mfg_ad →
    decode_advert → resolve_rbid_windowed(rbid, registry, epoch, 1) → hive_id. 2-board metal: ACM11
    (0dcadbf8) scans → `BLE scan -> peer hive=2cab5f69 (rbid baf6..)` resolving ACM1, both advertising +
    WiFi-synced. Full cross-board crypto chain proven. (BUG fixed: ScanSession must be HELD — its Drop
    cancels the scan.) registry=KNOWN_HIVE_IDS bring-up roster (real roster from peers.rs/persona later).
    (6) **M7 L2CAP CoC CONNECTIVITY on metal** — provider (lowest test hive 0dcadbf8) connectable-advertises →
    Advertiser::accept (ACL) → L2capChannel::accept(PSM 0x00D2); joiner (2cab5f69) central.connect →
    L2capChannel::create → send. METAL: provider `CoC RECV 7 B: [05,00,52,32,2d,4d,37]` = `[len_lo=5,len_hi=0,
    "R2-M7"]` — the LE len-prefix frame (R2-BLE §6.4) crossed BYTE-EXACT, matching workshop's esp-idf l2cap.rs
    (interop-ready). Repeatable. **So the two-plane is REAL on metal: S0 DISCOVER + control-plane data path both proven.**
    **NEXT: M8 NegotiationRadio** (re-integrate non-conn beacon + scan + HiveId↔addr map + HiveId↔Connection map +
    shared r2_discovery::ControlMsg codec [core landing]) → **M9 run S0–S4 engine** → **M10 network-forming + fallback/reform + telemetry**.
    Full plan: docs/r2-24-l2cap-implementation-plan.md.
    (7) **M8a — NEGOTIATION ENGINE LIVE on metal.** EspNegRadio (sync NegotiationRadio façade) over static
    bridge queues (SCAN_OBS/CTRL_OUT/CTRL_IN/DATA_PLANE) + engine_task running NegotiationEngine::<16>. METAL
    (ACM1): `NEG state -> Negotiate provider=Some(0x2cab5f69)` -> `Data` — the §4A S0→S1→S2 state machine RUNS,
    elected itself provider (alone, provider_capable), bring_up_provider→Available→Data (formed). Sync↔async
    bridge + engine integration PROVEN on metal. NEXT M8b: rewire ble_task to FEED the bridge — scan→SCAN_OBS
    (real peers) + conn-mgr (CTRL_OUT↔CoC↔CTRL_IN, the M7 CoC) → multi-board discover→negotiate→form; then
    M8c real WiFi bring_up/join (currently stubbed Available) + M10 fallback/reform + telemetry.
    (8) **M9 NETWORK-FORMING on metal — discover→negotiate→form, 2 boards.** Both elect 0dcadbf8 (lowest
    provider_capable, leaderless §4A.3); joiner sends WifiReq [0x01] over the L2CAP CoC → provider RECV →
    WifiOffer (7B) → joiner RECV → both reach DATA. serve_coc bridges CTRL_OUT/IN↔CoC; engine drives via the
    sync façade; shared ControlMsg codec byte-exact cross-board. Election-race fixes: continuous peer-obs
    refresh + ~3s discover-delay. **HONEST:** bring_up/join_provider STUB the WiFi (DATA_PLANE_AVAIL=true) →
    "Data" = forming-logic reaching S2, not a real SoftAP. So **discover→negotiate→FORM negotiation PROVEN on
    metal**; data-plane bring-up is M8c. NEXT: **M8c** real SoftAP/STA (runtime WiFi reconfig) → **M10**
    fallback/reform (lose-AP→S3→S4→reform) + composer telemetry (key13/14/15).
    (FIX noted: the crates index was stale → `cargo search` refreshes it before resolving trouble.)
    (9) **M8c — REAL two-board WiFi FORM on metal (BLE→WiFi network-forming COMPLETE).** Provider serves its
    own SoftAP "r2-tn-form" from boot; joiner is a STA configured for it but connects ONLY on the engine's
    join_provider (after the BLE WifiOffer) via DATA_PLANE_JOIN→wifi_task connect_async. METAL: joiner
    `data plane UP — joined r2-tn-form (REAL WiFi formed, B->W)` + provider `[ap] station joined` = a REAL WiFi
    association formed by BLE negotiation. Full chain on hardware: discover→elect lowest (0dcadbf8)→negotiate
    WifiReq/WifiOffer over the BLE L2CAP CoC→FORM real WiFi. **cfg-gated: default (mesh) build UNTOUCHED**
    (serve_ap=is_ap/r2-fieldlab/wait_config_up); ble = M8c (serve_ap=elected/r2-tn-form/form-on-negotiation).
    **THE WHOLE TN ON HARDWARE: S0 discovery + M7 CoC + M8 engine-bridge + M9 forming-negotiation + M8c REAL
    WiFi form** — it discovers, negotiates, and forms a real infra-less WiFi network. NEXT: **M10** = lose-AP →
    S3→S4→reform (self-HEALING) + composer telemetry (key13/14/15); the M8c boards form their own net
    (r2-tn-form) separate from the mesh — coordinate proof-surface wiring w/ composer at M10.
    (10) **FORM→SYNC VERIFIED ON METAL — acceptance criterion #1 COMPLETE (infra-mode).** 2 boards: discover →
    negotiate over BLE → form real WiFi → **lub-dub-SYNC together**. Joiner (2cab5f69): `HB<-192.168.4.1 cond=dcadbf8
    e=-0.000 (lock)` `synced=true dlv=5`; provider (0dcadbf8): `synced=true role=AP` `FIRE seq=27/28 (CONDUCTOR)`.
    Two fixes verified: (a) conductor-send TIMEOUT-guard (was stalling at beat 8 on SoftAP-no-STA) → fires
    continuously; (b) role-align is_ap=serve_ap → provider correctly role=AP. So discover→negotiate→form→SYNC
    works on hardware. **STRATEGIC PIVOT (Roy/supervisor): reality2-mesh ARC greenlit** (specs→core→hive) — the
    GENERAL case = ESP-NOW/WiFi/LoRa TRUE-MESH (no AP; mobile wearables, continual reform); this infra-mode
    (SoftAP-star) is KEPT as mode-1b (fixed/workshop). ESP-NOW verdict: docs/r2-espnow-mesh-verdict.md (feasible
    + favored; esp-radio has esp-now; reuses S0-M9+route+heartbeat; kills AP-role/two-IP bug). QUEUED for hive
    (after specs+core): platform Transport impls (ESP-NOW hive_id↔MAC + UDP) + mesh-mode + M10 runtime-elected-
    single-AP (infra). Rig: use /dev/serial/by-id MAC paths (provider F4:12:FA:50:23:E4, joiner F4:12:FA:B7:90:10).
  - **Per-carrier Cargo features** (composer board.toml mapping): `display` (DFR1195 LCD) + `psram` (XIAO
    octal-PSRAM@80MHz baked via PsramConfig in code — esp-hal has no psram Cargo feature); next deliverable.
  - **PRECISE NEXT STEPS:** (1) composer re-flashes its 3 with the persona-reader (personas survive app-flash)
    → all 5 OFF DEMO on the real TG; I verify 5-board real-TG sync. (2) **OTA network receiver (#17)** — the
    slot-switch is PROVEN (test b); remaining = UDP image transfer + write ota_1 with esp-radio QUIESCED
    (esp-storage#31) + sha256 + activate-on-commit; flash-touching = careful. (3) **health #18** — r2.hb.health
    CBOR, UNICAST to collector (NOT broadcast, per af4ebcb), every-5th-beat+on-change, ota_status from slot
    report. (4) dedup v0.4 (origin=route_stack[0]; future
    r2-route bump). (5) 4-board entanglement (cross-TG gate: GroupHmac first, then trial PeeringHmac; §7.5.4).
    (6) **LoRa rung** — core landed LoRaTransport (fb13b17, r2-transport/src/lora_transport.rs); impl LoRaRadio
    for Sx1262 (wrap lora-phy) → LoRaTransport::new → single-owner lora.service() in the radio task; send()=
    broadcast-on-air so RouteEngine+dedup+trust+conductor-PLL transfer UNCHANGED from WiFi. Swap the ref's
    RefCell<VecDeque> TX queue for an embassy/heapless channel (separate async radio task). Open before TX:
    region/duty-cycle gate, LBT/CAD, RXEN switch (SX1262-LORA-DESIGN.md). Ping core when starting.
  - **QUEUE (post-headline):**
    1. **OTA receiver (#17)** — plan ready (`docs/dfr1195-ota-receiver-plan.md`: OtaUpdater + esp-storage +
       UDP :21043 transfer + sha256 + software_reset). **2 go/no-go prereqs FLAGGED:** (a) espflash's default
       bootloader may not honor otadata for slot-switch → may need a custom OTA bootloader (BLOCKER candidate,
       coordinate core/workshop); (b) flash-write-while-WiFi can hang on dual-core S3 → quiesce radio around
       writes. Run the bootloader test (write ota_1 + flip otadata + reboot) before the full receiver.
    2. **Heartbeat → leaderless CONCAVE-M&S PRC** f(φ)=(1/b)ln(1+(e^b-1)φ) b=3 once specs pins v0.2 (NO rush;
       conductor-PLL holds; drop-in swap of the phase-update, keep the broadcast+jitter). (Canon flip-flopped
       v0.1 conductor-PLL → v0.2 leaderless-concave; supervisor's latest = leaderless-concave for no-SPOF.)
    3. **Real-TG provisioning** — consume composer's keystore (R2-PROVISION): replace hardcoded TG_UUID+hk +
       MAC-low3 hive_id with provisioned device_master_secret + TG persona → derive canonical hive_id
       (FNV(HKDF(secret,tg_id))) + group hk. Asked composer for the NVS layout/read API. Crypto path unchanged.
    4. **N-board scaling (#19)** — fire BROADCAST to all co-members (not 2-board unicast) + multi-peer table;
       converges with the leaderless-concave swap. Then 5-board mesh (my 2 + workshop's 3).
    5. **Health telemetry (#18)** — r2.hb.health CBOR companion (composer's HEALTH-TELEMETRY-CONTRACT), after
       OTA (needs ota_status). 6. **Entanglement** (2 TGs/4 boards, PeeringHmac, lexicographic pubkey order).
    Canon follow-ups: dedup origin = route_stack[0] self-stamp for multi-hop (3rd relay). Hardware → SPECS FIRST.
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
    when the screen is down → don't route LED via the render plugin. **Firmware follow-up:** init the LED
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
- ~~hive usb_pair.rs citation fix~~ DONE (`4c70d2c`) — usb_pair.rs §6.4.x → R2-PROVISION
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
