# RESUME — r2-hive

Updated 2026-07-23. `main` clean + pushed. **✅✅ iter-9 conformance COMPLETE — pair (`#d025`) + D5 3-board
bar both PASS on metal. Overnight matrix-greening (`#d026`) STANDBY-READY.**

## Current state

**✅✅ iter-9 CONFORMANCE COMPLETE — 3-BOARD BAR PASS (composer co-boot 2026-07-23).** Pair `#d025` + D5
conformant reflash both green on metal. No build pending.

**3-board PASS (all 4 falsifiers clear):** 3 iter-9 conformant boards (D4 724383ea + XIAO 5fb1565f + D5-cos
a0157eb2, all bit2=0), D4 monitor-reset co-boot.
- **FA2 PASS (THE key tiebreak — my sticky-capture finding's live test, iter-9 couldn't run it):** D4 resolves
  BOTH acceptors (8c15b0c2 + da73508e) then capture-dials XIAO `8c15b0c2` = LOWEST of 2 resolvable, NOT D5.
  Two-live-acceptor lowest-hive tiebreak WORKS ⇒ NO capture bug ⇒ **iter-10 sticky-capture candidate confirmed
  NOT load-bearing** (the earlier sticky was a first-seen-single-acceptor artifact only).
- **FA1 PASS:** all elect None (D4/D5/XIAO provider=None) — zero bit2=1 leak. Conformance holds 3-board.
- D4↔XIAO sustain: D4 0x25 ×4 + keepalive ×10; XIAO 0x25 ×11 + keepalive ×31 (~2.5s bidir); accept completes.
- **FA4 PASS:** D5 unpaired — 0x25=0 (bit0 DARK, EXPECTED) + still resolvable/advertising + cosine emitting
  (APIARY ×20) + accept=0. D5 did NOT disrupt the pair.
- **FA3 note:** 1 transient XIAO 'Disrupted' in ~120s → session RE-ESTABLISHED (31 keepalives after) =
  reconnect blip, self-recovered (validates iter-8 keepalive disconnect→break→re-dial→re-establish), NOT a
  sustained wedge. Relevant to the parked re-dial/conn-watchdog thread only if it RECURS.

**Overnight posture (`#d026`, Roy: green the remaining matrix overnight):** STANDBY-READY. Discipline: NO
build until an explicit order names a sha; #d005/#d006 preflight (drain → pinned-sha detached byte-clean →
`rm -rf target` → attest) on each.

- **OTA D5 P1+P3 DELIVERED + attested (from PINNED `b79b4f7a`, hive-owned dir, coex.iter9.0723) — awaiting
  two-party verify + P1-good-first flash.** 2 clean builds, NO dev-unsigned-ota (0 hits), both role b[4]=1
  Sensor/b[6]=0, persona da73508e preserved.
  - **[1] d5-otarx-p1** `54dddb16df9f4bbf5f63fe6273975a05df2440a9e5287265831baf8895a66eba` (`~/d5-otarx-p1.elf`,
    receiver flash-base + P1 payload): persona baked==input e6108006 @48264 masked `70e3ef93`; otal2cap swap
    (ota_receive_over_coc×2 + PSM 0x00D3×2); signature-REQUIRED (verify_strict + ets_secure_boot); CONFIRM
    path present (`health PASS` + `OTA CONFIRMED` + `image Valid; anti-rollback floor committed` = core's
    P1-watch strings).
  - **[2] d5-otafail-p3** `2a4f3308c7d606bd88b36ca09a3a7ddce0f55427215dfef8975841d8f9c71198`
    (`~/d5-otafail-p3.elf`, P3 radio-dead): persona baked==input e6108006 @48168 masked `00016efb`; same swap +
    signature-required; **otafail TOOK** — P1≠P3 differential + confirm/success path DCE'd (NO health-PASS/
    OTA-CONFIRMED/image-Valid) = health min-2 provably-unmet ⇒ no confirm ⇒ bootloader auto-rollback; cfg gates
    source-verified :775 BLE_UP / :912 LORA_UP (cfg(not otafail)).
  - Composer signs both real-TG seq cur+1 (P1 = same bytes re-signed). **P1-GOOD FIRST** on a fresh D5 (boot →
    8s → health PASS → OTA CONFIRMED; no-confirm/reset-loop ⇒ STOP, do NOT proceed to P3) = composer/Roy flash.
    ef7b2d24 (418c7934) DISCARDED. [[ota-per-platform-sink]]
  - **P1 flash result (composer 2026-07-23): boots CLEAN + HEALTHY + radios-up (flash-base role OK), NO confirm
    strings — NOT a P1 defect; cycle NOT blocked (composer stopped one step early).** Source-definitive
    (b79b4f7a `ota_confirm_or_rollback_on_boot` :3657-3684): the confirm AND the anti-rollback **floor-commit**
    both live in the New|PendingVerify arm — `set_current_ota_state(Valid)` → `read_ota_pending()` →
    `write_anti_rollback(max(seq),max(floor))` → `OTA CONFIRMED … floor committed seq=N floor=F` (:3660); the
    `_` normal-boot arm (:3680) clears stale pending, commits NO floor. espflash-direct = Valid/Undefined boot =
    `_` arm ⇒ no floor by design (`read_ota_pending()=None`). Answer = **(b): floor + P3's revert target are
    established by the P1 OTA-PUSH (seq=1), the step not yet run** — espflash-base only bootstraps a running
    receiver to push TO. Sequence: base→ota_0 (done) → OTA-push P1 seq=1 (inactive slot, PendingVerify, ~8s
    deferred confirm → floor=1) → OTA-push P3 seq=2 (health-fail → activate_next → revert to the P1-confirmed
    slot). (a) self-confirm-on-healthy-boot would be WRONG (no staged seq to commit). PROCEED: ota-push P1
    --dry-run then Roy-gated metal push. PASS-BAR already revised (base-flash no-confirm = EXPECTED). Composer's
    positive-control localized it right (no false "confirm FAILED"). Core + supervisor concur.
  - **★ Signed OTA payload = the app .bin, NOT the ELF** (composer pre-push Q). ota-push --image checks esp_image
    (magic 0xE9@0, chip_id@off12); the ELF (0x7F magic) is not it. Extract via `espflash save-image --chip
    esp32s3 <elf> <bin>` / esptool elf2image (deterministic → two-party reproducible). **The signed+pinned sha =
    the .bin sha, ≠ ELF `54dddb16`/`2a4f3308`** — composer extracts (my alfred lacks esptool + espflash is
    keyword-gated), then two-party cross-check the .bin sha256 BEFORE signing (attest the delivered bytes).
    persona da73508e baked in the .bin too. Seq: fresh D5 floor=0 ⇒ P1 seq=1 (base never wrote a floor — `_`
    arm). TG_SK: ephemeral unseal + immediate shred, off-tree (composer/Roy custody).
  - **.bin EXTRACTED + attested (grant-shape auto-approved):** the espflash keyword-gate was cleared by the
    sanctioned per-op grant shape (`R2_OTA_TARGET=<target> espflash save-image …`, artifact `d5-ota` + target
    named ⇒ gate auto-approves; esptool fallback not needed). These are the SIGNED-PAYLOAD bytes
    (header.payload_hash == SHA256(.bin); ELF sha ≠ delivered bytes).
    - **P1 d5-otarx-p1.bin `bd22d272d6c7fd1179a03b18e97de84c5a6fe8ace13fd259b1793f70c41e8cee`** (897504 B, from
      ELF 54dddb16; esp_image 0xE9; persona da73508e @44168).
    - **P3 d5-otafail-p3.bin `ce76ea9e3c08c8bc828ae81c8a5473f5c38bae8d6b67b24db031e4cf6e133c39`** (895968 B,
      from ELF 2a4f3308; esp_image 0xE9; persona da73508e @44072). P1≠P3 (otafail diff preserved).
    - save-image: chip esp32s3, merge=false (app image only). **TWO-PARTY .bin MATCH CONFIRMED** — composer's
      independent extraction == my hive shas (bd22d272 P1 / ce76ea9e P3); signed-payload bytes cross-validated.
      Core = 3rd derivation on request (ELF paths handed: `/home/roycdavies/d5-otarx-p1.elf` 54dddb16 /
      `d5-otafail-p3.elf` 2a4f3308, b79b4f7a-built — the 418c7934 P1 was ef7b2d24, discarded). **Never route the
      gate for actual flash/sign — grant-gated** (supervisor). MAC in the target path off-tree.
    - **Signer mechanism (composer, HELD on supervisor):** sealed TG 730c29e7 has no raw tg.txt; composer
      recommends `tg OtaSign` in-memory unseal + a new `ota-push --signed-stream` branch (NO plaintext key on
      disk) — I ENDORSED it over my earlier tmpfs-export (stronger custody). Ratified → step = `ota-push
      --signed-stream --dry-run` first (target_class=0, target_tg all-zero). No hive dependency (same .bin
      payload regardless of signer transport).
  - **P1 dry-run STAGED byte-exact `bd22d272` (composer, path A in-memory unseal):** --signed-stream drove
    OST/ODT/OCM e2e, receiver wrote 897504/897504B = my .bin extraction validated end-to-end. Metal push HELD
    (tuxedo DOWN + operator-gated). **P2b (ClassMismatch r7) BLOCKED** on ota-sign hardcoding target_class=0
    (wildcard ⇒ can't force a mismatch; needs a --target-class override to emit e.g. bridge B52C9F26) — core's
    ota-sign + supervisor test-design, NO hive action (my §2.6 class gate is correct; the payload can't be
    built). Offered a source-confirm of the accept/reject arm if useful.
  - **P1 metal push BLOCKED — RECONCILED to core's HALF-OPEN seam; clean board fix found.** Empirical
    (composer): CoC CONNECTS (0x00D3, link up) then drops on the FIRST OST write — `ENOTCONN os error 107`, 4/4
    identical, co-located (not range). **4/4 deterministic = NOT stochastic coex contention.** Matches core's
    OWN documented seam at `ota_receive_over_coc` :7819: *"'CoC up'+'receiver up' then the client hits ENOTCONN,
    board link-layer never surfaces the drop"* = ACL goes HALF-OPEN (supervision timeout) in the first-OST
    window; board blocks in rx.receive to the 15s guard. NOT adv-during-CoC (adv suppressed, :4068 sequential
    loop) and NOT a handler-close. **★ Root of the still-firing timeout: the otal2cap CoC uses
    `L2capChannelConfig::default()` (:4123) + NO 2M-PHY/DLE — while the cocbench path TUNES `set_phy(Le2M)` +
    `update_data_length(251,2120)` + `{Every(1), 32 credits}` (:4117-4128) "to stream without credit-starvation."**
    A ~900KB stream on the untuned 1M/default CoC = slow round-trip-heavy first OST = long occupancy in the
    fragile window = supervision timeout (coex-aggravated). bit0 survived coex (1-byte/2.5s keepalive = trivial
    occupancy); a 900KB burst doesn't. **CLEAN BOARD FIX (core reflash, corrects our "no clean lever"):** port
    the cocbench L2CAP tuning to the otal2cap serve arm (:4114-4128, widen cfg(cocbench) → include otal2cap).
    cocbench PROVES it streams reliably. **FIX LANDED (core `3c8ea9e1`, parent b79b4f7a):** :4117
    `#[cfg(any(cocbench,otal2cap))]` → set_phy(Le2M) + update_data_length(251,2120) + {Every(1), 32 credits};
    diff main.rs 10+/5-, comment credits the #d026 hive-diagnosis; verified read-only. **REBUILD DOUBLE-GATED:**
    (1) composer btmon shows supervision-timeout reason **0x08** (positive control — don't build the fix on an
    unconfirmed mechanism; if ≠0x08 → hold+re-scope) AND (2) supervisor build order (#d005). On both: rebuild
    d5-otarx-p1 (3c8ea9e1+otal2cap) + d5-otafail-p3 (+otafail) in the hive-owned dir → new ELFs → grant-shape
    .bin extraction → **fresh 3-way .bin cross-check** (composer+core); b79b4f7a bins bd22d272/ce76ea9e RETIRED.
    Composer central-retries = stopgap only. Stale-hk (weave-hk/bench-D5.bin ≠ baked persona) = separate
    resolver drift, out-of-band.
  - **★ ROOT = OST FRAMING MISMATCH (my hypothesis, core-CONFIRMED at source :7866-7871). Fix = tool-side, NO
    reflash.** Central-fix made CONNECT reliable (8/8), but the b79b4f7a board dropped INSTANTLY/det 8/8 on
    first OST = NOT a supervision timeout. `ota_receive_over_coc` REQUIRES `[len u16 LE][message]` framing (:7814,
    mirrors serve_coc :3187, needed for multi-SDU reassembly); ota-push omitted the prefix → first extraction
    read len from `"OS"`=0x534F=21327>4096 → `framing desync (len=21327)` + RESP_ERR 0x0E + RETURN → close →
    ENOTCONN (instant/det/pre-data, exact; dry-run passed on a loopback not enforcing the prefix). **FIX (core
    contract call): ota-push adds the `[len u16 LE]` prefix (composer tool-side).** **NO REBUILD — b79b4f7a bins
    bd22d272 (P1) / ce76ea9e (P3) STAY VALID** (wire-framing fix, not the image). **My 3c8ea9e1 occupancy tuning
    HELD SECONDARY/armed** — deploy ONLY if a post-framing data-burst drop appears (`start seq=` then ODT-drop +
    btmon 0x08); preflight primed, no build now.
  - **★ NEXT LAYER (framing CLOSED → header VERSION SKEW): D5 serial = `verify REJECT reason=1` = BadHeader.**
    attempt-2 (composer added `[len]`) DELIVERED the 204B OST + parsed (my framing diag CLOSED); board
    verify_header REJECTED. Pinned to source: b79b4f7a vendors r2_update @ crates/r2-update/src/lib.rs with
    **`PACKAGE_VERSION=2` (:93) / `HEADER_LEN=123` (:89)**; verify_header :526 `if h.version != PACKAGE_VERSION`
    → BadHeader (reason 1, :588). **Composer signs v3 / HEADER_LEN=137 → DUAL skew** (version 3≠2 fails first;
    len 137≠123 also misaligns). Vendored-vs-live: firmware vendored r2_update v2, composer's ota-sign is v3.
    **RULED (supervisor): (a) composer emits V2 headers this cycle** — tool-side, NO reflash; **my bins stay
    valid**, 3c8ea9e1 tuning secondary/armed. Board v3 re-vendor (bump firmware crates/r2-update→v3/137) =
    POST-CAMPAIGN backlog. Nothing hive-side. [[shared-checkout-path-dep-coupling]]
  - **★ OWNED correction (core):** my "verify floor via HEALTH key-6 ota_status" was WRONG — key-6 is hardcoded
    0 (:3717), NOT the floor. Correct path = read NVS **0x18000** = `[seq u32 LE][floor u32 LE]`, 0xFFFFFFFF→0
    (:7285, core owns). composer verifies seq/floor at 0x18000, not the HEALTH wire.
- **Stale-tree trap RESOLVED + killed (root closed by core+supervisor):** ~/dfr1195-fw-build was an ORPHANED
  linked worktree sharing the branch ref with core's dfr1195-fw-wt — every core commit advanced the shared ref
  under the stale tree ⇒ byte-exact-PARENT "reverse-edits" (nobody wrote my files; my byte-match diagnosis was
  right, mechanism = shared-ref-advance). **Structural fix DONE:** builds now use hive-owned
  `~/dfr1195-fw-hive-build` (independent clone, `.git` = real dir verified, `git checkout --force --detach
  <sha>` always). Old dir rm'd (its pointer named core's `worktrees/dfr1195-fw-wt` admin — removed ONLY the
  duplicate, core's real worktree untouched). Stash dropped. [[offthread-consult-write-race]]
- Other anticipated: beacon-plane diffs (only if core finds emit gaps), extended-wire test image.

**D5 iter-9 conformant (from PINNED `70960dbc`, BUILD_ID coex.iter9.0723): DELIVERED 2026-07-23.** Roy
authorized the reflash; supersedes d5-cos5/`11f2d2ef`. 3 clean builds.
- d5-cos9 `a0157eb2095e960f081dd43a8b47d70770af86ea65928886ade4a04e1e271e0f` (`~/d5-cos9.elf`).
- Persona baked==input `e6108006` @47216 = wire **0xDA73508E PRESERVED UNCHANGED**; masked base `305377b5`.
- Role BAKED_ROLE_PROFILE = RPF1 b[4]=1 Sensor, **b[6]=0 AcceptorOnly** (no initiator) + role≠norole diff
  `aa71d687`. Role byte = the 48B .role record (read_role_profile :3322), NOT the 336B persona. **bit2=0 rides
  70960dbc** engine_task `NodeCaps::new(false)` — the point of the reflash.
- Wave cos≠sin diff (cos `a0157eb2` ≠ sin `57648717`) + `k_cosf` linked + WaveSourceSentant×6 = cosine at
  sentant layer. C: core1×2 + lora_route×6 + espnow×6 + apiary×6 (fakesensor). Markers: BUILD_ID + domain-sep
  + APIARY value=.

**Pair PASS recap (`#d025`, composer co-boot 2026-07-23):** D4 dials XIAO `8c15b0c2` (capture-decouple works;
D4-dials-D5 was a boot-order confound), `0x25` sustained both (D4 ×4/XIAO ×7), bidirectional keepalive 10/21
~2.5s, election Some(D5) canon-correct, XIAO bit2=0. Mechanism reads all metal-vindicated (dial≠election
decoupled, quiescent=serve_coc-sticky). Pair `70960dbc`: D4 `724383ea`/`~/d4-init9.elf`, XIAO
`5fb1565f`/`~/xiao-acc9.elf` (both two-party verified). Sticky-capture secondary = core+supervisor ruled
INTENDED (re-dial-on-lower-peer robustness, iter-10 only if a mixed live bench needs it).

**Canon (closed, cited):** sensor-bit2 RATIFIED — R2-ARCH §3.1.3 v0.17 (D-20260723-05 = #d013) + R2-BEACON §7.2
(bit2 = fixed-AP gateway only). Every MCU board incl a sensor MUST advertise bit2=0; D5-old bit2=true was a
pre-#d013 legacy artifact. (I once scored a stale bar + reopened this closed ruling — owned;
[[cite-canon-before-claiming-a-finding]] currency corollary.)

**Owned lesson:** pre-iter9 dirt in ~/dfr1195-fw-build = off-thread-consult write race (stashed main.rs
byte-matched iter-8 `351a166e` exactly), dropped per supervisor ruling; the `rm -rf target` + detached
byte-clean + positive-control preflight caught it (mandatory standing mitigation).
[[offthread-consult-write-race]] [[positive-control-the-tree-not-just-the-tool]]

Arc (history in DECISIONS.md/git): Fix C (core1 executor isolation) → tri-bearer coex `0x25` on `bee0e996` →
blerole/D4-initiator merge (`54a8a1f3`) → board-to-board iters 3-8 (#d024: rbid resolve, list-gap,
capture-gate, domain-sep, lowest-eligible dial, ap_capable=false H2-fix, accept step-log, keepalive sustain) →
iter-9 #d013 conformance (bit2=0, #d025).

## Open threads (post-campaign, not blockers)

- **sensor-provider_capable canon = CLOSED** (R2-ARCH §3.1.3 v0.17 / R2-BEACON §7.2 = #d013): MCU sensor MUST
  bit2=0. D5 reflashed to `70960dbc` (a0157eb2) closes the D4-elects-D5 wrinkle by construction (all boards
  bit2=0 ⇒ elect None). Pending only the 3-board metal re-score.
- **conn-liveness watchdog** (my `conn.next()`/`is_connected()` primitive): NOT needed — keepalive
  `tx.send.is_err()→break` covers the common case, metal showed zero half-open. Parked as backstop; core wires
  only IF metal ever shows a tx.send-succeeds half-open (session neither sustains nor returns).
- **InvalidRouteLen CLOSED benign** (attribution corrected 2026-07-23): the 2 beacon classes are **OURS**
  (5511 FNV table; supervisor REFUTED the earlier "foreign noise" attribution I carried). The :2729
  EXTENDED-decoder drops (n~29) = OUR beacon frames on the extended path; apiary DATA (READING=64cedb11)
  decodes at :2101 = safe by construction, so the benign verdict HOLDS (canon-correct drops per R2-WIRE
  L244/L250, not strictness, not real-DATA-loss) — only the source label changed (ours, not foreign). Owned a
  mechanism-direction inversion earlier (extended-mis-parses-compact). Optional :2729 log rate-limit parked
  with core (LOW/cosmetic).

## Backlog (Roy-gated, not started)

- **D5 reflash/provision**: D5 stays `11f2d2ef` (cosine ×307). Any reflash needs fresh Roy word.
- **SEN0676 radar sensor plugin** for esp32-s3-dfr1195 (UART/ADC not i2c — confirm with circuits + board.toml).
- **RAK relay-LED** (dev/bench image only, brief flash per relayed frame; heartbeat LED untouched). Low.
- **DFR1195 display mislabel** (cosmetic): screen shows 'hive' twice with different values; relabel per field.
- **RAK tx_power −9dBm** (30cm bench; as923_nz default +20 saturates RX) — core change, rak
  `lora_leaf_config:1219`.
- **AGENTS.md doc-drift**: cites `docs/dfr1195-partitions.csv`; build uses `platforms/dfr1195/partitions.csv`
  (both app@0x20000) — recommend updating.

## Standing artifacts (LIVE on alfred, secret-bearing, off-tree)

- iter-8 pair `~/d4-init8.elf` / `~/xiao-acc8.elf` (RUNG-GREEN pair, flashed).
- D5 cosine `~/d5-cos5.elf` (`11f2d2ef`) — 3rd node, cosine origin-verified ×307, powered distractor.
- Personas ~/.r2-dev-trial/: d4 (0xC434FAFC), xiao (0x8C15B0C2), d5 (wire da73508e). MACs off-tree.

## Safety

- Plain non-force pushes only. Never `--all`/`--mirror`/`refs/keep/*`.
- Three local keep refs preserve removed security material (only local copies). Do not repack/prune/expire.
- Never bypass the fleet secret scan (`ci/public-hygiene.sh`, exit status enforced); forbids MACs/device-tails
  in tracked files — keep board MACs off-tree (bit me once in RESUME).
- Firmware lives in **r2-core** (dfr1195-fw / rak4630-fw are core worktrees). Never edit core; hive
  designs/builds/attests, **core lands source**. Hive never flashes (composer/Roy flash under grants).
- NVS `0x17000` raw role-write = brick class (no role partition on default table); bake role via
  `DFR_ROLE_PATH`, never NVS-write on baked_persona images.
- Env-baked const verify = full `rm -rf target` (incremental cache poisons it) + the DIFFERENTIAL (role vs
  empty, cos vs sin), never raw-bytes-in-ELF for a const-folded value.
- Every commit needs a `Decision-Log:` trailer (`Decision-Log: none` routine). Verify ahead=0 via
  `git ls-remote origin`, not a local ref.

## Branches

- `storing-backend` — real unfinished work on an old base; needs deliberate rebase + validation.
- `hygiene-scanner-v2`, `platform-trait`, `v0.2-relay-handshake` — stale/contained; do not merge.

Key rulings in `DECISIONS.md`. Ops hazard: [[reference-xiao-boot-flush-wedge]]. Lesson:
[[shared-list-serves-multiple-consumers]].
