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

**SF map delivered (sup7).** Base `as923_nz()` = 916.8/BW125/SF12/+20dBm/sync0x21
(`r2-sx1262:124`). Both DFRs run `lora_route_task` (fakesensor+xiaobridge both pull `loraroute`);
all three SF7 *by construction* under benchsf7 (DFR `main.rs:5312`, RAK `:1224`). Ground-truth SF =
DFR boot log `LORA-ROUTE up (SF{sf} …)` (`:5320`).

**Root cause (composer metal):** D4 `lora_dr=0` = **SF12** — benchsf7 did NOT take on the flashed D4;
RAK = SF7. SF split → mutually deaf → no mesh. The hive build RECORD claimed D4=benchsf7 but metal
refuted it → a non-benchsf7 (stale) ELF had been flashed; the board wins over the label.

**Ruling D-20260721-03: bench canon = ALL-SF7** (airtime: SF12 = 16× over the 1/s apiary duty).
Reflash the SF12 board(s) to benchsf7; do NOT downgrade the RAK.

**D4 fix built, differential-proven, brick-safe recipe handed:**
- image `~/d4-fakesensor-benchsf7-dev-baked-cbd6bf67.elf` sha `cbd6bf67` (fakesensor,benchsf7,dev,
  baked_persona; HEAD `dca5d126`; persona `0ad4a84d` → tg `0x6E31DEC6`/hive_id `0xC434FAFC`, D4
  identity unchanged). Differential proof benchsf7 took: `cbd6bf67` ≠ non-benchsf7 `a23c21ea`.
- partition table `~/d4-reflash-partitions-e0e49127.csv` sha `e0e49127` (platform r2cfg table; ota_0
  app **@0x20000**). Recipe `~/d4-reflash-recipe.txt` (also in composer's thread).
- **Brick avoided two ways:** (1) `--partition-table` forces app@0x20000 (espflash default 0x10000
  = the D4 brick, over the 0x12000 config plane); (2) `baked_persona` → `read_persona()` returns the
  compiled const (`main.rs:50`), NEVER touches flash 0x12000 → this reflash writes NO raw offsets,
  app-only. Pre-write tripwire: confirm espflash's plan shows app@0x20000 not 0x10000, else ABORT;
  no erase-flash. Post: boot must read `LORA-ROUTE up (SF7 …)`.
- SECRET-bearing (baked persona) → scp-only, uncommitted. Roy writes the flash-auth (artifact
  `cbd6bf67`, target tuxedo-os); flashing is fleet-gated from hive. **Staging gap:** artifacts are on
  **Alfred**, flash host is **tuxedo-os** — composer/Roy must scp Alfred→tuxedo + re-verify both shas
  on tuxedo before flashing (do not flash the Alfred paths). HELD for Roy's go; no hive action until.
- **Doc drift flagged:** AGENTS.md cites `docs/dfr1195-partitions.csv` (older phy_init); the build
  uses `platforms/dfr1195/partitions.csv` (r2cfg). Both app@0x20000. Recommend AGENTS.md → r2cfg
  table (owed, not yet edited — governance change).

Open: (1) XIAO boot SF after Roy reset — if SF12, build matching `xiaobridge,benchsf7` ELF; (2) RAK
tx_power `−9dBm` for 30cm — a **core** change to rak `lora_leaf_config` (`main.rs:1219`), then hive
rebuilds; (3) `labrig` ruled out.

## Active: tri-bearer tn_base + D5 image (Roy-directed, 2026-07-22)

Task: esp32-s3 tn_base running BLE+LoRa+ESP-NOW concurrently; PROVE coex RUNS (real traffic per
bearer, presence != reachability). ALSO a D5-persona fakesensor,benchsf7 image (D5 own identity).

Scoped (sup12/13): all 3 bearers already exist as tasks (`ble_task:748`, `espnow_task:764`,
`lora_route_task:854`); combo **`bridge,ble`** spawns all three (bridge→loraroute+dev,
ble→esp-radio/coex+esp-now+trouble-host; espnow re-enabled by bridge alongside LoRa, `:762`). Coex:
BLE+ESP-NOW share one 2.4GHz radio (esp-radio/coex time-slice); LoRa independent SX1262. **Feasibility
CONFIRMED:** `bridge,ble,benchsf7` links clean (exit 0, ELF 1357484B, HEAD `dca5d126`).

**Shaping resolved (supervisor 2026-07-22):** coex proof node = **XIAO** (`bridge,ble`, its EXISTING
TG-730c29e7 persona — no new mint); **D5 = separate** fakesensor/loraroute sensor image (composer
mints its own). Don't conflate role with bearer-set — that gating smell is a later canon-fix (all
radios on base, role=ensemble). Acceptance approved: key-10 driven by admitted-frame counters, each
bit set only on an admitted frame, all 3 in ONE health frame, SUSTAINED ≥10s continuous per-bearer.
Peers: LoRa=D4, ESP-NOW=2nd S3, BLE=CoC from a central (interim phone central pending Android).

**Instrumentation DESIGNED + filed (D-20260722-01), routed to core.** `build_health` key-10 is a
hardcoded `1`=WiFi false-green (`:3548`); fix = 3 per-bearer admitted-second atomics set at real
admit-RX (BLE `serve_coc:3891`, LoRa ingress, Mesh `espnow_task:1558`) → key-10 liveness bitset
bit0=BLE/bit1=LoRa/bit2=Mesh (W≈8s). Design `~/coex-health-design.txt`. **Boundary:**
`platforms/dfr1195/main.rs` is **r2-core's repo** (dfr1195-fw worktree) → AGENTS.md "never edit core"
→ hive designed it, **core lands it** (asked); key-10 is also composer's dashboard contract (flagged).
Hive builds the XIAO `bridge,ble` image + runs the metal proof AFTER core lands AND #d001 closes
(XIAO is the live #d001 observer — nothing flashes to it now).

**D5 persona — BLOCKED on a provision decision (Roy authority).** My "no D5 persona" was a
scope-limited null (checked only `~/.r2-dev-trial`). A D5 persona exists at
`~/.config/r2-composer/bench-personas/D5.bin` (sha `2951fedf`) — but `parse_persona` shows it is a
**different TG**: tg_id `211e0d75`, tg_hash `0x89BFBD4C`, hive_id `0x89E83D99`, role byte[4]=0 (Hive,
not Sensor). NOT the bench TG `730c29e7`/`0x6E31DEC6` → baking it puts D5 in another trust group,
can't join the bench mesh. A bench-TG D5 needs a fresh provision (key-mint gated + touches
D-20260721-02) = Roy's call via supervisor. Also unresolved (composer): the rig-map D5 MAC and the
MAC read off the ttyACM1 board called D5 DISAGREE (the board's MAC matches no rig-map entry; actual
MACs held off-tree per hygiene) — resolve which physical board is D5 before any bake. Escalated;
will NOT bake until both land.

**Core confirmed ownership** (2026-07-22): core lands it on `dfr1195-fw`, hive does NOT edit core
("read-only-reviewer role is correct") — AGENTS.md upheld. Design pasted into the core thread. Core's
contract challenge + falsifier: `build_health` is `map(15)`, keys 0–14 all occupied (11=uptime,
12=beat_seq, 13=conductor, 14=nbrs) — so my earlier "additive key-11" was WRONG (key 11 = uptime; I'd
read the map myself and missed it). Corrected ruling: **(A) redefine key-10** = breaking (kills the
false-green + proves; KAT + composer cutover in lockstep) **vs (B) add key-15** = additive
(`map(15)→map(16)`, key-10 stays, key-15 = liveness bitset, non-breaking; key-10 lie persists). Hive
rec = B + separate key-10 honesty-fix; **ruling owed (Roy/supervisor).** Core preconditions accepted.

**Bit layout revised to enum-ordinal (R-20260722-01, composer proposal, hive-verified):** anchor on the
`Transport` enum (Ble0 Wifi1 Lora2 Internet3 Usb4 WifiMesh5 Udp6) — bit_i = ordinal i live; ESP32
tri-radio = `0x25`; one layout spans the whole heterogeneous TN. Both prior layouts (mine + composer's)
were non-ordinal. Drift guard LANDED + hive-verified: core `31acf41a`
(r2-core-consolidation) locks both enums to the §2.2 ids + the `0x25` witness; hive ran
`cargo test -p r2-route transport_ordinals_agree` = PASS (129 total). Anchor = emit + byte-exact CBOR
KAT on `r2-route::Transport` ordinal (guard makes `TransportId` interchangeable). Composer updates its
contract + `health_reader.rs`.

**HELD** on: (a) the key-10-vs-key-11 ruling, then core landing the patch (KAT + composer cutover);
(b) D5 provision authority + board-identity (separate track). XIAO flash waits on #d001 close.

## RAK artifact (parked, flash-ready)

Relay-fixed image done: hex `858bc638`/ELF `d1aeefdc` (HEAD `70f442b9`), image_digest `e5c7073e`,
flash_package_digest `d51b5b86`. Persona TG ruled canonical (D-20260721-02). Awaits mesh-up + Roy
STEP3.
