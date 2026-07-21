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

Image is correct, PACKAGED, and RULED flash-ready. Hive side complete.

**Packaged (composer, verified):** canonical hex `rak-repeater-compact-70f442b9-858bc638.hex` sha256
`858bc638…` (== ELF `d1aeefdc`); image_digest `e5c7073e…` (3-way reproduced); flash_package_digest
`d51b5b86…` on `field-dfu/r2-rak4630-repeater-compact-70f442b9-devtrial.zip`. Roy STEP3 (serial-DFU;
`adafruit-nrfutil` on Alfred `~/rak-flash/nrfutil-venv/bin`).

**Persona-TG RESOLVED (D-20260721-02):** `0x6E31DEC6` / `0xCC788B17` (tg_id `730c29e7…`, blob
`8d5d099f`) is the `#d001`-ratified shared bench TG — supervisor/Roy ruling 2026-07-21. NO re-mint.
Composer's `0x3eb54833`/`0xd256dc00` criteria were stale/superseded. On-air relay (`route_len 1→2`)
proves RELAY not persona (same-TG members relay regardless); persona rests on `#d001` + the parser.
Owed by COMPOSER (not hive): correct criteria + trace origin of `0x3eb54833` (HALT to Roy if
deliberate). Harness kept: `scratchpad/persona-attest`.

## Active: LoRa mesh not forming (blocks the on-air relay proof)

Supervisor 2026-07-21: `#d001` relay not on-air, broader than RAK — mesh isn't forming. Capture: D4
emits 4 apiary `64cedb11` compact frames (ENQUEUED→LoRa) but XIAO forwards ZERO and hears NOTHING
direct from D4 (count=0); DFRs leaderless role=STA, nbrs~0, synced=false; no `route_len` anywhere.
Get the DIRECT D4→XIAO `route_len=1` working FIRST; RAK relay can't be tested until the mesh is up.
Firmware/radio = hive; physical (antenna/range/SF) = Roy.

**First deliverable — SF map — DELIVERED (sup7):** base `as923_nz()` = 916.8/BW125/SF12/+20dBm/
sync0x21 (`r2-sx1262/src/lib.rs:124`). Both DFRs run `lora_route_task` (not the SF12-only `lora_task`)
— `fakesensor` AND `xiaobridge` both pull `loraroute` (`Cargo:139/:294`), spawn gate `main.rs:853`.
All three SF7 by construction (benchsf7: DFR `main.rs:5312`, RAK `main.rs:1224`) — so SF is NOT the
likely D4↔XIAO cause. Ground-truth SF = DFR boot log `LORA-ROUTE up (SF{sf} …)` (`main.rs:5320`).

Open leads (need composer/Roy — no TTY here): (1) read D4+XIAO boot-log SF (SF12 there = benchsf7
missing = deaf); (2) exact flash feature-list per board to rule **labrig** — `#[cfg(labrig)]`
shifts freq to 919.8MHz (`main.rs:5301`) and the boot log HARDCODES "916.8" so it hides the shift;
(3) if SF+freq check out, chase the RX path. Separate real asymmetry (threatens RAK relay, not
D4↔XIAO): **RAK tx_power = +20dBm** (as923_nz default; benchsf7 only touches SF) vs both DFRs' −9dBm
— at 30cm +20 saturates the RX (CRC-fail). RAK needs −9 for the bench.

## RAK artifact (parked, flash-ready)

Relay-fixed image done: hex `858bc638`/ELF `d1aeefdc` (HEAD `70f442b9`), image_digest `e5c7073e`,
flash_package_digest `d51b5b86`. Persona TG ruled canonical (D-20260721-02). Awaits mesh-up + Roy
STEP3.
