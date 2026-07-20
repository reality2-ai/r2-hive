# RESUME — r2-hive

Updated 2026-07-21. Fleet worker stopped; `main` is clean, pushed, and has no active
task-owned changes.

## Safety

- Use plain, non-force pushes only. Never push `--all`, `--mirror`, or `refs/keep/*`.
- Three local keep refs preserve removed security material and are the only local copies.
  Do not repack, prune, expire unreachable reflogs, or pack refs until their owner rules.
- Never bypass the fleet secret scan. Run `ci/public-hygiene.sh` with its exit status
  enforced; its hostname findings remain advisory debt, not a clean-security claim.

## Branches

- `hygiene-scanner-v2` has a handoff-only local tip preserved on remote safety branch
  `safety/hygiene-scanner-resume-20260721`; do not merge that diary commit.
- `platform-trait` is stale handoff prose.
- `storing-backend` contains real unfinished work on an old base. It needs a deliberate
  rebase and validation, not a blind merge.
- `v0.2-relay-handshake` is already contained by `main`.

## Next action

Await a new objective. Fetch, verify the branch and clean tree, then run Hive's tests and
public-hygiene gate before any commit or push.
