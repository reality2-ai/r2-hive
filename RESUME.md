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

## Active: P0 RAK compact-relay (2026-07-21)

Supervisor P0: flashed RAK `850b0ec3` (2026-07-14 SF7 devtrial) is extended-only, silent-drops
D4's compact frame at `handle_rx_frame:864`, no relay. Finding: the compact re-vendor already
landed at core `6c8c0d44` (2026-07-18, #71; `main.rs:834 set_wire_format(Compact)`), ancestor of
RAK worktree HEAD `7011934e` — only the shipped artifact was stale.

Built (Alfred, `rak4630-fw-wt/platforms/rak4630`): compact hex
`field-dfu/rak-repeater-compact.hex` sha256 `8215b52a…` from ELF `320560b9…` entry `0x26101`,
features `dev,blespike,uf2,baked_persona,benchsf7`, persona `8d5d099f` baked. SECRET-bearing →
gitignored, never committed. genpkg + serial-DFU are fleet-gated → handed to composer/Roy. RAK has
no partition table (nRF resident UF2, app@0x26000). Reported to supervisor + composer.

## Next action

Await composer staging result (on-metal: D4 K=3 compact → RAK decode → RELAY route_len 1→2 → XIAO).
Then a new objective. Fetch, verify branch + clean tree, run Hive tests + public-hygiene gate before
any commit or push.
