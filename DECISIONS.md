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

## Records

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
- **Outcome:** The baked persona `8d5d099f` (tg_id `730c29e7-209f-4d2e-c8fd-b68e71f5f73b`,
  tg_hash `0x6E31DEC6`, wire_id `0xCC788B17`) IS the ratified shared bench TG. The relay-fixed
  image (ELF `d1aeefdc`, HEAD `70f442b9`) is flash-ready; STEP3 proceeds. No re-mint, no rebuild.
- **Decision-maker:** Roy (via supervisor relay).
- **Authority basis:** `#d001` ratification (Roy Q2: "shared TG `730c29e7`, all field boards"),
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
  intended TG contradicting `730c29e7`, HALT and surface to Roy.
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
  stash only, never a source commit to core, no gc (shared object store); (c) alfred `~/` build artifacts +
  `/tmp` scratch; (d) local scratchpad + private agent-memory. Read-only everywhere else.
- **Alternatives:** Looser per-lane discretion (rejected — produced the races).
- **Expected consequences:** Fewer wasted/again-superseded actions and no cross-lane tree writes, at the
  cost of an inbox-drain + order-currency check before each consequential step.
- **Evidence:** supervisor relay 2026-07-22 (`#d006`); D-20260722-03 (`#d005`); parked `455ae47a`.
- **Supersedes:** None (extends D-20260722-03).

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
