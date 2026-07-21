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

Image is correct and PACKAGED. One open item (persona-TG identity), composer/Roy-owned.

**Packaged (composer, verified):** canonical hex `rak-repeater-compact-70f442b9-858bc638.hex` sha256
`858bc638…` (== ELF `d1aeefdc`); image_digest `e5c7073e…` (3-way reproduced: objcopy ELF + hex +
zip-extracted bin); flash_package_digest `d51b5b86…` on
`field-dfu/r2-rak4630-repeater-compact-70f442b9-devtrial.zip`. Ready for Roy STEP3 (serial-DFU;
`adafruit-nrfutil` on Alfred `~/rak-flash/nrfutil-venv/bin`). The earlier stale-staging / filename
collision (composer had pulled superseded `8215b52a`) is resolved by the sha-distinct name.

**Open — persona-TG identity (contested, NOT hive-closeable):** lift-criteria demand tg_hash
`0x3eb54833` / wire_id `0xd256dc00`. Measured via `r2_trust::parse_persona` on baked blob `8d5d099f`
(== bytes in ELF `d1aeefdc` @115234): tg_id `730c29e7-209f-4d2e-c8fd-b68e71f5f73b`, tg_hash
`0x6E31DEC6`, wire_id `0xCC788B17`. tg_hash is DERIVED (`persona.rs:142 fnv1a_32(tg_id)`), never
stored → a rodata u32 scan is structurally blind (explains composer's 0-hits; parser is the only
instrument). All 4 bench personas share `0x6E31DEC6`; criteria name a different provisioning. On-air
relay proof will NOT settle it (D4/RAK/XIAO share the TG → relay succeeds regardless). Harness:
`scratchpad/persona-attest`.

## Next action — HELD on composer/Roy ruling

Await the canonical-TG ruling: accept `0x6E31DEC6` (artifact flash-ready as-is), or require
`0x3eb54833` (composer re-mints personas = a re-provision, then hive rebuilds). Nothing else
outstanding on hive's side.
