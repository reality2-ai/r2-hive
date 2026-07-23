# RESUME — r2-hive

Updated 2026-07-23. `main` clean + pushed. **✅ BOARD-TO-BOARD bit0 SUSTAIN RUNG GREEN — CAMPAIGN CLOSED.**

## Current state

Nothing owed, nothing building. STANDBY for the next supervisor order (+ pinned sha per #d005).

**Board-to-board bit0 ladder rung = GREEN (composer metal, 2026-07-23):** iter-8 keepalive pair sustains
`key0a = 0x25` (BLE bit0 | LoRa bit2 | Mesh bit5) **≥22s both boards** (>> the ratified 10s sustain bar),
bidirectional CoC keepalive ~2.5s (9-10 pings each way), zero wedge / no 120s-close / no half-open. Replaces
the earlier external laptop-CoC-pump with a genuine board-to-board CoC handshake. Campaign closed.

**Delivered iter-8 pair (core `351a166e`, BUILD_ID coex.iter8.0723, KEEPALIVE_MS=2500):**
- D4 initiator `1b0186dbd4278423ce41008acb089dedb4e2cbeb18c6a924a25071b00415743e` (`~/d4-init8.elf`,
  b[6]=1, ≠empty `0d14c684`, persona 0xC434FAFC @47108, masked `2a42e058`).
- XIAO acceptor `74857e1c3decb943260320e98235bd4c7f8245f57fc373d65a45388832afcd04` (`~/xiao-acc8.elf`,
  b[4]=0/b[6]=0, ≠empty `25be41aa`, persona 0x8C15B0C2 @46244, masked `0c1394a9`).
- Both: accept markers + domain-sep `r2-coc-ctrl-v1` + dial-falsifier baked, KEEPALIVE_MS=2500 source-verified.
  Composer TWO-PARTY VERIFY PASS both hosts (alfred+tuxedo). **PAIR ONLY** (supervisor ruling A) — D5 stays
  `11f2d2ef` powered distractor (stronger test: XIAO held provider WITH D5 live in roster, never pruned).

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
