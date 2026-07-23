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
`0xC434FAFC`, masked `0a8ad024…`, BAKED_ROLE_PROFILE=RPF1 b[6]=0x01 INITIATOR (role baked), RXDIAG=0. **FLASH PASS (2026-07-23): role=initiator on metal, differential attest VINDICATED.**
Board-to-board hit the v0.10 L3 rbid→identity gap (D4 dropped XIAO's NEG as identity-less = the bit0 scaffold
gap I flagged, `resolve_rbid_windowed` empty `registry:&[]`). **CORE FIXED IT → `3ed2f818`** (initiator
R2ScanHandler now carries the keyed member registry, `main.rs:4342` `derive_beacon_session_key` per
KNOWN_HIVE_IDS≠self → SCAN_RESOLVED → L3 admits; mirrors LoRa resolver :5668). Iter2 `ca00c094` (from 3ed2f818)
flashed + retested → **still bit0-dark: enumerated LIST gap** — the registry loop iterated `KNOWN_HIVE_IDS`
which did NOT contain XIAO's `0x8C15B0C2` (heard rbid 55ca UNRESOLVED → NEG-CTRL-DROP). Core fixed →
**`ede6ccf3`** (main.rs:4345 `KNOWN_HIVE_IDS.iter().chain(once(&BLE_PEER_HIVE))`, `BLE_PEER_HIVE:4612 =
0x8C15_B0C2`; + falsifier print :4363 "BLE resolver expects hive {h}->rbid"). **D4 INITIATOR iter3 REBUILT +
FULLY ATTESTED: `ef26d7d0…`** (BUILD_ID coex.d4init3.0723, from ede6ccf3 + DFR_ROLE_PATH; role b[6]=1
initiator; DIFFERENTIAL ef26d7d0 ≠ empty 1e58ff74, both from-scratch = clean; persona 0xC434FAFC baked==input
0ad4a84d @46596 unique; masked base_digest `e13fa273…` mask [46596,46932); C-in-binary core1 + lora_route_task
+ espnow_task; fakesensor apiary×3, RXDIAG=0; list-gap fix + falsifier string positive-controlled in binary).
Iter3 ef26d7d0 superseded by **iter-4a
`9e9ddb35…`** (from `a592ae70`, (a) fix: capture GATED on SCAN_RESOLVED :4419/:4681 + resolved-hive CoC label
:4635 — no capture against a stale/empty registry; diff vs ede6ccf3 = 33ins/22del main.rs only). Full attest:
b[6]=1 initiator; differential 9e9ddb35 ≠ empty 78c15133 (both from-scratch); persona 0xC434FAFC baked==input
0ad4a84d @46612 unique; masked base_digest `9463a4a7…` mask [46612,46948); C-in-binary core1 + lora_route_task
+ espnow_task; apiary×3; RXDIAG=0; falsifier string present; (a) fix positive-controlled pre-build.
iter-4a/9e9ddb35 superseded by **iter-4b PAIR (from `1556a65b`, (b) domain-sep string "r2-coc-ctrl-v1", specs
D-20260723-16):**
- **D4 initiator `e34c0ea2…`** (alfred:`~/d4-init4b.elf`): b[6]=1 initiator, diff ≠ empty d4e8b69e; persona
  0xC434FAFC baked==input 0ad4a84d @46808; masked base `d4647d8a…`; C apiary+espnow+lora_route+core1.
- **XIAO acceptor `e7e65ebc…`** (alfred:`~/xiao-acc4b.elf`, NEW — supersedes d12ddcc8): b[4]=0 b[6]=0 acceptor,
  diff ≠ empty f02ef4cb (explicit≠derived); persona 0x8C15B0C2 baked==input 43638da0 @46000; masked base
  `50c49946…`; C espnow+lora_route+core1 (observer, no apiary). Established recipe `...loratcxo,xiao,benchkeepalive`.
- BOTH: domain-sep string baked, falsifier present, BUILD_ID coex.iter4b.0723, table e0e49127 (both esp32-s3,
  app@0x20000 — supervisor confirmed no separate XIAO table, my flag caught the misconception). All 4 builds
  from-scratch. **BOTH FLASHED + retested on metal** — result: **roster-unfed root confirmed** (board-to-board
  still blocked pending a fed member-set/roster; **iter-5 = core's member-set work**). No flash pending. Supersedes
  9e9ddb35 + all prior D4 initiators + d12ddcc8.
**#d014 D5 COSINE second-sensor (Roy, parallel — D4 FIRST):** base bee0e996, D4 sensor set minus role blob,
D5 persona (composer delivers, reuse-vs-mint theirs), fakesensor=COSINE at distinct freq. **HELD — recipe approach FLIPPED by Roy:**
fakesensor was hardcoded (apiary.rs@bee0e996 `phase+=0.4` :88, `sinf` :92). Core first shipped an env-baked
knob (`dfr1195-fw-wave af0bf87b`: DFR_WAVE=sin|cos + DFR_WAVE_STEP, D4=no-env byte-identical, D5=cos/step) —
**but Roy RE-HOMED the waveform to the SENTANT layer (not an env-baked plugin knob); af0bf87b SUPERSEDED,
do-not-build.** Core re-layered to the sentant → **`7766f53c`** (dfr1195-fw-wave, supersedes af0bf87b; verified read-only:
bee0e996 IS ancestor so coex fixes ride; `WaveSourceSentant` owns waveform gen apiary.rs:35/90/104,
`TickSourcePlugin` = bare shim; **recipe input UNCHANGED** = D4 no-env sin/0.4 byte-identical, D5
`DFR_WAVE=cos DFR_WAVE_STEP=<step>`). **Supervisor UNHELD D5** (7766f53c satisfies Roy's layer
ruling). **Composer DELIVERED the D5 persona** (alfred:`~/d5-persona.bin`
sha e6108006 336B verified; wire da73508e / hive_id 4d49b381 collision-free; ensemble mariko-sensor). **Now
blocked ONLY on a supervisor ruling:** composer ALSO shipped `d5-persona.bin.role` (48B RPF1 b[4]=1 b[6]=0 =
role=sensor) — but #d014 ratified "sensor set MINUS role blob" (derived, like D4 fakesensor d818ffda). Supervisor
**RULED BAKE** (#d014 "minus role blob" meant no INITIATOR blob; b[6]=0 keeps D5 acceptor-only, no scan-dial).
**D5 COSINE SENSOR BUILT + FULLY ATTESTED: `656cab50…`** (from 7766f53c, BUILD_ID coex.d5cos.0723; features
+DFR_WAVE=cos DFR_WAVE_STEP=0.25 +persona +sensor role). **WAVE differential** A(cos)656cab50 ≠ B(sin)61a5578d
→ DFR_WAVE took (cosine, not sine). **ROLE differential** A ≠ C(no-role)3d6e9ec1 → role took; BAKED_ROLE_PROFILE
`[RPF1,1,2,0]` b[6]=0 sensor/acceptor. All 3 from-scratch = clean. Persona baked==input e6108006 @45916 unique =
wire da73508e/hive_id 4d49b381. Masked base_digest `14db136e…` mask [45916,46252). C-in-binary core1 +
lora_route_task + espnow_task; WaveSourceSentant×6 (cosine gen at sentant layer); apiary×3; RXDIAG=0. table
e0e49127. alfred:`~/d5-cos-role.elf`. Delivered composer+supervisor; **flash post-blerole-clear + Roy grant**
(composer two-party verify). Core flagged: KNOWN_HIVE_IDS receiver-side needs da73508e for D5 mesh resolve.
**Core appended da73508e → wave `5c13a3c5` (supersedes 7766f53c).** Verified diff = ONLY that const-append.
**656cab50 stays functionally complete** — da73508e is D5's OWN hive, resolver filters `h != my_hive` → no-op
for the emitter; the append is receiver-side (D1/D4/RAK resolve D5). **Supervisor RATIFIED KEEP 656cab50** (rebuild = zero
behavior + churns verify; const-appends dying if gate-6 baked roster adopts). Receiver-side 5c13a3c5 coverage =
separate, owned by core enumeration + the next receiver-image builder, not hive.
**656cab50 FLASHED — 3rd node LIVE, cosine origin-verified ×307** (the DFR_WAVE=cos differential proven on
metal). No flash pending on any hive artifact.
**iter-5 WAVE (3 images: D4 init + XIAO acc + D5 cosine, + value-print fold) — 2 CORE-OWNED BLOCKERS found on
`4165f675`** (member-set teachers landed: push_scan_obs T1 :4751 / T2 :4465, was 0-caller = metal root). Both
positive-controlled: (1) **value-print ABSENT** (apiary.rs:92 computes `value` but nothing prints it; ENQUEUED
print main.rs:7023 has no value) = CORE source edit, NOT hive-addable (a println, never edit core); (2) **D5
cosine can't build from 4165f675** — NO wave-sentant (apiary.rs = old SineSourcePlugin, no DFR_WAVE; `git
merge-base` wave 7766f53c NOT ancestor of 4165f675 = blerole line, wave never merged). Buildable NOW from
4165f675: XIAO acceptor (ready, value-print N/A) + D4 initiator (member-set/bit0-BOTH, no value-print). **RESOLVED → OPT-1
UNIFY (supervisor ruled):** core landed value-print (4f66adf4) + then CONSOLIDATED merge **`471f0cf7`** (wave
7766f53c merged into iter-5 line). **All 5 families + da73508e positive-controlled on 471f0cf7:** push_scan_obs×3
(was 0), SCAN_RESOLVED×7, domain-sep r2-coc-ctrl-v1, value-print `APIARY value={value}` (signed **i16** — parse
`value=-?\d+`), WaveSourceSentant×8 + DFR_WAVE×7, da73508e in KNOWN_HIVE_IDS (pair now resolves D5 = receiver-side
gap closed by construction, my argument). **ALL THREE BUILT + FULLY ATTESTED from 471f0cf7**
(BUILD_ID coex.iter5.0723, 7 builds from-scratch, all 4 differentials pass):
- **D4 initiator `c51ad8a6…`** (`~/d4-init5.elf`): b[6]=1 init ≠empty 725ae1ff; 0xC434FAFC @46912; masked
  `bb98625d…`; apiary+espnow+lora+core1; value-print `APIARY value=`; sine-default.
- **XIAO acceptor `90d3f489…`** (`~/xiao-acc5.elf`): b[4]=0/b[6]=0 acc ≠empty f5666376; 0x8C15B0C2 @46048; masked
  `518722c7…`; espnow+lora+core1 (observer, no apiary/WaveSentant = correct); value-print N/A.
- **D5 cosine `11f2d2ef…`** (`~/d5-cos5.elf`, SUPERSEDES 656cab50): b[4]=1/b[6]=0 sensor ≠empty 25398c06; **WAVE
  cos≠sin 8ccaf1f6 = cosine took NOT regressed**; da73508e @47016; masked `d8b56c86…`; WaveSourceSentant×6;
  value-print `APIARY value=`.
- ALL: domain-sep + falsifier baked, da73508e in KNOWN_HIVE_IDS (pair resolves D5), table e0e49127. value-print
  SIGNED i16 (log parse `value=-?\d+`). Delivered composer+supervisor. **Composer TWO-PARTY
  VERIFY PASS all 3 — hash-match on BOTH alfred + tuxedo**; value-print i16 decoder confirmed (readInt16BE +
  `value=-?\d+`). **bit0-BOTH FAILED on the 471f0cf7 pair
  — my da73508e-in-pair miss (OWNED):** KNOWN_HIVE_IDS is DOUBLE-DUTY (resolve-for-admit AND the initiator's
  DIAL POOL); adding D5's da73508e made D4 dial D5-sensor not XIAO. I verified the resolve angle, missed the
  dial angle. See [[shared-list-serves-multiple-consumers]]. **iter-6 FIX (core `ca198a5a`):**
  initiator dials the LOWEST-ELIGIBLE resolved acceptor (= NEG-elected provider), NOT freshest → D4 dials XIAO
  EVEN WITH D5 in KNOWN_HIVE_IDS. Supersedes the 4f66adf4/471f0cf7 split (D5-in-registry now safe for the
  initiator). **Supervisor RULED: build PAIR from `ca198a5a`** (BUILD_ID coex.iter6.0723) — STRONGER test than
  4f66adf4 isolation (D5 stays a live distractor; pass = D4 dials XIAO DESPITE resolvable D5 = validates the
  FIX not just absence-of-trigger; root already metal-observed iter-5 boot + static :4744). **4f66adf4 pair
  (D4 `03d4e677` / XIAO `41108c28`, BUILD_ID coex.iter5p.0723) = attested FALLBACK, do-not-flash** unless
  ca198a5a retest fails weird. **471f0cf7 pair (c51ad8a6/90d3f489) SUPERSEDED.** **D5 `11f2d2ef` STAYS**
  (acceptor-only sensor, no dial path; cosine proven ×307). **iter-6 PAIR BUILT + FULLY ATTESTED from `ca198a5a`**
  (BUILD_ID coex.iter6.0723): **D4 initiator `dc071b41…`** (`~/d4-init6.elf`, b[6]=1 ≠empty a08a414f, 0xC434FAFC
  @46912, masked f5db0736, apiary+espnow+lora+core1) + **XIAO acceptor `a961d808…`** (`~/xiao-acc6.elf`,
  b[4]=0/b[6]=0 ≠empty 8083cdb1, 0x8C15B0C2 @46048, masked a6facac0, observer). D5 da73508e STAYS in
  KNOWN_HIVE_IDS = live distractor (pass = D4 dials XIAO DESPITE resolvable D5). Both falsifiers baked. **Boot
  falsifier (composer reads D4, main.rs:4229):** `INITIATOR captured acceptor <mac> (hive X) — dialing` →
  hive=8c15b0c2=fix works (bit0 lights=DONE / dark=2nd root past dial); hive=da73508e=fix failed→fallback.
  Delivered composer+supervisor; **composer TWO-PARTY VERIFY PASS both hosts (alfred+tuxedo)**; flash + :4229
  monitor-read on supervisor grants. **Fallback 4f66adf4p** (03d4e677/41108c28,
  coex.iter5p.0723) do-not-flash unless da73508e recurs. **D5 `11f2d2ef` unchanged** (not re-issued). All lanes
  converged on B. **METAL RESULT: dial-fix PASSES** (:4229 `INITIATOR captured acceptor … hive 8c15b0c2 —
  dialing` — D4 dials XIAO DESPITE resolvable D5 = my prediction met), **but bit0-BOTH NO = 2nd root past dial**
  (no CoC RECV / membership-verified either side; XIAO self-elects provider→Data, bit0 stays 0x24). iter-6 pair
  flashed+verified; 4f66adf4p fallback not needed. **2nd-root LOCALIZED (hive read-only, ca198a5a):** BLE_ADMIT
  stamps at serve_coc :4426 on ANY inbound CoC PDU → bit0 needs a CoC control frame to FLOW board-to-board; the
  0x25 PASS used an EXTERNAL laptop pump, board-to-board has none → needs the NEG to push a WifiReq/Offer over
  the CoC (send_control :5304 → CTRL_OUT → serve_coc send :4487 → recv → BLE_ADMIT). Code comment :4767 names
  the class ("both self-elect ⇒ neither sends WifiReq ⇒ bit0 dark"). 3 splits: H1 CoC-never-up (create ERR :4297),
  H2 CoC-up-but-no-WifiReq (self-election completes sans CoC — likely), H3 WifiReq-sent-but-CoC-half-open (:7663).
  Asked composer for the decisive metal fact: did "CoC up" print (:4303/:4042)? **Supervisor RULED: CORE LEADS the
  2nd-root, HIVE SUPPORTS.** **Composer 4-way split → BRANCH-2 ASYMMETRIC** (clean un-wedged capture): D4
  opens the CoC (`CoC up` 120s, SENT=0, then closes→falls back to D5) but **XIAO's provider-accept NEVER fires**
  (`CoC up, serving`=0, accept-ERR=0, adv-ERR=0). **Hive localized (BLE-connect support):** XIAO AcceptorOnly
  blocked between `adv up` :4007 and `CoC up serving` :4045 = at `advertiser.accept()` :4017 OR
  `L2capChannel::accept` :4038; runner IS polled (join3 :4175, not a Fix-C starve). Asymmetry (D4 create-Ok, XIAO
  accept-stuck) + D4's 271ms create → **lead hypothesis = accept-listener RACE** (D4's L2CAP connect-req lands
  before XIAO registers the accept listener → dropped → one-sided 120s channel). **ROOT CONFIRMED (core+composer, my H2):
  self-elect RACES the roster feed** — `request_data_plane` fires fixed ~3s (:5373) but capture-scan only STARTS
  at 3s (:4185) → roster EMPTY at election → both boards self-elect (D4=0xC434FAFC, XIAO=0x8C15B0C2), Data has no
  re-elect (negotiation.rs:526), teachers feed too late → no WifiReq → no CoC frame → bit0 dark. H1(conn)/H3(half-
  open) RULED OUT (D4 CoC-up x2, create ERR=0, WifiReq=0). **FIX (core owns, hive concurred + VERIFIED
  SUFFICIENT): thread ble_role→engine_task + Initiator `ap_capable=false`.** State-machine walk (negotiation.rs):
  Initiator ineligible → `elect()` drops self → at 3s empty-roster `elect()`=None → **stays in Discover** (:494,
  never strands in Data:526) → re-ticks until capture-scan rosters XIAO → elects XIAO → WifiReq → bit0 BOTH.
  Flagged the one dependency: Discover must re-tick after roster fills. Confirm-before-fix MET. **BUT a 2nd break is STACKED (hive proved
  independent):** BRANCH-2 = XIAO's `L2capChannel::accept` never returns (`CoC up serving` :4045 prints the instant
  accept returns, BEFORE any traffic → its absence = genuine accept-hang, not a no-traffic artifact). H2-fix alone
  won't light bit0 — D4's WifiReq routes over the CoC, needs XIAO's serve_coc receiving = accept complete. **esp-
  radio hypotheses (hive lane):** HA create-optimistic (D4 CoC-up doesn't prove XIAO accepted), **HB (lead)
  accept-registration window-race exposed by esp-central timing** (D4's REQ 271ms after connect lands before XIAO
  registers L2capChannel::accept → dropped; BlueZ-pump worked because it paces slower), HC conn-not-serviced. **CONVERGED
  iter-7 = ONE build: core H2-fix (Initiator ap_capable=false) + XIAO accept step-log (ACL-accepted :4018 +
  L2CAP-accept entry/return) + election-timing markers** → splits H2 vs BRANCH-2 (ACL vs L2CAP layer) in one flash.
  **Core writes the spec → hive builds.** Sent core+composer+supervisor. **NEXT (post-metal): classify
  InvalidRouteLen per queue.** Ops hazard:
  [[reference-xiao-boot-flush-wedge]]. Lesson: [[shared-list-serves-multiple-consumers]]. **Step `DFR_WAVE_STEP=0.25` RATIFIED FINAL**
(supervisor, converged with my default; 1.6× D4's 0.4 period; Roy can override). **Build script pre-staged:
alfred:`~/build-d5cos.sh <persona-path>`** — resets to 7766f53c, full rm -rf, builds cos/0.25 then a sin/0.4
differential control (same persona) to prove the DFR_WAVE env took, saves `~/d5-cos-role.elf`. Fires the
moment composer hands the D5 persona. BUILD_ID coex.d5cos.0723, table e0e49127 (same DFR1195 class), full attest
(persona baked==input + identity, masked base_digest, C-in-binary, fakesensor, cos-vs-sin differential). D4
initiator unaffected (ca00c094 delivered, flash #d011). STANDBY — persona is the last gate.
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

- **iter-5 sensor value-print (supervisor, queued — NOT a build order yet; awaits core member-set land):**
  rate-limited `APIARY value={value:u16}` print at emission (every Nth reading OR 1/min) alongside the ENQUEUED
  line — the 29-B compact body's value is printed NOWHERE, blocking waveform-sample verification. Applies to
  D4+D5 sensor images. **Core lands the print** (emit path); hive builds. Hand core the exact spec when iter-5 fires.

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
