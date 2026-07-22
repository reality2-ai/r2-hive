# RESUME — r2-hive

Updated 2026-07-22. `main` clean + pushed. **✅ TRI-BEARER COEX PROOF PASSES — CAMPAIGN CLOSED.** key-10
**`0x25`** (BLE bit0 | LoRa bit2 | Mesh bit5) SUSTAINED 41.6s contiguous / 7-of-7 HEALTH frames ≥10s on the
`bee0e996` bit5-keepalive images (XIAO `d12ddcc8` / D4 `d818ffda`); composer metal-verified, personas intact,
no NVS writes. Acceptance `hive:D-20260722-01` MET (`hive:R-20260722-04`). Arc: Fix C (core1 executor
isolation) → join-suppress cleanup + the owned metal-refuted desense hunt → `benchkeepalive` 4000 + densify
gave the LoRa/Mesh margin under the 8s window; bit0 via CoC pump. **Canonical-base pin DONE (Roy-confirmed 2026-07-23, `hive:D-20260723-01`):** base = `bee0e996`; per-board
base_digest table (XIAO masked `d884bba3…`, D4 masked `071b702d…`); landed on 3 surfaces — GH
`reality2-ai/r2-core#19` (comment 5050303127), hive DECISIONS, recipe registry (`alfred:~/coex-flash-recipe.txt`);
known-gap honesty (bit0 pump-driven, beacons unattributed, D4 BLE-C unproven). **Remaining post-proof items
stay Roy-gated + out-of-scope** (D5 provision, SEN0676 radar).
**BLEROLE slot (Roy GO 2026-07-23) — D4 initiator image, merge escalated to core:** blerole `c01c9db9` and
pinned coex `bee0e996` are SIBLINGS off `e4031efd`; **c01c9db9 LACKS all 3 coex fixes** (densify, join-suppress,
keepalive still 8000 `:3387`). **Ghost-removal (`3b7079f1` in c01c9db9) makes join-suppress LOAD-BEARING** — the
ghost is why the WiFi-join-scan desense was unreachable; removing it lets a real election fire join_provider →
scan → bit5 desense RE-OPENS unless 56d39498's no-op rides. Escalated the MERGE to core (never-edit-core +
reproducible provenance + owner resolves the ghost/join-suppress composition). Core did the merge: **`54a8a1f3`** (branch dfr1195-fw-blerole-coex, `merge --no-ff` of c01c9db9 onto
bee0e996) — my load-bearing interaction CONFIRMED on the merged tree (join-suppress rides, `DATA_PLANE_JOIN.signal`=0
→ ghost-removal doesn't re-open desense). **D4 INITIATOR BUILT + ATTESTED: `8f5c5701`** (from 54a8a1f3,
`bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive` + `DFR_ROLE_PATH=~/d4-initiator.role`); persona
`0xC434FAFC`, masked `0a8ad024…`, BAKED_ROLE_PROFILE=RPF1 b[6]=0x01 INITIATOR (role baked), RXDIAG=0. XIAO
unchanged (`d12ddcc8`). **FLASH PASS (supervisor 2026-07-23): role=initiator on metal, differential attest
VINDICATED (banner confirmed).** **Board-to-board BLOCKED at rbid→identity resolution** — D4 drops XIAO's NEG
as identity-less (v0.10 L3); = the deferred bit0 scaffold gap I flagged (`resolve_rbid_windowed` matches empty
`registry:&[]`). Core diagnosing (pre-provisioning gap vs scan-path code gap); possible rebuild, base stays
**54a8a1f3 lineage**. HELD — no hive build until core hands a sha. STANDBY.
**BUILD GOTCHA (owned + memory'd):** first builds gave `2804223c` = EMPTY role (derived acceptor mislabelled) —
the shared target's incremental cache kept a stale empty `BAKED_ROLE_PROFILE`; 5 targeted cache-busts failed,
only `rm -rf target` baked the env const. Role proven by the DIFFERENTIAL (`8f5c5701`≠`2804223c`) since the
const is const-folded (not raw-in-ELF). See [[env-baked-const-needs-full-clean]]. STANDBY.
**bit0 CLOSED on metal** via composer's pump prefix fix → 0x25 reachable; board canon-correct
(prefix-always ratified). **NEXT: Roy ruled DENSIFY + RE-RUN, gate UNSOFTENED** (denser real LoRa admits
so bit2 sustains within W≈8s, NOT a wider W). **D4 BUILD-ON-SHA STANDING ORDER (Roy pre-granted, no
round-trip):** when core's densify sha lands (e4031efd lineage), build D4 immediately + attest → supervisor
writes the flash grant on attestation. **CONTINGENT on core's runtime-knob gap-check** — if a knob densifies
cadence, NO build. **D4 spec pre-confirmed to supervisor:** persona `d4-persona.bin`→`0xC434FAFC`;
features `bridge,ble,benchsf7,baked_persona,fakesensor` (v4 D4 apiary set; fakesensor pulls
loratcxo/loraroute/otaengine; NO `xiao`, NO explicit loratcxo); table `d4-reflash-partitions-e0e49127.csv`;
attest = baked-persona `0xC434FAFC` + C-in-binary + fakesensor-took (apiary_bus_task) + loratcxo differential.
Still #d005/#d006 preflight (drain+confirm+pinned-sha+byte-clean) at build time.
**HIVE RADIO-DOMAIN desense — my WiFi-scan mechanism REFUTED ON METAL; root RE-OPEN (2026-07-22, critical
path).** Composer run-5b: XIAO RX-blind bursts (all-or-nothing, quantized 7/14/21/28s, ~29s under CoC,
ambient no-BLE 39-53%); D4-TX FALSIFIED (80 beats, 0 pauses). **My WiFi-join-scan root was REFUTED (metal positive-control + code):** join strings NEVER printed → the
handshake never reached join_provider. Root of the unreach (core traced its own state machine): on the
bench BOTH boards elect the GHOST `0x0DCADBF8` → neither is provider → WifiReq goes to a phantom → no
WifiOffer → join_provider never called. `bring_up_provider`-always-true is a real landmine but only reached
if a board WINS provider — the ghost gates it out one level up. OWNED (I + core both verified the chain
FROM join_provider, not that it's REACHED — a reachability miss; see [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]]).
**Consequence: `56d39498` "suppress M8c WiFi-join" (core landed on my mechanism) LIKELY DOES NOTHING for
bit5** — it suppresses a path metal proves is never taken; it's a harmless CLEANUP (signed off as such),
NOT the cure. **Leading root now (grounded): core0 executor-starvation overflowing the 10-deep esp_now RX
queue** (`RECEIVE_QUEUE_SIZE=10`, `esp_now/mod.rs:33`; drop at `:890`) — espnow_task (core0 `:786`) +
io_task (core0) both on the one executor; 10 frames @2s HB = 20s buffer → a ~20-27s core0 stall overflows
→ drop-burst = the 27s blind span; Fix-C precedent fits. **v7-DIAG co-designed (core lands on 56d39498):**
C_recv (espnow_task.rx `receive_async()` count) vs C_admit (MESH_ADMIT). **Reachability correction:** the
esp-radio `:890` overflow is a SILENT `pop_front()` (drop-oldest ring, RECEIVE_QUEUE_SIZE=10, no counter,
private STATE) → the raw-callback count + drop-count are NOT firmware-reachable; C_recv is the lowest
readable RX point. **Split (with composer's ch1 SNIFF as radio ground-truth, sniff FIRST):** on-air +
C_recv gaps + NO burst → radio-level loss = COEX ARBITRATION (a); on-air + C_recv ≤10 catch-up burst on
resume → EXECUTOR STARVATION (b); C_recv steady + C_admit gaps → downstream. **Core0-occupancy scan (my
pre-work): NO hard multi-second blocker found** (runner.run()+accept async-yield, io_task select-loop,
SX1262 busy-spin on core1 post-Fix-C, flash event-not-ambient) → **tilts toward (a) coex-arbitration**
(esp coex arbiter parks ESP-NOW RX at PHY level, below embassy) over (b). Sniff+counters decide. **v7-DIAG PUSHED + hive-VERIFIED + PRIMED:
`78177f50`** (= 56d39498 + `rxdiag` counters; child, single commit) — `ESPNOW_RECV_CT:236`/`MESH_ADMIT_CT:238`
atomics, C_recv at `:6077` (earliest drain), C_admit `:6119`, "RXDIAG C_recv= C_admit=" print `:1849` on
the ~2s beat; join-suppress + cadence intact. **Build sets (pre-confirmed to core), on an XIAO-RX sniff
verdict + supervisor order:** XIAO `bridge,ble,benchsf7,baked_persona,loratcxo,xiao,rxdiag` (persona
`0x8C15B0C2`); D4 `bridge,ble,benchsf7,baked_persona,fakesensor,loratcxo,rxdiag` (persona `0xC434FAFC`);
both from 78177f50, #d005/#d006 preflight + full attest. **D4 `bb6565e6` (83a2a17f, cadence-only) attested
— do NOT flash as a bit5 fix; HELD.** LESSON: [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]].
**SEQUENCE LOCKED (all lanes):** Roy fires ch1 sniff → core counters authored (`78177f50`) → hive builds
only on XIAO-RX verdict. Three signatures pre-registered in the prediction ledger.
**BOTH v7-DIAG IMAGES PRE-BUILT + ATTESTED (2026-07-22, supervisor order — Roy offline ~2h; FLASH still
hard-gated) — HELD-NOT-FLASHABLE:** from PINNED `78177f50` (byte-identical). XIAO
`8a6dea89e9d2a45d…` (persona `0x8C15B0C2` @45024, RXDIAG-print took, lora_route_task, C-in-binary, no
apiary_bus_task=observer), masked `c3ef1aa6…`. D4 `3b412e548f3dfe3f…` (persona `0xC434FAFC` @45832,
RXDIAG took, apiary_bus_task=fakesensor took, lora_route_task, C), masked `7fea3ea1…`. Both BUILD_ID
`coex.v7.0722`, table `d4-reflash-partitions-e0e49127.csv`. **Flash ONLY on sniff=XIAO-RX + Roy grant;
D4-TX-side → archive.** Reported to supervisor + composer.
**bit5 FIX inventory (2026-07-22, #d007, no build):** v7-diag is DEAD; the fix = a keepalive-override role +
maybe densify. FLASHED images: **v4 D4 `f2a32e20`** (src `aa939299`, `bridge,ble,benchsf7,baked_persona,fakesensor`)
and **v5 XIAO `23e17d1c`** (src `e4031efd`, `…loratcxo,xiao`) — **both densify-ABSENT** (both are ancestors of
`83a2a17f`). **Role mechanism (CORRECTED — my first read was WRONG + brick-unsafe):** the role IS BAKED. Committed
build.rs @56d39498 (81 lines; I'd relied on the 44-line DIRTY-TREE-stripped version) bakes `BAKED_ROLE_PROFILE`
from `DFR_ROLE_PATH` (`:47/:67/:76`) under the `baked_persona` feature, alongside the persona. **dfr1195 has NO
role partition on the default table → a raw NVS `0x17000` write lands INSIDE the app + CORRUPTS it (D4 brick
incident, build.rs:41). So a role override MUST be BAKED (rebuild), NEVER NVS-0x17000-flashed.** My earlier
"keepalive = NVS flash, no rebuild" was brick-unsafe (owned; wrong instrument = Cargo features + stale
dirty-tree build.rs; 2nd instance — see [[positive-control-the-tree-not-just-the-tool]]). **RATIFIED FIX
(supervisor): `benchkeepalive` FEATURE (const 8000→4000, core-landing) + rebuild; NO role blob** (a blob
freezes the derive tail = mirror-error hazard; benchkeepalive is uniform). **BASE = `78177f50`-branch**
(core proved densify `83a2a17f` is its ANCESTOR → 78177f50 = densify+join-suppress+health-overflow+rxdiag;
my earlier "use 56d39498, not 78177f50" is SUPERSEDED — **rxdiag rides as free observability**, if bit5
stays dark the counters are already on metal). NVS `0x17000` role-flash vs baked_persona = NO-OP (core-confirmed: baked arm compiles out the read), never
stage. **#d005 ORDER EXECUTED — bit5-keepalive images BUILT + ATTESTED (2026-07-22), for #d007 flash.**
Base = `bee0e996` (branch dfr1195-fw-bit5-keepalive, off 56d39498; = densify+join-suppress+benchkeepalive
8000→4000, NO rxdiag — composer's decider killed rxdiag: bit5 ever-lit ×1033, 0x25 ×3). Byte-identical.
**XIAO `d12ddcc8…`** (`…loratcxo,xiao,benchkeepalive`, persona `0x8C15B0C2` @44984, no apiary_bus_task,
RXDIAG=0, C-in-binary), masked `d884bba3…`. **D4 `d818ffda…`** (`…fakesensor,benchkeepalive`, persona
`0xC434FAFC` @45796, apiary_bus_task, RXDIAG=0, C), masked `071b702d…`. No DFR_ROLE_PATH (derived-role
fallback = current behavior), no 0x12000/0x17000 writes, table `d4-reflash-partitions-e0e49127.csv`
(app@0x20000). benchkeepalive cfg-const in resolve_role_profile (`:3360`, core-verified). Delivered to
supervisor + composer for two-party SHA verify; composer flashes under #d007.
**v6-DIAG `2c5d41ef` = PERMANENT STAND-DOWN** (framing root proven on metal; archived
`alfred:~/xiao-v6diag-36811c9b-2c5d41ef.elf`, NEVER flash). It was XIAO from PINNED `36811c9b`
(byte-identical), feature set **B** (minimal-delta, no fakesensor); fully attested (persona `0x8C15B0C2`,
C-in-binary `start_core1_run<start_second_core<16384>>`, cocdiag TOOK 4×`DIAG-RX`, masked `a8df6619…`).
**ARTIFACT HELD → LIKELY STAND-DOWN.** bit0 root refined (core, corrections owned): **double length-framing**
at the CoC boundary — this BlueZ SEQPACKET is RAW-PDU passthrough (no sdu_len add/strip); rx.receive
returned `Ok(n=1)`, serve_coc `n<2` silent-dropped. FIX = composer pump sends SDU `03 00 01 00 41`
(sdu_len 3 + R2 [01 00 41]) → `Ok(n=3)` → stamp → bit0 → 0x25. **PUMP-SIDE, no hive/firmware change.**
Canon: R2-BLE-CONFORMANCE CT-L2CAP vectors + §6.4 (normative) beat MANUFACTURER-GUIDE:241 (informative) —
bare SEQPACKET was conformant; **prefix-always RATIFIED by Roy** (link-level, both modes, reject-bare) →
dfr1195 already prefix-always (serve_coc sends+parses) so board is now canon-CORRECT, NO core change. v6
flashes ONLY if composer's `coc-sdulen.py` re-test on resident v5 fails to stamp — and the **radio-domain
branch is NEAR-EXCLUDED** (btmon proved 6 inbound credit INDs = 6 app-drains reached the host stack;
rx.receive returns `Ok(n<2)`), so "neither host.rs:422 nor DIAG-RX" is very unlikely. C + v5 VALIDATED
(bit0 never a firmware/dual-core bug). Worktree re-dirtied AGAIN pre-checkout despite the hive-exclusive ruling
(mutation source still active) — `git reset --hard` + byte-verify handled it; re-flagged. Key rulings in `DECISIONS.md` (D-20260721-01..03, D-20260722-01, R-20260722-01).

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

**v4 BUILT + verified + handed (2026-07-22), awaiting Roy grant + reflash.** Core landed the persistent
CoC listener at `aa939299` (RUN-3 root: the observer Scanner starved `peripheral.advertise()` —
esp-radio can't advertise+scan at once; `:3884` never printed; NOT the NEG loop/ghost). Fix =
default-off `bleobserver`; default/coex path = advertiser + always-pending L2CAP acceptor as the sole
BLE role (`:4048` join3). #4 answered: no coex consumer needs the observer scan (R2ScanHandler only
under `bleobserver`; engine uses the synthetic roster). aa939299 also carries the serve_coc stamp
(934426d5) + LoRa beacon-admit (0b749eb3). **v4 images** (BUILD_ID `coex.0722.1411`, fw_sha
`0x586622AE`): D4 (sensor+emitter) `f2a32e20` (`+fakesensor`→apiary_bus_task, hive_id `0xC434FAFC`,
persona@45728, masked `2259fb22`); XIAO (observer) `7e7cd1e3` (v2-identical set, hive_id `0x8C15B0C2`,
persona@44868, masked `f2a11e00`). **v1-guard verified on the binaries:** loratcxo differential DIFFER
(took, not dropped by the fakesensor swap); D4 `apiary_bus_task` present (3 syms). Full per-board lists
published (supervisor grant + composer provenance). Supersedes v1/v2/v3 (do-not-flash).

**Run-4 result: `:3884 BEACON adv up` STILL SILENT** — the listener fix didn't unblock advertise.
**v1-class binary check (supervisor asked) — DONE, decisive:** the observer scanner is GENUINELY
compiled out of shipped v4. Exact nm: shipped OFF = `run_with_handler<DummyHandler>` + join3 only;
**R2ScanHandler / Join4 / Scanner ABSENT**; bleobserver-ON control has them present; OFF≠ON differential
confirms the gate is active. (False-positive trap flagged: a raw substring COUNT showed 2 in OFF, but
those are `DummyHandler` drop symbols, not R2ScanHandler — the demangled NAME is decisive, same
verify-what-it-instantiates discipline as loratcxo.) **So scan-starvation is REFUTED for the shipped
image → advertise is blocked BEYOND the scan** (advertise() HANGS — no "NEG provider adv ERR" print).
Advertise blocked beyond the scan.
**Hive coex read (my domain) — REFRAME: likely EXECUTOR starvation, not 2.4GHz radio coex.** Evidence:
(1) coex build is STA with `wifi_task` idle (waits on `DATA_PLANE_JOIN` `:7580`, never fires) — WiFi
not the contender; (2) esp-radio 0.18 exposes NO runtime BLE-adv-priority knob (coex-config fix
unavailable); (3) advertise-with-coex was M3-verified (`:3735`) = not a hard limit; (4) the
cocbench(advertises)-vs-coex(silent) differentiator is **`loraroute`**, and LoRa is sub-GHz (no 2.4GHz
BLE contention); (5) `:7244` documents the trouble-host BLE runner starved on the ONE esp-rtos executor
(`:327`) by a blocking op → `lora_route_task`'s blocking SX1262 SPI hogs it → `runner.run()` never polls
→ advertise never STARTS (silent, both v3+v4). Composer's `:3879`: advertise HANGS (no ERR).
**POSITIVE CONTROL found (refutes the ESP-NOW-radio hypothesis, core's + mine-earlier):** cocbench is
NOT BLE-alone — `cocbench=ble,dev`, the espnow gate `(not-loraroute OR bridge)` passes → espnow_task
spawned → cocbench = BLE + continuous ESP-NOW RX + it ADVERTISES. So ESP-NOW-RX + advertise coexist
today → ESP-NOW is not the starver. The sole cocbench→coex differentiator is **`loraroute`** (sub-GHz,
no 2.4GHz contention) → **executor starvation** of the trouble-host runner by `lora_route_task`'s
blocking SX1262 SPI (`:7244` documents it). **Isolation diag built + nm-verified:**
`~/d4-DIAG-noespnow-coexdiag.0722.1444.elf` sha `e2bba673` = `ble,loraroute,benchsf7,loratcxo,
baked_persona` (espnow_task=0 syms, lora_route_task=3) → BLE+LoRa, no ESP-NOW. Flash + watch `:3884`:
silent = LoRa-executor confirmed (predicted); prints = LoRa+ESP-NOW combination (still executor, not
ESP-NOW-radio, since cocbench proves ESP-NOW alone is fine). Core ACCEPTED the executor-starvation hypothesis (its
ESP-NOW guess refuted by the cocbench control, owned); composer's `:3879` = HANG confirms it.
**Confirm image built + nm-verified:** `~/xiao-DIAG-noloraroute-coexdiag-noloraroute.0722.1451.elf` sha
`9e0b76de` = `ble,benchsf7,baked_persona,xiao` (lora_route_task=0, espnow_task=3, ble_task=7) = XIAO
BLE+ESP-NOW, no LoRa. XIAO-only test; D4 stays v4. `:3884` prints → LoRa-executor locked + CoC pump
validates the listener/bit0. Handed for grant.
**esp-rtos fix read (hive advises, core lands):** splitting join3 across executors is FORBIDDEN
(peripheral/central/runner share one `stack.build()` borrow) — move the whole `ble_task` instead.
esp-rtos 0.3.0 has NO esp-hal InterruptExecutor (it's threads + a 2nd-core main thread); the
thread/2nd-core move risks BLE-controller affinity. **Hive rec: fix the block at the source.**
Core located the block at instruction level: `r2-sx1262::wait_busy()` (`lib.rs:322-330`) busy-spins
`while busy.is_high() { delay.delay_us(20) }` after every SX1262 command; sync `lora.service()` blocks
the async executor. **A-vs-B feasibility answered (hive, the blocking input for v5):**
- **Fix A (split trouble-host runner to a priority InterruptExecutor): INFEASIBLE** — (1) the Stack
  can't be shared across executors (peripheral/central/runner share one `stack.build()` borrow); (2)
  BleConnector unsafe/risky in interrupt context; (3) esp-rtos 0.3.0 exposes NO InterruptExecutor
  (threads/2nd-core only). Dead end; goes to backlog as a general pattern.
- **Fix B (async r2-sx1262): root cure BUT fleet-wide.** r2-sx1262 is SHARED (RAK thumbv7em + DFR
  xtensa + nrf54-lr2021), generic over sync `DelayNs`+`InputPin` → async = embedded-hal→async bounds +
  sync→async ripple through EVERY consumer, cross-runtime. Big coordinated migration → backlog, not
  v5-quick.
**FIX = C (core-RATIFIED), = A-prime renamed: move `lora_route_task` to an esp-rtos core1 executor.**
LoRa stays sync (RAK/LR2021 untouched); isolates the WHOLE LoRa task so it fixes advertise-START AND
CoC-connect AND ongoing runner-starvation, mechanism-agnostic (a startup-sequencing fix would fix only
advertise-start, not ongoing CoC — my `:7244` precedent). **A = dead** — trouble-host runner shares one
`stack.build()` borrow with peripheral/central + BleConnector unsafe in ISR. (CORRECTION, owned:
esp-rtos 0.3.0 DOES have `embassy::InterruptExecutor` `mod.rs:310`; the earlier "no InterruptExecutor"
claim was wrong. A's death is the Stack-sharing ground, NOT executor absence — and a core0
InterruptExecutor for LoRa would PREEMPT the runner = worse; the blocker MUST cross to a different CORE
→ C.) **B (async r2-sx1262) = fleet migration backlog** (core owns the graph: DFR
xtensa + nrf54-lr2021 + rak4630 + r2-ble, ALL sync embedded-hal 1.0 → async ripples cross-runtime;
needed for C6 single-core portability).

**Mechanism (core, code-grounded):** advertise HANGS FOREVER while the loop yields 5ms → NOT the
ongoing loop (that would let advertise eventually complete) → a PERMANENT STARTUP break: `LoRaTransport::new`
SX1262 `configure` (`:5386`, hw_reset 1.2ms + 5ms calibrate) collides with BLE advertise-enable →
dropped HCI response → advertise waits forever. RX already event-driven DIO1 (`:5366`). (My Fix-B
premise "async removes a long spin" was WRONG — driver has no long block; owned; C is mechanism-agnostic
so it holds regardless.)

**Hive VERIFIED C's data layer is SAFE (grounded, dfr1195 main.rs):** (1) `LoRaTransport::new` owned
WHOLLY inside `lora_route_task` (`:5391`) — no cross-core SX1262 handle share; (2) `lora_spi` is a
DEDICATED `Spi` (`:847`, separate from the display bus `:796`); `LoraRadioTy` (`:5041`) =
`ExclusiveDevice<Spi<Blocking>,Output,Delay>` all Send + captured `[u8;32]`/u32 → task future is Send →
spawns on a 2nd-core executor, no bound violation; (3) EVERY core0↔lora static already
`CriticalSectionRawMutex` (DATA_RX/DATA_TX_LORA/DATA_TX `:4588-4597` + LORA/BLE/MESH_ADMIT_S atomics
`:224-226`) = multicore-safe by construction. Residual (not blockers): CS now taken cross-core (bounded
stall, NOT the starve); confirm esp-rtos 0.3.0 embassy time-driver is multicore. **C ratified for v5,
B backlog, A dead.**

**GATE — mostly GREEN (supervisor, under Roy's live grant): `9e0b76de` flashed on XIAO (sha verified
both ends).** First-half PASSED (`:3884` adv prints without `lora_route_task` → loraroute IS the
blocker). 2nd-half listener chain PASSED — composer pumped the TRUE addr (after catching core's
byte-reversed boot-addr println, fixed `e6ae9cad`; runs 3-4 targeted MIRROR addrs so the listener was
never validly tested before): `CoC up` + **60 PDUs served + clean close = listener+serve chain
VERIFIED**. Only the **bit0 numeric read is in flight** (re-run with a 6s pump — CoC serving starves the
health printer = a **3rd starvation instance, BLE-side**; supervisor relayed to core for a serve-loop
yield in v5). bit0 lights → whole BLE inbound chain validated minus LoRa → C (keeps that core0 chain) is
sufficient → core commits the dual-core spike.
**XIAO v5 BUILT + FULLY ATTESTED + HANDED (2026-07-22) — awaiting Roy-granted flash.** Roy UPGRADED the
grant: "Go — v5 on XIAO" (supersedes the C-only grant); supervisor writes it, composer flashes, XIAO only
(D4 stays v4 control + apiary source). **The earlier C-only `455ae47a` (from 9c08c89f) is PARKED —
do-not-flash** (superseded target). v5 built from PINNED `e4031efd` (detached; tree BYTE-IDENTICAL,
`git diff e4031efd` empty) = C(9c08c89f)+println(e6ae9cad)+health-buffer 96→160(105eb4aa)+non-silent-overflow(e4031efd).
**ELF sha256 `23e17d1c375b49f8270c5f83c80e62c4a8e05f6e8e0fb170927397fbfc2522b2`**, alfred
`~/xiao-v5-e4031efd-23e17d1c.elf`; BUILD_ID `coex.v5.0722`; features
`bridge,ble,benchsf7,baked_persona,loratcxo,xiao`; table `d4-reflash-partitions-e0e49127.csv` (app@0x20000).
**Persona attested on BAKED bytes ex-ELF @44984/336B (baked==input):** tg_hash `0x6E31DEC6` / hive_id
**`0x8C15B0C2` = XIAO, collision-safe** (≠ D4 `0xC434FAFC`); recompute agrees; baked = no 0x12000 write.
**C PROVEN IN-BINARY (nm):** `start_core1_run::<start_second_core_with_stack_guard_offset<16384,__embassy_main_task…>>`
+ `lora_route_task` + `espnow_task`; R2ScanHandler absent (observer gated). **loratcxo DIFFERENTIAL:**
A(with)=`23e17d1c` ≠ B(without)=`50178af9` → TOOK. masked base_digest `d25f3e40…`. **v5 = health survives
CoC (buffer 160) → bit0/0x25 READABLE this flash.** Run 5 = :3884-LoRa-on-core1 + cross-core LoRa RX (D4
apiary) + CoC listener + HEALTH-survives-CoC + real 0x25 watch.
**#d005 build gate (Roy standing, ledger):** before ANY flashable build — (1) DRAIN inbox for
supersedes, (2) explicit current supervisor order + pinned sha, (3) clean detached checkout, tree-state
byte-verified. Applied on this build. Pinned-sha refusal of ambient HEAD is RIGHT (kept). **Worktree-dirt
SOURCE FOUND:** `~/dfr1195-fw-build` reflog shows the firmware commits `934426d5→105eb4aa→e4031efd` were
AUTHORED there ("commit:" entries) = it's a STANDALONE clone where CORE lands, so the re-dirty = core
working live in the same clone I build in. **Supervisor RULED `~/dfr1195-fw-build` hive-exclusive for
WRITES** (2026-07-22); my `git reset --hard <sha>` + byte-verify preflight stays (defense in depth,
mutation treated as live hazard). Stashes `hive-preCbuild` (main.rs −1213 + r2-core/cbor.rs +
tools/r2-bootstrap + 23 files) + `hive-preV5build` (33-line). **RESOLVED:** core inspected both read-only
— NEITHER is core's (both destructive/stale: {0}=inverse of the health fix = a regression, {1}=mass
deletion of already-committed content); core's fw is all committed+pushed, its own worktree
`~/Development/R2/dfr1195-fw-wt` clean at tip. Both DROPPED (guarded by message). Only `rak-ota-park`
(stash, rak lane — not hive's) remains, left untouched. Worktree clean at e4031efd. Earlier "no baked_persona feature" was a DIRTY-TREE grep read as source
fact (composer caught via `git show HEAD:`); owned — [[positive-control-the-tree-not-just-the-tool]].
esp GCC via `~/Development/homelab/export-esp.sh` (linker off-PATH in non-interactive ssh).

**FIX C LANDED + xtensa-compile-verified by core = dfr1195-fw `9c08c89f`** (hive source-verified the C
block clean: `lora_route_task` spawned ONLY in the core1 `esp_rtos::embassy::Executor` at `main.rs:893`
under `#[cfg(loraroute)]`; old core0 spawn GONE; non-loraroute `lora_task` stays core0; order
`esp_rtos::start()` `:406` before `start_second_core`; args Send). Stack path was MY paraphrase error —
`esp_hal::system::Stack`, not `esp_rtos::Stack` (core caught + corrected; owned). **Early C-only flash
RECOMMENDED (hive call) + grant routed to supervisor** — decisive falsifier for the highest-risk change
(virgin dual-core): `:3884` prints WITH loraroute present = first metal test of C's real mechanism;
orthogonal to the health defect (`:3884` is a startup print, pre-CoC); same board+laptop-CoC captures
core's HELD DATA_RX-flood datum. v5 = `9c08c89f` + `e6ae9cad` (println) + an OWED health-emitter-survival
fix (core traced to `io_task` select(Timer50ms, DATA_RX.receive()) starving under a DATA_RX flood; held
for the metal datum, no 4th guess). **C-image build BLOCKED on the persona-bake recipe:** on alfred
`~/dfr1195-fw-build@9c08c89f`, positive-controlled greps show NO `baked_persona` cargo feature (my v4
recipe mislabelled it), no `include_bytes`, no baked ELF symbol — the per-board persona is a POST-BUILD
inject (persona@45728 + masked base_digest) I can't reproduce from the worktree alone. Asked composer
(provenance owner, reproduced v4 3 ways) for the exact reproducible build+inject recipe; will NOT
blind-rebuild the D4 `0x12000` persona-offset brick path. Grant-fetch runs parallel (no net delay).
**Dual-core spawn pattern HANDED to core (grounded in esp-rtos-0.3.0 source):** `esp_rtos::start_second_core::<STACK>(p.CPU_CTRL, sw_int.software_interrupt1, stack, move|| Executor::run(spawn lora_route_task))` — int1+CPU_CTRL FREE (main uses only int0 `:406`); closure is `FnOnce+Send` (args Send-verified); ORDER = `esp_rtos::start()` then `start_second_core`; move ONLY lora_route_task, delete the core0 spawn `:869`; `esp_rtos::embassy::Executor` (NOT esp-hal-embassy). **Canon-cite rule (Roy standing):** grep specs +
cite `DOC §n` before architecture/contract findings — my "r2-sx1262 fleet-shared" was canon
(unified-architecture). See [[cite-canon-before-claiming-a-finding]].

**Bundle plan (supervisor):** v5-fix (C, core lands) + fallback in ONE Roy grant, v5 first.
`9e0b76de` (XIAO confirm) is **xiao-persona** (hive_id `0x8C15B0C2`) — collision-free, flashable.
`e2bba673` (d4-persona) is **DEAD for XIAO** per Roy's ruling ("no two hives same hive_id"; §16.6
rejects dup at JOIN, baking bypasses join → build discipline) — rebuild xiao-persona ONLY if/when the
fallback is needed. Standing rule: every image for board X carries X's persona, no diagnostic
exceptions. The drop-loraroute
positive image (`9e0b76de`) order was RETRACTED (v5-fix is the positive test). Acceptance still:
bit0 → `0x25`.

**Prior (v2) result:** XIAO key-10 = `0x24` = bit2 LoRa | bit5
WifiMesh CONCURRENT in one frame. `loratcxo`/`xiao` fix **proven** — LoRa+ESP-NOW coex on the S3 is
real. bit0 (BLE) was missing because **`serve_coc` (coex inbound handler) never stamped `BLE_ADMIT_S`**
— only blemesh's `serve_data_coc` did (core's find, the actual primary root; my scaffold trace was a
real *secondary*). **v3** = core stamped `serve_coc` (`:4158`) + boot-addr print → dfr1195-fw
`934426d5`. Built v3: D4 `47ad5200`@45284, XIAO `5cc8d835`@45304, BUILD_ID `coex.0722.1337`, fw_sha
`0x54B574C7`; reflashed. **v3 metal result:** LoRa+ESP-NOW admit healthy (key-10 `0x04`/`0x20`
cycling); boot-addr println works (exposed runs 1-2 never touched a real board). **But bit0 still
dark — the stamp is UNREACHABLE: L2CAP refuses PSM 0x00D2 (ECONNREFUSED), no persistent CoC listener.**
Source-corrected composer's "NEG-role-gated" read: `COC_PSM=R2_PSM=0x00D2` (`:4327`) matches the pump,
and the accept (`:3912`→`serve_coc:3928`) is UNCONDITIONAL in the `advertise_beacon=true` branch
(`:3845`), NOT NEG-gated — the stamp is correctly placed. The gap: the **sequential
advertise→accept→serve loop isn't a persistent listener** (holds one conn at a time; an inbound L2CAP
open between iterations / while the NEG engine holds the single slot gets refused). **Core's
persistent-listener restructure (dedicated always-pending 0x00D2 acceptor, independent of advertise/NEG)
is the right fix — core edit, escalation correct.** Core landed the v4 SECONDARY (LoRa beacon-stamp
un-gate, my finding, confirmed) at `0b749eb3`. Listener (primary) HELD on ONE composer serial line
(after `:3884 BEACON adv up`: `:3914 accept ERR` / `:3919 CoC up` / SILENCE? → acceptor-never-pending
[persistent acceptor] vs resources/coex refuse). Core hands the v4 sha; **hive does NOT build v4 until
then.** Core also owns raising the ESP-NOW HB-TX interval (no tty).
Secondary — sustained-`0x25` cadence (supervisor:
prefer denser real admits, NOT a wider W; W-widen weakens the truthful gate, Roy-visible). Traced:
stamps are faithful (per-RX, no dedup). (1) FIXABLE: LoRa §8.1 beacon-branch `LORA_ADMIT` stamp
(`:5552`) is `#[cfg(xiaobridge)]`-only → coex misses beacon admits; un-gate it (core, v4). (2) HARD
FLOOR: LoRa emit is airtime-duty-bound (SF7 nbrs=0 ~10% → ~1/16-30s) → even beacon-stamped, LoRa
admit ~30s ≫ W=8s; sustained-continuous LoRa needs a DENSE bench LoRa data stream (hive drives) or
nbrs>0, not a stamp change. (3) ESP-NOW ~45s — core CORRECTED "raise the interval": NO safe knob.
`HB_PERIOD_MS=2000` (`:1402`) is the load-bearing conductor-PLL/PCO period (must divide the 60s window),
MUST NOT shorten for a display. But the HB already broadcasts on ESP-NOW every 2s
(io_task→DATA_TX→espnow, `:1677`) — 2s ≪ W=8s. My neighbour-learning-gate hypothesis was REFUTED by
core: `can_hear` is a no-op on the coex set (`#[cfg(all(not(meshmask),not(routetest)))]`→true, `:4857`),
`MESH_ADMIT` stamps every recv. So the 45s is a **RECEPTION/coex-airtime** question (is XIAO actually
receiving D4's 2s HB over the air?) — possibly the 2.4GHz coex contention the proof exists to surface
(present≠reached one layer down: emitted@2s ≠ received@2s under BLE+WiFi-coex+LoRa-RX desense). Metal
resolves: XIAO ESP-NOW recv cadence vs D4 TX cadence (OTA_ACTIVE `:5973` / coex TX relief). Not cadence. Density
lever = D4 `fakesensor`; if cadences still don't fit W=8s WITH fakesensor → per-bearer W (Roy-visible),
NOT spamming the conductor. Core confirms §4.3 LoRa floor + per-bearer-W once bit0 lights.
v4 = core listener + beacon-stamp; hive drives dense LoRa traffic. **LoRa-floor RULING (supervisor): option (a)
— dense real apiary data** (#d003 sine sensor ~1.5/s), W=8s stays honest. Build gap found:
`apiary_bus_task` is `#[cfg(fakesensor)]` (`:715`); the coex `bridge,ble` build never emits apiary →
**v4 D4 must add `fakesensor`** (pulls loratcxo/loraroute; espnow stays via bridge; apiary replaces
engine_bus_task); XIAO stays observer. Duty math (hive calc): SF7/29B ToA ≈87ms → 10% duty nbrs=0 →
max ≈1.15/s (~870ms) ≪ W=8s → bit2 sustains; verifying exact ToA + §4.3 throttle with core (if max-legal
< 1/8s the floor is real → escalate per-bearer W, Roy-visible). **Honest claim scope:** PASS =
"0x25 sustained under bench apiary traffic", NOT "idle LoRa sustains 8s" (field-idle bit2 flickers =
known+accepted). nbrs>0 (post-D5/#d004) = free bonus, not a dependency.
**v4 build guard (v1-lesson, divergent sets):** D4 = `bridge,ble,benchsf7,baked_persona,fakesensor`
(cargo-tree-confirmed: fakesensor pulls loratcxo+loraroute; NOT dropped); XIAO =
`bridge,ble,benchsf7,baked_persona,loratcxo,xiao` (v2-identical, no regression). On hand-off: publish
the FULL per-board list (explicit + pulled) with the shas for the grant + composer provenance, and
DIFFERENTIAL-verify loratcxo compiled in the binary (not just the def).

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
- **BLE bit0 defect (deferred, D4-suffices) — root corrected by core; RAM bump DROPPED:** blocks the
  full `0x25`. NOT the slot sizing (`HostResources<_,1,1>` suffices with 1-dials/1-accepts). Real root:
  (BUG1) provider election hardcoded to the M8b scaffold `M7_PROVIDER_HIVE=0x0dcadbf8` (`:4337`) —
  neither coex board matches → both non-providers → both inject the ghost (`:3833`) → both JOINER →
  both `central.connect` an absent board → endless retry, nobody accepts. (BUG2) joiner dials a
  hive-derived addr `[hive_bytes,0x52,0xC0]` (`:4030`), incompatible with the now-HWRNG-random BLE
  addr (`:3768`) — must dial the SCANNED BdAddr. **Fix split:** core = engine election + role over the
  real pair (lowest hive XIAO=provider, D4=joiner) + retire M7; **hive = scan-address plumb (DESIGN, core
  lands):** SCAN_OBS today carries only hive_id (`:4449` `(u32,bool,u8,Option<u8>)`), no BdAddr — a NEW
  capture (grab addr at `R2ScanHandler`, widen `SCAN_OBS`/`push_scan_obs`, joiner dials scanned addr,
  drop the 3 synthetic `push_scan_obs`). Boot-addr print landed (v3). **Scanner/rbid fork (answered):**
  the coex board CANNOT resolve peer rbid→hive today — `resolve_rbid_windowed` matches `registry:&[]`
  (empty, `:4033`); hk alone insufficient (no co-member roster). Role-fix path DECIDED (core): (ii)
  dev-gated deterministic `BENCH_ADDR` (`:4342` precedent) for the deferred bench fix; (i) scan-learned
  addr = field follow-up (needs co-member roster, likely spec-first). **Security guardrail (core owns):**
  prod provably HWRNG-random; the deterministic addr strictly behind a dev/bench feature + a
  compile-time assert `prod != deterministic`; no identity-derived addr ever in prod (§7.4.0). Core
  lands CORE (retire M7 + real-pair lowest-live-id election, XIAO=provider/D4=joiner) + the (ii) addr;
  hive hands the scan-plumb design. ALL PARKED — contingent on v3 bit0. The *primary* bit0 root was the
  missing `serve_coc` stamp (fixed in v3); this scaffold is the *secondary* board-to-board path.
- **RAK tx_power −9dBm** (30cm; as923_nz default +20 saturates RX) — a core change to rak
  `lora_leaf_config:1219`. **AGENTS.md doc-drift:** cites `docs/dfr1195-partitions.csv` (older); build
  uses `platforms/dfr1195/partitions.csv` (r2cfg) — both app@0x20000; recommend updating.
