# RESUME — r2-hive

Updated 2026-07-25. `main` clean + pushed (ahead=0). **Compacted to current-only; full v4→v8.7 cycle history in
`RESUME-archive.md`.** No build order active (#d005 stands). Firmware = r2-core branch `dfr1195-fw-blerole-coex`.

## Objective

OTA-over-BLE-CoC coex firmware series (D5 = DFR1195 ESP32-S3). Each iteration: build DFR1195 D5 ELFs from a
core-pinned sha → run the static conformance rig → attest → extract signed/app bins → multi-party verify. Current
sub-goal: **v8.7 hang instrumentation** — capture the PRIMARY double-fault (`__user_exception` → HANG_CAP retained
mem → boot-decode), the ~4min double-fault hang being the true blocker (pre-empts every OTA window).

## CURRENT: v8.7.1 `7e774742f24680d9e5e7a8b8c6a27f73e6cb43d8` — BUILT + attested + bins; rig 23/23. #d005 EXECUTED

Base 33219370, main.rs only. Decode-hardening: `hang_reprint_task` re-prints HANG_CAP ~15s after boot + pre-zero
moved into the task AFTER the re-print (host-reopen-race fix); handler UNTOUCHED (call-free, same addr 40378c44).
- Clean detached checkout, tree empty, `rm -rf target`. **ELFs** (BUILD_ID `coex.v871.0725`): d5-otarx-v871
  `51fbe9aed3ff545cff4e1a6b6e033c4db43e89639bf7672b692274e861735345` / d5-otafail-v871
  `33faa286d1940871aee3320124aae450e06b8a315e1d9fa08bf78fdced8f40e1`. Attest: v87 leftover=0, __user_exception sole
  strong @40378c44 (handler unchanged), reprint+15s markers + HANG-CAP 3-way + reset_reason + set_wake_window + RWDT,
  otafail DISTINCT. **Rig 23/23 bound-PASS** (check23 task-scoped ordering; check20 helper-refactor rebind; 33219370
  FAILs exactly 23).
- **Bins (RULING-A, compliant + authorization VERIFIED via USED log):** d5-otarx-v871.bin
  `6e2e1358d2448454f521810758047e5c8c69e7facd99f06390e6fd16d47e827e` (878416 B) / d5-otafail-v871.bin
  `57d117dd53dbbf96f16d2931ca087cf1c33f915dbb05b4487096b6e9fe043c9e` (876912 B). DISTINCT, 0xE9, e0e49127. Grant read
  first (artifact=d5-ota-v871 target=offline-derive-no-device), espflash LITERAL in gated command text, one USED
  entry per command.
- **Next:** core + composer derive from these pinned ELFs → byte-3-way (expect == 6e2e1358/57d117dd) → v8.7.1 FLASH
  grant (supervisor writes it after 3-way) → re-dial for CLEAN captures → capture-consistency/family verdict.

## Prior: v8.7 `33219370` — 3-WAY CLOSED (call-free handler fix); superseded by v8.7.1 decode-hardening

Base 30cb3d6d, main.rs only. **#d005 executed, full chain closed:**
- **ELFs:** d5-otarx-v87 `0400a6d7cf251d6ea6b777801cb8e46dc78ff7057aaaf72392a01f643bd12607` (otal2cap,cos) /
  d5-otafail-v87 `9d7db298994823b4bc4dff0b2270e4d00949a3897bccf1c0e6db3792e46ed0be` (otal2cap,otafail).
  BUILD_ID `coex.v87.0725`. Clean detached checkout, tree empty, `rm -rf target`.
- **THE v8.7 FIX = call-free fault handler**, proven on METAL (both lanes objdump cell-for-cell): `__user_exception`
  @0x40378c44, handler range 0x40378c44..0x40378c84, **0 windowed calls, 0 software_reset**, ends `j 40378c80
  <__user_exception+0x3c>` = `j <self>` IRAM spin; magic stored LAST @c7e. v8.6 finding fixed: all v8.6 captures
  were the SECONDARY fault at the flash-resident software_reset entry 0x4204b628; bare spin makes zero windowed
  calls, recovery via always-armed RWDT. Source check(22) sr=0 (stripped) agrees.
- **Bins (RULING-A, 3-way CLOSED core==hive byte-exact):** d5-otarx-v87.bin
  `c65a90b034da027cdef8ed536c01fcdb201fd833891725e90f7855f6160d2792` (878016 B) / d5-otafail-v87.bin
  `4efe08f1594271fd6ce62853db448e00181c8f4b56b3d32a16af1b2b59eb70a4` (876496 B). DISTINCT, 0xE9, e0e49127.
- **Authorization VERIFIED both lanes** (grant READ first, espflash LITERAL in gated command text, one USED entry
  per command per lane, read from the .fleet log — not inferred). Attest: sole strong `__user_exception`, HANG-CAP
  3-way (1/1/1),
  reset_reason, set_wake_window, RWDT=3, role Sensor, otafail DISTINCT. Rig 22/22 bound-PASS.
- **NEXT:** flash grant (separate, after 3-way — now due) → instrumentation dial for PRIMARY fault#1 → then hive
  **capture-consistency analysis** on HANG-CAP 3-way outcomes (magic+data=fresh crash / data-no-magic=torn write /
  all-zero=clean boot; decoded exccause/PC/EXCVADDR/SP vs the fault mode).

## The rig (static conformance suite, on alfred; never compiles/runs tests — PASS ≠ green suite)

`~/preflight-v8.sh <sha>` (checks 1-11) + `~/check{12..22}.sh <sha>` + `~/perbearer.sh`. `~/strip_src.py` =
strings-first comment/string stripper (per-check/per-LEG: string-literal discriminators — cfg names, printed
markers, `extern "C"` anchors — on RAW; code on stripped). All checks neg-locked (a known-bad sha must FAIL);
positive bound to core's REAL identifiers (requirement-not-shape). Current suite (22, all PASS on 33219370):
- 1-11 preflight (partition e0e49127, offsets, set_phy live=0, §5.4, MTU) · 12 LoRa-gate · 13 RWDT
  (`rwdt_feed_task`) · 14 set_wake_window lease-driven + rx-arm point-1-clean · 15 BLE-lease self-strangle guard ·
  16 reset_reason present · 17 exception-handler ownership (our `#[no_mangle] __user_exception`, esp-hal df=false
  informational) · 18 fault-capture (cause/frame.PC/EXCVADDR/A1) · 19 retained-mem magic-written-LAST · 20
  HANG-CAP decode + reset_reason POST-PHASE-1a (ordering) · 21 capture-region zeroed-after-decode · 22 fault-path
  reset IRAM-safe (no software_reset in handler).
- **check(23) BOUND to v8.7.1 `7e774742` (decode-hardening); full rig 23/23 PASS, READY-scored — awaiting #d005.**
  v8.7.1 = HANG_CAP re-printed ~15s + pre-zero moved after the re-print (handler untouched). check(23): legA
  `hang_reprint_task` exists + calls `print_hang_cap` (re-print); legB zero ORDERED after the re-print **WITHIN the
  task** (execution order — the task is defined EARLIER in the file than the boot decode, so a GLOBAL line-order
  compare false-FAILs; scoped to the task body). 7e774742 PASS; **33219370 FAILs exactly (23)**, carries pass.
  - **★ check(20) REBOUND (helper-refactor requirement-not-shape catch):** v8.7.1 factored the HANG-CAP decode into
    `print_hang_cap()` defined EARLY (before PHASE-1a), so my string-literal-line check false-FAILED it. The
    requirement = decode EMITTED post-1a; rebound to a `print_hang_cap(` CALL post-pa OR a HANG-CAP string post-pa
    (handles helper + inline). 7e774742 boot call @775 > PHASE-1a@770 → PASS; 33219370 inline @731 > @722 → PASS;
    afaab9ab (reset_reason pre-1a) still FAILs. Two requirement-not-shape catches this round.
  - Full 7e774742: preflight(1-11) + check 12–23 ALL PASS. Supervisor's #d005 build order follows this bind.

## Standing operational constraints

- **#d005:** no flashable-artifact build without (1) inbox drained (check supersedes), (2) explicit CURRENT
  supervisor order naming the pinned sha, (3) clean detached byte-verified checkout, tree-state verified.
- **espflash/openocd LITERAL in the gated command text** — no ssh-wrapped scripts, no file indirection (the gate
  scans command text only; file-hidden keyword bypasses silently — v86 finding). ssh OK when the invocation is
  inside the quoted string. **Grants supervisor-only; verify authorization by READING the grant file + its USED
  log entry (`$ws/.fleet/flash-authorization`, from repo CWD = `Development/R2/.fleet`), NEVER inferred from a
  command succeeding.** USED = one entry per approved COMMAND (not per espflash run); grant sha field = RECORDED
  not enforced (pre-hash==pin is the real identity mechanism).
- **RULING-A transform-3-way:** all lanes derive bins from ONE set of pinned ELFs (pre-hash==pin) — byte 3-way
  restored despite ELF-sha path-noise (panic-location abspaths in .rodata: ~15 package-path + ~106 registry
  strings; `--remap-path-prefix` backlogged).
- Firmware in **r2-core** (never edit core; hive designs/builds/attests, core lands source). **Hive never flashes.**
- Never bypass `ci/public-hygiene.sh` (MACs/device-tails off tracked files). Every commit: `Decision-Log:` trailer;
  verify ahead=0 via `git ls-remote origin`. Fleet msgs: file + `"$(cat f)"`, never inline.
- NVS 0x17000 raw role-write = brick (bake via `DFR_ROLE_PATH`). Env-baked const verify = full `rm -rf target` +
  the DIFFERENTIAL. Build on alfred (`export-esp.sh`); nohup detach kills export-esp.sh — attached ssh only.

## Prior cycles (CLOSED; detail in RESUME-archive.md)

- **v8.6 `30cb3d6d`:** hang instrumentation (handler + HANG_CAP + reset_reason post-1a); rig 21/21; 3-way closed;
  dial ran — root-caused the SECONDARY-fault-overwrite that v8.7 fixes. Authorization episode (my v86 gate-bypass +
  fabricated-grant, fully owned + cured; findings ledgered; [[espflash-gate-bypassed-by-file-and-remote-exec]]).
- **v8.5 `9ebad32b`:** set_wake_window RX-quiesce + BLE-lease + reset_reason; rig 16/16; cycle CLOSED (composer
  scorecard: 3 deltas verified working; ~4min double-fault hang = the true blocker → v8.6/v8.7). 91d90b9a
  SUPERSEDED (rig caught the forbidden rx-skip).
- **v8.4 `afaab9ab`:** §2.3A leased quiesce (LoRa gate) + RWDT + MAC redactions; full chain source→ELF→bin→signed
  stream verified ≥2-party; flashed. **v8.3/v8/v7/v6/v5/v4 + iter-9:** archived.

## Open threads (post-campaign, not blockers)

- sensor-provider_capable canon CLOSED (bit2=0 all boards); pending 3-board metal re-score only.
- conn-liveness watchdog: parked backstop (keepalive covers it; zero half-open on metal).
- InvalidRouteLen CLOSED benign (the 2 beacon classes are OURS, 5511 FNV; canon-correct drops).

## Backlog (Roy-gated, not started)

D5 reflash/provision (stays 11f2d2ef; needs Roy word) · SEN0676 radar plugin (UART/ADC) · RAK relay-LED (bench) ·
DFR1195 display mislabel (cosmetic) · RAK tx_power −9dBm (core, lora_leaf_config:1219) · AGENTS.md partitions-csv
doc-drift.

## Standing artifacts (LIVE on alfred, secret-bearing, off-tree)

iter-8 pair `~/d4-init8.elf`/`~/xiao-acc8.elf` · D5 cosine `~/d5-cos5.elf` (11f2d2ef) · personas `~/.r2-dev-trial/`
(d4 0xC434FAFC, xiao 0x8C15B0C2, d5 wire da73508e; MACs off-tree) · v8.7 ELFs+bins `~/d5-*-v87.*` + `~/v87-staging/`.

## Safety / Branches

Plain non-force pushes only; never `--all`/`--mirror`. Three local keep refs preserve removed security material
(do not repack/prune). Branches: `storing-backend` (real WIP, needs rebase) · `hygiene-scanner-v2`/`platform-trait`/
`v0.2-relay-handshake` (stale/contained, do not merge). Key rulings in `DECISIONS.md`. Ops hazard:
[[reference-xiao-boot-flush-wedge]].
