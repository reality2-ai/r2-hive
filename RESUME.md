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

Two-part fix landed: DECODE (`set_wire_format(Compact)`, core `6c8c0d44`) + RELAY egress
(`dp.set_relay_egress(RelayEgress::SameCarrier)`, core `70f442b9`, `main.rs:844` — CrossCarrier default
had masked LoRa out so `relay_on==0`, `route_len` stuck at 1).

Final artifact (Alfred, HEAD `70f442b9`): `field-dfu/rak-repeater-compact.hex` sha256 `858bc638…`
(ELF `d1aeefdc…` entry `0x26101`, features `dev,blespike,uf2,baked_persona,benchsf7`, persona
`8d5d099f`). SECRET-bearing → gitignored/scp-only. Supersedes decode-only `8215b52a`. Handed composer
for genpkg; reported to supervisor. RAK has no partition table (nRF UF2, app@0x26000).

Image is correct. Two open blockers before flash/lift, both composer-owned:

1. **Stale staging** — composer's staged candidate `a3c7791` bound the SUPERSEDED decode-only
   artifact (ELF `320560b9`/hex `8215b52a`); verified before the rebuild overwrote the files.
   Corrected: re-stage against on-disk `d1aeefdc`/`858bc638` (HEAD `70f442b9`).
2. **Persona-TG mismatch (lift-blocker)** — composer lift-criteria demand tg_hash `0x3eb54833` /
   wire_id `0xd256dc00`. Measured via `r2_trust::parse_persona` (scratchpad harness, fnv recompute
   agrees): baked persona `8d5d099f` = tg_id `730c29e7-209f-4d2e-c8fd-b68e71f5f73b`, tg_hash
   `0x6E31DEC6`, wire_id `0xCC788B17`. ALL 4 bench personas (rak/field-APPROVED/d4/xiao) share
   `0x6E31DEC6` (D4/RAK/XIAO one TG, as relay needs); criteria match none. Reconcile = composer:
   fix criteria to `0x6E31DEC6`, or re-mint the bench (re-provision). Harness:
   `scratchpad/persona-attest`.

## Next action — HELD on composer

Await composer ruling on the canonical bench TG + re-stage against `d1aeefdc`/`858bc638`. genpkg
(`adafruit-nrfutil` present on Alfred at `~/rak-flash/nrfutil-venv/bin`) + serial-DFU are Roy/flash-host,
fleet-gated. On-air proof owed: D4 K=3 compact → RAK decode → RELAY `route_len 1→2` → XIAO.
