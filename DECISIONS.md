# Key decisions — r2-hive

Durable index of key repo-local rulings. Read it before changing established behaviour.
It is not a task log and does not replace specifications, ADRs, or code.

## Rules

- Append key human/canonical rulings, explicit holds, and consequential agent
  implementation choices. Routine edits, experiments, and task status stay out.
- Name the actual decision-maker and authority basis. An agent choice is delegated
  judgment; never label it human-ratified or let it override canon.
- A decision records context, rationale, alternatives, expected consequences, and
  evidence. Existing records are immutable.
- Review a decision by appending an `R-...` record naming the decision, reviewer/date,
  observed outcomes, evidence, recommendation, and one finding: `appropriate`, `revise`,
  or `insufficient evidence`. A review does not itself change the ruling.
- Change a ruling only through a new decision that names the old ID in `Supersedes`.
  Current means the latest applicable decision not superseded by a later one.
- Newer explicit authority or normative material wins a conflict; append the correction.
- IDs are `D-YYYYMMDD-NN` for decisions and `R-YYYYMMDD-NN` for reviews.
- **Namespace (fleet rule 2026-07-22):** every `D-`/`R-` record in THIS file is the **`hive:` local
  ledger** — cite it in cross-lane comms as `hive:D-YYYYMMDD-NN` (e.g. `hive:D-20260722-02`). A **bare**
  `D-YYYYMMDD-NN` denotes the **specs ledger** (canon authority) by convention. Any hive record that
  MIRRORS a specs-ledger ruling MUST carry an explicit `specs ledger: D-x` cross-ref line at creation.
  Existing immutable records are not renamed; where a pre-rule mirror lacks a cross-ref, append one.
  (D-20260721-03 mirrors the `benchsf7` PHY ruling by citing the normative spec section `R2-LORA §5`
  directly — the canonical reference — not a bare specs-ledger ID. specs confirmed (grepped) there is NO
  specs-ledger record to cross-ref — see R-20260722-03.)

## Records

### D-20260723-01 — Canonical tri-bearer coex base pinned (base-digest pin)

- **Kind:** Decision
- **Date:** 2026-07-23
- **Scope:** The single linkable base for the tri-bearer coex reference implementation
- **Specs ledger:** D-20260722-02 (base canon) + confirm: specs ledger D-20260723-02 (Roy's confirm 2026-07-23) — mirrored here on the PASS-proven set
- **Outcome:** Pin **`bee0e996`** (dfr1195-fw, branch `dfr1195-fw-bit5-keepalive`, off `56d39498`) as the single
  linkable base all coex images derive from (feature-set per board, NO forks). Per-board-type **base_digest**
  (persona-region-masked sha256, two-party recomputable): esp32-s3-xiao-wio-sx1262 (observer) image
  `d12ddcc8…` → masked `d884bba35e8298d12301798d0797dca1e764d49560c107188a279565c1a482b7` (mask `[44984,45320)`),
  persona `0x8C15B0C2`; esp32-s3-dfr1195 (superset/emitter) image `d818ffda…` → masked
  `071b702dad94338ff8910b9b7bcfb4fde54ba1d64b666916e995075b0104abd4` (mask `[45796,46132)`), persona
  `0xC434FAFC`. Table `d4-reflash-partitions-e0e49127.csv` (app@0x20000; baked_persona ⇒ no 0x12000/0x17000).
- **Decision-maker:** Roy (confirmed 2026-07-23, via supervisor).
- **Authority basis:** Roy confirmation; the base-canon (ensemble-composition, no per-target forks) is
  established policy (AGENTS.md No-Go) + specs base-canon.
- **Mechanism:** Fix C (`hive:D-20260722-02`, lora_route_task on core1) + join-suppress + LoRa densify 30s→4s +
  `benchkeepalive` 8000→4000 — puts LoRa/Mesh emit inside the 8s liveness window.
- **Evidence:** key-10 `0x25` sustained 41.6s / 7-of-7 HEALTH frames 2026-07-23 (+ `0x24` 94.2s); acceptance
  `hive:D-20260722-01` MET (`hive:R-20260722-04`). Personas intact, banner app@0x20000, no NVS writes.
- **Known gaps (honest):** bit0 pump-driven (no persistent BLE peer); LoRa beacons unattributed (unkeyed-origin
  admit still stamps bit2); D4 BLE-central/provider election unproven (M7 ghost scaffold, deferred).
- **Surfaces landed:** GH `reality2-ai/r2-core#19` (reference-impl green cell, comment 5050303127); recipe
  registry (`<build-host>:~/coex-flash-recipe.txt`, canonical bit5-keepalive entry); this record.
- **Supersedes:** None (builds on hive:D-20260722-01).

### D-20260721-01 — Repository decision log

- **Kind:** Decision
- **Date:** 2026-07-21
- **Scope:** Repository process
- **Outcome:** Keep a terse repo-local log of key decisions and later reviews.
- **Decision-maker:** Roy
- **Authority basis:** Explicit user ruling
- **Context:** Key rulings were dispersed across transcripts, handoffs, and design files.
- **Rationale:** A uniform durable record makes reasoning and later appropriateness
  analysis discoverable without treating temporary agent prose as authority.
- **Alternatives:** Transcript/RESUME-only history was rejected as transient; ADR-only
  history was rejected because not every important ruling is architectural.
- **Expected consequences:** Easier audits and fewer re-litigated decisions, at the cost
  of one concise record when a key ruling is made.
- **Evidence:** Roy's 2026-07-21 request; [AGENTS.md](AGENTS.md).
- **Supersedes:** None

### D-20260721-02 — RAK bench persona TG `0x6E31DEC6` is canonical; no re-mint

- **Kind:** Decision
- **Date:** 2026-07-21
- **Scope:** RAK4630 compact-relay bench artifact (P0), persona identity
- **Outcome:** The baked persona `8d5d099f` (tg_id `<scrubbed-tg-uuid>`,
  tg_hash `0x6E31DEC6`, wire_id `0xCC788B17`) IS the ratified shared bench TG. The relay-fixed
  image (ELF `d1aeefdc`, HEAD `70f442b9`) is flash-ready; STEP3 proceeds. No re-mint, no rebuild.
- **Decision-maker:** Roy (via supervisor relay).
- **Authority basis:** `#d001` ratification (Roy Q2: "shared TG `<scrubbed-tg>`, all field boards"),
  confirmed by the authoritative parser `parse_persona(8d5d099f) = 0x6E31DEC6 / 0xCC788B17`.
- **Context:** Composer's lift-criteria demanded tg_hash `0x3eb54833` / wire_id `0xd256dc00`, which
  matched none of the 4 provisioned bench personas. Hive measured the baked blob via
  `r2_trust::parse_persona` and refused to fabricate an attest to the expected values.
- **Rationale:** tg_hash is DERIVED (`persona.rs:142 fnv1a_32(tg_id)`), never stored, so a rodata
  u32 scan is structurally blind — the parser is authoritative. On-air relay (`route_len 1→2`) proves
  RELAY, not persona (same-TG members relay regardless); persona-correctness rests on `#d001` + the
  parser. Clean separation.
- **Alternatives:** Re-mint the bench to `0x3eb54833` was rejected — the criteria, not the personas,
  are stale/superseded.
- **Expected consequences:** Flash unblocked. Composer owes: correct criteria to
  `0x6E31DEC6`/`0xCC788B17` and trace the origin of `0x3eb54833`; if that trace shows a DELIBERATE
  intended TG contradicting `<scrubbed-tg>`, HALT and surface to Roy.
- **Evidence:** `parse_persona` harness `scratchpad/persona-attest`; ELF `d1aeefdc` @offset 115234
  == `8d5d099f`; supervisor ruling 2026-07-21; [RESUME.md](RESUME.md).
- **Supersedes:** None (composer's `0x3eb54833` criteria were never a ratified decision here).

### D-20260721-03 — Bench LoRa SF canon is ALL-SF7; reflash the SF12 board(s), not the RAK

- **Kind:** Decision
- **Date:** 2026-07-21
- **Scope:** LoRa bench mesh (D4 + XIAO + RAK), spreading factor unification
- **Outcome:** The bench mesh MUST be one SF, and that SF is **SF7** (`benchsf7`). D4 (measured SF12,
  benchsf7 did not take) reflashes to a confirmed-benchsf7 image; XIAO reflashes iff its boot
  `LORA-ROUTE up (SF..)` line shows SF12; the RAK (already SF7) is NOT downgraded to SF12.
- **Decision-maker:** hive (delegated by supervisor's 2026-07-21 ask to rule the SF direction).
- **Authority basis:** Re-affirms the existing `benchsf7` core-ruling (R2-LORA §5, 2026-07-10,
  spec-blessed v0.4.19); not new canon. Grounded in airtime governance, not preference.
- **Context:** Ground truth split the mesh — D4 `lora_dr=0`=SF12 vs RAK SF7; SF7 and SF12 are
  mutually deaf, so no mesh forms. Supervisor asked hive to rule all-SF7 (reflash the DFRs) vs
  all-SF12 (reflash the RAK).
- **Rationale:** A 29 B compact frame at SF12 ≈ 1647 ms ToA → ~1 per 16.5 s at the nbrs=0 10%
  neighbour-scaled duty ≈ 16× too slow for the ~1/s apiary stream; SF7/BW125 ≈ 67 ms ToA → ~1.5/s,
  meets 1/s with margin. All-SF12 would put the whole bench below its apiary throughput requirement.
  Compact frame/§5.1 vector unchanged (PHY-only).
- **Alternatives:** All-SF12 (downgrade the RAK) rejected — it regresses the campaign's 1/s stream.
- **Expected consequences:** D4 reflashed with a confirmed-benchsf7 sha-pinned image (removes the
  "did benchsf7 land" ambiguity that split the mesh); XIAO conditional on its boot SF; RAK separately
  owed tx_power −9 dBm (as923_nz default +20 saturates the 30 cm RX). Physical reflash = Roy/composer.
- **Evidence:** composer metal `lora_dr=0`=SF12; `dfr1195 main.rs:5305-5315`, `rak main.rs:1219-1227`,
  `r2-sx1262 lib.rs:124`; memory `sf12-airtime-cant-carry-sensor-stream`; supervisor thread 2026-07-21.
- **Supersedes:** None.

### D-20260722-01 — key-10 transports bitset is a Phase-0 false-green; coex proof = making it real

- **Kind:** Decision
- **Date:** 2026-07-22
- **Scope:** Tri-bearer coex proof (esp32-s3 tn_base), `dfr1195 build_health` key-10 transports field
- **Outcome:** `build_health` key-10 is hardcoded `e.uint(10); e.uint(1)` (`main.rs:3548`) = a board
  with zero bearer traffic still reports transports=1 — a genuine false-green. The tri-bearer coex
  proof IS replacing it with a per-bearer admitted-frame liveness bitset (bit0=BLE, bit1=LoRa,
  bit2=Mesh/ESP-NOW), each bit set ONLY on a real admitted frame within a liveness window.
- **Decision-maker:** supervisor (Roy-directed tri-bearer task), 2026-07-22; hive designs/verifies.
- **Authority basis:** supervisor directive; acceptance criteria supervisor-approved.
- **Context:** Tri-bearer task requires proving BLE+LoRa+ESP-NOW coex RUNS (real traffic), not
  compiles. Presence flags (`BLE_UP`/`LORA_UP`) and the hardcoded key-10 cannot distinguish a starved
  bearer from a live one — presence != reachability.
- **Rationale:** Admission (a frame the transport accepted) is the only signal that separates a
  carrying bearer from a spawned-but-silent one. LoRa already has admission telemetry
  (`tx_hi_admitted`); BLE/ESP-NOW need admit counters; key-10 then reflects real per-bearer liveness.
- **Acceptance:** all 3 bits set in ONE health frame, sustained ≥10s continuous per-bearer traffic;
  peers LoRa=D4, ESP-NOW=2nd S3, BLE=CoC from a central (interim phone central pending Android).
- **Ownership routing:** the firmware change is in `platforms/dfr1195/src/main.rs` = **r2-core's repo**
  (dfr1195-fw worktree). Per hive AGENTS.md ("Never edit r2-core") hive does NOT commit it — hive
  designed it (`~/coex-health-design.txt`), **core lands it**, and key-10's semantics change is a
  **composer dashboard-contract** change (r2.hb.health key-10 parse). Hive builds the XIAO
  `bridge,ble` image from the landed core HEAD and runs the metal coex proof.
- **Alternatives:** Leaving key-10 hardcoded (rejected — it is the false-green the proof must remove).
- **Expected consequences:** A real coex proof; a cross-repo change (core firmware + composer contract)
  coordinated in dependency order.
- **Evidence:** `dfr1195 main.rs:3548` (hardcode), `:3891/:3894` (BLE CoC), `:1558` (espnow RX),
  `:5435` (LoRa admit); supervisor thread 2026-07-22; design `~/coex-health-design.txt`.
- **Supersedes:** None.

### D-20260722-02 — BLE-advertise executor-starvation fix = C (move lora_route_task to esp-rtos core1)

- **Kind:** Decision
- **Date:** 2026-07-22
- **Scope:** dfr1195-fw coex build — the tri-bearer proof's BLE bit0 blocker (advertise never starts)
- **Outcome:** Fix **C** = relocate `lora_route_task` to a dedicated esp-rtos core1 embassy executor
  (LoRa stays sync; RAK/LR2021 untouched). C isolates the WHOLE LoRa task → fixes advertise-start AND
  CoC-connect AND ongoing runner-starvation, mechanism-agnostic. **A** (split trouble-host runner to a
  priority InterruptExecutor) = **dead** (Stack un-shareable across executors; no InterruptExecutor in
  esp-rtos 0.3.0). **B** (async `r2-sx1262`) = **backlog** (fleet migration: DFR xtensa + nrf54-lr2021 +
  rak4630 + r2-ble all sync embedded-hal 1.0 → cross-runtime ripple; needed for C6 single-core).
- **Decision-maker:** core (owns `r2-sx1262` + the firmware); hive designed A-prime (= C) and VERIFIED
  C's data-layer safety; supervisor ratified A-prime-v5 + B-backlog earlier.
- **Authority basis:** core ownership of both crates; hive's Authority-Chain role = design + host/metal
  verification, not landing (AGENTS.md "never edit r2-core").
- **Context:** The coex build's `peripheral.advertise()` hangs forever (bit0 dark). Isolation diag
  `9e0b76de` (BLE+ESP-NOW, no `lora_route_task`) → `:3884` adv prints → `loraroute` IS the blocker.
  Mechanism (core, code-grounded): startup `LoRaTransport::new` SX1262 `configure` (`:5386`, hw_reset
  1.2ms + 5ms calibrate) collides with advertise-enable → dropped HCI response → permanent hang. RX is
  event-driven DIO1 (`:5366`).
- **Rationale:** C is mechanism-agnostic — it fixes both the startup collision AND ongoing CoC-connect
  starvation (`:7244` precedent) that a startup-sequencing fix would miss. Hive's earlier Fix-B premise
  ("async removes a long spin") was WRONG (driver has no long block — `wait_busy` bounded, `service()`
  non-blocking); owned, and it reinforces C over B.
- **Hive data-safety verification (grounded, dfr1195 main.rs):** `LoRaTransport` owned wholly inside
  `lora_route_task` (`:5391`); dedicated `lora_spi` (`:847`, separate from display `:796`); `LoraRadioTy`
  (`:5041`) `ExclusiveDevice<Spi<Blocking>,Output,Delay>` + `[u8;32]`/u32 args = Send → 2nd-core spawn
  legal; all core0↔lora statics `CriticalSectionRawMutex` (`:224-226`, `:4588-4597`) = multicore-safe.
  Residual: cross-core CS stall (bounded); confirm esp-rtos time-driver is multicore.
- **Gate (HELD):** C-commit HELD until the 2nd-half `9e0b76de` laptop-CoC→bit0 result. bit0 LIGHTS →
  chain proven, C delivers bit0; bit0 DARK → separate serve_coc defect C alone won't fix. Result needs
  metal (Roy grant + composer console); routed to supervisor.
- **Alternatives:** A (dead — verified), B (deferred to backlog), a targeted startup-sequencing delay
  (rejected — fixes advertise-start only, not ongoing CoC starvation).
- **Expected consequences:** virgin dual-core bring-up (isolated, metal-testable); v5 = C + fallback in
  one Roy grant. B remains owed for single-core portability.
- **Evidence:** core fleet msg 2026-07-22; diag `9e0b76de` `:3884`; dfr1195 main.rs `:5386/:5391/:847/
  :5041/:224-226/:4588-4597/:7244`; supervisor A-prime-v5 ratification; [RESUME.md](RESUME.md).
- **Supersedes:** None (refines the A-prime lean in RESUME; A-prime == C).

### R-20260722-04 — D-20260722-01 acceptance MET: tri-bearer coex proof PASSES

- **Kind:** Review
- **Decision reviewed:** D-20260722-01 (key-10 real per-bearer admitted-frame liveness bitset; coex proof)
- **Reviewer/date:** hive + composer (joint metal verdict), 2026-07-22
- **Observed outcome:** On the `bee0e996` bit5-keepalive images (XIAO `d12ddcc8`, D4 `d818ffda`), composer's
  soak showed key-10 **`0x25`** (BLE bit0 | LoRa bit2 | Mesh bit5) **SUSTAINED 41.6s contiguous / 7-of-7
  HEALTH frames ≥ 10s = PASS**. bit2+bit5 solid (0x24 sustained 94.2s vs ~47% flicker pre-fix) from
  `benchkeepalive` 4000 + LoRa cadence densify; bit0 via the CoC pump. Personas intact, app@0x20000, no NVS
  writes. The D-20260722-01 acceptance criteria (all 3 bits in ONE frame, sustained ≥10s continuous, on a
  real admitted-frame bitset) are MET.
- **Campaign arc (what got it there):** Fix C (`hive:D-20260722-02`, lora_route_task on core1) unblocked
  BLE advertise; the join-suppress (56d39498, harmless cleanup) + the metal-refuted WiFi-join-scan hunt
  (owned) re-aimed onto the real bit5 root; `benchkeepalive` 8000→4000 + densify gave the LoRa/Mesh margin
  under the 8s liveness window. The 0x17000-NVS-brick correction kept the fix on the bake-only path.
- **Evidence:** composer soak 2026-07-22; images `d12ddcc8`/`d818ffda` from `bee0e996`; [RESUME.md](RESUME.md).
- **Finding:** appropriate — the false-green key-10 (D-20260722-01 origin) is now a real, sustained
  tri-bearer liveness signal. Proof CLOSED.

### D-20260722-03 — #d005 build-preflight gate (drain inbox, pinned sha, clean tree)

- **Kind:** Decision
- **Date:** 2026-07-22
- **Scope:** Any flashable-artifact build in this repo (firmware images)
- **Outcome:** Before starting ANY flashable-artifact build, hive MUST: (1) DRAIN its inbox and check for
  supersedes/retractions of the build order FIRST; (2) have an explicit CURRENT supervisor build order
  naming the pinned sha; (3) do a clean detached checkout of that sha with tree-state verified
  byte-identical to the commit (`git diff <sha>` empty), never ambient HEAD. Advice/analysis/source-reads
  are ungated. Gate active until Roy lifts it.
- **Decision-maker:** Roy (standing directive, via supervisor relay).
- **Authority basis:** Explicit Roy ruling (`#d005`).
- **Context:** Two avoidable races on 2026-07-22 — hive built the retracted drop-loraroute confirm image,
  then built the superseded `9c08c89f` C-only (`455ae47a`) while Roy's "Go — v5 on XIAO" upgrade stood
  unread in the queue. Both avoidable by reading the queue before spinning cargo. Separately, a shared
  build worktree kept re-dirtying (a 1148-line then a 33-line main.rs strip) — a dirty-tree build voids
  sha provenance (flashing neither HEAD nor any known state = the brick-history class).
- **Rationale:** Order-currency + tree hygiene are cheap preflight checks that prevent flashing a
  superseded or unprovenanced artifact. The pinned-sha discipline (refusing ambient HEAD) was already
  correct and is retained; this adds the two guards around it.
- **Alternatives:** Relying on push-propagation of supersedes (rejected — races; pull/drain-verify beats
  it). Building on ambient HEAD (rejected — the branch advanced `9c08c89f→105eb4aa→e4031efd` mid-session).
- **Expected consequences:** Slightly slower build start (one inbox drain + a tree-verify), far fewer
  wasted/again-superseded builds and zero dirty-tree provenance voids.
- **Evidence:** supervisor relay 2026-07-22 (`#d005`); parked `455ae47a` (do-not-flash); stashes
  `hive-preCbuild-20260722`/`hive-preV5build-20260722`; memory [[positive-control-the-tree-not-just-the-tool]].
- **Supersedes:** None.

### D-20260722-04 — #d006 fleet-wide tight-rein (drain-first, workspace ownership, report-don't-act)

- **Kind:** Decision
- **Date:** 2026-07-22
- **Scope:** All lanes (recorded here as binding on hive); every consequential action
- **Outcome:** Standing order, effective now until Roy lifts. (1) **Drain-first:** before ANY consequential
  action (build, flash, commit/push to a shared tree, canon change, bench mutation) drain the inbox +
  verify the latest supervisor order is still CURRENT; act only on a current order naming the exact
  target/sha. (2) **Workspace ownership:** write only in declared owned paths; shared trees read-only
  unless the named owner. (3) **Report, don't act:** findings are reported, not acted on; no self-expanded
  scope without an order covering it; a crossed/stale-suspect order = ASK, don't act.
- **Decision-maker:** Roy (standing directive, via supervisor relay).
- **Authority basis:** Explicit Roy ruling (`#d006`), 2026-07-22.
- **Context:** Extends `#d005` ([[DECISIONS#D-20260722-03]]) fleet-wide after avoidable races (hive built
  a retracted then a superseded image; general risk of lanes acting on stale/crossed orders and writing
  outside their paths).
- **Hive write-paths declared:** (a) `/home/roycdavies/Development/R2/r2-hive` — the only repo hive
  commits/pushes; (b) `~/dfr1195-fw-build` (r2-core linked worktree, hive-exclusive) — checkout/reset/build/
  stash only, never a source commit to core, no gc (shared object store); (c) <build-host> `~/` build artifacts +
  `/tmp` scratch; (d) local scratchpad + private agent-memory. Read-only everywhere else.
- **Alternatives:** Looser per-lane discretion (rejected — produced the races).
- **Expected consequences:** Fewer wasted/again-superseded actions and no cross-lane tree writes, at the
  cost of an inbox-drain + order-currency check before each consequential step.
- **Evidence:** supervisor relay 2026-07-22 (`#d006`); D-20260722-03 (`#d005`); parked `455ae47a`.
- **Supersedes:** None (extends D-20260722-03).

### R-20260722-03 — no specs-ledger cross-ref exists for D-20260721-03 (benchsf7)

- **Kind:** Review
- **Decision reviewed:** D-20260721-03 (bench LoRa SF = ALL-SF7)
- **Reviewer/date:** hive, 2026-07-22 (specs-answered, per the 2026-07-22 namespace rule)
- **Observed outcome:** Under the namespace rule, a hive record mirroring a specs-ledger ruling owes a
  `specs ledger: D-x` cross-ref. specs grepped its ledger: **no `D-x` exists for benchsf7** — the specs
  ledger began 2026-07-21 (first record D-20260721-01) whereas the benchsf7 ruling is 2026-07-10, recorded
  spec-section-only. So there is no ledger ID to append (fabricating one is forbidden).
- **Canonical cites (specs-affirmed, precise):** R2-LORA §3.0 changelog v0.4.23 (RAK-beacon RESOLVED, Roy
  2026-07-10) + the §5.1:170 allow-list entry (SF7 bench-arm conditions, incl. graduation-MUST-adopt-SF12).
  D-20260721-03's generic `R2-LORA §5` cite stands canonical as-is.
- **Evidence:** specs fleet msg 2026-07-22 (grepped, not recalled); [DECISIONS.md](DECISIONS.md) Rules preamble.
- **Finding:** appropriate — the mirror correctly cites the normative section; no cross-ref record is owed
  because no specs-ledger record exists. Flag closed.

### R-20260722-02 — correction to D-20260722-02: esp-rtos 0.3.0 HAS an InterruptExecutor

- **Kind:** Review
- **Decision reviewed:** D-20260722-02
- **Reviewer/date:** hive, 2026-07-22 (grounded in esp-rtos-0.3.0 source)
- **Observed outcome:** D-20260722-02's A-death rationale stated "no InterruptExecutor in esp-rtos 0.3.0."
  That is **wrong** — `esp-rtos-0.3.0/src/embassy/mod.rs:310` defines `pub struct InterruptExecutor<SWI>`
  with `pub fn start(&'static mut self, priority: Priority) -> SendSpawner` (`:380`).
- **Correction:** A stays **dead**, on the correct ground: the trouble-host runner shares one
  `stack.build()` borrow with peripheral/central (can't split across executors) + BleConnector unsafe in
  ISR. A core0 InterruptExecutor for LoRa would PREEMPT the runner (worse, not better) — the block must
  cross to a different CORE, which is exactly C. Conclusion unchanged; only the sub-reason corrected.
- **Dual-core pattern handed to core (grounded):** `start_second_core::<STACK>(p.CPU_CTRL,
  sw_int.software_interrupt1, &'static mut Stack, FnOnce()+Send)` running an `esp_rtos::embassy::Executor`
  that spawns only `lora_route_task`; int1+CPU_CTRL free (`main.rs:406` uses int0 only); scheduler-start
  before second-core; core0 BLE/wifi/espnow/io unchanged.
- **Evidence:** `esp-rtos-0.3.0/src/lib.rs:355` (`start_second_core`), `src/embassy/mod.rs:185/217/310/380`;
  dfr1195 `main.rs:406` (int0 only), `:869` (core0 lora spawn to delete).
- **Finding:** revise (A-death sub-reason corrected). D-20260722-02 outcome (C for v5, A dead, B backlog) stands.

### R-20260722-01 — review of D-20260722-01: bit layout should be enum-ordinal

- **Kind:** Review
- **Decision reviewed:** D-20260722-01
- **Reviewer/date:** hive, 2026-07-22 (composer proposal, hive-verified)
- **Observed outcome:** D-20260722-01 specified a compact layout (bit0=BLE, bit1=LoRa, bit2=Mesh).
  Composer showed both that and its own contract (`1=wifi 2=lora 4=ble`) are NON-ordinal — neither
  matches the canonical `Transport` enum (`repr(u8)`: Ble0 Wifi1 Lora2 Internet3 Usb4 WifiMesh5 Udp6).
- **Revised layout:** key bit_i = (Transport ordinal i live): BLE<<0, LoRa<<2, WifiMesh<<5 → ESP32
  tri-radio = `0x25`; WASM = Internet<<3 | Udp<<6; decode = `(bits>>ordinal)&1`. One layout spans the
  whole heterogeneous TN; firmware cost trivial (shift constants). Admit-atomics, W≈8s, admit-RX-only,
  WiFi-false-green-drop all unchanged.
- **Evidence:** `r2-route/src/transport.rs:43` (`Transport`), `r2-transport/src/transport.rs:39`
  (`TransportId`) — same ordinals, TWO enums (drift guard owed: a test they agree, or one canonical).
- **Finding:** revise (layout → enum-ordinal). The D-20260722-01 outcome — kill the false-green via a
  real admitted-frame bitset — stands; only the bit assignment is corrected.

### D-20260728-01 — evidence durability: no archive-based retrospective claims; per-board append-only capture directory

- **Kind:** Decision (recorded by hive; NOT hive's ruling — see Decision-maker).
- **Date:** 2026-07-28 (ruling made 2026-07-27 night).
- **Scope:** All lanes. Recorded here as binding on hive's reporting, immediately.
- **Outcome, two halves with different timing:**
  1. **BINDS NOW — no brief, ledger entry or report may cite a log archive as if one exists.** Until a
     durable archive exists, retrospective device questions are answered by **MEASURING HARDWARE** or
     marked **UNANSWERABLE**. *"The logs show no X" is not a weak claim — it is NOT A CLAIM.*
  2. **QUEUED — board captures persist OUTSIDE session scratchpads:** a durable per-board capture
     directory, **appended to, never recreated**. Owner: **composer** (bench + capture tooling).
     Deliberately not done the night it was decided (new work, nothing pressing on it, composer under a
     nothing-touches-a-board order). **First application already made:** the D4 anti-rollback read grant
     (2026-07-28) directs its capture to `~/.local/share/r2-bench/captures/D4/`, not a scratchpad.
- **Decision-maker:** supervisor (policy call explicitly claimed as theirs), on hive's raised gap.
  Supervisor's own fleet-side ledger IDs for these are **D-20260727-76** (no board-log archive) and
  **D-20260727-77** (durable capture). This entry is hive's durable record OF that ruling, not a
  re-issue of it; hive holds no authority to make or revoke it.
- **Authority basis:** supervisor standing-policy authority; raised by hive, decided by supervisor,
  cited as binding in the 2026-07-28 D4 grant.
- **Context / evidence:** an attempt to establish whether any board ever committed an anti-rollback floor
  failed three times on instrument errors (`grep | head` returns head's rc; `timeout` cannot exec a shell
  builtin ⇒ rc=127 and the search never ran; then rc=124 timeout). Re-run correctly with a reachability
  control planted inside the corpus, it found the corpus was **two files, both from that day, both one
  board** — the entire preserved board-capture archive on the machine, every earlier capture having died
  with its session. **The question was unanswerable in principle: an archive that was never an archive.**
- **The inversion that motivated it (hive's observation, supervisor's wording):** had the three instrument
  bugs NOT fired, the search would have returned a fast, clean, **confident null over two files** and been
  believed. **An instrument failure is loud; an empty corpus is silent** — so the dangerous null is the one
  that comes back quickly and cleanly, because nothing about a tidy `0 results` invites *"what did you
  actually search?"*. The bugs were the only reason anyone examined the denominator.
- **Forcing asymmetry:** a live board can be measured; **a REFLASHED board cannot be asked about its past
  at all** — and boards are reflashed weekly, so every reflash permanently destroys answerable history
  while capture lives only in a session scratchpad.
- **Alternatives:** (a) keep citing logs with caveats — rejected: it invites exactly the confident-null
  failure above; (b) fix the archive but keep citing it meanwhile — rejected: the citation is unsound
  until the archive exists. Adopted verbatim from hive's framing: *either fix the archive or stop citing
  it.*
- **Expected consequences:** retrospective device questions get measured or refused, at the cost of more
  hardware reads and more grants; once (2) lands, "unanswerable in principle" becomes "grep it".
- **Compliance check already run (hive):** swept all tracked `.md` for archive-citation patterns —
  denominator **214 files**, `docs/archive/` excluded — **2 hits, 0 violations**. One pre-policy citation
  (`RESUME-archive.md:1330`, a contemporaneous console read used live as a discriminator, a genuine
  measurement when made) is now **permanently unverifiable** because that capture died with its session;
  flagged, **not rewritten** — the archive's job is recording what was believed and on what basis.
- **No history rewrite.** Prior records stand as written.
- **Supersedes:** None. Complements `D-20260722-04` (report-don't-act) by constraining what a report may
  assert as evidence.

### D-20260728-02 — vector/union method: settled rules, and the stop that ended the thread

- **Kind:** Decision (recorded by hive; supervisor's ruling — see Decision-maker).
- **Date:** 2026-07-28.
- **Scope:** All lanes. Recorded here as binding on hive's comparison and control work.
- **Outcome — SETTLED, no further debate without a falsifier:**
  1. Hex matching is **case-insensitive on both sides**. Canon itself is mixed-case — measured: **24 of 40**
     canon vector files contain uppercase 16+hex runs, so this is the majority condition, not an edge case.
  2. "Entropy" means **STRUCTURE** — ramps, repeats, fills, constant byte-deltas — **never character
     diversity**. `1122334455667788` has eight distinct characters and is a ladder.
  3. **Dedupe AFTER matching, display only**, and merge only when the home set is identical **AND** one run
     contains the other. A filter applied *before* the comparison changes what can be **found**; applied
     *after* it changes only what is **shown**.
  4. Conviction = **entropy + semantics + citation**. **Dating EXONERATES, never convicts** — a content
     match establishes shared origin, not direction of copy.
  5. Resolve canon **at run time**; a cross-repo comparison **names a REF, never a path**; gates **print
     the ref they used**, so the record is generated rather than maintained.
  6. A negative control carries its own **VACUITY GUARD** — assert the perturbation actually removed the
     thing, or a green means the fixture failed and is indistinguishable from the pass you wanted.
  7. **Exoneration is not coverage. Nominal is not effective.**
- **Decision-maker:** supervisor (ANNEAL ruling, on Roy's call for convergence). hive holds no authority to
  revoke or extend it.
- **Authority basis:** supervisor standing-policy authority; Roy called the convergence.
- **hive's own state under these rules:**
  - **Route-2 CLOSED** (`0fe80cd`): 11 transcribed hex literals in `src/usb.rs` + `src/usb_serial.rs` pinned
    **individually** against the vendored vectors, plus an orphan check. Both negative controls fire. The
    first version asserted `pinned > 0` and was a **false green** — an aggregate assertion survives
    individual drift.
  - **Gated column — CORRECTED 2026-07-28, the first wording was WRONG and is struck.** Proven by
    execution: `.git/hooks/pre-push` → `pre-push.local` → the vector gate. **`pre-push.local` is a SYMLINK
    to `.githooks/pre-push`, which is TRACKED** (`ls -l`), as are `ci/check-vendored-vectors.sh` and
    `scripts/setup-hooks.sh`. Only the **WIRING** is absent on a fresh clone, and `scripts/setup-hooks.sh`
    installs it **at the extension point so both gates run**. Corrected status: **EFFECTIVE ON ANY CLONE
    WHERE `setup-hooks.sh` HAS BEEN RUN** — content tracked, activation one documented command, **zero CI
    invocations**. The remaining honest gap is CI, which is what it always was.
    - **How the error was made:** I read "untracked" off `git ls-files` and never ran `ls -l`. **I asked
      git about the PATH and git answered about the PATH** — the symlink target was never in the question.
      A null from one tool is not a property of the object; see the different-construction rule below.
    - **`core.hooksPath` MUST NOT be set as a remedy** (supervisor retracted their own instruction to do
      so, in full). Setting it makes git run the tracked hook **INSTEAD OF** `.git/hooks/pre-push`, which
      **IS the fleet secret-scan**: 81 secret/scan references there versus 3 in the tracked copy. The fix
      for a wiring defect would have **silently disabled the secret failsafe on every repo that applied
      it**. `scripts/setup-hooks.sh:8` already documents why it deliberately does not set it. hive never
      set it (local, global and effective all UNSET, verified) — nothing to undo here.
  - **STANDING RULE (supervisor, out of the above):** **before a remediation crosses a repo boundary,
    prove the defect with a DIFFERENT CONSTRUCTION than the one that reported it, and name what else the
    fix touches.** A cleanup inherits the same ownership boundary as the mistake and is **more** tempting
    because it feels like undoing. Corollary from specs: **a repaired instrument inherits the habit that
    broke the original unless the repair changes the KIND of evidence, not just the key** — a wiring fix
    for a wiring defect changes nothing about how the claim is evidenced.
  - **Union:** hive gates 4 of 40 canon files. Holes: `0053a1b2…` **2**, `851fdee3…` **1**. Reconciled with
    composer — `851fdee3` UNION 3/3, `0053a1b2` UNION 3/3, `425ed4e4` **UNION 1/2, the one real hole**.
  - **Zero transcriptions from the vectors** — four hits, four exonerations, each on a different mechanism.
- **STATED ONCE, WITH A FALSIFIER, NOT RE-LITIGATED (composer's finding):** content-addressed matching sees
  **one encoding**. A value also present truncated, re-cased, hyphenated (a UUID form breaks the hex run),
  base64'd or prose-quoted is invisible to it. **So every hole count in this thread is a LOWER BOUND, not a
  measurement.** *Falsifier:* find a canonical value whose non-hex-run encoding has a home no lane gates,
  and the bound moves.
- **The honest accounting, recorded because it is the reason for the stop:** every finding was real and
  **not one moved task #7**. The attested ELF has sat since attestation with no `.bin`, no board and no
  grant. **A correction thread is finished when the next correction costs more than the defect it finds.**
- **Supersedes:** None. Complements `D-20260728-01` (evidence durability).

### D-20260728-03 — a borrowed category carries a borrowed severity

- **Status:** STANDING. **Decision-maker:** supervisor. **Authority basis:** supervisor standing-policy
  authority. **Scope:** global (recorded here because hive's own report was the vector).
- **Rule:** **when you match a defect to a named class, compare MECHANISMS, not surfaces.** A borrowed
  category imports the severity of the thing it was borrowed from, and that severity may not travel.
- **The worked example, in which hive's report was the thing borrowed:** specs classified its hook
  situation as *"the same defect as hive's"*. The surface fact was true; the **CLASSIFICATION was
  flattering**, because it silently imported **hive's recoverability**.
  - **hive:** tracked `.githooks/pre-push`, a tracked installer `scripts/setup-hooks.sh`, and a tracked
    `ci/check-vendored-vectors.sh`. A fresh clone is one documented command from effective.
  - **specs:** **no tracked hook, no installer, no `.githooks/`.** A fresh clone gets **NO secret scan and
    NO identity-leak scan** — nothing to activate. Setting `core.hooksPath` there would have pointed git
    at a **missing directory**: both gates silent, **no fallback**.
  - **composer:** same shape as specs, and **`r2-composer` is PUBLIC**.
  - Same surface ("host-local hook, hooksPath unset"), **materially different blast radius**.
- **Consequence for hive specifically:** hive's corrected status in `D-20260728-02` is **hive's**, and must
  **not** be cited as a template for any other lane's structure. hive states its own mechanism; each lane
  measures its own.
- **Not being fixed tonight** — both logged with falsifiers; supervisor sequences with Roy. hive owns
  neither repo and opens nothing.
- **Relation:** the mechanism-side companion to `D-20260728-02`'s different-construction rule. That one
  governs how you PROVE a defect before crossing a boundary; this one governs how you NAME it.
- **Specs ledger:** none — this is a supervisor standing-policy ruling, not a mirror of a specs-ledger
  record. Cite it cross-lane as `hive:D-20260728-03` per the namespace rule above.
- **Supersedes:** None.

### R-20260728-01 — review of D-20260728-02's gated-column status: correct a third time, and a live hazard

- **Kind:** Review. **Reviews:** `hive:D-20260728-02` (gated-column status) and `hive:D-20260728-03`.
- **Date:** 2026-07-28. **Reviewer:** hive, prompted by supervisor's `# CLOSED` on the fresh-clone gap.
- **Finding:** **revise** — the status was right in its second form but incomplete, and the completion
  matters.
- **Observed outcome — TWO installers, two different repos, and they CHAIN:**
  1. **`fleet install-git-hooks`** (source of truth `claude-fleet/hooks/git/`) installs
     `.git/hooks/pre-push` — **the fleet secret scan**. Verified: hive's active hook sha256
     `96ccc7d61046b4dac8d1e5d7e2975945cf658f02327e712aa3094b44b8f7445b` **==** live
     `claude-fleet HEAD:hooks/git/pre-push`, worktree clean; **all six R2 repos byte-identical** (491
     lines, 81 secret/scan references).
  2. That artifact **carries the extension point itself** — `hooks/git/pre-push:274`
     `if [[ -x "$hookdir/pre-push.local" ]]; then exec "$hookdir/pre-push.local" "$@"; fi` — which is
     where **hive's** `scripts/setup-hooks.sh` chains `.githooks/pre-push` (vectors + hygiene).
  - So the fresh-clone answer is **centrally tracked, not per-repo**: clone `claude-fleet`, run
    `fleet install-git-hooks`, then this repo's `scripts/setup-hooks.sh`. Neither is a per-repo mechanism
    and **hive must not design one**.
- **★ LIVE HAZARD, hive's own finding, unowned:** a **SECOND `claude-fleet` checkout** exists at
  `/home/roycdavies/Development/R2-codex/claude-fleet` @ `ee8d90c` — a **strict ancestor, 249 commits
  behind**. Its `hooks/git/pre-push` is sha256 `15faddd4…`, **275 lines, 47 secret/scan references**.
  `TOOL_ROOT` is derived from the **invoked script path** (`bin/fleet:23`) and that copy is `-rwxr-xr-x`,
  so **invoking it by path installs the 47-reference hook over every repo** — a silent downgrade of the
  secret scan from 81 to 47 references, after which `fleet doctor` reports *current* against the stale
  source. `PATH` resolves to the live checkout, so the risk is **path-invocation only**. Reported;
  **owner is supervisor/Roy — hive changes nothing outside `r2-hive`.**
- **★ hive's own error, recorded because the construction is the lesson:** hive first reported this
  **REFUTED**, having resolved `claude-fleet` with `find` and read the stale checkout. `command -v fleet`
  was the correct construction and would have taken two seconds. This is the **same shape** as reading
  "untracked" off `git ls-files` in `D-20260728-02`: **a tool answered truthfully about the object it was
  handed, and the wrong object was handed to it.** `D-20260728-02`'s different-construction rule is about
  proving the DEFECT; the gap it missed is **identifying the SUBJECT** — resolve the artifact the way the
  system resolves it, not the way a search finds it.
- **Recommendation:** no change to the rulings; both stand. `D-20260728-02`'s corrected status is amended
  by this record to name the fleet installer as the first of two, not the only one.
