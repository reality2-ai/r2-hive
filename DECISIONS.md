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
