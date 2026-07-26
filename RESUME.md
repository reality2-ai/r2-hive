# RESUME — r2-hive

Updated 2026-07-25. `main` clean + pushed (ahead=0). Compacted to one current snapshot; full v4→v8.7 cycle history in
`RESUME-archive.md`. Firmware lives in r2-core branch `dfr1195-fw-blerole-coex` (hive designs/builds/attests; never edits core).

---

## ‼ READ FIRST — device + authorization state (ledger wins any conflict below)

- **DEVICE (D5 / DFR1195 ESP32-S3):** runs **v8.7.3 at `513c949db0f9ec0eebbf7d6df3febec39561a13a`** — flashed and verified.
  The OTA-coex hang campaign is **CLOSED** (double-fault family verdict RULED AND RATIFIED).
- **g18 (NEW, 2026-07-26):** D4 + X1 fault-forensics **rebuild at `8530327309b82fdc0707063b72a8c00c0166a9c6`** — BUILT +
  attested, **both variants ELIGIBLE=YES**. See the g18 build record below.
- **AUTHORIZATION: NO device operations are authorized** (no flash, no serial open, no derive, no reset, no JTAG). The
  v8.7.3 grant is RETIRED; the g18 order is **build-and-attest ONLY** — the flash grant is a SEPARATE supervisor
  decision not yet made. **No pending flash, no owed 3-way.** Ignore any stale "awaiting flash / on metal" language.
- **HIVE POSTURE:** g18 build+attest legs **COMPLETE**. Record-and-report only. Re-engage ONLY on an explicit
  supervisor order; do not poll, do not assign peer legs ([[fleet-posture-authority]]).
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

## CURRENT: relay-floor OTA X1 — empty-creds A/B built+eligible; but ⚠ STOPPED on a staota-WiFi-connect finding + nobt ruling + synthetic-AP creds

**⚠ ACTIVE STOP (reported to supervisor+core): staota's WiFi STA may never associate.** wifi_task's
`controller.connect_async()`@9027 is gated behind `DATA_PLANE_JOIN.wait()`@9021; DATA_PLANE_JOIN has NO `.signal()` site
(all refs grepped: def@5623/wait@9021/comments — matches the bit5 "never fires in coex" finding). So the STA associates
ONLY IF `wifi::new(Station)`@957 auto-connects at creation (esp-radio semantics — UNCONFIRMED statically). If not,
OTA-over-WiFi can't work in nobt OR non-nobt. Core owns the answer (esp-radio auto-connect + whether staota-WiFi-OTA was
ever metal-proven — the v8.7.x campaign was OTA-over-BLE-CoC/otal2cap, NOT staota). **No A/B build until core resolves it.**
- **★ PERSONA-GATE inside the OTA health check:** `ota_health_check` items 3+4 (@4084+) require a present+non-degenerate
  persona (hive_id!=0, tg_pk!=0) AND a GroupHmac sign/verify round-trip ⇒ **image B ROLLS BACK if X1's persona doesn't
  validate**, independent of OTA transport (R2-LORA 6.5.1, landing IN the health gate). X1 enrolment UNREAD (composer).
- **nobt RULING (supervisor, variable isolation — an OTA failure then means OTA, not coex-stall ambiguity):** build A+B
  with `nobt`. STOP-check result: `staota,lora,xiao,nobt` COMPILES; radio-liveness gate passes on LORA_UP (nobt OK there);
  nobt gates ONLY the ble_task spawn @1090. BUT blocked by the staota-connect STOP above (nobt pulls ble → wifi_task takes
  the DATA_PLANE_JOIN-gated branch).
- **CORE-MAPPING (load-bearing, was invisible in the feature list):** under staota,lora,xiao ALL THREE radios on CORE0
  (wifi_task + ble_task + lora_task@1236, main spawner). Only loraroute isolates LoRa on core1. Tri-radio-on-core0.
- **SEQUENCE (once unblocked):** A+B with nobt over a SYNTHETIC AP → prove round-trip; THEN the tri-radio (ble-spawned)
  image as a SECOND over-air push, rollback as the net (a boot-hang at advertise → health gate never marks valid →
  bootloader auto-reverts) → answers the coex question the prediction is about. Re-attest each build + state each radio's core.

### superseded: earlier Option-B framing (empty-creds attest stands)

Bare R2 relay-floor OTA-capable X1 image, build+attest only. Supervisor RULED **Option B** (`staota,lora,xiao` at pinned
`85303273`) — the true bare `staota` floor does NOT compile (3× E0425, `current_beacon_epoch` lora consts — THIRD
instance of the radarprobe defect class; core's held cfg fix owed, DEFAULT must be in its regression set). OTA is
WiFi-independent of LoRa (ota_task off the RouteEngine), so LoRa is present-but-incidental; xiao = honest XIAO Wio-SX1262
pin-map.
- **Feature justification (from code):** `staota` = WiFi OTA (ota_task@1056, signed :21043 on WiFi netif); `lora` =
  **REQUIRED** (Roy: dual-bearer BLE+LoRa beacons — incidental framing WITHDRAWN); `xiao` for the SX1262 bringup @1158
  pins (XIAO physically carries the Wio-SX1262). staota,lora,xiao = the compile-forced set == the capability set Roy wants.
- **Beacon reachability (image A, verified static, NEITHER gated):** BLE §7 = `ble_task`@1099 (ble via staota, not nobt)
  → `peripheral.advertise(`@4590 (encode_advert @4518), BLE_UP@1101. LoRa §8.1 = `lora_task`@1236 (cfg not(loraroute)
  TRUE) → `radio.transmit(pl)`@6454 (build_lora_beacon@6451), print "LORA-TX §8.1 beacon"@6462, LORA_UP@1237. STATIC
  reachability only — on-air TX is metal (⚠ coex advertise-hang note @1208; tri-radio combo metal-unverified).
- **STOP check (mark-image-valid) PASSES — health-gated, not blind:** `set_current_ota_state(Valid)`@main.rs:4162 runs
  only inside `if ota_health_check()`@4161, deferred 8s (`ota_confirm_task`@4136), PendingVerify-only; FAIL → Invalid +
  rollback record + revert + reboot (`ota_health_check`@4074 = §5.2 min-2 radios-up). Rollback protection intact.
- **Images A + B** (clean detached, `rm -rf target` each, differ ONLY by baked `R2_BUILD_ID`, same size 1346876 B):
  A `relayfloor.A.0726` sha256 `0722485d8d49160a3d036b6325b5e13fdeff5a38e5ed800c1c1f030834da83f9` /
  B `relayfloor.B.0726` sha256 `a571710ad42df8355b4abb147831ee2c0069f7a74a8a6e6b0fe726c2da518387`. Differential:
  A-has-only-A / B-has-only-B (0 cross); BUILD_ID printed at boot → B running is observable = OTA round-trip proof.
- **Two-leg eligibility on THESE artifacts (not inherited):** A + B both **ELIGIBLE=YES** (HANG_CAP@0x600fe000;
  `__user_exception`@0x40378c44 size 0x52, 0 windowed calls). Pos control D5 v8.7.3=YES; neg control xiao-acc8=LEG1 FAIL.
- **⚠ CREDS — reason CORRECTED (supervisor):** built with EMPTY WiFi creds. The reason creds must stay out of the
  artifact record is **R2-SECRETS 3.1 — no real value as a LITERAL in ANY tracked file/commit/recipe/attestation/message**
  — NOT the g23 hold (USE ≠ publication; build.rs reading env is compliant; Roy has issued NO ruling on the creds, g23
  open). The FLASH build bakes `R2_WIFI_SSID`/`R2_WIFI_PASS` via build.rs env → different sha256 → **re-attest at
  flash-build** (both images, both legs, controls). The sha above attests structure+instrument+BUILD_ID+eligibility
  (creds-independent).
- **CREDS = SYNTHETIC AP, RESOLVED (g24 real-creds REVERSED).** Composer stood up an AP on alfred's spare phy2 (AP
  sustained, 0 drops/7 polls/~80s, sole-uplink wlp3s0 default-route untouched throughout — capable-caveat retired BY
  TEST). Creds are CHOSEN/synthetic (band bg, ch6, WPA2-PSK, DHCP 10.42.0.1) — no custody, no extraction (withdrawn; had
  no clean anchor anyway). **⚠ HANDLING: synthetic-by-construction ≠ safe-to-publish when the value GRANTS ACCESS — a
  chosen PSK is a WORKING PSK while the AP is up.** So the SSID/PSK are fine in fleet mail + composer's mode-700
  dev-trial file, but **MUST NOT enter any tracked/public file** (this RESUME included) — values live in the build env /
  composer's dev-trial only, deliberately NOT recorded here. build.rs reads env (R2-SECRETS 3.1). **Empty-creds
  attestation STANDS** (creds-independent). [[use-is-not-publication-secrets-boundary]]
- **★ PROVISIONING IS A PRECONDITION, not a follow-up (supervisor ruling — persona load-bearing TWICE: R2-DEVICE-LIFECYCLE
  publish + the OTA health-gate items 3+4).** Revised sequence: flash A **app-only** (NVS survives) → A boots + REPORTS
  whether its persona validates → if NOT, composer mints the dev-TG persona (its delegated class) + provisions → ONLY
  THEN push B. Else B is GUARANTEED to auto-revert and a correct rollback reads as broken OTA.
- **★ otal2cap REROUTE ON THE TABLE (supervisor→core):** Roy asked for OTA working, no transport specified. otal2cap
  (OTA-over-BLE-CoC) drops 3 radios→2 (retires the tri-radio-core0 hazard) and its receive path is more static-verified
  than staota's. BUT I confirmed **otal2cap is NOT proven end-to-end** (no slot-flip / running-image on record; the
  campaign fought the hang that pre-empted every OTA window; closed on the hang fix, not an OTA completion). So it's the
  better-UNDERSTOOD unproven path, not a proven one — either transport still needs a first real round-trip.
  **★ NO OTA HAS EVER COMPLETED ON THIS HARDWARE IN ANY TRANSPORT — tonight is a FIRST, not a re-run.** My
  better-understood read is DEVICE-SIDE ONLY; the uncosted half = **does a HOST-SIDE pusher exist on alfred tonight**
  (otal2cap needs a BLE central opening an L2CAP CoC + streaming; staota needs TCP over the now-proven AP) — composer's
  lane, asked. A device path is worthless if nothing can talk to it.
- **Open before any grant (NOT hive):** composer read of X1 persona + OTA-TG `730c29e7` membership — X1 must VERIFY the
  update signer or OTA is rejected on arrival. Board currently unplugged from both hosts.
- **NO FLASH** taken; no grant.

### Next-phase context — ENSEMBLE CANON (specs Q1/Q4/Q5 + Roy #69; DO NOT build until specs finishes + creds unblock)
- **OTA delivers the rest:** image A over USB (ONE USB write all night), B over air proves round-trip, then each
  capability arrives OVER THE AIR. Sequence: (1) core hive+OTA [done, pending re-attest], (2) memories, (3) sensor, (4)
  battery.
- **Ensemble = a SCORE, not a binary/feature** (R2-ENSEMBLE v0.3, schema R2-DEF 7; NOT installed, no binary; two part
  types = Sentants + plugins). Shape = **ONE hive binary, FIVE scores** (each ≥1 Sentant). Active ensembles AND the
  900s-vs-5s cadence = **NVS ROLE-PROFILE boot config** (R2-RUNTIME 210 — role+knobs selected AT BOOT from NVS, NOT
  compile-time; proven no_std), NOT cargo features. Encoding COMPILED (Roy #69); boundary = the score. **Cite #69 for
  encoding, R2-ENSEMBLE for boundary; NEVER §2.2B (bearer-presence only).**
- **★ C1 — memories/battery/LED are HIVE-SHARED SINGLETON PLUGINS WITH REGISTRATION, not ensemble-owned** (R2-ENSEMBLE
  2.1.2: a plugin wrapping an OS-exposed-once singleton — port/device/hw-cap — MUST be hive-shared + have a registration
  mechanism). NVS/FRAM/ATECC608 each exposed once; the ADC (battery) once; the LED once. The ensemble is the thing that
  USES + REGISTERS against the shared plugin (R2-WEB pattern: one HTTP server, N ensembles register routes). **NON-
  CONFORMANT: N ensembles each owning an NVS driver — do NOT build that.**
- **★ HARD MUST-NOT (structural, R2-KEYSTORE 184):** secret key material MUST NOT leave the protected boundary in
  plaintext ⇒ a generic read-NVS capability is non-conformant the moment it CAN address the key region. The NVS cap MUST
  be **REGION-SCOPED BY CONSTRUCTION — incapable of EXPRESSING the key range** (not well-behaved, not documented-forbidden:
  structurally incapable). Make-bad-state-unrepresentable-by-type. [[identity-verified-is-not-function-verified]]
- **Q4 capability names:** no canonical storage classes (R2-CAP 3.2 — no central registry; social agreement + docs).
  Ruling: **MINT `ai.reality2.cap.storage.*` + DOCUMENT in the SAME commit** (docs ARE the registry; undocumented =
  unregistered). `ai.reality2.cap.env.scalar` is NOT canon — a fleet string; keep using, stop calling it canonical.
- **★ LED — R2-INDICATOR v0.5 is NORMATIVE (do NOT invent a pattern); indicator is a PLUGIN byte-identical across boards,
  output stage RENDERS only, never re-decides a state ⇒ LED logic in platform main.rs is NON-CONFORMANT.** Fixed
  envelopes: Healthy = dub-dub double-pulse (lub@0.00, dub@0.18, 20 BPM, DIM — NOT 25BPM/full-bright, the rejected edge)
  · Updating/OTA = 0.18s strobe white · Boot = ~100ms flash then dark · Error = 0.25s pulse red · Identify = solid white
  · Low-batt = 1.5s pulse orange. Overlay priority (highest first): Identify > Updating > Low-batt > underlying (Identify
  outranks OTA deliberately). Healthy AND Updating MUST signal in BOTH dev+prod. **Dev-only delta = the R2-INDICATOR 6
  event-arrival BLIP** (quick tick on event arrival, may collide with HB, collision OK/no arbitration; PROD MUST NOT show
  it; gated on **R2-BUILDMODE**, not a hand-set flag). sensor-read/battery-read are NOT signature states; app extensions
  allowed but MUST NOT redefine a reserved envelope.
- **Sensor = enable-settle-read MUST be REAL code** (only the VALUE simulated while USB attached — USB cuts the 5V rail):
  enable 5V gate → WAIT settle → attempt read → substitute value when rail known-cut. NO stub skipping gate+settle
  (never-run-path class). Await circuits for gate pin/polarity/settle (do NOT invent settle). **SIM MARKER MANDATORY**:
  a simulated reading MUST be distinguishable AT THE EVENT (same path ⇒ event carries the signal); SIM MUST NEVER LEAK
  INTO LIVE. Cadence = score param. **duty_class RETRACTED (was: "likely SCF duty-class"): duty_class is a RADIO-SLEEP
  property, NOT peripheral-power** (R2-DIAGNOSTICS 58 {Unknown,AlwaysOn,Intermittent}, wire R2-WIRE 12.6 `dc`; its ONLY
  job = telling peers whether to BUFFER-on-fade while your RADIO sleeps — R2-RUNTIME 236). Toggling the 5V sensor rail
  does NOT make the radio fade (board stays reachable) ⇒ the 5V cycle is INVISIBLE to duty_class, MUST NOT drive it.
  **USB-powered X1 = AlwaysOn.** Power source OVERRIDES role; class set STATICALLY from provisioning (a runtime flip would
  leave a battery node AlwaysOn+flooding) — if X1 ever runs on battery it MUST be provisioned DutyCycled BEFORE, never
  flipped at runtime. Settle: canon SILENT (a driver detail), so 1500ms is a config knob not a spec obligation.
- **X1 enrolment UNKNOWN:** send-over-TN needs X1 = a TG member with a working persona; persona + OTA-TG `730c29e7`
  membership UNREAD (composer's lane). Do NOT design around X1 enrolled. Button-free entry PROVEN on D5, UNPROVEN on X1
  — gates the first USB write (composer, amended grant).

### Electronics (circuits, AUTHORITATIVE for firmware — for the sensor/memory/battery/LED ensembles, not yet built)
- **5V gate = D0/GPIO1, ACTIVE-HIGH** (TPS61023 EN; low = true 0.1µA disconnect). **Settle = 1500ms proven-good** (radar
  replied on battery at that value); min UNCHARACTERISED — do NOT go below a few hundred ms without a test. Use 1500 as
  a CONFIG KNOB, not a literal.
- **Radar UART = TX GPIO43 / RX GPIO44, 115200 8-N-1**, Modbus RTU slave 0x01; water level = holding reg 0x0003 in mm;
  request `01 03 00 03 00 01 74 0A`, reply `01 03 02 mm_hi mm_lo crc_lo crc_hi`. **XC4486 = PASSIVE bidirectional shifter
  — NO DE/RE, NO enable, auto-direction; FIRMWARE DOES NOTHING FOR IT — strip any direction handling** (the withdrawn
  radarprobe's RADAR_DE_RE=GPIO6 was WRONG for this shifter).
- **FRAM 0x50 = MB85RC, 16-bit addressing, BYTE-WRITABLE**, no page boundaries, no write delay, endurance ~unlimited at
  5-10s; WP+A0-A2 strapped GND (write-enabled, no WP handling). **SIZE UNKNOWN (32KB or 512KB) — PROBE it, do not assume.**
  **⚠ PAGING HAZARD (circuits): a 512KB MB85RC4M addresses BEYOND 16 bits — the high bits page into the I2C DEVICE-ADDRESS
  byte, NOT the 2 address bytes. The wrap test still distinguishes 32KB from larger, but full addressing of the large part
  REQUIRES those page bits — without them WRITES ALIAS (silent corruption past 64KB). So DETECT size AND implement paging;
  aliasing-writes-that-appear-to-succeed is the worst failure shape for a store-and-forward buffer.**
- **Storage-queue canon (for the FRAM plugin):** R2-ROUTE 3B.3 **OUTQ-1..4** @769a255 (read the clauses, not a summary) —
  a SEPARATE section from peer-custody: OWN-ORIGIN custody ends when YOUR OWN transmission succeeds; PEER-custody ends when
  the PEER reappears — conflating them mis-sizes both buffers.
- **claim_state persistence (R2-DEVICE-LIFECYCLE 3 invariant 2):** requires 3 hw roots — irreversible VIRGIN sentinel +
  dedicated monotonic hw_epoch counter + per-device HUK. Lacking ANY ⇒ FAIL CLOSED at build/provision; canon permits BENCH
  BUILDS AS EXPLICITLY NON-PERSISTENT ONLY. If the S3 lacks any of the three, **DECLARE the bench build NON-PERSISTENT
  deliberately IN THE BUILD** (don't let it emerge as a surprise).
- **ATECC608 0x60 = LOCK STATE UNKNOWN — QUERY the lock before assuming anything; do NOT provision blind.**
- **Battery = D1/GPIO2, 2×470k divider (read ×2), 100nF at pin.** Dry joint NOT confirmed reflowed; fed from LiPo+ which
  USB CUTS → reads ONLY on battery. Code it, do NOT expect a green tonight, say so in the attestation.
- **LED = GPIO21, ACTIVE-LOW**, free (not shared with LoRa B2B pins). **0x7C = CONFIRMED PHANTOM** (only 0x50 + 0x60 are
  real) — bind nothing to it, don't let an enumerator claim it.
- **Testable on USB tonight:** FRAM, ATECC608, SX1262 SPI, LED, I2C scan, gate GPIO logic. **NOT testable:** real radar
  comms, battery, actual 5V rail rise.

## radarprobe build order (2026-07-26) — WITHDRAWN by supervisor (wrong artifact); defect recorded, no hive build

**ORDER WITHDRAWN.** Roy's actual goal = a per-component bring-up smoke test, whose instrument is a **throwaway Arduino
sketch circuits already has** (I2C scan / FRAM + secure-element probe / battery ADC / gated Modbus), NOT an R2 no_std
feature — so this is NOT hive's artifact. Supervisor owned the mis-inference chain (radar test → radarprobe feature →
DFR1195 crate-path=board), all unverified. Hive builds nothing here. The finding below STANDS (recorded by supervisor;
core HOLDS a ready-but-UNAPPLIED fix — supervisor told it to HOLD, must NOT move the tip since g18 artifacts are pinned
to 85303273). My build attempt surfaced it. Nothing in flight on radarprobe. (To know if core landed anything, read the
TIP — the emitter — not a summary or intent.)

Explicit #d005 build order (supervisor, Roy GO). Feature `radarprobe = ["dev"]` (Cargo.toml:290). Pinned sha = MY
ls-remote authority `85303273` (== branch tip; supervisor's `4e82c36a` is on origin/main + r2-core-consolidation, NOT
this branch, NOT an ancestor — I pinned my own ls-remote as instructed). **Reachability verified in code:** radarprobe
block @main.rs:901 spawns `radar_probe_task`@913 then a diverging `loop{}` → radio/mesh spawns (net/wifi/io/ota/ble/lora,
all >line 919) unreachable (`allow(unreachable_code)`@603). Pre-probe spawns (rwdt_feed@623, hang_reprint@840) still run.
- **BUILD FAILS — 5× E0425 cannot-find-in-scope:** `role_class_hash` (def `#[cfg(any(lora,ble))]`) used in ungated
  `io_task`@1729; `DATA_TX` (`#[cfg(ble)]`) @1760; `COARSE_TIME_ANCHOR_S` + `LORA_BEACON_T_ROTATE_S` (`#[cfg(lora)]`) in
  ungated `current_beacon_epoch`@6264. radarprobe=[dev] pulls neither ble nor lora, so the unguarded refs don't resolve;
  every shipping set (fakesensor/xiaobridge/bridge) pulls lora and/or ble, so radarprobe has never compiled standalone.
- **No ELF → attest moot** (two-leg instrument un-checkable). Not fixing (core owns source; build-only order). No flash,
  no target prepared. Owed: core gates those refs under cfg (or radarprobe stubs them); then re-issue on the fixed sha.
- **TARGET = X1 (XIAO ESP32-S3 + Wio-SX1262)** — Roy corrected supervisor's DFR1195-only inference; the radarprobe
  RS-485 pins are XIAO-authored (RADAR_UART_TX=43=XIAO D6). **radarprobe needs NO `xiao` feature** (verified code +
  build): `xiao=[]` pulls nothing; its pin-map cfg sites (main.rs:1178-1241) are radio-region, unreached under the
  radarprobe diverging loop; the radar block uses hardcoded GPIO43/44/6. `radarprobe,xiao` FAILS the identical 5 E0425
  (xiao adds no ble/lora). SX1262/DIO2-RF-switch warning does NOT bind (radio init lora-gated + unreached). **On the
  fixed sha, build `--features radarprobe` ALONE.** ⚠ an X1 radarprobe flash REPLACES the bridge role + the g18
  x1-xiaobridge image (`cee2f004`) X1 currently carries — stated for Roy.

## g18 build record — D4 + X1 fault forensics rebuild `8530327309b8…a9c6` (BUILT + attested; NO FLASH)

Explicit #d005 order (supervisor, Roy-ruled, 2026-07-26). A REBUILD, not a port: xiaobridge and fakesensor are FEATURE
VARIANTS of the one platforms/dfr1195, not separate platforms. The ~40 staged XIAO/sensor siblings were STALE (built
before the instrument landed); a rebuild carries the full HANG_CAP/`__user_exception`/reprint instrument (unconditional
in main.rs, `esp-hal default-features=false` platform-wide so the strong handler symbol wins). ledger: g18 (Roy).

- **Provenance:** ls-remote authority == pinned tip; `verify-build-target 85303273 main.rs HANG_CAP __user_exception
  hang_reprint_task` → touches main.rs + all present → OK. (85303273 is comment-only touching main.rs — the RTS-EN
  premise fix — but NOT a docs-sha trap: the instrument IS in the tree, unlike 514c31a4 where the ordered markers were
  absent.) Preflight: HEAD==pinned, tree clean, worktree-instrument present, `rm -rf target` per build.
- **Features** (Cargo.toml:273 canonical pairing, both carry benchsf7 — mixed-SF can't demodulate; baked_persona OFF):
  **D4 = `fakesensor,benchsf7`** · **X1 = `xiaobridge,benchsf7`**. BUILD_ID `g18.0726`.
- **ELFs:** d4-fakesensor-g18 `30ffcdfdcd7a6ac11bc00b4ced0564af28ef6381f7ad85a9516062d50224870f` (1383864 B) /
  x1-xiaobridge-g18 `cee2f004fd6f51f9ef839461ecd3b456b8ecee0b96640930a54b218e7158d983` (1153904 B).
- **★ TWO-LEG ELIGIBILITY — both ELIGIBLE=YES** (`alfred:~/eligibility-g18.sh`, in order, on the EXACT artifact via
  toolchain nm/objdump): **LEG1** HANG_CAP static present (both @0x600fe000); **LEG2** `__user_exception` call-free
  within its true `nm -S` extent (D4 @0x40378c48 size 0x52, X1 @0x40378bd0 size 0x52 — 0 windowed calls, 0
  software_reset each). Positive control: D5 v8.7.3 → ELIGIBLE=YES. Neg-control: stale siblings xiao-acc8 + d4-init8 →
  LEG1 FAIL (no instrument) — the check discriminates real presence from real absence.
- **★ INSTRUMENT-BUG SELF-CATCH:** my first eligibility run false-FAILED LEG1 (`<none>`) — the Rust static is MANGLED
  (`_R…8HANG_CAP`) so `grep -w` missed it + host-nm fallback. I did NOT report absence; positive-controlled against the
  known-good D5 ELF (it ALSO showed `<none>` under the broken check ⇒ instrument bug, not artifact), fixed to toolchain
  nm + case-sensitive `HANG_CAP`. A false-absent is the worst misreport (presence-before-quality).
  [[positive-control-the-tree-not-just-the-tool]] [[never-conclude-from-a-null]]
- **Does NOT carry g15:** the dataplane join-carriage (core `259ea8e5`) is on branch r2-core-consolidation; 85303273
  does not contain it. Intentional (g18 = fault forensics, not the join relay). Per core **D-20260726-08** the firmware's
  vendored `r2-dataplane` is a **pre-g15 PIN = structurally cannot-carry = SAFE**; g15 arrives at the next re-vendor. Do
  not reason "rebuilt after g15 ⇒ siblings have it" — they will not.
- **NO FLASH taken/authorized** — build+attest only; the flash grant is a separate supervisor decision.

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
