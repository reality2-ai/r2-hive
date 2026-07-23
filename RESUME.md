# RESUME — r2-hive

Updated 2026-07-23. `main` clean + pushed. **✅ bit0 SUSTAIN RUNG GREEN (iter-8). iter-9 conformance PAIR
DELIVERED — awaiting two-party verify + re-score.**

## Current state

iter-9 pair delivered + attested. STANDBY for composer two-party verify + the metal re-score.

**iter-9 conformance PAIR (core `70960dbc`, BUILD_ID coex.iter9.0723, #d013): DELIVERED 2026-07-23.**
`70960dbc` = iter-8 `351a166e` + bit2=0 beacon + NodeCaps FALSE constant (supersedes iter-7 AcceptorOnly
proxy) + :4807 capture-decoupled + request_data_plane inert-documented. 4 clean builds (1m48-2m01s).
- D4 initiator `724383ea11194728c949c502e0724dba9e70031498bf3c47f9fba9f1f184a041` (`~/d4-init9.elf`,
  b[4]=2/b[6]=1, ≠empty `2eb48979`, persona 0xC434FAFC baked==input 0ad4a84d @47108, masked `5fa838e6`,
  C apiary+espnow+lora_route+core1).
- XIAO acceptor `5fb1565f71b2efc5b06280f14057ddfb7715a106045cc1751a25a56e3cb542a9` (`~/xiao-acc9.elf`,
  b[4]=0/b[6]=0, ≠empty `8aae9d8b`, persona 0x8C15B0C2 baked==input 43638da0 @46244, masked `fbfca876`,
  C espnow+lora_route+core1 observer, no apiary).
- Both: accept markers (ACL-accepted + L2CAP-ENTRY + CoC-up-serving) + keepalive= + membership-verified +
  domain-sep `r2-coc-ctrl-v1` + dial-falsifier + BUILD_ID baked. Conformance source-verified at 70960dbc:
  `provider_capable: false` :3913, capture `if connectable {` un-gated :4818, engine_task no-ble_role :5427.
- **PAIR ONLY** — D5 stays `11f2d2ef` (distractor persists for re-score: elect-None must hold WITH D5
  resolvable in roster). **RE-SCORE EXPECTATION: 0x25 sustained UNCHANGED both + D4 NEG elect None (no
  'Negotiate provider=da73508e').**
- **★ 2026-07-23 pre-iter9 dirt, unattributed, likely off-thread-consult write race, DROPPED** (supervisor
  ruling, core disclaimed). ~/dfr1195-fw-build carried an uncommitted main.rs presenting as REVERSED #d013;
  did NOT build it — stashed non-destructively, byte-verified clean at 70960dbc, built the pinned commit.
  Evidence: stashed main.rs byte-matches iter-8 `351a166e` EXACTLY (both sha `3ee577c410d18a10`; 70960dbc HEAD =
  `b79a140e`) = pre-iter-9-era content, the recorded off-thread-consult live-checkout write hazard, not a rogue
  actor. Committed HEAD wins → stash dropped. Preflight `rm -rf target` + detached byte-clean + positive-control
  is the standing mitigation (mandatory) — it caught this.
  [[offthread-consult-write-race]] [[positive-control-the-tree-not-just-the-tool]]

**Prior rung GREEN (iter-8 `351a166e`, composer metal 2026-07-23):** 0x25 sustained ≥22s both, bidirectional
CoC keepalive ~2.5s, zero wedge. Board-to-board CoC replaced the external pump. Campaign #d024 closed
(iter-6 dial → iter-7 eligibility+accept → iter-8 sustain).

Arc (history in DECISIONS.md/git): Fix C (core1 executor isolation) → tri-bearer coex `0x25` sustained on
`bee0e996` → blerole/D4-initiator merge (`54a8a1f3`) → board-to-board iters 3-8 (L3 rbid resolve, list-gap,
capture-gate, domain-sep, lowest-eligible dial, ap_capable=false H2-fix, accept step-log, keepalive sustain).

## Open threads (post-campaign, not blockers)

- **sensor-provider_capable canon** (core+specs own): D4 still elects D5 (0xDA73508E) as DATA-provider at boot
  (`Negotiate provider=da73508e`) — orthogonal cosmetic wrinkle, CoC/bit0 SUSTAINS regardless. Question relayed:
  should a SENSOR be `provider_capable` at all (same class as Initiator `ap_capable=false`)?
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
