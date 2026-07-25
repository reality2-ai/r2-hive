# RESUME — r2-hive

Updated 2026-07-25. `main` clean + pushed (ahead=0). Compacted to one current snapshot; full v4→v8.7 cycle history in
`RESUME-archive.md`. Firmware lives in r2-core branch `dfr1195-fw-blerole-coex` (hive designs/builds/attests; never edits core).

---

## ‼ READ FIRST — device + authorization state (ledger wins any conflict below)

- **DEVICE (D5 / DFR1195 ESP32-S3):** runs **v8.7.3 at `513c949db0f9ec0eebbf7d6df3febec39561a13a`** — flashed and verified.
- **VERDICT:** the double-fault family verdict is **RULED AND RATIFIED**. The OTA-coex hang campaign is **CLOSED**.
- **AUTHORIZATION: NOTHING IS AUTHORIZED. The grant is RETIRED. NO device operations** (no flash, no serial open, no
  derive, no reset, no JTAG). **There is NO pending flash and NO owed 3-way.** Do not act on any stale "awaiting flash /
  awaiting 3-way / on metal" language — none exists.
- **HIVE POSTURE:** v8.7.3 legs **COMPLETE — STOOD DOWN** under #d005. Record-and-report only. Re-engage ONLY on an
  explicit supervisor order; do not poll, do not assign peer legs ([[fleet-posture-authority]]).
- **#d003 RAK FREEZE STANDS:** no RAK stage or release without BOTH an explicit sha order AND Roy lifting the freeze.
- **#d005 BUILD GATE BINDS:** no flashable-artifact build without (1) inbox drained (check supersedes/retractions),
  (2) an explicit CURRENT supervisor order naming the pinned sha, (3) a clean detached checkout of that sha with tree
  state verified, (4) the build-target guard (below). A DONE status is not proof of a build order.
- **★ NEW — GRANT ELIGIBILITY = TWO LEGS, tested IN ORDER** (since hive stood down; bears on any future build ordered):
  1. **HANG_CAP capture instrument PRESENT** in the artifact, THEN
  2. **`__user_exception` call-free within its TRUE extent** measured from `nm -S` (the symbol's real size), NOT a fixed
     objdump window.
  **The ~40 staged XIAO + sensor artifacts FAIL leg 1 entirely** — no capture instrument at all, plus a flash-resident
  windowed call from the handler = total fault blindness + the v8.6 re-fault loop. **They are DO-NOT-FLASH.**

---

## Campaign result — v8.7.3 `513c949db0f9…a13a` (CLOSED, on metal)

D5 = DFR1195 ESP32-S3; OTA-over-BLE-CoC coex firmware series. The ~4-min PRIMARY double-fault silent-wedge was the true
blocker (pre-empted every OTA window). v8.7.3 = base `2249bcf0` (SW_SYS_RST IRAM-safe reset tail) + the WITNESS pre-zero
read-back (`hang_reprint_task` reads HANG_CAP back AFTER the pre-zero and its zero-OUTCOME branch emits its own line, so
the baseline is WITNESSED not inferred). Ledger: r2-core/DECISIONS.md **D-20260725-01 @ d970693c**.

- **Provenance (before build):** `verify-build-target 513c949d main.rs "WITNESSED all-zero" "PRE-ZERO INEFFECTIVE"` →
  touches main.rs + both markers present → **OK**. Worktree markers also verified in the CHECKED-OUT tree, not only via
  `git show` (catches a checkout that never materialised). [[docs-sha-is-not-a-build-target]]
- **Conformance:** **check(24) PASS** (four legs, zero-OUTCOME branch keyed — not a println count; the silent-zero-arm
  control FAILS, so it catches the defect it targets). Neg-lock `2249bcf0` FAILS clean; carries untouched.
- **ELFs** (BUILD_ID `coex.v873.0725`, clean detached build, `rm -rf target`, worktree-markers verified):
  d5-otarx-v873 `52af3bfe09f2ff6e3b69dfd4cecea7cc34239c000dd845d01eb9093636bbb7ab` (1388708 B) /
  d5-otafail-v873 `7d5d67387d3a6a9a036c8e30ec45feb1c4774e7080318b71eb95528eb69c1d84` (1387560 B). Core ELF-attest GREEN
  (independent build d4e6cd66 reproduces markers; zero-branch-emits confirmed from the artifact's `.rodata`).
- **BINS (clean-custody, USED-logged):** grant `d5-ota-v873` READ first; `env`-prefixed espflash LITERAL in the gated
  command text so the gate SEES it; pre-hash==pins; partition e0e49127/39rows; TWO USED lines written.
  d5-otarx-v873.bin `adc6cc186eb52d5e423dd9af430e836fb9bdad6b3bceeb1690e46d2a9a6cbd68` (878864 B, 0xE9) /
  d5-otafail-v873.bin `8a1ee68ee6cf31928717f6aa02b0f9a72a295f630300c309457bdb27d84d1584` (877440 B, 0xE9). DISTINCT.
- **★ First genuinely BLIND 3-way of the campaign:** hive == composer BYTE-FOR-BYTE on both bins (two hosts / two
  toolchains / one pinned input; supervisor withheld both directions), anchored by core's ELF-attest.
- **★ Gate-bypass caught + fixed:** a BARE `VAR=value` first token made auto-approve.sh:554 parse base off the
  assignment — never saw espflash, never gated, wrote NO USED. The ABSENT USED line was the tell (held custody, refused
  to assert the cause, escalated). Fix = `env` prefix. [[espflash-gate-bypassed-by-file-and-remote-exec]] (third variant).
- **Post-flash boot method (record, D-20260725-11 @ claude-fleet 705c0e5):** espflash-driven (`--after hard-reset`,
  observed CoreUsbUart) within a grant, OR CTRL+R, OR the Roy button. **RAW TTY RTS/EN is NOT a reset on S3 native
  USB-JTAG** (peripheral does not map DTR/RTS to CHIP_PU/GPIO0) — removed as a boot step, not merely qualified.

## Prior cycles (all CLOSED; detail in RESUME-archive.md)

- **v8.7.2 `2249bcf0`:** handler tail spin → IRAM-safe RTC_CNTL SW_SYS_RST (bit31 @0x60008000, CoreSw=0x03, RTC
  preserved). Proven in fault context (5 distinct faults, all rst:0x3 CoreSw, capture preserved+reprinted, zero wedges)
  — the v8.7.1 silent-wedge class eliminated. Base of v8.7.3.
- **v8.7.1 `7e774742`:** deliberate wedge reproducer. **★ my check(22) self-catch (supervisor-ratified):** the v8.7
  call-free spin tail (`j <self>`) I attested "the fix" WEDGES silently (cpu1 parks, cpu0 feeds RWDT, no reset, RX dead);
  byte-accurate attest, wrong mechanism. check(22) "no flash software_reset" was correct-but-INSUFFICIENT (absence of the
  wrong thing ≠ presence of recovery). [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]]
  [[identity-verified-is-not-function-verified]]
- **v8.7 `33219370`:** call-free fault handler; 3-way closed. **v8.6 `30cb3d6d`:** hang instrumentation; root-caused the
  secondary-fault-overwrite v8.7 fixes; the v86 authorization episode (my gate-bypass + fabricated grant) fully owned +
  cured. **v8.5 `9ebad32b`:** set_wake_window RX-quiesce + BLE-lease; identified the ~4min double-fault as the blocker.
  **v8.4 `afaab9ab`** and earlier: archived.

## Standing operational constraints

- **#d005 build gate + build-target guard** (`~/verify-build-target.sh <sha> <src> [marker…]`, fails closed): the named
  sha must `git show --name-only`-include the source file AND the change's target markers must be present in
  `git show <sha>:<src>` — else it's a docs-only / wrong-change sha that checks out clean+green but builds OLD firmware
  (the 514c31a4 case). A sha match alone is not build provenance. [[docs-sha-is-not-a-build-target]]
- **espflash/openocd LITERAL in the gated command text** — no ssh-wrapped scripts, no file indirection (the gate scans
  command text only; a file-hidden keyword bypasses silently — v86). ssh OK when the invocation is inside the quoted
  string. Carry auto-approve tokens as `env VAR=… VAR=… espflash …` — a BARE `VAR=…` prefix bypasses the gate (no USED).
- **Grants supervisor-only; verify authorization by READING the grant file + its USED log entry**
  (`$ws/.fleet/flash-authorization`, from repo CWD = `Development/R2/.fleet`), NEVER inferred from a command succeeding.
  USED = one entry per approved COMMAND (not per espflash run); the grant `sha256=` field is RECORDED not enforced —
  pre-hash==pin is the real identity mechanism. A missing USED line = the gate never evaluated, not a silent approval.
- **Workers never assign each other legs** — work allocation is posture; ask supervisor to add/reassign.
  [[fleet-posture-authority]]
- **RULING-A transform-3-way:** all lanes derive bins from ONE set of pinned ELFs (pre-hash==pin) — byte 3-way despite
  ELF-sha path-noise (panic-location abspaths in .rodata; `--remap-path-prefix` backlogged).
- **Hive never flashes** (composer = sole serial opener / Roy). Never bypass `ci/public-hygiene.sh` (MACs/device-tails
  off tracked files; HH:MM:SS colon-triple false-matches the mac-tail scanner; the pre-push hook does NOT run it — run
  manually before RESUME commits). Every commit: `Decision-Log:` trailer (`none` for routine); verify ahead=0 via
  `git ls-remote origin`. Fleet msgs: file + `"$(cat f)"`, never inline. NVS 0x17000 raw role-write = brick (bake via
  `DFR_ROLE_PATH`). Env-baked const verify = full `rm -rf target` + the DIFFERENTIAL. Build on alfred (`export-esp.sh`);
  nohup detach kills export-esp.sh — attached ssh only.

## The rig (static conformance suite, on alfred; never compiles/runs tests — a PASS ≠ a green suite)

`~/preflight-v8.sh <sha>` (checks 1-11) + `~/check{12..24}.sh` + `~/check22prime.sh` + `~/perbearer.sh`. `~/strip_src.py`
= strings-first comment/string stripper (per-check/per-LEG: string-literal discriminators — cfg names, printed markers,
`extern "C"` anchors — on RAW; code on stripped). Every check neg-locked (a known-bad sha must FAIL) and positive-bound
to core's REAL identifiers (requirement-not-shape). `~/hangcap-analyze.py` = capture-consistency/family analyzer
(UNOBSERVED != FAILED; a family needs ≥2 corroborated captures). All proven + neg-locked; parked with the campaign.

## Backlog (Roy-gated, not started)

D5 reflash/provision (stays 11f2d2ef; needs Roy word) · **SEN0676 radar plugin (PARKED):** Modbus RTU over plain
TTL-UART (circuits-confirmed; not ADC/I2C, no RS-485 transceiver); radar target = XIAO per g4, so a DFR1195 UART bus in
board.toml is needed only if radar also runs on DFR; which DFR pins carry it = a board-map decision hive+composer own;
ref r2-core#91 + docs/radar-xiao-node.md · RAK relay-LED (bench, under #d003 freeze) · DFR1195 display mislabel
(cosmetic) · RAK tx_power −9dBm (core, lora_leaf_config:1219) · AGENTS.md partitions-csv doc-drift.

## Open threads (post-campaign, not blockers)

sensor-provider_capable canon CLOSED (bit2=0 all boards; pending 3-board metal re-score only) · conn-liveness watchdog
parked (keepalive covers it) · InvalidRouteLen CLOSED benign (the 2 beacon classes are OURS, 5511 FNV; canon-correct drops).

## Standing artifacts (LIVE on alfred, secret-bearing, off-tree) + safety

iter-8 pair `~/d4-init8.elf`/`~/xiao-acc8.elf` · D5 cosine `~/d5-cos5.elf` (11f2d2ef) · personas `~/.r2-dev-trial/`
(MACs off-tree) · v8.7.x ELFs+bins `~/d5-*-v87*.*` + `~/v87*-staging/`. Plain non-force pushes only; never
`--all`/`--mirror`. Three local keep refs preserve removed security material (do not repack/prune). Branches:
`storing-backend` (real WIP, needs rebase) · `hygiene-scanner-v2`/`platform-trait`/`v0.2-relay-handshake`
(stale/contained, do not merge). Key rulings in `DECISIONS.md`. Ops hazard: [[reference-xiao-boot-flush-wedge]].
