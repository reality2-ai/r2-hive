# RESUME — r2-hive

Updated 2026-07-22. `main` clean + pushed. Active work: tri-bearer coex proof (images built,
awaiting flash-auth). Key rulings in `DECISIONS.md` (D-20260721-01..03, D-20260722-01, R-20260722-01).

## Safety

- Use plain, non-force pushes only. Never push `--all`, `--mirror`, or `refs/keep/*`.
- Three local keep refs preserve removed security material and are the only local copies.
  Do not repack, prune, expire unreachable reflogs, or pack refs until their owner rules.
- Never bypass the fleet secret scan. Run `ci/public-hygiene.sh` with its exit status enforced;
  its hostname findings remain advisory debt. It also forbids MACs/device-tails in tracked files
  (bit me in RESUME — keep board MACs off-tree).
- Firmware lives in **r2-core** (dfr1195-fw / rak4630-fw are core worktrees). AGENTS.md: never edit
  core. Hive designs patches + builds/flashes/verifies; **core lands source changes**.

## Branches

- `storing-backend` — real unfinished work on an old base; needs deliberate rebase + validation.
- `hygiene-scanner-v2` (tip on `safety/hygiene-scanner-resume-20260721`), `platform-trait`,
  `v0.2-relay-handshake` — stale/contained; do not merge.

## Active: tri-bearer coex proof — IMAGES READY, awaiting flash

Roy-directed: esp32-s3 tn_base running BLE+LoRa+ESP-NOW concurrently; PROVE coex RUNS (real per-bearer
traffic, presence != reachability). All 3 preconditions now MET: core landed the key-10 ordinal
liveness bitset + key-18 schema=2 + KATs (**dfr1195-fw HEAD `97175901`**, r2-cbor 46/46 + r2-route
drift guard `31acf41a` hive-verified); composer cutover ready (key-18≥2 gate, `f74baf4`); **#d001
CLOSED** (RAK relay counterfactual passed — boards free).

**Coex images built + verified** from HEAD `97175901` (base `bridge,ble,benchsf7,baked_persona`,
BUILD_ID `coex.0722.1225`, fw_sha `0x6616A287`):
- XIAO (proof node) `~/xiao-coex-tribearer-coex.0722.1225.elf` sha `9031ffa2`, hive_id `0x8C15B0C2`
- D4 (LoRa+ESP-NOW peer) `~/d4-coex-tribearer-coex.0722.1225.elf` sha `2cc2c2d6`, hive_id `0xC434FAFC`
- both TG `0x6E31DEC6` (baked identities parse-verified); table `~/d4-reflash-partitions-e0e49127.csv`
  (`e0e49127`, app@0x20000); recipe `~/coex-flash-recipe.txt`. SECRET-bearing → scp-only.
- Brick-safe: `--partition-table` → app@0x20000 (default 0x10000 = the D4 brick); `baked_persona` →
  no flash-0x12000 write (app-only). Pre-write tripwire: espflash plan must show app@0x20000 else ABORT.

**Acceptance (D-20260722-01):** XIAO health key-10 = **`0x25`** (bit0 BLE | bit2 LoRa | bit5 WifiMesh,
enum-ordinal), all 3 in ONE frame, sustained ≥10s continuous. Traffic: LoRa D4↔XIAO, ESP-NOW D4↔XIAO,
BLE phone(nRF Connect) central→XIAO CoC. Dashboard decodes ordinal via key-18≥2.

**Held:** flashing is fleet-gated → Roy writes flash-auth (2 boards, brick-history), composer flashes.
Then hive metal-verifies key-10=0x25 and hands back the result.

## Queued (Roy directives, AFTER the coex proof)

- **Canonical base (MUST):** once coex-proven, pin the tn_base sha as the single linkable base ALL
  images derive from (ensemble-composition, no forks); record REFERENCE-IMPLEMENTATION in GH project
  reality2-ai #1 item **#19** (green cell + pinned sha + mechanism + inheritance + known gaps). File
  in DECISIONS. Build ONE base identically for D4/D5/XIAO; XIAO adds the bridge-leg (USB-Android)
  ensemble, not a fork. **DFR1195 = superset reference board** (coex + sensor + bridge).
  **base_digest definition (flagged to composer):** the XIAO/D4 coex ELFs are byte-identical except
  131 bytes, all inside the baked-persona region — so `base_digest` must be persona-INDEPENDENT (the
  provenance tuple HEAD 97175901 + features + toolchain, or a persona-excluded hash), NOT a per-board
  raw-ELF sha. Composer's provenance call (D-20260722-02..06).
- **D5 back on path:** all 3 S3 boards run ESP-NOW (each other's peers). D5 = 3rd ESP-NOW node —
  needs Roy-gated persona mint (existing `D5.bin` is a DIFFERENT TG `0x89BFBD4C`, not bench
  `0x6E31DEC6`) + MAC/board-identity resolution (rig-map vs ttyACM1 disagree). Sequenced after proof.
- **SEN0676 radar sensor plugin** for esp32-s3-dfr1195 (D-20260722-04 gap: no S3 sensor plugin binds;
  SEN0676 = UART/ADC not i2c — confirm with circuits + coordinate board.toml). Closes superset sensor.
- **RAK relay-LED (dev/bench only):** add a brief LED flash on each relayed frame, DEV image only,
  exclude prod; heartbeat LED untouched. Low priority.
- **DFR1195 display mislabel (low/cosmetic):** screen title shows 'hive' on two lines w/ different
  values — relabel each field (hive_id / TG / wire); report the actual two values.
- **RAK tx_power −9dBm** (30cm; as923_nz default +20 saturates RX) — a core change to rak
  `lora_leaf_config:1219`. **AGENTS.md doc-drift:** cites `docs/dfr1195-partitions.csv` (older); build
  uses `platforms/dfr1195/partitions.csv` (r2cfg) — both app@0x20000; recommend updating.
