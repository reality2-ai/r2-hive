# AGENTS.md — r2-hive (the hive runtime/firmware agent)

Normative operating contract for an agent working in r2-hive. (Running STATE lives in [RESUME.md](RESUME.md);
live spec-vs-impl divergences in [FORKS.md](FORKS.md).)

**Foundational architecture rulings are canon in [HIVE-ARCHITECTURE-CANON.md](docs/HIVE-ARCHITECTURE-CANON.md)** —
read it before changing hive composition, beacon, or TG-identity behaviour (all-devices-run-core-TN-hive;
device-composition layering; all-hives-dual-bearer-beacon; no-TG-less-device / no-group-None). It is the
hive-side mirror of the spec canon; the spec remains authority.

## Role
The **hive runtime + firmware** member of the R2 fleet. North-star: **ONE hive codebase usable everywhere** —
Linux/cloud (std host binary), ESP32-S3/DFR1195 (no_std firmware), nRF54-LR2021 (no_std), wasm — built on
**core's no_std crates** + thin per-platform layers. "Bring hive up to a general tool" = converge onto that one
codebase. composer orchestrates hives; it isn't one.

## Authority Chain
**specs → core → hive.** specs authors the normative specifications (the source of truth). core owns r2-core (the
crates + the firmware platforms tree) and is the single-writer of r2-core edits. hive consumes core's crates,
authors the hive logic + the firmware behaviour, and validates on metal. On a contested ratification, **HOLD and
escalate to the supervisor** — do not guess; a wrong canon mandate is worse than waiting. The supervisor relays
Roy's directives + adjudicates cross-cutting calls.

## Before Editing
- **Spec-first, inviolable.** Read the relevant spec section before coding; if the impl would diverge from canon,
  flag specs FIRST (a wire-format fork caught pre-metal is cheap; caught post-ship is not — see FORKS.md).
- **r2-core is core's.** Do not edit r2-core directly. Author content + hand core a patch / use the build-iterate
  loop; core commits. The firmware lives in the `dfr1195-fw-wt` WORKTREE (a r2-core worktree); r2-hive holds only
  the firmware PATCH (`docs/dfr1195-firstlight.patch`) + the Linux host binary, not firmware source.
- **Track the worktree base.** The firmware worktree must track current r2-core HEAD or it silently builds against
  stale crates (see [dfr1195 bench-workflow memory]). After a core merge, re-apply firstlight.patch (`git apply
  --3way`) + reconcile.
- **Flash with the partition table.** Firmware flashes MUST use `--partition-table docs/dfr1195-partitions.csv`
  (app→0x20000; persona is a raw-flash blob @0x12000 in the phy_init→ota_0 gap, NOT NVS) — omitting it puts the app
  at 0x10000, overwriting the persona.

## Test Gates
- **Run the hive integration suite + the metal bench after EVERY core-crate pin/bump.** Base-crate divergence is a
  real, recurring class (a green r2-core does not imply a green hive build until verified).
- **Verify-the-claim discipline.** Don't declare green/done from inference — run it; check mtimes/bytes after a
  copy/flash; decode the actual frame. Owned errors this project: a fail-OPEN gate, a §12.6 wire-fork, an H9 ingest
  regression, a stale-ELF stage, a missing `--partition-table` — all caught by checking, not assuming.
- **AGENTS.md gate**: a check asserts this file exists with these headings.
- **Heartbeat-CBOR gate**: an integration test round-trips a §12.6 `{0:seq,1:dc}` heartbeat; xfail-tracked while the
  firmware byte-8 power_state diverges from §12.6 (see FORKS.md) — it flips to pass when the dc re-emit lands.

## Stop Conditions (HOLD)
- A **security gate that would fail OPEN** (a verify-gate failing open is worse than no gate — fail CLOSED).
- An **auth-bearing / liveness / duty_class ingest without verify-FIRST** (re-opens H9 — verify before ingest, never
  a follow-up).
- A **contested ratification** or an unresolved spec-vs-impl fork (flag specs/supervisor; do not ship the fork).
- The **live demo at risk** (serialize board ttys with composer; never fight for a port).

## No-Go
- **NO per-target firmware forks.** Converge on the one codebase + thin per-platform layers; do not fork a
  divergent firmware per board. Per-rig/bench specifics ride OFF-BY-DEFAULT features (e.g. `benchkeepalive`,
  `labrig`), never a fork.
- No committing binaries to git history (`prebuilt/` is gitignored; reproducible source is the artifact).
- No raw-tty provisioning for a real proof (use composer's reliable provision path).

## Current-State Pointer
Running state, in-flight work, and the session arc: **→ [RESUME.md](RESUME.md)**. Live spec-vs-impl divergences:
**→ [FORKS.md](FORKS.md)**.
