# AGENTS.md — r2-hive

Hive runtime, Linux host, and firmware integration. Running state: `RESUME.md`.
Live spec/implementation divergences: `FORKS.md`.

## Role

One hive codebase across Linux/cloud, ESP32-S3, nRF54/LR2021, and WASM, built on
canonical `r2-core` crates plus thin platform layers. One writer owns this repo;
reviewers remain read-only until explicit handoff. Read
`docs/HIVE-ARCHITECTURE-CANON.md` before identity, beacon, or composition changes.

## Authority Chain

`r2-specifications -> r2-core -> r2-hive`

Specs define behaviour; core owns shared crates and firmware source; hive consumes and
validates on host/metal. This repo MUST NOT edit or fork core. Contested canon or a
spec/implementation split: HOLD and ask specs/core/supervisor.

## Before Editing

- Read relevant spec and `FORKS.md`. Do not invent downstream fixes.
- Core/firmware changes go to core owner. Hive keeps its host code and documented patch.
- Firmware worktree MUST track current core HEAD before build/flash.
- DFR1195 flash MUST use `--partition-table docs/dfr1195-partitions.csv`; persona is
  raw flash at `0x12000`, app at `0x20000`.
- Security/auth/liveness/duty-class ingest MUST verify first and fail closed.
- Serialize board TTY access with composer; live demo ports are shared resources.

## Test Gates

- After every core pin/bump, run relevant hive integration suite and metal bench.
- Verify copied/flashed mtimes and bytes; decode actual frames; reject stale ELF claims.
- Canonical heartbeat is R2-WIRE §12.6 CBOR `{0:seq, 1:dc}`.
- Treat patch/review claims as conjectures. Run strongest concrete falsifier; high-risk
  work SHOULD receive one independent read-only refutation, then converge.

## Stop Conditions

- Fail-open security gate.
- Unverified auth/liveness/duty data enters state or relay.
- Unresolved spec fork or contested ratification.
- Flash parameters, worktree base, artifact freshness, or port ownership is uncertain.
- Destructive action lacks explicit authority.

## No-Go

- NO per-target firmware forks. Use one codebase, thin platform layers, and off-by-default
  rig features.
- No binaries, secrets, or raw-TTY provisioning for real proof.
- Never edit r2-core from this repo.
- Never stage user/peer/unrelated dirt; named task-owned paths only.

## Git and decisions

Commit verified increments and non-force-push upstream. Before idle/done, no local
commit may remain silently ahead. Report push blockers; never force-push or bypass gates.
`DECISIONS.md` is the durable repo ledger. Read it before changing established behaviour
and append ratified outcomes with rationale/evidence. Append later reviews without
rewriting history. A challenge wounds confidence only; authority must
revoke or ratify successor. Never impersonate authority.

## Current-State Pointer

Read `RESUME.md`, `FORKS.md`, then task-relevant canon. `RESUME.md` MUST be one concise
current snapshot, not history. Messages stay terse: claim, falsifier, path, consequence,
required next step. Human documentation remains clear prose.
