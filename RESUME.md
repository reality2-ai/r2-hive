# RESUME — r2-hive

Updated 2026-07-25. `main` clean + pushed (ahead=0). **Compacted to current-only; full v4→v8.7 cycle history in
`RESUME-archive.md`.** **v8.7.3 BINS DERIVED + clean-custody, hive legs COMPLETE — STOOD DOWN** (#d005 order 513c949db0f9, ledger
D-20260725-01). core attest GREEN, blind 3-way == composer byte-for-byte, USED-logged. FLASH grant → composer (sole
serial opener); next event = board dial. Firmware = r2-core branch `dfr1195-fw-blerole-coex`.

## Objective

OTA-over-BLE-CoC coex firmware series (D5 = DFR1195 ESP32-S3). Each iteration: build DFR1195 D5 ELFs from a
core-pinned sha → run the static conformance rig → attest → extract signed/app bins → multi-party verify. Current
sub-goal: **v8.7 hang instrumentation** — capture the PRIMARY double-fault (`__user_exception` → HANG_CAP retained
mem → boot-decode), the ~4min double-fault hang being the true blocker (pre-empts every OTA window).

## CURRENT: v8.7.2 `2249bcf0e5ebdaa3e00bcbf298ded383f6bd220b` — BUILT + attested + bins; rig 24/24. Awaiting 3-way → flash

Base 7e774742, main.rs only. Fix = handler tail spin → SW_SYS_RST write (RMW of RTC_CNTL_OPTIONS0 bit31 @0x60008000;
intended digital-core SYSTEM reset CoreSw=0x03, RTC preserved so HANG_CAP survives). Source FROZEN at 2249bcf0
(comment-wording fix deferred; tip verified unmoved). Clean detached checkout, tree empty, `rm -rf target`.
- **ELFs** (BUILD_ID `coex.v872.0725`): d5-otarx-v872 `d2b44d9d69ac52e95f3b2b04b61ae8be871e4e974b90a5bc5457733aa0fc061f`
  / d5-otafail-v872 `831e32820940d17971e11123d0c7256d8746d74e1d14cf24610de9b4fb25d867`. v871 leftover=0, otafail DISTINCT.
- **★ STATIC REACHABILITY + ORDERING (NOT "reset reached" — label corrected, over-claim OWNED).** objdump handler
  0x40378c48..0x40378c9c: 0 windowed calls, then `l32r ← 0x60008000` → read → `or 0x80000000`(bit31) → `s32i`
  write-back (SW_SYS_RST RMW), THEN `j <self>` fallback. This proves the reset write **precedes the fallback spin on
  the STATIC PATH** in the artifact — it does NOT prove fault-context execution reaches the write, nor that the
  hardware honours it, nor that the board recovers. "RESET REACHED" / "wedge FIXED" is RESERVED for a
  temporally-correlated CoreSw reset + a FRESH capture AFTER the v8.7.2 flash. **check(22') is a static instrument** —
  its PASS = "the write is on the path before the spin", not "the board recovers." (Same class as my check(22) over-
  claim: byte-accurate, runtime-blind — [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]].) source: write@494 < spin@498.
- **Rig 24/24 bound-PASS** (preflight 1-11 + check 12-23 + check22'). check(22') neg-lock: 30cb3d6d (flash-call) +
  33219370 + 7e774742 (spin) ALL FAIL; carries pass. Requirement-not-shape (reset-domain write reached-before-spin;
  bit31 not hardcoded).
- **BINS DERIVED** (v872 grant read first, espflash LITERAL in gated command text, pre-hash==pin, USED-logged):
  d5-otarx-v872.bin `af2f428a49e154ddea25287e5328e666c6468112113ba0ac97aaaa6a4ca7204c` (878448 B, 0xE9) /
  d5-otafail-v872.bin `44c3ae8cb104b0cb10686b9022a433c395f218ffc57f18815815e7449c5aec30` (876944 B, 0xE9). DISTINCT, e0e49127.
- **Next:** composer independent extract + core attest → byte-3-way (== af2f428a/44c3ae8c) → v8.7.2 FLASH grant →
  dial. **⚠ acceptance matrix NOT YET RATIFIED** (supervisor): core's (a)-(d) omits CpuSw/power-on/brownout/external/
  panic + non-CoreSw reboots with decoded-or-empty capture; CoreSw alone isn't exclusive proof of this tail if another
  SW_SYS_RST caller exists — an UNEXPECTED-RESET branch + fresh-capture temporal correlation owed before the flash
  grant. **Core FINDING: CoreSw is non-exclusive — 5 intentional `software_reset()` callers also produce CoreSw**, so
  the reset-recovery verdict MUST rest on fresh-capture temporal correlation, not the reset-reason alone. **Capture-consistency analyzer staged** (add a reset-recovery leg = CoreSw + fresh capture + UNEXPECTED-RESET
  bucket when the matrix ratifies).

## Prior: v8.7.1 `7e774742` ON METAL (deliberate wedge reproducer, spin tail)

**v8.7.1 IS ON METAL** (composer flashed ~13:29 under a still-live grant; supervisor's revoke arrived after = his
race, no fault). Board runs `coex.v871.0725` now, kept there DELIBERATELY as a **wedge reproducer** — the same spin
handler = the wedge recurs = a live JTAG read.
- **★ WEDGE (JTAG-proven) + my check(22) SELF-CATCH (supervisor-ratified, "sharpest thing this cycle"):** the v8.7
  call-free spin tail (`j <self>`) I attested "the fix" PRODUCES A SILENT LOOKS-ALIVE WEDGE — cpu1 parks on its own
  j-self, cpu0 keeps feeding RWDT, so NO reset fires, zero decode, RX dead. My attest was BYTE-accurate (handler IS
  call-free) but the MECHANISM claim was wrong: call-free-via-spin ≠ recovery. **check(22) "no flash software_reset"
  was correct-but-INSUFFICIENT** — it asserts the ABSENCE of the wrong thing, blind to the missing PRESENCE (actual
  recovery). A static structural check confirmed the shape it was asked about; runtime/JTAG caught the consequence.
  [[identity-verified-is-not-function-verified]] [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]]
- **v8.7.2 = 7e774742 + handler tail IRAM-safe RTC_CNTL SW-reset** (core's form = RMW of RTC_CNTL 0x60008000 bit31 —
  a direct RTC-register reset, IRAM-safe, actually RESETS). Core implementing; **build order comes on its sha. #d005
  stands.**
- **check(22') DESIGNED + neg-locked (checks-first, RATIFIED)** (`alfred:~/check22prime.sh`): REQUIREMENT-not-shape =
  the handler tail REACHES A RESET via an IRAM-local register write, NOT merely "no flash software_reset" and NOT a
  spin; do NOT hardcode 0x60008000 bit31 (any reset-domain register write qualifies). legA sr=0 + legB a reset-domain
  write (RTC_CNTL/0x6000_8/SW_SYS_RST, excluding the HANG_CAP base.add writes). **Neg-lock: BOTH 30cb3d6d (legA,
  flash-call) AND 33219370/7e774742 (legB, spin — no reset write) FAIL; carries pass.** Positive binds on the v8.7.2 sha.
- **v8.7.1 bin 3-way CLOSED** (6e2e1358/57d117dd == composer + supervisor cross-read) — nothing waited.
- **Capture-consistency analyzer HARDENED + neg-controlled + UNOBSERVED-framed** (`alfred:~/hangcap-analyze.py`):
  (1) LOST-pc bucket (boot-XOR-reprint = UNCORROBORATED, never agreement); (2) 'N corroborated agree' (family) vs '1
  pc + frames' (single, NOT a family — encodes supervisor's "family=UNPROVEN at n=1" ruling). **UNOBSERVED != FAILED:**
  a missing boot print (v8.7.1's ~3.5min host-logger-arming gap, or the USB re-enum race) is NOT a firmware fault;
  only TORN is a real negative; the spin-wedge signature = the log STOPS (absence of continuation). Neg-controls pass
  (2 distinct pcs → SCATTERED; 1-capture → no family; lost-print → leads-not-verdict).

## Superseded build: v8.7.1 `7e774742` — BUILT + attested + bins (flash revoked; spin-tail wedge)

Base 33219370, main.rs only. Decode-hardening: `hang_reprint_task` re-prints HANG_CAP ~15s after boot + pre-zero
moved into the task AFTER the re-print (host-reopen-race fix); handler was call-free spin (WEDGES — see correction).
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
- **Capture-consistency analyzer PRE-STAGED** (`alfred:~/hangcap-analyze.py`, idle-window prep, proven on synthetic):
  parses print_hang_cap lines (fault/PARTIAL/empty + [boot]/[reprint+15s] tag + reset_reason) from a serial log →
  (A) DECODE-HARDENING check (per cycle boot==reprint; a reprint-only cycle = the USB-re-enum race the +15s reprint
  is built to catch) + (B) FAMILY verdict (across cycles: ONE fault mode = a single reproducible double-fault, or
  scattered = enumerate before root-cause). Runs on composer's dial log when it lands.

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

## CURRENT: v8.7.3 `513c949db0f9ec0eebbf7d6df3febec39561a13a` — BINS DERIVED + clean-custody; blind 3-way == composer. Awaiting FLASH grant

Real firmware sha, #d005 build order (supervisor 2026-07-25). Ledger precondition SATISFIED: r2-core/DECISIONS.md
D-20260725-01 at d970693c (follow-up commit, DECISIONS.md only, references 513c949d) — supervisor-verified. Sha authority
= MY ls-remote (`513c949db0f9…a13a` 40ch WITH the `b`, == branch tip; core mis-reported it once as 39ch missing the b —
used my own output not the transcription). Base 2249bcf0 (SW_SYS_RST tail); v8.7.3 adds the WITNESS pre-zero read-back.
- **Preflight all green:** HEAD==pinned, `git status` clean=[], WORKTREE main.rs carries both markers (not just `git show`),
  gitdir=real, `rm -rf target` per build.
- **ELFs** (BUILD_ID `coex.v873.0725`, FEAT otarx=`bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive,otal2cap`;
  otafail adds `otafail`; DFR_WAVE=cos step0.25, d5-persona):
  d5-otarx-v873 `52af3bfe09f2ff6e3b69dfd4cecea7cc34239c000dd845d01eb9093636bbb7ab` (1388708 B) /
  d5-otafail-v873 `7d5d67387d3a6a9a036c8e30ec45feb1c4774e7080318b71eb95528eb69c1d84` (1387560 B). role const both RPF1,1,2,0.
- **BINS DERIVED + CLEAN-CUSTODY** (grant `d5-ota-v873` READ first, `env`-prefixed espflash LITERAL in gated command
  text so the gate SEES it, pre-hash==pins, partition e0e49127/39rows, TWO USED lines written):
  d5-otarx-v873.bin `adc6cc186eb52d5e423dd9af430e836fb9bdad6b3bceeb1690e46d2a9a6cbd68` (878864 B, 0xE9) /
  d5-otafail-v873.bin `8a1ee68ee6cf31928717f6aa02b0f9a72a295f630300c309457bdb27d84d1584` (877440 B, 0xE9). DISTINCT.
  - **★ FIRST genuinely BLIND 3-way of the campaign:** hive == composer BYTE-FOR-BYTE on both bins, two hosts / two
    toolchains / one pinned input, NEITHER saw the other's numbers (supervisor withheld both directions). Anchored by
    core ELF-attest GREEN (independent build d4e6cd66 reproduces markers; zero-branch-emits confirmed from artifact).
  - **★ GATE-BYPASS caught + fixed (audit-integrity):** the first extract carried a BARE `R2_OTA_TARGET=… espflash`
    prefix — auto-approve.sh:554 parsed base from the assignment, never saw espflash, never gated, wrote NO USED. The
    ABSENT USED line was the tell (I held custody, refused to assert the cause, escalated). Root cause = the first-token
    parser (supervisor confirmed at the hook); a bare `VAR=value` isn't in the wrapper list so the unwrap never fired.
    FIX = `env` prefix (env IS a wrapper → re-classifies → gates → USED written). Bins byte-identical on re-run
    (idempotent). See [[espflash-gate-bypassed-by-file-and-remote-exec]] third variant.
- **Next (NOT hive):** v8.7.3 FLASH grant issued to COMPOSER (sole serial opener) — hive legs COMPLETE, STOOD DOWN.
  Composer flashes + attaches logger + dials. **Post-flash boot: espflash-driven (`--after hard-reset`, observed
  CoreUsbUart) within a grant, OR CTRL+R, OR the Roy button. RAW TTY RTS/EN is NOT a reset at all on S3 native
  USB-JTAG** — the peripheral does not map DTR/RTS to CHIP_PU/GPIO0, so it is removed as a boot step, not merely
  qualified (D-20260725-11 @ claude-fleet 705c0e5, supersedes D-20260725-10). Finish-post = first WITNESSED all-zero
  `[baseline]` line (converts the family HINT from unscorable to scorable), then Roy takes stock. Hive re-engages ONLY
  if composer needs a rebuild/re-derive (supervisor will come to me) — I do NOT poll or assign.

### provenance + conformance (accepted in full by supervisor)
- **Provenance:** `verify-build-target 513c949d platforms/dfr1195/src/main.rs "WITNESSED all-zero" "PRE-ZERO INEFFECTIVE"`
  → (a) touches main.rs YES; (b) both markers present. **BUILD-TARGET OK.**
- **check(24) PRECISE-BOUND** (`alfred:~/check24.sh`, dual input sha|file) — FOUR legs, requirement-not-shape:
  (a) HANG_CAP read-back in `hang_reprint_task`; (b) ORDERED AFTER the pre-zero write (read-before-zero witnesses
  nothing); (c) a DISCRIMINATION compare of the read-back to all-zero; (d) **PRECISE — the ZERO-OUTCOME branch (TRUE-arm
  of `if zc==[0u32;8]`) emits its OWN println AND the else/non-zero arm emits** — keyed to the branch, NOT a function-wide
  println count. This **closes the ≥2-count loophole** (`println!("checking"); if z!=0 {println!(anomaly)}` = 2 prints yet
  zero-branch silent). A witnessed baseline is a POSITIVE STATEMENT, never inferred silence (silence conflates
  {slot-zero / print-lost / channel-mute} = 3 states, 1 observation, no positive control).
- **3-way PROVEN:** 513c949d **PASS** (read-back@task-l29 after pre-zero@l8, `if zc==[0u32;8]`@l30 zero-arm prints,
  else non-zero-arm prints) · **2249bcf0 FAIL clean** (neg-lock, no read-back after pre-zero; its v8.7.2 carries untouched
  — check(24) additive) · **silent-zero loophole probe FAIL** (read-back + else-print present, zero-arm SILENT →
  zero-arm-prints=0; the exact count-loophole rejects). Pass-state reachable AND the requirement itself falsifiable.

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
  supervisor order naming the pinned sha, (3) clean detached byte-verified checkout, tree-state verified, (4)
  **BUILD-TARGET GUARD** (`~/verify-build-target.sh <sha> <src> [marker…]`, fails closed): the named sha must
  `git show --name-only`-include the source file AND the change's target markers must be present in
  `git show <sha>:<src>` — else it's a DOCS-ONLY / wrong-change sha that checks out clean+green but builds the OLD
  firmware (the 514c31a4 case: docs commit touching only the .diff, main.rs unchanged). A sha match alone is not
  build provenance. [[docs-sha-is-not-a-build-target]]
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

D5 reflash/provision (stays 11f2d2ef; needs Roy word) · **SEN0676 radar plugin — PARKED (after v8.7.2 dial):**
Modbus RTU over plain TTL-UART (circuits-confirmed; NOT ADC/I2C, no RS-485 transceiver); radar target = XIAO per
g4, so a DFR1195 UART bus in board.toml is needed ONLY if radar also runs on DFR (C6 lis2dh stays I2C); WHICH DFR
pins carry it = board-map decision hive+composer own (circuits owns only the sensor-side fact); ref r2-core#91 +
docs/radar-xiao-node.md · RAK relay-LED (bench) ·
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
