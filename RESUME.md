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

**v1 coex images (coex.0722.1225) were BROKEN → refuted on metal:** LoRa bit2 dark — XIAO silent,
D4 drop-storm. Root cause: I built `bridge,ble,benchsf7` but **`bridge` pulls neither `loratcxo` (TCXO
1.8V, REQUIRED, `main.rs:819`) nor `xiao` (LoRa SPI pin selector, `:841`)** — the proven SF7 images
had them via `fakesensor`/`xiaobridge`; I dropped them swapping to `bridge`. So SX1262 unclocked (both)
+ XIAO on DFR pins (silent). Discard v1 + `ad9fc529`.

**v2 CORRECTED (coex.0722.1251, fw_sha `0x6A27F1F4`), from HEAD 97175901:**
- D4 (DFR, ESP-NOW+LoRa peer) `~/d4-coex-tribearer-coex.0722.1251.elf` sha `8b93c3e5`
  (`bridge,ble,benchsf7,baked_persona,loratcxo`; hive_id `0xC434FAFC`; persona@45192; DFR-base masked
  `be16b5c7`)
- XIAO (Wio, proof node) `~/xiao-coex-tribearer-coex.0722.1251.elf` sha `d61ef967`
  (`+loratcxo,xiao`; hive_id `0x8C15B0C2`; persona@45212; Wio-base masked `88e6cdd7`)
- both TG `0x6E31DEC6`; table `~/d4-reflash-partitions-e0e49127.csv` (`e0e49127`, app@0x20000); recipe
  `~/coex-flash-recipe.txt` (v2). Brick-safe (app@0x20000 tripwire; baked_persona = no 0x12000 write).
- **Base is per-board (composer base_digest ruling):** canonical base = ONE provenance tuple
  (HEAD + features + toolchain) instantiated per board-type; base_digest = a per-board-type
  persona-masked sha256 TABLE — `esp32-s3-dfr1195=be16b5c7` (mask 45192), `esp32-s3-xiao-wio-sx1262=
  88e6cdd7` (mask 45212), each two-party recomputable. "One base" stays machine-checkable = same tuple
  + each board's masked sha matches its row (pin-cfg = a carrier fact, D-20260722-03/04). Runtime
  board-pin detection would give a truly single binary (#19 known-gap).
- **Flash grant LAPSED** (was premised on v1 shas 9031ffa2/2cc2c2d6) → composer asked supervisor for a
  refreshed grant naming the v2 shas; I gave supervisor the authoritative v2 facts. Core supersedes
  v1 coex.0722.1225/0x6616A287 (do-not-flash). **Core insight:** the metal refutation VALIDATED the
  key-10 bitset — a genuinely dead LoRa correctly showed dead (present!=reached), not a false-green.

**Acceptance (D-20260722-01):** XIAO health key-10 = **`0x25`** (bit0 BLE | bit2 LoRa | bit5 WifiMesh,
enum-ordinal), all 3 in ONE frame, sustained ≥10s continuous. Traffic: LoRa D4↔XIAO, ESP-NOW D4↔XIAO,
BLE phone(nRF Connect) central→XIAO CoC. Dashboard decodes ordinal via key-18≥2.

**Flashing (2026-07-22):** v1 → LoRa refuted → v2 corrected. Refreshed Roy grant LIVE
(`.fleet/flash-authorization`, artifact coex.0722.1251, shas `d61ef967`/`8b93c3e5`); composer
two-party-verified v2 + pre-staged; XIAO back on tuxedo; flash after composer's BLE watch. Awaiting
re-read: XIAO key-10=0x25, all 3 bits, ≥10s. ESP-NOW works both ways; BLE awaits Roy's nRF Connect.

**USB-Android bridge SYNC-silence (supervisor's "2nd coex bug") — RULED not-foldable, v2 proceeds.**
The SYNC responder is `xiao_bridge_task`, `#[cfg(feature="xiaobridge")]` (`main.rs:727`); the coex
image is `bridge`+`xiao`, NOT `xiaobridge`, so no responder — Roy's 'opening…' on the coex image is
expected. It CANNOT fold into the coex image: `xiaobridge` requires `esp-println/no-op` (mutes EVERY
println, incl `log_health`/key-10) for the clean binary pipe, but the coex proof READS key-10 on that
**same single usb-serial-jtag CDC**. So coex-console-observation and USB-Android-clean-pipe are
**mutually exclusive on one CDC** → separate images (matches the earlier arch ruling: bridge-leg
validated apart from coex). A single unified image = real code work (health→UART while USB-CDC stays
the clean pipe, or a framed CDC multiplex) — a follow-up, not a flag.

## Queued (Roy directives, AFTER the coex proof)

- **Canonical base (MUST):** once coex-proven, pin the tn_base sha as the single linkable base ALL
  images derive from (ensemble-composition, no forks); record REFERENCE-IMPLEMENTATION in GH project
  reality2-ai #1 item **#19** (green cell + pinned sha + mechanism + inheritance + known gaps). File
  in DECISIONS. Build ONE base identically for D4/D5/XIAO; XIAO adds the bridge-leg (USB-Android)
  ensemble, not a fork. **DFR1195 = superset reference board** (coex + sensor + bridge).
  **base_digest RULED + verified:** composer ruled persona-EXCLUDED hash (my point confirmed); candidate
  **`ad9fc529d03ea1fdefd77d9c6c2437ecb509edd5798fd2618b61d9ccf1ced531`** = sha256(coex ELF, mask
  `[45192,45528)` zeroed) — hive independently recomputed, XIAO==D4 converge. Record with mask +
  method + provenance tuple (dfr1195-fw 97175901 / bridge,ble,benchsf7,baked_persona / coex.0722.1225 /
  fw_sha 0x6616A287). **Pins as canonical base on coex-proof PASS.**
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
