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
