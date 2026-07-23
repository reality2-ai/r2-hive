# RESUME — r2-hive

Updated 2026-07-23. `main` clean + pushed. **✅ iter-9 conformance pair PASS (`#d025`). D5 conformant reflash
DELIVERED + attested (Roy-authorized) — awaiting two-party verify + 3-board re-score.**

## Current state

**✅ iter-9 conformance PAIR PASS (`#d025`, campaign closed). D5 CONFORMANT REFLASH DELIVERED + ATTESTED
(Roy-authorized) — awaiting two-party verify + 3-board re-score.** No build pending.

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

**3-BOARD BAR (core declared, supervisor locked — analyze metal against THIS):** all elect None; D4 dials XIAO
(lowest of TWO live resolvable acceptors = stronger tiebreak test, directly exercises my sticky-capture
secondary — D4-dials-XIAO ⇒ tiebreak robust/iter-10 unneeded, D4-dials-D5 ⇒ sticky-capture bug real); D4↔XIAO
`0x25` ≥10s + keepalive; D5 resolvable + bit0 DARK = EXPECTED. Falsifiers: elect-Some leak / D4-dials-D5-both-
resolvable / pair drop / D5-disrupts-pair.

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
- **InvalidRouteLen CLOSED benign**: triple-confirmed foreign SF7 beacon noise (2 classes 43895e89/bafe8ac1,
  ~1000/2158) mis-parsed by the EXTENDED decoder (:2729, n~29 not n~54); apiary (READING=64cedb11) decodes at
  :2101 = safe by construction. Verdict (canon-correct drops per R2-WIRE L244/L250, not strictness, not
  real-DATA-loss) HELD; owned a mechanism-direction inversion (extended-mis-parses-compact, not the reverse).
  Optional :2729 log rate-limit parked with core (iter-9+, LOW/cosmetic).

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
