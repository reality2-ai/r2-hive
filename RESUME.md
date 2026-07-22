# RESUME — r2-hive

Updated 2026-07-22. `main` clean + pushed. Active work: tri-bearer coex proof. **bit0 ROOT-CAUSED = composer's
pump framing** (missing 2-byte length prefix) — board/firmware canon-compliant → C + v5 VALIDATED. v5
`23e17d1c` resident on XIAO; composer re-pumping v5 with fixed framing (if 0x25 greens, no run-6 flash).
**v6-DIAG BUILT + ATTESTED + HELD:** XIAO `2c5d41ef` from PINNED `36811c9b` (byte-identical) = e4031efd +
cocdiag CoC-rx instrumentation + fail-open Err. Feature set **B** (supervisor-ruled minimal-delta) =
`bridge,ble,benchsf7,baked_persona,loratcxo,xiao,cocdiag` — NO fakesensor (that was D4-flavored carry-over;
XIAO stays observer). Attest: persona baked @49552 hive `0x8C15B0C2` (collision-safe); C-in-binary
(`start_core1_run<start_second_core<16384>>`); cocdiag TOOK (4 `DIAG-RX` strings); masked `a8df6619…`.
**ARTIFACT HELD** — do NOT stage for flash without a new order (backup diag if the v5 re-pump fails to
green 0x25). Worktree re-dirtied AGAIN pre-checkout despite the hive-exclusive ruling (mutation source
still active, not a lane) — `git reset --hard` + byte-verify handled it; re-flagged. Key rulings in `DECISIONS.md` (D-20260721-01..03, D-20260722-01, R-20260722-01).

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
