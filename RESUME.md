# RESUME — r2-hive (hive-worker)

> Older closed arcs live in RESUME-archive.md (rotated 2026-07-06; this file holds LIVE state only — keep it readable in one pass).

## ✅🛰️ RAK4630 FIRST-LIGHT — CONFIRMED IN TEXT (2026-07-07, task #44 first-light DONE)
- ✅✅ FIRST LIGHT CONFIRMED (supervisor relayed Roy's /dev/ttyACM0 capture, via pyserial — cat doesn't assert
  DTR, the flow-control hunch was right): 'R2-BEACON advert encoded (30 B)'. Can't beacon without a LIVE radio,
  so configure(LoRa) SUCCEEDED + membership/beacon stack running on-metal + 30-byte advert every 30s. Combined
  with N=0/dark-off (clean run, no fault handler ran) → RAK4631 is ALIVE running R2 firmware. Chain that nailed
  it: APPROTECT reset-loop root cause + corrupt-buffered-drag insight (the "faults" were bad images, not bugs)
  + legible-count diag + serial-DFU packaging. Serial-DFU (adafruit-nrfutil venv on tuxedo) = reliable PRIMARY.
- ✅✅ CDC 't'-QUERY CONFIRMED END-TO-END ON METAL (supervisor, 2026-07-07): flashed a03c37b4, sent a byte,
  board dumped 'diag node=0x52414b34 mode=dev class=2 / diag tg=0x00000000 group=keyless duty=AlwaysOn / diag
  neighbours=0 routes=0 / diag end' + 'alive t=31s neighbours=0' every 5s + 'R2-BEACON advert encoded (30 B)'.
  Every field correct (node-id=RAK4 ascii, dev/class=2, keyless TG-of-one, AlwaysOn, lone-board 0/0, live loop).
  = Roy's "show me the internal TN working" fully delivered: full TN stack instantiated + queryable on-metal.
  FIRST LIGHT PROVEN THREE WAYS (N=0 clean run + beacon text + TN dump). LOCAL physical-access diag (#30-class),
  NOT #41 on-mesh. Capture needs pyserial (DTR); printf/cat alone don't assert DTR. Findings [[rak4630-uf2-firstlight]].
- MILESTONE BANKED. Supervisor stood down on the RAK bring-up thread. task #44 first-light DONE. Repeater RELAY
  build HELD (Roy has only ONE LoRa board at work → 2-node relay blocked). OPTIONAL later: 2-node LoRa test →
  neighbours>0/routes>0 in the same dump. #44 remaining = repeater relay (io_task → r2_dataplane, task #32 seam).
- 🟢 HEARTBEAT-LED SENTANT SHIPPED (Roy's ask; rak4630-fw b1870f7, pushed): idle LED was DARK so alive/dead
  looked identical. Added a dev-only heartbeat sentant — brief ~50ms green pulse every ~1.5s, visually distinct
  from every other LED state (boot 3-blink / configure-ack 1s solid / recv 400ms flash / FAILED 1Hz / panic
  strobe / diag replay). Plugin+sentant at the SHAPE level (struct HeartbeatLed = state; .tick() = behaviour,
  driven by the service-loop-as-EventBus) — honest: NOT a runtime-registered sentant (that's #21), re-homes when
  #21 lands. YIELDS to the recv-flash (traffic legibility > idle, task #33). Non-blocking (off-deadline like
  flash_off, never awaits). dev-gated → prod --no-default-features STRIPS it (R2-BUILDMODE §6, verified). =
  task #34 realized on RAK. All 4 variants build green. Artifacts regen'd: out/r2-rak4630-usbserial.{uf2,hex}
  (52936 B, vec MSP 0x20040000/reset 0x26101/LMA 0x26000) + diag.{uf2,hex}. adafruit-nrfutil NOT on this host →
  supervisor runs genpkg + serial-DFU on tuxedo. VERIFICATION = Roy eyeballs the pulse on-metal (the refutation).
- 🔵 BLE-OTA SCOPE (Roy wants single-board OTA-over-Bluetooth de-risk; supervisor asked, I VERIFIED the repo
  2026-07-07): (1) RAK firmware has ZERO BLE RUNTIME today — grep src/ = only the R2-BEACON advert CODEC
  (main.rs:557 'Phase 2 hands the bytes to the BLE advertiser; the spike logs the size'). So 'advert encoded
  (30 B)' = bytes produced, NOT a BLE packet radiated. Only LoRa + USB-CDC are live. (2) R2-native BLE-L2CAP-CoC
  OTA receiver = NOT built; ARCHITECTURE.md §5 = inc-2 (nrf-sdc + trouble-host, pure-Rust host), GATED on core
  vendored-set move 41adbd1. (3) OTA APPLY backend (ImageSink/FlashSink dual-bank) also NOT built — described in
  ARCHITECTURE.md §2.1 only, no src/ impl. (4) ESP32 BLE-OTA (#18/#19) does NOT transfer (diff chip/host; only
  R2 protocol layers above are shared). SO: no quick R2-native BLE-OTA on this board. Only bluetooth path that
  works TODAY = stock Nordic BLE-DFU (phone app → resident S140 6.1.1 + Adafruit bootloader), which ARCH §1
  DELIBERATELY REJECTS (auth outside R2 = sovereignty bypass) — usable ONLY as a throwaway 'BLE physically
  reaches the board' check, NOT the R2 path. HIGHER-VALUE single-board de-risk avail now = the OTA APPLY
  mechanism (ImageSink dual-bank verify→stage→bank-flip→boot-select), transport-AGNOSTIC, buildable WITHOUT BLE.

## 🟢 BLE inc-2 — ROY GO'd (2026-07-07); 2b-2d GATED on GREEN 2a; 2a JOINT spike CO-SCOPED + building (task #58)
- ✅ ROY'S GO: inc-2 approved with MY recommended gate (2b/2c/2d gated on a green 2a). Locked boundary confirmed.
  2a = host runs on-metal + R2-BEACON RADIATES. When 2a fw ready → supervisor serial-DFU flashes on Roy's double-tap.
- ✅ 2a CO-SCOPED (joint, both non-idle): HIVE half DONE = the peripheral-partition map (BLE-PLAN.md §8, rak4630-fw
  aae487f). HEADLINE — the scary conflict is AVOIDED: embassy-time=RTC1, MPSL=RTC0 → NO RTC clash; TIMER0/RADIO/TEMP/
  SWI free (no timer use; LoRa=external SX1262 so internal radio free; thread-mode executor = no SWI). Conflict surface
  = 3 SOFT points, now 2 RESOLVED with core (e255af4): RNG = MPSL-owns (seed fp_seed BEFORE MPSL init, never touch
  after); CLOCK/LFCLK = drop manual HFXO under `ble`, LFCLK = LFXO (RAK4630 WisBlock 32.768kHz xtal, BSP-confirmed;
  definitive xtal-populated check = the 2a metal run). SOLE OPEN item = the exact PPI set MPSL reserves (nRF52840 =
  PPI not DPPI) — core pulling it from nrf-sdc/mpsl source (must-be-exact, not guessed). `ble` feature gate scaffolded
  (Cargo.toml stub, 0948d94, green). BLOCKED-NEXT on core's descriptor (PPI set + nrf-sdc version/pin) + the r2-ble
  binding skeleton (workspace-excluded thumbv7em crate, core-writes/hive-builds-on-metal = esp32 pattern, I'm the
  on-metal verify); then wire host into main.rs behind `ble` + advertise R2-BEACON bytes. → task #58.
- ✅ ITEM-1 DESCRIPTOR LANDED + CONSUMED (core 474ee09, crates/r2-ble/src/descriptor.rs; BLE-PLAN.md §9): 2a
  partition FULLY SPECIFIED — PPI app-free=CH0..=16 (MPSL/SDC own 17..=31); LFCLK=LFXO (drop HFXO under ble);
  RNG=SDC-owned (seed BEFORE sdc::Builder::build); RTC1/RTC0 no clash; 6 IRQs (RNG,EGU0_SWI0,CLOCK_POWER,RADIO,
  TIMER0,RTC0). Source alexmoon/nrf-sdc@f54b6389 (NOT embassy-rs). ALL 3 soft points now resolved.
- 🔨 2a FOUNDATION TASK IDENTIFIED = embassy git→0.7 crates.io migration (the real 2a effort). Pin: nrf-sdc 0.3.0 +
  nrf-mpsl 0.3.0 + embassy-nrf 0.7 + bt-hci 0.4 (crates.io; NOT git-master=nRF54L). Firmware uses ALL embassy from
  git → whole platform migrates to 0.7-family. TOUCHES PROVEN first-light/heartbeat → FOCUSED+TESTED (keep non-ble
  builds green throughout), held for a clean session start (NOT rushed at turn-tail; a half-done embassy migration
  in-tree is worse than a scoped handoff). NEXT ACTION = the migration → then partition + co-author BleHost w/ core.
  Core CONFIRMED ^0.7 RANGE (not exact): r2-ble is my path-dep → cargo unifies embassy-nrf to ONE 0.7.x, my lockfile
  pins the patch (no soup). MAIN RISK = embassy-nrf 0.7 SPIM/GPIO/RNG API deltas vs r2-sx1262 (loop core, co-owns it).
  ✅✅ MIGRATION DONE (2026-07-07, rak4630-fw 851fdc0): whole platform migrated embassy git→0.7-family crates.io.
  Core's cargo-VERIFIED set (80ebe9d) resolved the embassy-time split: embassy-nrf 0.7.0 / executor 0.9.1 (was my
  0.7 → the fix; 0.7 pulled old executor-timer-queue 0.1 pinning time 0.4) / time 0.5.1 / sync 0.7.2 / usb 0.5.1 /
  bt-hci 0.4 / nrf-sdc+mpsl 0.3. ONLY ONE API delta: executor 0.9 task macro returns SpawnToken directly →
  must_spawn (was spawn(..).unwrap()). r2-sx1262 UNTOUCHED (eh-1.0 trait-generic, core-proven). ALL 4 variants GREEN
  (default/usbserial/diag/prod); usbserial vec MSP 0x20040000/reset 0x26101/LMA 0x26000 intact, image 53064→51384B.
  ⚠️ RUNTIME equivalence to proven git-embassy first-light/heartbeat UNPROVEN until a metal flash (new embassy stack
  under proven R2 code) — recommended supervisor a MIGRATION-ONLY metal check (flash usbserial hex sha 60040e05, confirm
  first-light+heartbeat+t/b) BEFORE the BLE host = clean bisection. NEXT 2a: r2-ble reachable from rak4630-fw (core's
  call: git-pin 474ee09 vs land-on-base) → add under ble + apply partition (PPI 0..16/HFXO-drop/seed-before-SDC/6 IRQs)
  + co-author BleHost on spike + advertise → GREEN 2a. The version-axis 2a-foundation blocker is RESOLVED.
- 🟢 r2-ble WIRED BLOB-FREE (rak4630-fw a0feb69): ble=[dep:r2-ble], git rev 3da1330, default no-features = the
  pure-data descriptor (core's feature-gate fix; --features ble = NO nrf-sdc/mpsl/bt-hci, cargo-tree verified).
  Embassy graph anchored. Binding (r2-ble/binding=blob) flips on only on the on-metal spike. 2a foundation progressing.
- ✅ PROVENANCE FIXED (supervisor): out/*.hex is gitignored/regenerable so its sha moves per rebuild. KNOWN-GOOD
  'b'-image = commit a3ddff9, usbserial.hex sha d2bbe502 — VERIFIED REPRODUCIBLE (clean rebuild @ a3ddff9 = same
  sha). Chain: 75e55fed@9ebf7ef (pre-migration) → 60040e05@851fdc0 (migration, pre-LED) → d2bbe502@a3ddff9 (+LED) =
  current good. I do NOT scp (one-writer); supervisor re-copies + verifies by rebuilding. Has 'b'+migration+LED.
- 🔵 LED-REUSE DIRECTIVE (supervisor) → task #59: REUSE r2-workshop's canonical LED envelope fns (dfr1117/led.rs:
  heartbeat=2 gaussians@0.00+0.18=dub-dub, strobe=OTA, single_tick=event, pulse) + LedState enum, don't fork my
  a3ddff9 square-wave. RAK considerations: (a) gaussian .exp/.powi → no_std needs micromath; (b) mono LED → PWM
  P1.03 (SimplePwm) for the smooth envelope + wall-clock phase-lock (lockstep). Core: state→pattern = SHARED
  core-primitive candidate (flag specs; GPIO rendering stays firmware). Focused rework next (not rushed at tail);
  a3ddff9 square dub-dub works meanwhile for the 'b'+migration metal-check.
- 🟢 CANONICAL LED SIGNATURE SHIPPED (Roy directive, rak4630-fw a3ddff9; supersedes the single-pulse heartbeat):
  HEALTHY = slow dub-dub DOUBLE-pulse (2×~50ms blips ~120ms apart, ~1.5s pause = human lub-dub, reads ALIVE);
  UPDATING = rapid ~5Hz flash (OTA hook led_signature::UPDATING, set when inc-2c OTA lands); EVENT-ARRIVAL = ~30ms
  blip on RxDone. dub-dub + updating in BOTH dev+prod (verified); event-blip DEV-ONLY. New led_signature tick-driven
  state machine replaced the dev-only heartbeat + 400ms recv-flash. All 4 variants green. Canonical across R2 boards
  (specs canonicalizing → DFR1195 adopts same); relates to task #34. IN the same usbserial build as the embassy
  migration → ONE migration-metal-check flash validates embassy-0.7 runtime + the LED signature + t/b commands.
  Updated artifact: out/r2-rak4630-usbserial.hex sha d2bbe502 (51392B). Reported supervisor for Roy's metal-check.
- 🟢 HEADLESS-FLASH WIN SHIPPED (rak4630-fw 9ebf7ef, supervisor request): CDC 'b' command → writes Adafruit DFU
  magic 0x57 to NRF_POWER->GPREGRET @0x4000_051C + SCB::sys_reset() → board reboots to UF2/DFU, NO double-tap. Live
  in the CURRENT usbserial build (carries into 2a); any other byte still = table dump. Roy PRE-AUTHORIZED all 2a
  flashes (build-go + flash-go standing; I hand the .hex path when 2a builds green). usbserial artifact sha 75e55fed.

## 🔵 (superseded) BLE inc-2 PLAN + split LOCKED with core (Roy greenlit 2026-07-07) — awaiting Roy's go, NO build
- ✅ OWNERSHIP SPLIT LOCKED (core+hive converged, both favour core-owns-binding; rak4630-fw 141775b, BLE-PLAN.md §7):
  CORE owns a new no_std BLE binding crate (nrf-sdc + trouble-host) + CoC transport-seam adapter (OTA bearer +
  beacon-radiate) + nrf-sdc vendoring/pin (= the GATE, unmet). HIVE owns rak4630 firmware + metal bring-up. Contract
  seams verified drift-clean by core (A7/A8 receiver feed/finish is bearer-agnostic + ALREADY delivered; BlePhy moves
  WireFormat::Compact). 41adbd1 = non-gate (discovery/CoC hardening), corrected in plan + ARCHITECTURE.md.
- ✅ I SENT core the BINDING API SURFACE hive wants (mirrors r2-sx1262): (1) peripheral-requirement declaration as
  DATA (the 2a crux — what nrf-sdc claims: RADIO/TIMER/RTC/RNG/PPI — so I partition embassy-time; requested FIRST),
  (2) BleHost::new(resources, config), (3) set_advert(bytes) beacon, (4) pollable L2CAP-CoC PHY, (5) OTA-over-CoC via
  core's receiver. 2a is CO-SCOPED (core's requirement API + partition scaffold; my metal spike proves it). NEXT:
  await core's peripheral-requirement descriptor + Roy's go (my rec = gate go on inc-2a de-risk spike).

## 🔵 (prior) BLE inc-2 PLAN — awaiting Roy's go, NO build started
- Committed scoping doc: rak4630-fw 4d3c446, platforms/rak4630/BLE-PLAN.md. 3 purposes (beacon-radiate / L2CAP-CoC
  data-plane alongside LoRa / OTA-over-BLE PSM 0x00D3 TG-gated). DEP CHAIN CORRECTED: NOT gated on 41adbd1 (core:
  that's CoC hardening, not a crate-set move); nRF52840 BLE stack (nrf-sdc + trouble-host) = GREENFIELD, hive-owned
  on rak4630-fw, nothing to wait on. Core owns OTA/transport CONTRACTS (r2-update::ImageSink, A7/A8 receiver,
  r2-transport::WireFormat) + budget ledger, NOT a BLE crate. LoRa(ext SX1262) + BLE(int nRF radio) = separate
  radios, coexist. INCREMENTS 2a host+beacon (hard bring-up, #1 risk = peripheral arbitration) → 2b CoC transport
  → 2c OTA receiver → 2d §5.3.1 proximity enrol. FLASH fits (~200KiB image in 412KiB bank). MY REC: gate Roy's go
  on inc-2a de-risk spike first. Reported to supervisor for Roy's go. = task #44 forward / inc-2 (ARCHITECTURE.md).

## ✅ SEAM CLOSED: relay device-auth REMOVED (Roy ruling 2026-07-07) — /r2 §3.2 handshake DISSOLVED; my view WON
- ★ ROY RULING: a below-TG relay is now an AUTH-FREE dumb tg_hash pipe (trust = end-to-end TG-HMAC at member
  devices; routing by tg_hash never needed identity). My owner-view (REMOVE, steelmanned + verified vs my impl)
  was the ruling. DISSOLVES the /r2 §3.2 handshake AND its extraction ENTIRELY — crypto-extract thread CLOSED,
  nothing to lift. Also CLOSES the phone cross-TG device_id linkability leak = an M1 SOVEREIGNTY WIN.
- ✅ KS1/hkdf consolidation UNAFFECTED — stands (KS1 resident in r2-trust::hkdf). That part was always real.
- ✅ KS1 DE-DUP SHIPPED by core (95eee98 on r2-core-consolidation): identity.rs now DELEGATES to r2-trust::hkdf
  RAW (deleted its private hkdf_expand + v4-forcing); hive_id now RAW/canon; r2-trust 85 green + r2-hive-core 37
  green. Core confirmed my trace: NO-OP for hive (id never flows through derive_hive_id) + MasterSecret/
  DerivedIdentity/IdentityStore have 0 consumers in r2-core. RE-VENDOR behind the pin whenever (no urgency — it's on
  the consolidation branch, no-op for my wire id). self_hive_id=FNV(name) §6.2.1 flag → still task #57 (raise to specs).
- (context) I traced + cleared it before core shipped, told core SHIP IT: core
  collapses r2-hive-core::identity::derive_hive_id (v4-forced UUID) onto r2-trust::hkdf (RAW). Core asked A/B: is
  hive's wire u32 = FNV(RAW derive) [A, no-op] or FNV(v4 derive) [B, live bug]? ANSWER = NEITHER (C): hive's on-air
  ensemble/routing hive_id = raw FNV-1a-32(--name) at main.rs:367 (documented main.rs:365 'no canonicalisation';
  --name default 'r2-hive' main.rs:125); ensemble.rs:492/500 broadcast uses it; ONLY one self_hive_id assignment in
  the crate; derive_hive_id + its v4 UUID have ZERO hive consumers (identity.rs:38 re-exports TYPES only). So v4→RAW
  can't change hive's wire id = de-dup SAFE, no migration. I re-vendor behind the pin after core's push.
- 🐛→⚖️ hive_id DRIFT — RULING LANDED (specs R2-WIRE v0.42 §6.2.1, 2026-07-07): my FNV(name) hive_id is
  NON-CONFORMANT. Canon = FNV-1a-32( UUID-format( HKDF("r2-hive-id-v1", master, tg)[0:16] ) ) =
  r2_trust::hkdf::derive_hive_id(master, tg).1. 3 problems: addressing divergence + default-'r2-hive' collision +
  name-on-wire = cross-TG LINKABILITY handle (same class as the device_id leak just removed). = task #57 (now
  IN-PROGRESS, my fix). DESIGN = PER-TG (not core's single-tg one-liner): verified hive is MULTI-TG (hive.rs:255
  group_hmacs HashMap<u32,GroupHmac> per-TG), master in DaemonState (state.rs:53), self_hive_id = ONE global
  FNV(name) @main.rs:367 used as ensemble originator + dedup self-check (ensemble.rs:472/492/500). FIX = thread
  master into HiveState + cached hive_id_for_tg(X)=derive_hive_id(master,X).1 + swap the ensemble call-sites (NOT
  just main.rs). Migration: ids change; pre-1.0 dev = re-derive next boot; TG nodes re-derive together. SECURITY-
  RELEVANT (dedup self-check) → implement carefully + PEER-REFUTE before done. Flagged core the per-TG shape
  (awaiting their confirm it's the intended §6.2.1 read). core de-dup (95eee98) gives the fn; the HiveState/ensemble
  switch is my half. specs CONFIRMED (self-corrected): de-dup does NOT auto-fix my wire id — the main.rs switch is
  mine + PENDING. WRINKLE to resolve first: derive_hive_id takes the tg_id STRING but the ensemble has ev.trust_group
  as a u32 WIRE HASH — so derive+cache at PROVISION time (main.rs:737, tg string known) keyed by u32, not on-demand.
  NEXT ACTION when I resume this (implement carefully + peer-refute; security-relevant dedup self-check).
  CORE CONFIRMED (2026-07-07): tg_id = the trust group's UUID STRING (§2.3 / §6.2.1 line 548 / KS1 vector
  '550e8400-...'), NOT a hash. My 'a1b2c3d4...' tg_hash is a HASH → deriving from it = the bug.
  🔴 DEEPER FINDING (verified r2-hive-bin, refutes core's 'tg-of-one proceed now'): the daemon holds ActiveTg.tg_id
  as [u8;32] = the 32-BYTE KEY HASH (hive.rs:150), has ZERO UUID string anywhere (grep-clean), AND already has an
  ActiveTg.hive_id:u32 field (non-canonical, no UUID to derive from). So EVEN tg-of-one can't derive today — the
  UUID input isn't present. #57 = bigger than a derive-swap; needs the UUID ESTABLISHED. BLOCKER asked core (may
  route specs): is the [u8;32] a hash-OF-a-UUID (→ generate+persist UUID at first boot = identity change) or IS it
  the canonical tg_id rendered as a UUID string (→ render+derive, bounded)? + joined-TG conveyance gap (specs).
  HELD on that answer (verified-before-implement — deriving from the wrong input reproduces the bug). SECURITY +
  now IDENTITY-establishment → focused session + peer-refute when unblocked.
  ✅ RESOLVED to (b) + 🔴 NOW HELD ON ROY (2026-07-07): specs CONFIRMED (b) FORCED (self-mastered join unimplementable
  as written) + pre-specced tg_id = UUID-format-RAW(SHA-256(TG_PK)[0:16]). BYTE-CHANGES the wire-locked KS1 vector →
  ESCALATED TO ROY (operator-gated). ON ROY'S YES: specs regens KS1 vector + reconciles §6.2.1/§2.3/§4.1; core ships
  r2_trust::hkdf::trust_group_uuid(tg_pk) vector-locked; I render ActiveTg.tg_id[u8;32]→trust_group_uuid→derive_hive_id
  →swap ensemble originator+dedup-self-check. My part BOUNDED (core's earlier find = it was a spec name-collision:
  trust_group_id = §4.1 [u8;32] TG_PK vs §6.2.1 UUID string; daemon holds §4.1). DO NOT implement until Roy yes +
  core ships the fn/vector. Verify-before-implement caught this whole chain (would've shipped non-canonical hive_id).
- ✅ AUTH-FREE §3.2 SHIPPED (r2-hive 99b336a, task #56 DONE): specs landed v0.11 (441a94b); I dropped
  compat/handshake.rs + protocol.rs to the auth-free path. Connection = version-3 SUBSCRIBE {version, trust_group,
  timestamp} — no device_id/signature/challenge. Ephemeral per-connection handle (next_conn_handle) replaces
  FNV(device_pk); rejects 4400 stale-ts / 4401 retired-version / 4429 too-many; §3.2.2 token field IGNORED (open
  relay). Removed obsolete Ed25519 VEC_* consts + orphaned ed25519-dalek dep. New structural tests vector-locked to
  r2-transport-relay-vectors.json v0.11 — 4 green, workspace clean. CLOSES the cross-TG device_id linkability leak.
  FOLLOW-UPS: (a) the optional §3.2.2 unlinkable capability token = later (spec wire/vectors are a follow-up rev);
  (b) CHECK wasm hive if it carries the same compat handshake (may need the same drop). No wasm handshake found in
  the quick grep, but verify when touching wasm next.
- 🔑 KEY-FACT VERIFIED (2026-07-07, specs asked): my handshake signs/verifies with device_id-SK (STABLE class-2
  identity), NOT mesh_sk. Evidence: protocol.rs:16 device_id field; handshake.rs:263 VerifyingKey::from_bytes
  (device_id); :354 vk.verify over nonce:trust_group:device_id:timestamp. ZERO mesh/KS1/hkdf/derive in compat/
  (grep-clean; only Rust #[derive] attrs). Faithful to §3.2 canon (device_id). ⚠️ I RETRACT my earlier
  "KS1-derived DEV_SK" wording — WRONG conflation: device_id is the NON-derived stable first-firmware key
  (r2-trust::hkdf says device_id is NOT derived); mesh_key is the KS1/per-TG derived one. Handshake = pure
  prove-possession-of-device_id-SK.
- 🕵️ PRIVACY ASYMMETRY FLAGGED to Roy (canon tension, NOT a hive bug): §3.2 puts the STABLE device_id ON THE WIRE
  in cleartext + signs with it → device LINKABLE across TGs at an untrusted relay. CONTRADICTS §6.2.2 (device_id
  NEVER on-air, carry per-TG mesh_pk — r2-trust tests assert this). Roy question: should §3.2 switch to a per-TG
  key (mesh_pk) for relay-unlinkability, matching §6.2.2? Today it does NOT (device_id, linkable) = a §3.2 CANON
  CHANGE if Roy wants it. My impl mirrors §3.2-as-written; no unlinkability intent was in my code. Supervisor
  surfacing to Roy.
- ✅ HOME (favoured) = r2-trust, relay_handshake joins the KS1 family already in r2-trust::hkdf. My
  earlier r2-keystore vote FLIPPED on ground truth I verified myself (honest update, not rubber-stamp): r2-trust::
  hkdf ALREADY houses KS1 (derive_group_keys@55, derive_hive_id@120, derive_mesh_key@148, device_id_selector@166,
  vector-locked to specs KS1 key-schedule, composer-byte-exact + android-consumed), AND firmware+everyone already
  deps r2-trust (GroupHmac/§7.5.4). So my ONE load-bearing premise (r2-keystore saves dep-weight) is FALSE — the
  weight is already paid; a new crate would MIGRATE working vector-locked code = churn + re-vector risk for ZERO
  saving. By my own line (behaviour identical → optimize the dep-graph), the dep-graph points r2-trust; and cohesion
  now points there too (the handshake proves the device_id SK; the device_id/mesh split already lives in hkdf).
- WHAT SURVIVES: (1) authn/authz principle stays, as INTRA-crate structure — relay_handshake = the AUTHN sub-surface
  of r2-trust (identity: derive key + prove possession), sans-IO + feature-light, distinct from the AUTHZ surface
  (certs/group-mgmt/GroupHmac). Same principle, no migration. (2) My cond-(a) refinement is HOME-INDEPENDENT + still
  the impl contract: engine owns ONLY signed-msg construction + Ed25519 verify + timestamp-window + nonce-lifecycle,
  FRAMING-AGNOSTIC (JSON/CBOR envelope + socket IO stay per-consumer), exposes BOTH server+client roles. (3) specs
  keeps the device_id-SK-not-mesh_sk precision — handshake proves the STABLE first-firmware device_id key, NOT the
  per-TG mesh_sk (r2-trust already enforces device_id-never-on-air; handshake must not blur it).
- ▶️ SEQUENCE: core does the ADDITIVE Phase-1 (relay_handshake engine into r2-trust::hkdf's authn surface, both
  roles, vector-locked; r2-hive-core deps it → I'm UNAFFECTED) → core sends grep-map HEADS-UP before any breaking
  push → HIVE PHASE-2 RE-POINT = my next action (drop my compat crypto core in r2-hive-bin/src/compat/handshake.rs,
  KEEP my WS/JSON/TG-resolution glue, consume the shared engine, re-vendor for fw behind the pin). DO NOT re-point
  until core's heads-up lands. 3 conditions are the contract (no_std/sans-IO/alloc-optional; vector-locked b5cbba2;
  vendored-pin + grep-map heads-up — core warns me before it bites my path-dep, [[shared-checkout-path-dep-coupling]]).
  No M1 block. Verified vs my impl (compat/handshake.rs): handshake = pure prove-possession-of-device_id-SK
  (v0.2 nonce challenge-response; v0.1 legacy), no session-key/cert/group-state in the crypto core.
- (original position + reasoning below, kept as the rationale of record:)
- Q from supervisor (android+core surfaced): §3.2 relay handshake was RULED to hive's compat driver, but
  android core-ffi now has a 2nd sans-IO impl + composer/phone need byte-identical = 3 drifting impls. Proposal:
  consolidate to ONE shared no_std sans-IO engine at r2-trust::relay_handshake, hive+composer+phone consume it
  (vector-locked). Same Q for KS1 derivation → shared r2-trust/r2-keystore.
- MY POSITION (as current owner): RATIFY the extract, give up the private copy, CONSUME the shared engine.
  Key facts backing it: (a) it's a TRUST-layer security handshake — one impl makes drift STRUCTURALLY impossible;
  vector-locking 3 impls only catches drift after-the-fact. (b) ZERO new dep surface — RAK/DFR fw ALREADY links
  r2-trust no_std (path dep, default-features=false; how §7.5.4 deliver-gate + H9 sign reach me). (c) sans-IO =
  pure state machine, I drive it over LoRa / composer WS-TCP / phone BLE from one engine. Keep-owned only buys
  velocity, but a spec-defined handshake has NO legit hive-private variation = illusory.
- 3 CONDITIONS (contract, not blockers): (a) engine stays no_std + sans-IO + alloc-free/optional (480 KiB bank);
  (b) vector-locked by the canonical vectors hive already vendors (b5cbba2); (c) land behind vendored-pin +
  grep-map heads-up before the breaking push ([[shared-checkout-path-dep-coupling]]). Spec-first: want specs to
  bless the r2-trust home as canonical location, then core implements + I re-point. No M1 block. AWAITING ratify.

## ✅ REAL HOSTED CI GREEN (task #55 DONE): r2-hive compiles + tests on runners for the FIRST time
- ci.yml modernized (1138eb3) + vector vendoring (b5cbba2): all 5 jobs GREEN (run 28778737556) — test
  PROD+DEV (the §5.1 BUILDMODE both-modes gate, on a runner), feature-builds (ble/keyring), wasm
  (host+wasm32), cross-aarch64 (Pi5/UNO-Q), lint (non-fatal). **"hosted-CI-green" is now sayable for
  r2-hive; bump-core.sh's honesty upgrades from local-green to hosted-green.**
- The stale ci.yml was TRIPLE-dead: main-only trigger (work is on platform-trait), pre-fold sibling-checkout
  model (dead since the rev-pin), no private-dep auth. Fixed: trigger platform-trait+main, R2_CORE_READ_TOKEN
  insteadOf auth in every job (Roy's docs-site token, earning its keep twice today), drop the dead checkout.
- LATENT DEFECT SURFACED + FIXED (the whole point of real CI): host_api_conformance.rs (compile-time
  include_str!) + vector_coverage.rs (runtime) both coupled the ENTIRE workspace test build to a SIBLING
  r2-specifications checkout — green on every dev box, RED on any clean runner/clone (non-hermetic build,
  wrong for open-source). Fixed by vendoring the 4 consumed canonical vectors in-tree (tests/vectors/ +
  _SYNC.md, the fleet vendoring norm; @ specs 6bebcd1). Build now standalone-hermetic. Specs flagged
  (heads-up + override offer: shared path or specs-read token if they prefer).
- DO-NOT-ASSUME: r2-usb-pair-vectors.json trips the local pre-push secret-scanner on synthetic
  `secret:"1111..."` fields + deterministic public X25519 `shared_secret` outputs — VERIFIED false-positive
  (public spec test data, Roy-hygiene-gate clean); re-vendors need FLEET_SKIP_SECRET_SCAN=1. lint job shows
  non-zero in logs but conclusion=success BY DESIGN (clippy/fmt continue-on-error — a visible signal, not a
  gate; a clippy sweep is a separate scheduled tidyup).
- ✅ SPECS BLESSED THE VENDORING (v-ruling) + two robustness follow-ups DONE (3cae033): (1) .gitleaks.toml
  allowlists tests/vectors/*.json (specs' fleet pattern @ cce4896); (2) ci/check-vendored-vectors.sh = the
  drift ALERT specs welcomed — diffs vendored vs canon sibling, shouts on drift, NEVER auto-syncs (pin is
  deliberate; reproducible CI must not follow canon HEAD), hermetic-safe (exit 0 where sibling absent).
  Specs KEEP-VENDORED ruling: do NOT switch conformance vectors to a live specs-checkout even with a token —
  reproducibility requires the pin (DISTINCT from the #52 rustdoc seam where PAT-widening was right: docs
  RENDER current canon, not conformance-pinned — do not conflate). Specs recorded the consumer-notify
  obligation (they heads-up hive on vector changes). FLEET-TOOLING GAP flagged to supervisor: the shared
  pre-push bash hook has no allowlist-config → local re-vendor pushes still need FLEET_SKIP_SECRET_SCAN=1
  until the hook learns to read .gitleaks.toml [allowlist].paths. SUPERVISOR OWNS THE FIX (confirmed at
  source claude-fleet/hooks/git/pre-push; track-2, fail-safe allowlist-read + fleet-wide re-sync + falsifier)
  — NOT hive's to chase; the manual FLEET_SKIP_SECRET_SCAN=1 is the sanctioned interim workaround.

## 📚 RUSTDOC SITE ✅ LIVE: **reality2.ai/r2-hive/** (task #51 CLOSED; Roy's rustdocs ask delivered)
- Run 28769099459 both jobs green; site 200; hygiene spot-check clean. /programmers/ SLOT FLIPPED +
  deployed + live-page-probed by specs (crossed with my announce — supervisor's GO beat it); two of Roy's
  three rustdoc slots delivered, composer remains. Template gotchas + wasm-surface inclusion banked on
  specs' record. Root causes closed: Roy's R2_CORE_READ_TOKEN (the structural finding is FIXED —
  this workspace now compiles on hosted runners) + the github-pages ENVIRONMENT branch policy needed
  platform-trait explicitly allowed (template gotcha — bit core too; on the record for composer's
  replication). FOLLOW-ON OPENED (task #55, not Roy-gated): a real build/test ci.yml is now possible —
  hosted suite verification both modes + wasm; upgrades bump-core.sh's claim from local-green to
  hosted-green. Schedule at the next between-blocks slot.

## 📚 (superseded — the build/blocked record) RUSTDOC SITE (task #51): BUILT, BLOCKED ON ONE ROY CREDENTIAL
- Workflow live (.github/workflows/docs.yml, f9b53e5 + 9cb28d5): core's template replicated — two-build
  split (public-API Pages / org-only internals artifact), render-level hard-fail hygiene gate verbatim,
  false-green guard docs the workspace-EXCLUDED r2-hive-wasm into the same tree, deploy branch-gated to
  **platform-trait deliberately** (my canonical-branch call; flip-to-main = an item on the Roy-gated
  main-merge checklist). Local pre-verify: public + wasm + internals doc builds green; all four hard-fail
  classes CLEAN against the actual render.
- FIRST RUN (28768940350) SURFACED A STRUCTURAL FINDING: r2-hive's hosted CI has been hygiene-scan-ONLY
  forever — the docs job is the first thing to ever compile this workspace on a runner, and it failed at
  cargo's git fetch of the PRIVATE r2-core dep (default GITHUB_TOKEN is repo-scoped).
- UNBLOCK (Roy-minted, escalated via supervisor with exact spec): Actions secret **R2_CORE_READ_TOKEN**
  (fine-grained PAT → reality2-ai/r2-core, Contents: Read-only). Workflow already consumes it (insteadOf
  config, both jobs) — until the secret exists it is HONESTLY RED, not skipped.
  STATUS: Pages on r2-hive ENABLED by supervisor (build_type=workflow, future URL reality2.ai/r2-hive/);
  token with Roy (one mint serves BOTH repos — composer's R2_CORE_RO_TOKEN closes their F7/#42 too);
  supervisor re-runs the docs workflow the moment Roy says done. My structural finding (hosted CI never
  compiled this workspace) is on Roy's record; the same token is the fix.
  On green: announce URL + specs flips the /programmers/ slot.
- Same token = prereq for ever having a real build/test ci.yml on this repo (all suite-green claims to
  date are local-only — correctly stated as such, but hosted-unverified).

**SCOPE FENCE (specs c26d1b3, via supervisor-codex 2026-07-06):** B3 closed — local multihop WITHIN an island
stays required; global mesh-multihop through stranger devices is explicitly NOT required; world-crossing =
Internet relay only. Do NOT chase cold-reach-a-stranger/global-mesh-multihop as hive work unless Roy reopens.
(Checked against the queue same-day: #31/#32/§3A viability arms are all island-local; bridge/Pillar-2 = the
relay model. Nothing needed re-scoping.)
Companion wording addenda (R2-INTRO v0.8 / R2-ARCH v0.13: shouting-not-dialling, bounded reach as feature,
fit-or-route-elsewhere) swept same-day across hive+fw surfaces: no fixed-endpoint-dialing or broadcast-only
wording found — "dialect"=score format, "dial"=§2.3C knob sense, "point-to-point"=accurate PHY/link descriptions
(USB/CoC/harness). No doc or UI change owed.
R2-INTRO v0.9 (da509f1: FAR+MUCH = local ad-hoc radio + infra UDP/IP e.g. GSM/satellite; bearer-metadata
exposure = honest cost of infra reach) swept same-day: ZERO global-mesh/worldwide framing on any hive/fw/rak
surface, and zero long-reach-bearer mentions to reframe. No change owed.

**TASK #52 MANDATE SHARPENED then CORRECTED same-day (supervisor-codex, 2026-07-06):** final boundary per
R2-WIRE v0.33 §8.2a: core adds a route-layer replay guard for TARGET-ME DIRECTED deliveries (keyed
origin,msg_id); BROADCAST duplicate delivery remains effect-layer residual (route-level dedup marking for it
rejected — re-opens transit-censorship risk). My obligation UNCHANGED either way: IdempotencyGuard keyed
(origin,msg_id) BEFORE sentant/effect handling on EVERY arm (wasm handleRx / Linux router deliver / fw
io_task), then DispatchEnvelope+trust context as assigned. Do NOT lean on receiver-FSM idempotency — specs
found R2-SENTANT does not guarantee it. Correctness-MANDATORY. Acceptance checks owed: broadcast replay →
single effect on each arm (mine); directed target-me replay → blocked by core's §8.2a guard (verify at uptake).
Exposure note stands until #52 lands: no current hive arm calls the guard (fbee20d postdates pin 9943448), so
replayed broadcast frames double-fire effects on current builds — flagged to supervisor-codex for release
timing. (#52 still HELD for Roy's plan review.)
Cluster-II(A) signed off (round-9 clean, hosted green) — uptake follow-ons banked in #52+#32 metadata:
consume hold/SCF as applicable + relay_backoff_ms at every platform-IO frame-send site (fw io_task / Linux
router relay / wasm), replay boundary per §8.2a as above.

**CI OWNERSHIP BOUNDARY (supervisor-codex FYI, 2026-07-06; guard LANDED GREEN same-day at core 8cd230a,
hosted run 28690391458; carpark F2/F3/F4 harness primitive then signed off same-day at 3c49a40 / run
28692193276 — pure r2-harness behavioral aggregate testing, NOT firmware/RF simulation, so no firmware-facing
evidence ask materialized; GROUP_MGMT priority preservation now has a system falsifier there):**
2b BOOTLOADER: core's grounding pass ran and CORRECTLY STOPPED same-day — rollback-enabled bootloader ruled
metal/flashing/provisioning territory, not solo-core. PARKED as low-pri hive/composer-coordinated
production-hardening (ledger task #53) requiring human/firmware authority; interim = app-level
ota_confirm_or_rollback + documented residual, sufficient for current gating. Boundary facts staged for
whenever it wakes: (1) FLASHING/sign/key-mint = HUMAN-ONLY (Roy) — harness+fleet
gates, non-negotiable; (2) FlashSink/OTA apply backend = hive fw's (ImageSink trait per the OTA per-platform
sink model; verify = shared r2_update); (3) flash sector registry 0x12000-0x1B000 incl. persona@0x12000 +
TG@0x14000 + RoleProfile@0x17000 is load-bearing for mode-flip/provisioning semantics (v0.4 persona catch);
(4) wasm OTA today = verify+stage only, NO persistent anti-rollback — any custom-rollback design must not
assume one exists there. No action until core's scoped proposal returns.
core's xtensa CI guard covers ITS OWN DFR1195 skeleton only — a no_std API-drift build check over r2-wire/transport/route/discovery/sx1262 etc.
DISTINCT from the full DFR1195 firmware (FlashSink/OTA, dfr1195-fw branch), which stays HIVE-owned with NO
hosted CI (local-xtensa + peer-refute only — the say-it-distinctly rule). No hive action unless core's
skeleton exposes a boundary issue. Upside to note: core's guard will catch no_std API drift against the
vendored-crate set BEFORE my re-vendor cycles hit it.

## ✅ REFUTATION CONFIRMED — CORE ADOPTED THE COUNTER VERBATIM (2026-07-06, awaiting their green sha)
- Core: "your refutation is correct and severe" — their test only checked a PROD node (their own
  verify-then-record miss, owned). ADOPTING verbatim: Observation.build_mode → Option<BuildMode> (None = no
  declaration), sticky last-DECLARED on NeighbourEntry (None never clobbers Some), undeclared-ever entry
  MODE-TRANSPARENT in §4.4. Wire untouched. Core lands with a dev-node-undeclared-viable regression test +
  flags specs on canon wording + residual. I requested 3-case pin coverage: Dev+undeclared=viable /
  sticky-Dev survives None refresh / Prod+declared-Dev=non-viable.
- ✅ SPECS RULED SAME-DAY (R2-BUILDMODE v0.7, 64d8ab9): trichotomy CANON — same-declared viable /
  different-declared non-viable (incl unknowns) / NO observed declaration = MODE-TRANSPARENT. Specs'
  strengthening: default-to-Prod at the obs layer = the exact fail-open collapse v0.6 already forbade for
  unknown values, one layer down — core's shape was doubly wrong by canon's own logic. Absence MUST be
  representable; the two absence states (no-byte-in-observed-beacon = declared prod vs no-observation =
  transparent) MUST stay distinguishable. Beacon-to-engine feeds = SHOULD (tightening, not precondition).
  Roy-bounceable flagged (v0.2 carriage precedent); fallback = declaration-on-every-path — representable
  absence is needed in BOTH worlds, so the Option shape + my None call sites are bounce-proof.
- ✅ WIRED ALL THREE TIERS SAME-DAY (core's corrected sha d01725d, specs ground-truth-verified + my requested
  sticky regression included): HOST 1431997 — pin b420fb3→d01725d (manual move, breaking API rides the same
  commit), MY_BUILD_MODE cfg-fold → RouteEngine::new, all frame-formed obs None, wasm Dev everywhere (ctors +
  DataPlane 6th param); 16 suites both modes + wasm32 green. FW c638693 — COHERENT VENDORED-SET move
  (r2-route + r2-transport [WifiMesh rename + §2.2B features] + r2-dataplane + r2-wire [wifi module restored]
  + r2-discovery [TV5/TV6 vector tests = the banked pickup]; every crate verified zero-local-edits vs the
  1275732 baseline before copying; adopted core's route_stack Occam cut); own mode = from_wire(BUILD_CLASS);
  HB obs None; **MODE_DECL declaration feed LIVE** (scan handler → io_task → engine, Some(from_wire);
  resolve_rbid_windowed is trust-registry-scoped so strangers cannot label members); LoRa declaration
  documented blocked on the §6.1 rbid→hive resolver. Six local-xtensa arms green; vendored suites
  hosted-green at d01725d (distinct claims).
- TRAIL v0.64/v0.65 PORT (forced by re-vendor): msg_id u16→u32, TYPE-field is_reply gate (3d43838 fw
  sibling), NO_SUCCESSOR at origination AND broadcast relay — honest WEAK-MODE: fw replies are stackless so
  §4.6.1 retrace cannot fire regardless; old sender-credited strong-reinforce was the spoofable behavior
  v0.64 killed (not preserved). fw v0.65 stack-carrying reply = follow-on (rides with #32/#45 sibling).
- ✅ v0.8 RULED (specs e0d3434): my trust-registry-scoped-attachment observation generalized into canon —
  §3A.2 SHOULD: declaration-to-entry attachment identity-bound at least as strongly as the entry's own
  formation evidence (one node's frame never re-labels another's entry). The fw MODE_DECL feed is cited as
  the FIRST SHIPPED INSTANCE (verified in-code at c638693 before ruling) and CLOSES the v0.5
  forged-dev-beacon-demotion DoS lever on that path (residual → closed case). Deliberately SHOULD not MUST
  (weaker feeds still better than none; v0.5 pricing governs them). PRICING NOTE for the future Linux-scanner
  feed: it ships as the weaker-attribution instance (provisional-id, no registry resolution) under v0.5
  pricing — state distinctly in that wiring commit; do NOT borrow the fw feed's closed-case status.
  Specs' record: **#50 hive-side SHIPPED IN FULL.**
- ✅ §3A ARMS REALIZED SAME-DAY (core 3a835a5 landed both bytes): NEGOTIATION arm BUILT (fw b27c83c) — the
  refusal lives at poll_scan INGEST in my radio façade (declared-cross-mode never rosters → never electable
  → never offered; mutual, structural both modes). The three synthetic keep-alive push_scan_obs callers ride
  as internal-Option UNDECLARED = transparent (v0.7) — no fabricated declaration; engine ride-along byte =
  self-mode placeholder (never consumed; a prod-claim would misdeclare on dev benches). Six arms green.
  HOST same cycle: pin → 3a835a5 (91d04f2, bump-core.sh clean) + Linux scanner feed now DECLARES (6a39a29,
  Some(from_wire(obs.build_class)) — stated in-code as the WEAKER-attribution v0.5-priced instance).
- ✅✅ BOTH BLESSED — R2-BUILDMODE v0.9 (specs f712759): **§3A COMPLETE on the dfr1195 tier; #50 CLOSES
  END-TO-END there** (specs' record + mine). CoC-accept blessed as a GENERAL placement rule with a
  sharpening I own if it fires: refusal ATTACHES AT FIRST ATTRIBUTABILITY (a live anonymous session later
  attributed to an opposite-mode declaration MUST be refused from that point — deferred, never waived;
  banked in-code at the accept site, 8e60241). Provision-accept vacuity blessed with §3A.3(5): (a) MUST
  trigger-pin — any future over-mesh provisioning path lands its arm IN THE SAME CHANGE (inherited onto #43);
  (b) the wired path's invariant TRANSFERS TO THE OPERATOR — SHOULD: console echoes own mode at the decision
  point → SHIPPED same-turn (8e60241, PROVISION-APPLIED now prints build_mode; it did not before).
  #50 residue (independent of §3A): routetest split + recipe stamps (mine), rak4630 inc-2 dev+vendored-set.
- ⚠️ CoC-CLOSURE CAVEAT (core refuted my transitive claim; CONFIRMED in my own code same-hour): the
  control-served path bypasses scan-ingest — inbound CoC accept is unconditional, CTRL_IN has no
  scan-precondition (pinned-bench path labels anonymous centrals with the constant peer_hive), and core's
  handle_control does NO sender validation → WifiReq hands creds to peers the §3A.1 filter never saw. Canon
  UNAFFECTED (attributability qualifier stands; claimed hive_id ≠ declaration; unscanned = transparent).
  FIX PAIR agreed with core: WifiOffer only-from-elected-provider (core has ready) + WifiReq
  only-for-ROSTERED-requesters (requested — makes scan-ingest the structurally mandatory creds gate).
  RE-VERIFY the closure on uptake of core's sha; specs' record carries the caveat meanwhile. Hole class =
  roster/creds discipline, not mode-inference: presence only, no viability/delivery gain (GroupHmac gates).
  ESCALATED (specs adversarial check + carriage angle): code-pinned answers delivered — main.rs:3370
  (serve_coc labels CTRL_IN with the CALLER-supplied peer = pinned-bench constant → identity FABRICATED for
  anonymous centrals), :3981 (poll_control drains verbatim), :3217 (accept unconditional). LAYERED FIX:
  L1 core WifiOffer guard + L2 engine WifiReq roster gate + L3 mine.
  ✅ L3 BLESSED (v0.10, specs d4fd6f7 — my three-layer shape = the REQUIRED REALIZATION; v0.9's refuted
  transitive sentence REWRITTEN in canon with my file:line pins) and BUILT same-cycle (fw 884f424):
  CTRL_IN identity attaches ONLY via scan-resolution (boot-scoped SCAN_RESOLVED set = resolve_rbid_windowed
  successes, the v0.8 bar); identity-less control frames DROP ENTIRELY (disposition stated as ruled) with a
  NEG-CTRL-DROP observable line; the pinned-bench label is RULED §5.1-class and cfg'd dev-only — a PROD
  image drops ALL inbound control on this path fail-closed until the R2-PROVISION in-session identity
  exchange lands (canon now records that mechanism as BOTH the session-binding fix AND the v0.9
  first-attributability event — one mechanism, both jobs; carry the dual framing into the PROVISION
  proposal). PROD-ble arm (blemesh alone) verified specifically to exercise the None fail-closed path.
  ✅ ALL THREE LAYERS COMPLETE + UPTAKEN (same-day): L1+L2 = core 41adbd1 (falsification-verified — neuter
  either guard and its regression test reds); L3 = fw 884f424; uptake = fw e2f0e96 + hive pin 01e3e48.
  COMPOSED CHAIN re-verified as fact: identity-less control drops (L3; prod drops ALL) → §3A.1 filters
  declared-cross-mode at poll_scan BEFORE rostering → roster populated ONLY by poll_scan → is_rostered
  gates creds (L2) → provider guard stops rogue-AP joins (L1). Scan-ingest = the structurally mandatory
  carriage gate.
- ✅✅✅ **UNQUALIFIED — R2-BUILDMODE v0.11 (specs 042ee74): dfr1195 §3A closure stands CLEAN on all three
  records.** Caveat DISCHARGED in canon (converted to the landed record; every sha verified in specs'
  sibling checkouts: 41adbd1 / 884f424 / e2f0e96 / 01e3e48). The load-bearing sentence is canon verbatim:
  "scan ingest — where the §3A.1 mode filter lives — is load-bearing for AP carriage AS FACT." Residual
  scoped to exactly the named live-session-binding item; the PROVISION-lineage handshake recorded as both
  its fix and the first-attributability trigger. Specs' ledger note: the arc = the method working (core
  refuted specs' blessing premise → I confirmed against myself with file:line → canon corrected not
  defended → three layers across two repos same-day → composed re-verify → discharge).
  **#50 residue is now ONLY: routetest telemetry split + recipe-card mode stamps (mine, scheduled) +
  rak4630 dev feature & vendored-set move to 41adbd1 at #44 inc-2.** Specs RECORDED L3 (884f424 verified in-worktree, all four
  sharpenings confirmed) + weighed the composed PROD consequence ON THE RECORD as an ACCEPTABLE named
  capability cost: bulk SoftAP = optional Mode-1b on-demand path, common case elects nothing, so PROD
  loses an optional optimization fail-closed — named not hidden; the PROVISION-lineage handshake is the
  unlock. (Answer to any future "why doesn't prod negotiate over CoC": it's a ruled cost, not a bug.)
- (superseded by the blessing above) §3A REMAINING PLACEMENTS CLOSED BY ANALYSIS (with specs for blessing):
  CoC-connect = transitively covered in the negotiated flow (only rostered peers reach CoC) + structurally
  IMPOSSIBLE for anonymous inbound accepts (BLE address opacity → no peer identity at connect; anonymous
  sessions are governed by what they can DO — admission + deliver gates). Provision-accept = VACUOUS today
  (sole producer = wired console PROVISION verb, physical-possession operator authority; no over-mesh
  provisioning path exists — arm becomes real when one does, #43 lineage). Specs bless → #50 closes
  end-to-end; refute → I build the named gap.
- REMAINING #50 (mine-owned): routetest telemetry split (DESIGNED — see next block), recipe-card mode
  stamps; rak4630 dev feature + the same coherent vendored-set move at #44 inc-2 (its vendored crates
  predate d01725d too).

## 🔀 ROUTETEST SPLIT DESIGNED + REFUTATION-QUEUED (fw 25c02c2, the last #50 residual, NOT executed)
- Analyzed all 47 cfg(routetest) sites. The relay BEHAVIOUR is already UNGATED (every build relays);
  routetest gates 4 concerns: (1) RT test-harness, (2) bench topology mask, (3) msg.* telemetry emission —
  all cleanly dev-class — + (4) fr_origin, the per-(origin,msg_id) multi-hop dedup SEED, field-relevant but
  ROUTETEST_HASH-tied with a not(routetest)⇒fr_origin=0 fallback.
- DESIGN (docs/routetest-split-design.md): bisect routetest → `meshrelay` (field, non-dev, owns bucket 4) +
  `routetest` (=meshrelay+dev, buckets 1-3). Then loraroute/D4 are pure field-relay non-dev, the RT harness
  declares dev, and field×routetest = field×dev = compile_error stays coherent.
- **KEY FINDING (verify-then-record): the clean split is COUPLED to task #32.** Bucket 4's fr_origin is
  extracted ONLY for ROUTETEST_HASH frames — a test-frame-specific hack whose principled home is #32's
  route_stack[0] dedup. Recommend OPTION B (do routetest-split as the telemetry/harness half of #32, one
  refactor sharing the fr_origin seam) over OPTION A (split now, keep the ROUTETEST_HASH hack inside a
  field-named feature = a fresh dirty-split of the same kind).
- ✅ hive-codex REFUTATION CONFIRMED against the live worktree (fw 9631761): bucket 4 was under-scoped as
  "dedup seed" — CORRECTED to "field relay identity/origin plumbing". Ground truth: ROUTETEST_HASH is already
  cfg(any(routetest,fr4)) (the fr4 SCF-hold gate main.rs:2059 consumes it as the app-traffic discriminator);
  fr_origin feeds relay dedup AND the fr4 SCF msg.hold telemetry (2077); msg.silence is field-clean (keys
  sensor_seen, not fr_origin). meshrelay must lift ALL of fr_origin (+not(meshrelay)⇒0 fallback for
  fr4-standalone) / ROUTETEST_HASH-share / relay-fingerprint together — never just the dedup line. The
  refutation HARDENED Option B: ROUTETEST_HASH-as-discriminator IS the #32-class problem, so routetest-split
  lands as the telemetry/harness half of #32.
- HOLD execution on a SCHEDULING CALL (surfaced to supervisor): do routetest-split WITH #32 (Option B,
  recommended — shared fr_origin seam) vs ship the interim Option A lift now. Refute-before-execute honored;
  design is refutation-hardened and ready to build the instant the B-vs-A ruling lands.

## 🛑 §4.4 API LANDED (core a5d2d7e) BUT HELD — MY IMPLEMENTATION-REFUTATION IN FLIGHT (2026-07-06)
- Core landed the BuildMode API (enum+Other(u8) ✓, from_wire ✓, ctor arg ✓, viability equality in
  try_directed+build_flood_plan ✓, getters ✓ — all as converged). BUT the realization made
  Observation.build_mode a REQUIRED field defaulting Prod on frame-formed observations, and the evidence
  shows NO tier feeds beacon-declared mode into the engine today: fw main.rs:1654 = the ONLY fw ingest site
  (HEARTBEAT-formed; beacon decodes feed negotiation/dashboard, never the engine); core's own sync_host.rs =
  7 ingest sites, all frame-formed; my Linux router obs = frame-formed, bearer has no declaration channel.
- CONSEQUENCE (why I refused to wire it): every Dev-built engine (bench boards, bench Linux boxes, the wasm
  hive — canonically a DEV device) sees ALL neighbours as Prod → equality fails → try_directed skips all +
  flood finds zero → Drop(NoViableNeighbour) on everything → every dev mesh dies on uptake. Only all-prod
  meshes survive. Core's don't-downgrade offer is necessary but insufficient (never-declared entries read
  Prod forever).
- COUNTER-PROPOSAL (to core; canon nuance to specs): Observation.build_mode → Option<BuildMode>; None = this
  observation carries no declaration; entry keeps sticky last-DECLARED mode; never-declared entries are
  MODE-TRANSPARENT in §4.4 viability. Key distinction argued: absence-of-the-BYTE in an observed beacon =
  declared prod (ruled, retroactive, stands); absence of ANY beacon observation is NOT a declaration.
  §3A safety unaffected (refusal arms sit where declarations exist by construction); honest residual flagged
  (a never-beacon-ingesting prod engine can't demote a dev neighbour via §4.4 alone — admission still
  excludes).
- CROSSING NOTE: core's hosted-green announcement (a5d2d7e ci SUCCESS) crossed with my refutation; I
  re-pointed them at the queued evidence + answered their refresh-guard re-offer (insufficient alone —
  never-declared entries default Prod forever). API SHAPE is NOT in dispute, only undeclared-defaults-to-Prod.
- HOLDS until core+specs converge: NO r2-hive bump past b420fb3, NO fw r2-route re-vendor. Wiring plan
  pre-agreed on acceptance: fw beacon RX upserts Some(from_wire(byte)) (LoRa p[16] + BLE AD-22), HB passes
  None; Linux/wasm pass None everywhere; ctors = Dev under dev feature, wasm Dev always.

## 🧭 R2-BUILDMODE §4.4 VIABILITY API IN FLIGHT (2026-07-06 — core proposed, I ack'd with ONE counter)
- Core proposed the r2-route mode-viability shape (the gate on my §3A drop arms): BuildMode on Observation +
  NeighbourEntry (resolved at MY decoder, absence-is-prod there — r2-route never guesses), own-mode on the engine,
  equality added to the is_viable gate in try_directed + build_flood_plan (selection-not-formation, §2.1.3 stands),
  getters (NeighbourEntry::build_mode / RouteEngine::my_build_mode) so the dataplane frame-DROP arms stay my half.
- ACKED the shape; COUNTERED the two-variant enum: unknown wire values (the deliberately-skipped 0x01, a future
  0x03) must NOT collapse to Prod — that would make an undefined class silently viable inside prod meshes. Asked
  for raw u8 OR BuildMode{Prod,Dev,Other(u8)} (my preference); plain equality is then fail-closed AND
  forward-compatible for free (class-N nodes route among themselves, non-viable to both shipped modes).
- (b) answered: CTOR ARG and NO setter at all (v0.4 persona catch — mode flip = reflash + RE-PROVISION; a runtime
  set_my_build_mode is a mutation surface canon forbids). (c): closed set today (the legacy-BLE gap was the
  missing codec FIELD, ruled v0.26/v0.27, not a third mode) but size for extension via Other(u8).
- My decode mapping committed in the ack: LoRa len15/16→Prod; len17 p[16]=0x02→Dev, 0x00→Prod (belt-and-braces,
  my emitter never sends 17B-prod), else Other; BLE AD-offset-22 byte gets the same mapping.
- ✅ RULED SAME-DAY (R2-BUILDMODE v0.6, specs HEAD): unknown-not-Prod blessed AS PROPOSED. Mode homogeneity =
  EQUALITY on the declared class value; unknown values (reserved 0x01, future 0x03) are cross-class to BOTH
  shipped modes — non-viable + refused at connection/admission exactly like dev-vs-prod; same-value peers
  mutually viable; unknown-collapse-to-prod explicitly FORBIDDEN as fail-open. Other(u8)+plain-equality = the
  canon-named realization. FUTURE-PROOFING PIN: if a reserved value is ever ACTIVATED (0x01 prod-bench etc.),
  its interop relation gets ruled explicitly AT ACTIVATION — equality is only the meanwhile default, nothing
  pre-judged. Also folded: the §4 wasm-bridge registry row now names r2-hive-wasm's wasm-bindgen surface as THE
  observability seam (Roy-confirmed 3-way wording). Core relayed — nothing waits on specs; API can land.
- When core lands the r2-route side: wire the Observation feed + §3A drop arms same day (task #50c).
- ✅ BLE HALF CLOSED SAME-DAY (core codec b420fb3 → my emit fw 37f23b1 → vector to specs, one cycle):
  re-vendored r2-discovery wholesale from b420fb3; build_class = BUILD_CLASS set at the LegacyBeacon
  construction; SIX local-xtensa arms green (incl. bare field — see below — and the canonical
  field,carrier,loraroute compose) + field×dev gate still fires. Conformance vector generated via the REAL
  codec (scratchpad host crate, asserts: AD-22 placement, round-trips, dev/prod differ in exactly one byte,
  18B truncation decodes prod) and delivered to specs with all inputs stated (demo hk 0xA5×32, hive 0x480E900E,
  epoch 0, sensor class 0x43895E89). LATENT-BREAK FIX en route (got.3 class, PRE-EXISTING — proven red at
  715064a): mesh_broadcast was cfg(ble) but the fr4 SCF-FWD call is carrier-agnostic → bare field never
  compiled AND pure-LoRa field composes had no SCF forward; gate now any(ble,loraroute) + ble-gated ESP-NOW
  arm + carrier-less no-op stub. Honesty correction on the old record: the 715064a "field-prod" arm evidently
  composed carriers; bare field was NEVER green before this.
- ✅ PIN BUMPED 9943448 → b420fb3 (bump-core.sh, CI-green gated, full suite + wasm + hygiene green, 5c7de73).
  Side effect: #52's prereq is SATISFIED (b420fb3 contains fbee20d — ancestry verified); only Roy's plan-review
  hold remains on the claim-11 assembly.
- ✅ TV6 STAMPED (R2-BEACON v0.31): specs independently recomputed my ENTIRE derivation chain byte-exact
  (session_key HKDF, RBID HMAC, every §7.3 offset, flags 0x04) and stamped the owner-bytes vector as canon —
  "the better of the two legacy vectors" (TV5 = core synthetic). Legacy-BLE declaration CLOSED end-to-end:
  codec + emitter + vectors, all cross-verified. Follow-up banked: core 3f053c9 (specs-vector re-vendor) has
  CI in_progress — pick up TV5/TV6 byte-anchored tests at the NEXT wholesale r2-discovery re-vendor / pin bump
  once green.
- Still waiting on #50: core's r2-route BuildMode API (→ same-day §3A wiring); routetest telemetry split =
  MINE + unblocked; recipe-card mode stamps; rak4630 dev feature at inc-2.

**PILLAR-2 LOOPBACK SUPPORT POSTURE (composer drives; I sanity-check, 2026-07-06):** composer builds the
3-hive loopback proof (WS bridge as wire, UDS mgmt feed for live delivery, honest bridge-not-P2P badge). TWO
seams flagged to them PROACTIVELY: (1) R2_GROUP_KEYS_BENCH is COMPILED OUT of prod binaries since dc9d4ae —
their B2 config-only plan needs hives built --features dev (version string +dev suffix = the check); (2) UDS
socket choice confirmed right (/r2/mgmt WS web-auth has NO prod bypass BY DESIGN; --web-dev-mode is
dev-feature-only). No hive build task unless they hit a real API seam.

**✅ B2b DENY-EVENT CLOSED (task #54 done, bb17f5e):** specs ratified R2-HOST-API v0.4 (d057780) — and the
FINDING: the implementation ALREADY EXISTED (deny_inbound + build_denied_frame + three router reject arms,
from the earlier deliver-gate batch; the spec's §3.2.1 ratifies the shipped shape — my "proposal" crossed
with standing canon). Today's delta = point-for-point verification vs the ratified text (key map 0/2/3/5/7
+ 8/9, omissions 1/4/6 pin-tested; taxonomy verbatim grounded in classify_extended_full; Relay no-denies;
unkeyed opt-in delivers-never-denies; ships BOTH modes) + the missing ACCEPTANCE PROOF: new flow test —
a reject actually arrives on subscriber channels per §3.2.1 match rules (denied-class-filtered receives /
from_tg-filtered NEVER matches / broadcast distinguishes by class hash). 109 lib tests green both modes.
Composer sent consumption guidance incl. the key-state nuance (forgery needs the TG key HELD — their B2
must load the bench TG in a dev build; zero-keys denies fail_closed; never filter the RED feed by from_tg).
Wasm half not owed (UDS is the loopback surface; wasm has RxDisposition visibility from #36).
POST-CLOSE RULINGS (specs, same-day): (b) my ambiguity-qualifier suggestion REFUTED and accepted — H=0 is a
deterministic frame discriminator; within forgery, wrong-key/corrupted/zeroed are cryptographically
INDISTINGUISHABLE, so the coarse taxonomy reports exactly what the gate knows (my 3-way triage stays a
BENCH tool, not wire). (c) RULED MY WAY at v0.6 (specs 915d862) — LOAD-BEARING BUILDMODE PRECEDENT: an
R2-HOST-API event class is a §5 STANDARD PROTOCOL SURFACE (ships BOTH modes, MUST NOT be compiled out of
prod), vs §4 registered DEV surfaces (e.g. my queued nz.r2.diag responder #41 sits on THAT side). Rate-
bounding = MAY in both modes. STANDING DUTY: composer holds the RED renderer until core's adversarial pass
over the deny CBOR clears — I own SAME-DAY relay of any wire-detail change from that pass (none expected;
map is pin-tested against the ratified text). Specs ACKED closure (bb17f5e verified in-repo; from_tg match
rule = "exactly the right reading"). FORWARD HOOK banked on #41: §4 DEV surfaces carry a register-BEFORE-
shipping MUST — before nz.r2.diag implementation, specs gets surface description + proposed gate (reach:
local-only v1 lean vs on-mesh TG-gated later) and registers the §4 row FIRST.

## 🔒 R2-BUILDMODE §5.1 LINUX HALF SHIPPED (task #50 — the flip-a-flag class killed)
- New `dev` cargo feature on r2-hive-bin (default = PROD). Prod builds COMPILE OUT all five runtime security
  bypasses: --web-dev-mode, --usb-auto-confirm-unsafe, --usb-allow-any, R2_DELIVER_UNKEYED_OPEN, and
  R2_GROUP_KEYS_BENCH (the fifth was my addition — same class, specs' list had four). Structural absence proven
  observable: prod binary answers "unexpected argument" to --web-dev-mode; no env read exists in the image.
- CONSEQUENCE (deliberate, documented in Cargo.toml): a PROD-built Linux daemon today is UNKEYED + fail-closed
  (relay-only) until R2-KEYSTORE §4 sealed custody lands. FR-2b/bench boxes build --features dev.
- §6.3: version string mode-stamped (BUILD_MODE_VERSION: bare semver = prod / "+dev" suffix = dev) — flows through
  daemon.status + logs = runtime which-code-was-flashed declaration. §6.2 n/a on Linux (dev IS the selector).
- Tests: BOTH modes green (15 suites each). Three dev-bypass-dependent web tests gated cfg(feature="dev") — the
  six fail-closed web-auth assertions stay in the PROD suite (the prod-relevant ones). mgmt version assertion is
  mode-aware. Verification gate from here = run the suite BOTH modes.
- REMAINING #50 (fw side, awaiting specs §8.1 bytes + my build block): generalized prod×dev compile gate on
  dfr1195, build_class=2 emission (BLE + wasm + LoRa-17B once landed), mode-stamped fw artifact names.

## 🧬 FOLD CUTOVER DONE (r2-hive-core now lives in r2-core; my re-point+delete landed)
- Core landed the crate at my freeze d9d4429 into r2-core crates/r2-hive-core; I bumped BOTH manifests to
  9943448 (their CI-green sha — NOT bbd7771, which had silently dropped no_std; core caught it on bare-metal and
  added hive-core to their CI no-std cross-build), repointed r2-hive-bin (workspace dep) + wasm (git dep),
  retargeted router.rs's sync-twin pointer cross-repo, DELETED crates/r2-hive-core. Suite 16 green (hive-core's
  2 suites now run in core's tree), wasm 19/19 + wasm32 + fresh pkg (sha 5e8b04c6, 151449 B) for composer.
- The sync-twin pair is now CROSS-REPO (router.rs in r2-hive <-> sync_host.rs in r2-core): drift-guard =
  coordinate through core; both heads state it.

## 📌 CORE REV-PIN LANDED (task #49 DONE — deliberate uptake, Roy ratified)
- r2-hive now consumes r2-core as GIT DEPS pinned to ONE CI-green rev (785b3c4, core's r2-core-consolidation HEAD)
  in root [workspace.dependencies]; all 3 member crates inherit (13 dep declarations, feature shapes preserved:
  wire/engine default-features=false base, members re-enable). Live path-deps RETIRED — core's pushes no longer bite.
- Mechanics = core's recommended shape verbatim: git-dep(rev) > worktree (pin is repo-committed + can only target
  PUSHED revs); .cargo/config.toml git-fetch-with-cli (reuses gh creds, no deploy key); scripts/bump-core.sh =
  the only sanctioned pin move (refuses un-pushed/CI-red revs, atomic multi-line sed + consistency guard, commits
  only on full-suite+hygiene green; --force-ci escape documented for no-hosted-run cases); commented [patch] block
  in Cargo.toml = local-loop escape hatch for the fold migration (never commit uncommented).
- WASM PINNED TOO (same cycle): first wasm build against the host pin FAILED with dual-crate type mismatches
  (r2-hive-core resolved core via the git pin while wasm's own deps were still live-path = two r2_engines) —
  exactly the skew the interim note predicted, surfaced in minutes not weeks. Fixed by pinning wasm's manifest to
  the SAME rev (incl. r2-dataplane, wasm-only dep); bump-core.sh now moves BOTH manifests atomically + runs the
  wasm build in its gate (the WifiMesh-rename lesson codified). r2-hive-core dep stays path (in-repo until fold).
- 18 host suites + wasm 19/19 + wasm32 check green on the pin. Uptake protocol: core names a sha -> bump-core.sh <sha>.
  NO live coupling remains anywhere in r2-hive (fw branches were always vendored).

## 🛡️ /tmp FALLBACK GUARDS (R2-TG-TOOL §5.1 v0.4, specs 8ea8e22 — both MUSTs shipped)
- Specs REGISTERED my no-env fallback as canon (resolution order = default_socket_path verbatim, source-verified),
  then added two MUSTs the world-writable /tmp path introduces (foreign-UID pre-bind squat = daemon impersonation
  toward CLIENTS; the daemon-side same-UID accept check cannot protect the connecting side):
  (a) CLIENT peer-verify: r2hive-cli connect() now SO_PEERCRED-checks the listener's uid == ours whenever the path
  is the /tmp fallback (is_tmp_fallback_socket, shape-pinned by unit test; XDG/TMPDIR per-user paths exempt per canon).
  (b) DAEMON loud-fail: mgmt/socket.rs spawn() refuses to bind (PermissionDenied + SECURITY log) when the existing
  socket file is foreign-owned — never silently renames (silent rename would defeat the normative-filename ruling).
- REFUSAL ARM NOW RUNTIME-TESTED (coverage caveat RETIRED): specs suggested unshare --map-root-user; empirically
  REFUTED (own files map together with own uid — both read 0 inside the ns; mismatch never occurs). Simpler no-root
  construction found: the guard path is exists->stat->foreign-uid->refuse and file-type-agnostic, so spawn() against
  root-owned /proc/version fires it naturally — integration test squat_guard_refuses_foreign_owned_socket_path
  (root-skip guarded). No Roy recipe needed.

## 🔌 SOCKET FILENAME NORMATIVE (specs ruling fa94443 — fix_impl EXECUTED)
- Specs ruled my tranche-2b divergence flag: the mgmt-socket FILENAME is part of the R2-TG-TOOL §5.1 contract
  (well-known address = zero-config UI discoverability; path+0600+same-UID+filename = ONE contract, not layers).
- RENAMED r2-hive.sock -> r2tgd.sock everywhere (default_socket_path is the single behaviour site — daemon bind +
  r2hive-cli connect share it, cannot disagree; /tmp fallback co-renamed r2tgd-<uid>.sock; tests/docs/packaging swept).
  Doc claims of "filename is daemon-local" corrected in main.rs/mgmt/mod.rs/socket.rs heads with the canon cite.
- MIGRATION NOTE: any out-of-repo client hardcoding the old r2-hive.sock path breaks on next daemon restart — in-repo
  CLI moves in lockstep; composer uses /r2/mgmt WS (unaffected); carrier-bridge doesn't touch the UDS.

## 📖 DOCUMENTATION CAMPAIGN ACTIVE (task #48 — Roy's standing directive, 2026-07-06)
- **The standard (banked in memory roy-commenting-standard.md, OVERRIDES match-density):** file heads = why the file
  exists + grep-verified interlink map + canon refs (full r2-specifications paths); every fn = purpose + dependencies +
  used-by (grep-verified, never guessed); audience = first-time reader; inconsistencies fixed en route; **OCCAM
  (Roy's 4th directive): redundant code REMOVED, on evidence only** (zero callers + tests green; pub API consumed by
  other crates/wasm/JS counts as a caller). Core runs the same campaign (its batch-1 = 3345028, incl. an Occam cut of
  the dead route_stack module) — style aligned with core's convention (narrative why, full canon paths).
- **Tranche 1 (ca56477):** router.rs exemplar. Fixes: now_monotonic→now_unix_secs (wall-clock misnomer, NTP caveat);
  congested:false documented as the tracked §3A Linux-tier seam.
- **Tranche 2 (this commit):** hive.rs + main.rs to the standard. **Occam cuts (all evidence-verified):**
  (1) main.rs fnv1a_addr = byte-identical reimpl of r2_fnv::fnv1a_32 → replaced with the real crate call (same basis/
  prime/no-canonicalisation; self hive_id derivation UNCHANGED); (2) hex_decode/hex_encode duplicated verbatim in
  hive.rs + compat/handshake.rs → single pub(crate) copy in hive.rs; (3) clear_active_tg: zero callers incl. tests →
  removed (set_active_tg KEPT — mgmt_integration.rs:660 uses it; detach lands with the TG lifecycle flow);
  (4) main.rs dead `existed` computation (value discarded via let _) → removed; (5) unreachable post-loop log line in
  start_lora → removed; (6) dead group_r fn in examples/heartbeat_sync_sim.rs → removed.
- **Inconsistency FOUND + flagged in both file heads: "R2-HIVE §x.y" is cited 17× across the crate but NO R2-HIVE spec
  exists in r2-specifications** (specs/r2-core/README.md says so explicitly — implementation repo name, not canon).
  Heads now mark those as daemon-local design lineage; spec-gap question owed to specs.
- Remaining warning EXT_AUTH_MAX (never used) is in r2-wire = CORE's crate — flag to core, not mine to cut.
- **Tranche 3 (this commit):** sync_host.rs + wasm lib.rs + router↔sync cross-refs. sync_host head now names its
  wasm production caller + the task-#32 pending MCU consumer (poll_inbound documented as designed-surface-no-caller,
  same ruling as set_active_tg); router.rs and sync_host.rs heads now cross-reference each other as async/sync twins
  (MUST-NOT-drift pair). **Inconsistency FIXED in wasm lib.rs:** deliver_event's doc block + a stray duplicate
  #[wasm_bindgen] attribute were stranded on deliverEventQueued (task-#47 insertion artifact) — docs re-seated on
  their own fns, redundant attribute removed (binding surface byte-identical: 19/19 + wasm32 release green).
  handle_rx documented (was the only other doc-less pub fn). Wasm head upgraded to full standard (refutation-not-demo
  rationale + composer-consumer map + canon block).
- **Tranche 4 (this commit):** USB family (usb.rs 1810 / usb_hotplug 1110 / usb_serial 537 / usb_pair 421) — heads
  gained grep-verified interlink maps + canon blocks (these files were already inline-rich; only ONE doc-less pub fn
  existed across all four). Occam: encode_length_prefixed narrowed pub→pub(crate) (zero external users);
  build_sync_frame narrowed further to #[cfg(test)] — the narrowing EXPOSED a stale doc claiming production use via
  send_sync (send_sync frames its own SYNC; doc corrected). usb_pair's ellipsis canon path fixed to the real
  R2-PROVISION.md path.
- **Tranche 5 (this commit):** mgmt family — all 12 files (~4.2k lines). 28 doc-less pub fns documented (handlers +
  client builders, each with grep-verified used-by: api.rs dispatcher / r2hive-cli / integration tests); interlink+canon
  sections appended to all ten substantive heads (dispatcher topology now legible: socket+ws -> api -> namespace
  handlers -> HiveState). Occam: FileStore::path() CUT (zero callers anywhere); FileStore::exists() -> cfg(test)
  (test-only lifecycle probe). Inconsistency fixed: framing.rs cited "R2-HIVE spec §5.2" (missed by the tranche-2b
  grep — different phrasing) -> re-anchored to R2-HOST-API §2.2 len_be32.
- **Tranche 6 (this commit):** bin-crate TAIL — web/web_auth/autoconfig/config/compat(handshake+protocol+buffer)/
  plugins/platform/systemd/lib. 11 doc-less fns documented (systemd stubs, catchup ring, word-codes TTL store);
  interlink+canon heads on the five substantive files. **BIN CRATE NOW 100% at the standard.**
- **SCOPE CHANGE (Roy GO via supervisor):** r2-hive-core EXCLUDED from sweep — crate migrates INTO r2-core (core =
  receiving owner; sync_host travels pre-documented). NEW task #49 = rev-pin core deps + bump script (deliberate
  uptake, Roy ratified; mechanics asked of core — 11 path-dep'd crates today). Sequencing: pin lands BEFORE core's
  migration churn.
- **Tranche 7 (this commit):** r2hive-cli (1246 lines) — 34 fns documented (command runners with their verb
  semantics, CBOR field readers, renderers; role_name + session_state_label flagged as keep-in-sync mirrors of
  daemon wire values); head gained the pure-client interlink statement (build_* encoders shared with integration
  tests; §5.1 v0.4 peer-verify guard in connect) + canon block.
- **Tranche 8 (this commit):** carrier-bridge py + ws-mesh JS (1.55k lines, 13 files) — heads were already strong
  (DTR/RTS banner, no-gateway doctrine, unicast-only UDP note); pass added per-fn docs (11 py fns incl. the
  safety-critical open_safe + the no-serial-access router-child construction; gateway accept(); test mains +
  helpers). All syntax-verified (py_compile + node --check). NOTE: alfred's deployed bridge copy is now behind
  source by COMMENTS ONLY — sync at next functional change (sha-verify norm will flag it; deliberate).
- **Next tranches (fw branches, LAST):** dfr1195 main.rs (~5.9k, own tranche) + rak4630 delta. (usb/usb_hotplug/usb_serial/usb_pair) →
  web/web_auth/ensemble/ota/identity/config/autoconfig/systemd → r2-hive-core lib.rs + carrier-bridge py + ws-mesh →
  fw files on branch (dfr1195 main.rs = own tranche; rak4630 delta). Vendored crates EXCLUDED (canon docs = core's).
  One hygiene-gated commit + supervisor note per tranche. ALL new code ships to the standard.

## ✅ CARPARK BINDING SHIPPED (task #47 CLOSED — 5fe9f69, wasm 0.6.4, pkg cf06c2d0…; composer endorsed pre-build)
- Congestion: tick() drives the core sensor INTERNALLY from real bus depth/capacity (core's same-hour queue_depth/
  queue_capacity getters — landed with honest-theatre docs citing this binding); route_inbound_sync grew `congested`
  (hardcoded-false retired; 37/37 core green); congested() + relayBackoffMs getters; **deliverEventQueued** = the honest
  burst surface (found mid-build: deliver_event drains per call so backlog could never form — enqueue-only between-tick
  arrivals model what a real io_task sees). Falsifier: latch trips ≥25/32, hysteresis-clears on drain.
- Airtime: real bucket (starts FULL 3600 ms), refill per tick from real peer count, LoRa sends pay real SF12 ToA in
  route_frame, refused sends GATED OUT + counted (+ per-call airtime_refused JSON). Falsifier: budget dies <6 floods.
- GM pays airtime like everyone (composer AGREED: regulatory ≠ §3A never-damped; its F3 rhyme rescoped to the congestion
  axis). Capacity=32 answer delivered (latch at 25+, clear at <15). 19/19 wasm; composer builds scene+selftests next.

## 🅿️ CARPARK THEATRE BINDING = task #47 (designed + grounded, objections window open; build next block)
- Composer's §3A congestion + R2-LORA §4 airtime scene ask, core-blessed seam. Shape sent (one step MORE honest than
  asked): tick() drives the DataPlane sensor INTERNALLY from real bus depth (zero JS-supplied numbers — needs core's
  EventBus depth getter, asked, same-hour offer); congested() getter; route_inbound_sync grows `congested: bool`
  (replacing the hardcoded false); relayBackoffMs exposed (core's refute: THE bite on broadcast media). Airtime:
  real bucket from real neighbour count, LoRa sends pay real ToA (as923_nz params) inside route_frame, refused sends
  GATED OUT of sends[] + counted. NO setCongested. **Semantics flag raised: GROUP_MGMT does NOT bypass airtime
  (regulatory) unlike the §3A damper (F3 never-damped) — spec question if contested.** Full ground truth in task #47.

## 🧩 buildReplyFrame SHIPPED (wasm 0.6.3, 29c6013 — composer's C2b ask, same-hour)
- Composer found the real gap in my 0.6.2 emit set: no wasm method emitted a **Reply-TYPE** frame, and the is_reply
  anti-spoof gate (by design) grants only weak evidence to marker-in-Event — its 0.265→0.302 weak bump was the designed
  behaviour, empirically confirmed. `buildReplyFrame(target, eventHash, markerBytes, replySeq)` closes the JS loop:
  routeStackOf → replyMarkerWithStack → buildReplyFrame(replyMsgIdExt) → route_frame → STRONG retrace.
- End-to-end test through the wasm surface added (twin of the core-tier invariant + regression falsifier for the Reply
  type). 17/17 green, wasm32 clean, pkg sha 2ac6d98d…. No origination note on replies (in-flight ring stays
  request-only). Composer notified with the full adoption recipe.

## 📜 GATEWAY SPEC v0.5 LANDED (specs 375f0d0 — the promote_after_ms pin) + CODEC ADOPTED SAME-DAY (fw c0bd522)
- The §5.1.1 promotion trigger my #34 build question surfaced is canon: slot-0x01 layout = `[slot][promote_after_ms
  u32 LE — NEXT only][ad_bytes]`; relative countdown on local monotonic clock; expiry promotes atomically (zero boundary
  bridge traffic); 0 = stage-only; slot-0 overrides anytime; promotion consumes the slot; never-zero-beacons throughout.
  **My inc4 interim (current-slot + stage-only 0x01) is blessed CONFORMANT-DEGRADED in the spec text itself.**
- **r2-hw codec adopted the layout same-day** (c0bd522, pushed): typed `BeaconAd::Current / ::Next{promote_after_ms, ad}`;
  NEXT without the full 4-byte countdown = Malformed (never partial). Wire-safe break — no shipped emitter existed, and
  the fw dispatch ACKs BEACON_AD unparsed until inc4. 15/15 + no_std + radiofrontend xtensa green.

## 🌿 RAK BRANCH ESTABLISHED (core ruled: BRANCH MODEL — dfr1195-fw precedent; I am sole writer of rak4630-fw)
- **rak4630-fw branched @ 5100933** (core's pinout-VERIFIED commit, tip of its r2-core-consolidation line) + PUSHED;
  worktree `/home/roycdavies/Development/R2/rak4630-fw-wt`. **Baseline build GREEN in my worktree** (43.6 KiB flash
  sections, matches core's number) — the build loop is proven before any integration code.
- **First-light killer banked from 5100933:** P1.05 = the RF-switch POWER rail — HIGH for the node's whole life (RX AND
  TX; direction is chip-managed DIO2). The spike now drives it; event-driven RX would have heard nothing otherwise.
  Remaining bench unknown: DIO3 TCXO voltage (3.3 chosen / 3.0 alt; wrong pick = BusyTimeout, not damage).
- **Division ratified:** main's platforms/rak4630 stays core's decision instrument (memory.x slot gate + thumbv7em CI,
  run INSIDE the platform dir); my branch owns the integration delta; core's pre-push heads-up discipline now covers
  this platform's API surface. **BLE budget measurement is MINE**: send core `size -A` deltas when trouble+nrf-sdc first
  links — it folds the MEASURED figure into main's README ledger (replacing the ~150 KiB allowance). DIO1 async-Input
  endorsed; r2-sx1262 driver changes route through core (same-hour service).
- **inc-1 LANDED (rak4630-fw 4d69f5a, pushed):** event-driven RX — select3(DIO1 wait_for_high / outbound recv /
  100 ms housekeeping deadline) replaces the 5 ms poll; DIO1 level-high-until-cleared makes the wait race-free; drain
  loop empties all pending events before re-sleep; TxDone re-arms listen(). HWRNG fp_seed (16 B, bias-corrected) —
  all-zero const gone. **45,316 B = 9.2% of slot (+1.7 KiB vs baseline); thumbv7em green in-platform-dir.** Zero driver
  changes needed. Core folded inc-1 into main's README ledger (f80da11) + confirmed the DIO1 read matched driver intent.
- **inc-2 (BLE advertise) SURVEYED + PLANNED (start with fresh context — dependency engineering deserves a clean block):**
  GREENFIELD (no nrf-sdc/mpsl anywhere; nrf54 never did BLE). Trap pre-identified: nrf-sdc's embassy-nrf dep vs the
  workspace git pin (0.11.0 #56b52e66) = two-copy version soup → `[patch.crates-io]` in the PLATFORM manifest (own
  Cargo.lock, outside root workspace). mpsl claims RTC0/TIMER0 — time-driver-rtc1 already avoids the clash. Advertise-
  ONLY peripheral task via the (unused) _spawner; AD bytes fed from the existing beacon arm via a Watch; size -A deltas
  → core retires the ~150 KiB allowance. Full plan in task #44 metadata.

## 🔁 ROLES RESUMED + RAK #51 UNPAUSED + #45 SHIPPED (2026-07-05 late-night block)
- **First-responder returned to me** (quota recovered; composer covered and keeps its ready recipes — ACM3 flash-verify,
  cb87c8aa OTA push on green pre-flight, D4 board-info→csv — COORDINATE, don't duplicate). Roy's three bench gates
  unchanged: ACM3 flash done-signal, optional D4 4/8MB word, theatre acceptance.
- **Task #45 SHIPPED (3ac81b6, wasm 0.6.2, pkg sha f5d9d37a…):** replyMarkerWithStack + replyMarkerAuto (bearer-budget,
  never-truncate) + routeStackOf exports; roundtrip+budget tests 16/16; composer notified with adoption notes.
- **RAK #51 (= local #44) UNPAUSED — Phase-2 delta mapped from the spike source** (it's already a working keyless repeater
  in POLLING form): my delta = DIO1-async continuous-RX, trouble-host+nrf-sdc advertise, health+OTA ensemble, hwrng
  fp_seed, provisioning hooks. **Ownership seam ASKED of core** (rak4630-fw branch à la dfr1195-fw = my favoured, vs
  migrate-to-hive) — integration code HELD until core rules. **Falsifier peer prereq BUILD-PROVEN:** DFR
  `loraroute,multitg,viz,benchdist` compiles green (ELF sha 07b558d9…, stage-only).
- **Joint verdict: CO-SIGNED + DELIVERED to supervisor** (composer did it before my nudge — stale-view on my side).
  Its data half: **D1 4/4, D2 2/2 viable nbrs, both stable 60 s, D1↔D2 MUTUAL** — control and subject consistent on the
  rx side (richer than my counters-don't-discriminate prediction: on the engine viable-nbr table both look HEALTHY).
  Dark-board saga fully closed pending supervisor/Roy ack.
- **Blockers reduced (supervisor):** specs' write access RESTORED (R2-WIRE v0.39 TV5/TV6 stamped 23:22) → open gates =
  Roy's bench items + core's seam ruling (nudged). Checked: the resident-gateway **v0.5 edit has NOT flushed yet** (spec
  still v0.4) — non-blocking (#34 inc4 ships on the blessed v0.4 semantics); watch for the promote_after_ms landing. Composer adopts the 0.6.2 stack-markers in its C2b
  reply-trail sim at its next #21 touch (held behind its corpus re-audit; not urgent).

## ⚖️ JOINT VERDICT IN FLIGHT (supervisor requires hive+composer co-signature before Roy hears anything)
- Contradiction to resolve: my no-defect verdict vs composer's runtime-issue-persists. Supervisor proposed an rx-side
  nbrs-stability crux test — but I hold the CONTROL DATUM that voids it: **ACM3 (crypto-proven L5 member) shows
  `synced=false nbrs=0` IDENTICAL to D4 in today's captures.** Source ground truth: `nbrs` is formation-DECOUPLED
  (counts unverified peers, task #28 by design); the real rx key gate is HB-COUPLING verify; bench HBs are sparse
  (~1/20-25 s per board) so the WHOLE bench sits unsynced. A test where control == suspect carries no information.
- Draft joint verdict sent to composer (its half = D1-vs-D2 status samples from its own streams; prediction: identical
  patterns; if D2 materially differs from the D1 control we REOPEN honestly): no demonstrated runtime defect; GO stands
  on the dual-codec crypto proof; reimages = scheduled HYGIENE not fixes; **erase permanently OFF the table** (persona
  AND override both hold weave-hk byte-identical — composer's own datum); honest residual = a D2/D4-specific rx defect
  is not fully excludable until real-tagged ADDRESSED traffic exists (#39 or post-reimage), zero positive evidence for one.
- Awaiting composer's co-signature/amendment; the co-authored statement then goes to supervisor.

## 🔄 TOTAL FLIP — ALL KEYS WERE ALWAYS CORRECT; the whole red saga was task #39's zero-tag artifact (task #46 CLOSED)
- **Supervisor's file-epoch discriminator run live and it flipped everything:** captured on-air frames from two board
  mirrors, verified OFFLINE against composer's weave-hk.bin with the REAL r2-wire/r2-trust code. **Every board's HB
  signature verifies: D1 3/3, D2 3/3, D4 2/2, carrier 4/4** (HBs signed by the same GroupHmac the deliver-gate uses =
  the deliver-key proof, cryptographic + per-board). File-epoch hypothesis REFUTED; **my D4-wrong-key verdict RETRACTED**.
- **The real defect: all 71 captured req Events = origin 00000000 + ALL-ZERO 32 B tag** — task #39's known origination
  non-conformance (pre-ROUTE-ORIGIN-1 path; sign_extended's route-less zero-tag fallback) shipping in the flashed images.
  `hmac_ok=false` is THREE-WAY ambiguous (absent / zero / wrong-key tag) — every key signal in the saga (composer's
  post-PROVISION check, my DELIVER-BLOCKED reads, flat dlv) was reading the artifact. **The gates behaved perfectly
  throughout.** dlv-climb = the WRONG go criterion until #39 lands; the key box is GREEN by crypto proof.
- Bench restored: ACM3 `SENDTO 0` acked (note: ack still prints 'BL-200 origin' — verify reqs actually stopped at next
  read); throwaway probe deleted (never committed). D4 reimage stays worthwhile (live-swap + REBOOT verb) but NOT
  key-blocking. **#39 elevated with metal evidence** (top conformance item alongside #32).
- **LESSON (banked): on any hmac_ok=false, inspect the TAG BYTES first** — capture-mirror + offline-crate-verify is the
  standing instrument (method: R2RX hex → decode_extended/compact → verify against key-file bytes).
- **CONFOUND-KILL (supervisor's codec-version worry): re-ran the same 83 frames through the VENDORED r2-wire (the boards'
  own compiled codec) — byte-for-byte identical verdicts: 12/12 HBs real-tag verify=TRUE per board, 71/71 reqs
  origin=0 + zero-tag. Both probes deleted, fw worktree clean. The HB half was confound-proof anyway (a valid HMAC
  cannot arise from a wrong key/parse); now the req half is too. Flip verdict = double-grounded.**

## 🔬 D4/D2 DISCRIMINATION ROUND 2 (2026-07-05; task #46 updated; supervisor's three questions answered live)
- **REBOOT verb fired on ACM4 by me: NO-OP** (beats never reset, no ack) — D4's old image predates BOTH the verb and the
  live-install path (landed ~06-26 ebfa5c8 era). **So no verb bug exists in current firmware**: persist-without-live-install
  is the old image's designed behaviour; 29e250cf HAS the live swap. Agent-side paths for D4 = exhausted (toggle-reset
  forbidden; flash tool human-only).
- **⚠ OPTION-A ERASE IS NOW WRONG FOR D4** — composer's PROVISION WROTE the correct key to @0x14000 (byte-identical,
  read-back-confirmed); only LOADING it is missing. Erase would delete the right key and regress to the stale persona.
  Correct human action = Roy's ALREADY-PENDING D4 reimage (29e250cf, app-only): its reboot loads the key; zero new work.
  Sent to supervisor in the gate's escalation format (artifact/target/authority/reason).
- **D2 tightened toward wrong-key too:** retargeted ACM3's member-signed reqs at D2 (`SENDTO b14b07d8`, acked) — 50 s,
  ZERO acks, ACM3 dlv flat. Coherent story: D2's app-only reimage PRESERVED its stale @0x14000 override (NVS by design) →
  new image booted back into the old key; "held apiary" framing likely rationalized this. **D2 fix = composer console
  re-key on ACM5, installs LIVE on the new image (no reboot, no Roy). My ACM3→D2 stream LEFT ARMED as its self-verifier.**
- **Fleet-gate note:** my first status message tripped the firmware/key lexical gate on CONTENT (it mentions flashing/keys
  while requesting no agent operation) — resent in the gate's own escalation format. Not a policy violation; a lexical
  false-positive worth remembering when reporting flash-adjacent findings.
- Sequencing recorded: D2 greens on composer's action now; D4 greens at Roy's flash (I retarget the stream to 495b1b62
  just before his window); `SENDTO 0` restore after both proofs.

## 🔴 D4 RE-KEY REFUTED ON LIVE METAL (2026-07-05; task #46; BLOCKS Roy's 4-board GO)
- Supervisor asked for the deliver-gate proof status; I ran it live. ACM4 was free: baseline read showed identity right
  (495b1b62 / tg 04bc57e7), beats alive, dlv=0 — but VACUOUS (census: the only on-air traffic is D2→D1 directed reqs;
  nothing addressed to D4; D4 RELAYS them fine — relay is keyless, proves nothing). **Falsifier armed: ACM3 (09a07e47,
  L5-verified member = known-good signer) given `SENDTO 495b1b62` (acked) → addressed member-signed reqs every ~6 s.**
- **RESULT: D4 emits `DELIVER-BLOCKED msg_id=N tg_ok=true hmac_ok=false (relay unaffected)` on EVERY req** (msg_id 6,7,8…),
  dlv flat 0, no acks originate. **D4 still holds a WRONG KEY.** The interim "clean erase → 495b weave" acceptance was an
  on-air target_group observation, never a key proof. The gate itself = perfect (fail-closed, structured first-class red,
  zero log-scrape — the real-red rule vindicated end-to-end).
- **Fix path unchanged = Roy's ruled option-B PROVISION on ACM4 (composer executes my recipe). The armed stream is the
  self-verifier: dlv climbs within ~6 s of key install.** D2's proof = one datum from composer's stream (two D1 dlv samples
  30 s apart; its adapter holds ACM2/ACM5). **RESTORE DUTY (mine, after proofs): `SENDTO 0` on ACM3** (NVS-persisted;
  ACM3 = #49 target; app-only flash preserves it; coex mute covers OTA overlap — no conflict, but return bench to
  found-shape). Supervisor told: no 4-board GO until both boxes green; both minutes-scale once composer acts.

## ✅ RAK RADIO PLAN CLOSED (core aff9928): spike calls as923_nz() DIRECTLY (byte-identical exports, cannot drift); 42.5 KiB / 8.9%, verdict unchanged
- TCXO + pinout CORE-VERIFY markers remain for bring-up. Two engine gates recorded on task #44 for the falsifier's
  path-table assertions (re-verified green against aff9928: 37/37 + 15/15): reply legs MUST be MsgType::Reply frames
  (is_reply gate), and egress-masked transit lays NO trail evidence (carried gate reads FINAL relay truth) — so masked
  directions legitimately have NO path entries; arm-3's through-RAK attribution is cleaner for it, but don't assert
  entries on masked paths.

## 🔒 is_reply TYPE GATE ABSORBED (2026-07-05, third+final trail step, core 3d43838 codex-HIGH; mine = 4a51717 pushed)
- Reply-ness now rides the frame TYPE field: on_received gained in-signature `is_reply` (no call site can omit it) — kills
  the trail-poisoning lever where an authenticated Event with a marker-shaped payload spoofed a retraced reply,
  strong-reinforced, and CONSUMED a pending forwarded record. My one call site passes `header.msg_type == Reply`.
- My strong-reinforce invariant test WAS the exact masking (Event-typed reply frame) — switched to MsgType::Reply; it now
  doubles as the gate's regression falsifier on this tier. 37/37 + 15/15 + wasm32 clean; **wasm 0.6.1** (pkg sha 293f9144…)
  rebuilt; composer told (its sim replies must be Reply-typed / msg_type 2 or trails converge slower).
- `reply_marker_auto(origin, msg_id, stack, bearer_budget)` (v0.67 centralized bearer-budget fallback: full marker if it
  fits, else bare, never truncate — SF10/BW125 = 51 B bites) noted on task #45 for the emit side.

## 📬 BATCH-2 CLOSURE (2026-07-05 night): #49 correction ACCEPTED both supervisors; ADV theory refuted at source; specs WRITE-DARK (escalated); v0.65 = already aligned; BEACON_AD ruling in hand
- **#49 SETTLED:** my stale-artifact correction ACCEPTED by supervisor-codex AND supervisor (its 'sign ab1f1cb6' recommendation
  explicitly WITHDRAWN as stale-premise). Standing plan: Roy flashes ACM3 with `~/r2-dfr1195-weave-coex.elf` **29e250cf** (turnkey
  command in this file, by-id F4:12:FA:50:23:E4; `~/dfr1195-partitions.csv` verified present Jul 1); composer wrapper pre-flight =
  pull phase-3-hardware-tier ≥ fc817b3 (bounded retry + scanner-stop 61ad26d), then pushes `~/cb87c8aa-app.bin`. **Both open
  diagnostic branches answered from source and sent:** (1) ADV-contention REFUTED — ONE advertising set, consumed at accept(),
  serve runs inside the loop, re-advertise only after 'CoC closed' ⇒ no advertising while an OTA CoC is open, by construction
  (main.rs:3033-3083). (2) Coex claim true ONLY of the old running image — 3aae196 (ESP-NOW TX mute under OTA_ACTIVE) is inside
  29e250cf. Interim artifacts ab1f1cb6 (framing-only) + 296017c4 (defer-only, `~/r2-dfr1195-weave-defer.elf`) = superseded, do
  not flash. My first-responder watch unchanged: serial `OTA(L2CAP) start seq=` on ACM3 post-flash.
- **🚨 SPECS WRITE-DARK (escalated to supervisor pair):** python3/Read/Edit/fleet-send all prompt for approval on specs' side;
  reads OK; tree clean at 0ae1bd5; it reached me only via the ask-reply channel. The resident-gateway spec's **v0.5 edit is
  fully drafted** in its scratchpad and lands on access restoration. Needs Roy/fleet-root.
- **BEACON_AD SWITCH-TRIGGER RULED (content complete despite the outage; task #34 metadata carries the full text):** inc4
  plan BLESSED conformant-degraded (ship current-slot + stage-only 0x01); eventual pin = staged countdown `promote_after_ms`
  u32 LE on the slot-0x01 layout (local monotonic, 0 = stage-only, promotion consumes the slot, survives a sleeping brain =
  the literal no-round-trip promise); add the parse+countdown when v0.5 text lands. (b)-as-definition/(c)/(a) rejected.
- **v0.65 trail step (core f3b0715, supersedes v0.64): ALREADY ALIGNED** — my fc08e7a was built against the landed tree (the
  6-arg on_received I adapted to WAS the v0.65 shape; 37/37 re-verified green at f3b0715). Emit-side follow-up =
  **task #45** (replyMarkerWithStack in wasm; non-blocking — stackless markers lay weak evidence, nothing breaks).
- **Inbox hygiene note:** `fleet inbox` retains months of processed history (the consolidation/relay-v0.2 era) — read the TAIL
  for new items; do not re-action old arcs (relay v0.2 handshake work etc. was a PRIOR era, largely superseded).
- **RAK4630 gate LIFTED (core Phase-1 spike eef3baf: 42.3 KiB / 480 KiB slot, 8.8%, full TN stack, two-entry-point seam
  verbatim, linker-enforced slot + in-platform-dir CI) — and the CORE-VERIFY cross-check CAUGHT A PRE-BENCH RADIO MISMATCH:**
  spike literals 923.0 MHz / SF7 / sync 0x12 vs the DFR canon as923_nz = **916.8 MHz / SF12 / CR4:5 / sync 0x21** (vendored
  r2-sx1262 lib.rs:112 — the metal-proven FR/18km config). Each of the three differences alone = zero cross-reception at
  first light. Recommendation sent (core + Roy via supervisor): RAK Phase-2 adopts as923_nz verbatim (match the proven side;
  SF12 ToA fine for the event-counting falsifier); SF7 bench plan only on Roy's explicit preference (touches proven DFR
  config too). Task #44 updated with all Phase-1 facts; my Phase-2 shape = event-driven continuous-RX io_task.
  **DECIDED (supervisor endorsed as default, relayed to Roy):** as923_nz verbatim; core told to swap the spike literals
  (better: call as923_nz() directly so it cannot drift). Radio-plan half of CORE-VERIFY = resolved-by-decision;
  TCXO voltage + pinout markers remain for bring-up.
- **best_transport/RSSI tiebreaker: hive-bin seeding CONFORMS (no fix).** Core proved selection is quality-driven (rssi
  recorded, unused; falsifier 33780e0). My audit of all 3 hive-bin ingest sites: inbound-frame Direct(0.9) covers ALL IP
  peers; reinforce_delivery Direct(0.95); the only sub-0.9 seed is BLE scan-only discovery (Direct(0.6)/Mobile — deliberate,
  above viability floor, floods regardless, upgrades on first real traffic). Allow-mask defaults ALL; §2.3B arrival=None skip.
  **Pattern across all three tiebreakers (D-exclusion, bridge, C-vs-E): no off-thread scenario reproduced against ground
  truth in core OR hive — the sim/harness wiring + dlv-reading remain the only unaudited layer (composer's).**
- **Unicast flood fan-out audit: hive CONFORMS everywhere (A/B/C answered: NONE — no Roy escalation).** Specs landed the
  per-neighbour fan-out canon (ff5555c); audit of all four egress layers: hive-bin router.rs Flood arm sends per
  DirectedHop.neighbour (send_to_hive_via, per-hop logging); hive-bin flood_tg_peers_not_in EXCEEDS the contract (per-peer WS
  unicast to fresh TG peers the engine hasn't observed); sync_host Flood arm per hop.neighbour; wasm captures per-target sends.
  (A) under-reach not present; (B) no concrete truncated-bridge scenario in bench records (Pillar-2 e2e passed) — if composer
  surfaces one it becomes NEW elevated-trust wiring (§13.7.2 is NOT wired into spray ranking today, core confirmed); (C)
  closed previously. Off-thread fork's bridge-problem framing = overstated; in-thread audits authoritative.
- **Flood D-exclusion tiebreaker: hive layers CLEAN (evidence sent).** Core proved its flood is not auth-gated; my inspection
  refuted the hive-wrapper-filter option on BOTH paths: route_inbound_sync ingests the sender Observation on every
  structurally-valid frame (unconditional, pre-plan_forward; only ROUTE-ORIGIN-1 drops earlier, auth-independent), and the
  green test handle_rx_broadcast_relay_respects_8_4b_origin_quota seeds its relay target from an UNVERIFIED heartbeat.
  Remaining forks are harness-side: (b) sim JS pre-gating routing calls on verifyFrame (a conflation of the documented
  route-vs-deliver split) or (c) the dark-board signature misread (D floods fine, dlv=0 is the DELIVERY refusing).
  Discriminator sent: assert on the router's sends[]/relay_on output, never on D's dlv counter. Composer owns the wiring.
- **Multi-TG relay key-awareness: CLOSED from hive's side (no Roy fork).** Specs answered an off-thread fork's question: canon
  says relay stays TG-agnostic/keyless (R2-RUNTIME §13.2 L4/L5 isolation), one hive = one TG. My in-thread authoritative
  position (sent, supersedes the fork): NO concrete scenario needs relay-layer keys — two-TG bench transit is metal-proven;
  cross-TG delivery = L5 classify + peering_hmacs (#38, delivery-layer, don't conflate); multi-TG membership = the ratified
  multi-process pattern (Pillar-2 ran it). Only adjacent seam = §8.4b quota keying on unverified route_stack[0] at keyless
  relays = exactly TN-L1-IT-BL-506 (catalogued, open-by-design, instrumentation-direction not key-awareness).

## ⚡ TURN CONSOLIDATION (2026-07-05 late): v0.64 break absorbed; #49 STALE-ARTIFACT correction sent; 4/4 PROVISION verify armed; RAK4630 task opened
- **core v0.64 trail break absorbed SAME-HOUR (fc08e7a pushed):** core landed 1cc8cd1 on the shared checkout (the instant-bite
  path-dep coupling, as normed — heads-up honored). Fixes: `on_received` gained `my_hive` (§4.6.1 retrace — heads-up said
  signature-unchanged, landed reality differs, compiler caught it); Directed arm notes `hop.neighbour` as recorded successor;
  Flood arm restructured to **one note_forwarded PER FORWARDED COPY** (v0.64 fan-out rule); wasm originate sites →
  `trail::NO_SUCCESSOR`. 37/37 r2-hive-core tests green (my three §4.3.4 invariant tests HOLD under v0.64), workspace green,
  wasm crate tested separately + wasm32 clean. **wasm rebuilt 0.6.0** (pkg sha256 starts 3c08e9b7, 144 877 B) — carries v0.64
  trails + `RxDisposition.authenticated` (core 0e59a7a) for the 700 dedup/refutation arm; composer pulls the fresh pkg.
- **🚨 #49 STALE-ARTIFACT CORRECTION (urgent, sent to supervisor pair):** the message-batch cutover step-1 pointed Roy at
  `~/r2-dfr1195-weave-fixed.elf` (ab1f1cb6, Jul 3 17:51) — **two generations stale**: predates defer-OtaUpdater (7a40bed 19:12),
  half-open guard (69a2d90), coex mute (3aae196), AND Roy's fix-first §5.1 brick-safety (472e1d4+0225ceb Jul 4). The CORRECT
  staged artifact = `~/r2-dfr1195-weave-coex.elf` **29e250cf** (1 362 756 B, Jul 4 12:37 = one minute after the brick-safety
  commit), sha-verified INTACT on BOTH tuxedo and alfred this turn, and it is the same image already healthy on D1+D2.
  Board-side #49 work is COMMITTED + INSIDE the staged image — nothing left to stage; do not let anyone re-point Roy at
  ab1f1cb6. Composer host levers (scanner-stop 61ad26d, debugfs supervision_timeout) are image-independent and compose.
  Sha-archaeology lesson: the 0f4e367/9240217 shas in the ACK trail exist in NO checkout (superseded identities); trust the
  tree + dated commits, not message shas.
- **4/4 PROVISION (Roy ruled option B): VERIFY DUTY ARMED.** Composer executes the d57df16 recipe on D2/ACM5 (b14b07d8) +
  D4/ACM4 (495b1b62): `PROVISION <wire> 79452135 <weave_hk_64hex>`, steady DTR=1/RTS=0. My falsifier chain on its report:
  PROVISION-APPLIED acks → `PROVISION installed live` (no reboot) → HEALTH tg 04bc57e7 → nbrs flap stops → **dlv increments on
  the next signed inject (decisive)**. Then ping supervisor with evidence. Honest debt recorded: option B leaves the @0x14000
  override ACTIVE (shadows persona on future hk rotations) — #43 DEPROVISION verb = the eventual cleanup, HELD spec-first.
  (Note: an earlier batch said D4 was already erased-to-weave and 4/5 accepted with b14b held apiary; Roy's 4/4 ruling
  supersedes. PROVISION on an already-weave D4 is idempotent-harmless.)
- **RAK4630 TN-repeater = task #44 (Roy GO):** gated on core's thumbv7em flash-fit spike; acceptance falsifier DESIGNED NOW
  (4-arm relay-necessity: baseline-off fails / live-on delivers / attribution-through-RAK / negative-control reversibility;
  isolation via existing MASK + VDIST-on-LoRa-ordinal verbs); bench prereq = loraroute DFR rebuild for A/B peers (29e250cf is
  ESP-NOW-only); frequency plan read from lora.rs, band choice flagged to Roy, never chosen silently.
- **Absorbed FYIs:** §8.4b per-origin quota closed both ends (specs v0.30 canon, core bc158ab, TV5/TV6; TN-L1-IT-BL-506
  aggregate residual open by design). Naming: R2-Mesh = the id-5 WiFi-band bearer ONLY; L1 umbrella = "connectionless-mesh
  bearer role"; L3 = R2 Logical Mesh / Transient Network — fw/log/UI labels must follow (audit at #32's re-vendor). #31 canon:
  radio-restriction = BUILD-TIME transport composition (R2-TRANSPORT v0.29 §2.2B, 0193398); runtime transport_allow_mask only
  masks within compiled-in bearers; NO runtime radio-disable hook, bench-only silencing banned from field builds.

## 🔨 TASK #34: increments 1–3 of 4 LANDED (fw ea3d2f0 → f05e0d3 → a239123, all pushed + xtensa-verified)
- **inc2 (f05e0d3) — bus plumbing:** `radiofrontend` feature (implies ble + r2-hw). The §4.2 binary decoder rides uart_rx_task's
  byte stream ALONGSIDE the console line parser (coexistence: provisioning verbs stay alive; frame bytes land in the line buffer
  as garbage, benign — next newline flushes). bus_tx_task keeps the TX half as the ONLY binary writer: COMPLEX_HIVE_PEER (0,
  "SENTINEL") at boot, 30 s STATUS (swap-to-zero since-last counters per §4.2), queued ACKs. **TRANSMIT wired for real** (verbatim
  DATA_TX broadcast, INJECT-parity egress gate under benchdist). CONFIG parses + HW4 reject-unknown-via-ACK; known-but-unwired ids
  ACK generic-fail (an OK would claim an apply that didn't happen); BEACON_AD/SLEEP/SET_TIMER/READ_LOG → ERR_UNSUPPORTED;
  unknown CMD → audible reject. **Honest-ACK doctrine throughout — never silent, never falsely-OK.**
- **inc3 (a239123) — radio-RX → PACKET forward:** espnow_task rx mirrors every over-the-air R2-WIRE frame to the brain through
  the §8.4-lite pipeline: structural decode_extended (keyless stage 3) + global token bucket 32/s burst 64 (stage 5, sub-token
  credit preserved). NO trust filter yet (zero TG state by design — brain gates). io_task DATA_RX dual-feed kept until inc4.
- **inc4 REMAINS:** verbatim BEACON_AD BLE advertiser (cold boot = NOT advertising until first feed — MUST-NOT-originate) +
  io_task/DATA_RX gate-off (zero key material by construction). **SPEC SEAM found + sent to specs:** v0.4 pins the current/next
  slots but NOT the trigger by which the keyless front-end knows the RBID epoch boundary arrived (ad_bytes opaque, schedule is
  brain-side); my inc4 will promote NEXT only on a slot-0 arrival (correct under every reading) pending their pin.
- **Branch-debt note (pre-existing, NOT mine):** the DEFAULT (no-feature/UDP-infra) build is broken at fw HEAD — `got.3` at the
  v0.18 arrival-transport seam only exists on the ble DATA_RX tuple (verified present at HEAD~ before my edits). No load-bearing
  build uses it; fix candidate = infra-path Udp fallback. Every landed increment re-proved the canonical bench set
  (carrier,multitg,routetest,viz,benchdist,otal2cap) green.

## (superseded by the block above — original increment-1 record kept for the audit trail)
## OLD: TASK #34 — increment 1 LANDED (fw ea3d2f0): r2-hw §4.2 bus codec crate, all 4 vectors byte-exact green
- **What landed:** `crates/r2-hw` on the dfr1195-fw branch — no_std zero-dep codec for the R2-HW §4.2 MCU-SBC bus:
  CRC-16/CCITT-FALSE (0x1021/0xFFFF/no-reflect, check 0x29B1 asserted), `encode_frame`, streaming resync `Decoder`
  (tolerates interleaved ASCII console noise — tested), full §5.4 command table (legacy + cohort 0x90–0x9A + BEACON_AD 0xC0),
  pinned CONFIG ids + `ConfigError::UnknownId` (the HW4 MUST-ACK-reject case), WAKE_REASON_EXT 0x07–0x0B, peer/status/ack
  payload builders. **15/15 tests green incl. HW1–HW4 byte-exact from r2-hw-vectors.json; `--no-default-features` clean.**
  ACK status bytes: only 0x00-vs-nonzero is interop-bearing (spec leaves values unpinned; local taxonomy documented) —
  candidate spec question for specs when convenient, NOT blocking.
- **Increment plan (seam map, verified against main.rs):** the mode = `radiofrontend` feature (implies ble).
  (2) bus plumbing: keep `usb_tx` (main.rs:505 currently drops it; esp-println owns TX FIFO via raw regs — binary frame
  writes interleave-race with log prints, mitigation = front-end goes console-quiet after boot, CRC resync covers residue);
  new bus_tx_task (static channel → frame writer); uart_rx_task feeds every RX byte to the r2-hw Decoder alongside line
  accumulation; dispatch: TRANSMIT→verbatim ESP-NOW broadcast (carrier INJECT machinery), CONFIG→parse+apply/ACK-reject-
  unknown (HW4), BEACON_AD→length-check + current/next slot store + BLE adv update (reject ⇒ ACK ERR_INVALID + keep airing
  last-known-good, never-zero-beacons), SLEEP/SET_TIMER/READ_LOG→ACK ERR_UNSUPPORTED (honest stand-in), boot PEER announce
  (component_index 0, "SENTINEL"), STATUS 30s with real radio counters.
  (3) radio RX→PACKET forward with §8.4-lite pipeline (structural decode + counters + token bucket), NO GroupHmac.
  (4) the §4.1 hard part: io_task spawn (main.rs:494) gated OFF in this mode (no mesh participation, no hk install =
  zero-key-material by construction), ble_task (:523) swapped for a verbatim-AD advertiser (cold boot = NOT advertising
  until first BEACON_AD — the front-end MUST NOT originate any payload bit), espnow_task (:539) RX side → bus forward.
  Each increment xtensa-build-verified before the next. **STAGE for Roy — no flash.**

## ✅ CATCH-UP CONSOLIDATION (2026-07-05, supervisor-codex batch; every claim below re-verified locally before recording)
- **DARK-BOARD ARC CLOSED ON METAL (task #42 → completed):** @0x14000 override mechanism PROVEN. Roy's clean `erase_region` +
  weave-persona flash flipped D4 (495b1b62) onto the weave TG; the interim "still ea6c5a9d after erase" observation traced to Roy's
  FIRST (malformed) erase, not to any rewrite. **REFUTED en route (recorded honestly): the "host connect-time PROVISION rewrites it
  after reset" hypothesis — disproven by composer code ground truth + the clean-erase result.** FINAL BENCH: **4/5 boards on weave
  04bc57e7; b14b07d8 (D2) INTENTIONALLY HELD on apiary TG ea6c5a9d** (deliberate, not dark). Composer's on-air native target_group
  decode was the confirming instrument. Task #43 (DEPROVISION verb) stays HELD.
- **#49/OTA ACCEPTED STATE (task #35 updated):** receiver CODE-COMPLETE on ELF cb87c8aa (otal2cap/PSM 0x00D3, verify_header +
  PayloadVerifier + inactive-slot write + anti-rollback + coex-mute 3aae196 + half-open guard 69a2d90) — but **real-HW push NOT
  proven e2e**; slot-switch metal proof + verify-before-write wasm proof are separate pieces only; NO fleet-scale OTA/USB-replacement
  recommendation until the one-board metal e2e passes (signed image → verify → inactive-slot write → COMMIT/activate → reboot →
  new-boot + floor bump). **Authorized REMOTE on a MESH board** (not carrier/live bridge; receiver fail-safe, USB-JTAG = human
  recovery). **Artifacts sha-VERIFIED on disk:** ~/r2-dfr1195-weave.elf (sha256 = cb87c8aa337b…), ~/cb87c8aa-app.bin 863 440 B
  (sha256 1b8092d508a9…) — extracted by SUPERVISOR under explicit offline-only authorization (espflash stays harness-gated for
  agents; command: save-image --chip esp32s3, Merge=false, no device/port/keys). **Key custody: composer signs the UpdateHeader
  with weave TG_SK (persona-minter/signed-ota-deploy); hive NEVER holds TG_SK.** Header pinned seq=1 / target_class=0 /
  authority_epoch=0 (board floor verified 0). Gate = composer pusher readiness + signed image. 200 B MTU fine for staging.
- **TASK #34 UNBLOCKED — BUILD TARGET PINNED (→ in_progress):** the resident-gateway product spec **v0.4** (Publish:Private tree;
  its product/spec name MUST NOT appear here — narrow hygiene guard e5bc905 verified live at HEAD) pins the brain→radio-front-end
  **BEACON_AD wire as CMD 0xC0** with payload layout = the AUTHORITATIVE USB contract (cross-repo interop, supersedes the ad-hoc
  proposal round). **Beacon model:** Linux brain encodes the COMPLETE AD/RBID with its keys; the MCU front-end airs it VERBATIM;
  **zero key material on the MCU**. Also build to specs e0f926d (verified present in the local specs HEAD, unpushed to origin):
  COMPLEX_HIVE_PEER = 1 B component_index + 8 B NUL-padded ASCII role_tag; R2-CAP v0.4 power-state keys 0x04–0x08 (battery reuses
  0x02); R2-COMPLEX-HIVE v0.8 WAKE_REASON_EXT 0x07–0x0B; R2-HW v0.9 CONFIG ids 0x01 TX_POWER_DBM + 0x02 WAKE_INTERVAL_MS,
  CRC-16/CCITT poly 0x1021 init 0xFFFF no-reflect, unknown config_id MUST reject-via-ACK; r2-hw-vectors.json = 4 byte-exact frames;
  R2-USB v0.7 error payload implementation-defined BY DESIGN. Plus the §4.1 Sentinel bar. Target = B6:0A:A0. **STAGE, do not flash.**
- **Hygiene state:** specs fixed + deployed the public dashboard labels; remaining exposure was structural path text in the generated
  dashboard blob (narrow suppression approved on specs' side). My side: ONLY the narrow gateway-naming guard (e5bc905); broad
  scrubs/guards + historical-ID cleanup + the README marketplace-branding question are ALL HELD as Roy-level policy — do not "fix".

## 🎯 DARK-BOARD MECHANISM CONVERGED (2026-07-05): stale NVS @0x14000 TG-override, NOT personas — I own the fix procedure (task #42) + DEPROVISION proposal (task #43, HELD)
- **Ground truth (supervisor-codex recorded, refutation accepted):** personas @0x12000 are ALL weave-correct; my earlier key-epoch-on-persona
  framing was wrong at the *storage layer* — the wrong-epoch key lives in the **runtime-PROVISION record @0x14000** (magic R2TG,
  `[magic u32 BE][tg_id u32 BE][key 32B]` = 40 B, own 4 KB sector; `main.rs:2191`), which **OVERRIDES the persona at boot**
  (`main.rs:265-276`, serial line `PROVISIONED TG restored from NVS`). Dark boards D2 (B7:90:10 / b14b07d8) + D4 (52:99:28 / 495b1b62)
  carry a stale override with tg_id 04bc57e7 + an OLD-epoch hk → HMAC verify fails → correct fail-closed refusal. Fix = ONE-SECTOR
  clear/overwrite, **NOT** persona rewrite, **NOT** a reflash.
- **The two operational fixes (Roy chooses intent — NO NVS clearing until then; standing directive):**
  - **(A) Roy download-mode erase (human-only, pristine end-state):** `esptool.py --port /dev/ttyACM<n> erase_region 0x14000 0x1000`
    (or `espflash erase-region 0x14000 0x1000`). Erased flash = 0xFF → magic check fails → `read_provisioned_tg()` = None → boot
    falls back to the (weave-correct) persona. ⚠ offset-typo hazard: 0x12000 would kill the persona — the command above is exact.
  - **(B) composer console overwrite (no download mode, NO reboot):** send to each board's OWN tty (steady DTR=1/RTS=0 discipline):
    `PROVISION b14b07d8 79452135 <weave_hk_64hex>` (D2/ACM5) and `PROVISION 495b1b62 79452135 <weave_hk_64hex>` (D4/ACM4).
    79452135 = decimal of 0x04bc57e7 (the §6 tg_id IS the wire target_group). Path: `parse_provision` validates (exact-32B key) →
    `write_provisioned_tg` erase+write+read-back-verify → ACK `PROVISION-APPLIED wire=… tg_id=…` → io_task swaps GroupHmac +
    target_group LIVE (`main.rs:1074-1085`). Re-runnable/idempotent; failure ACKs PROVISION-ERR, installs nothing.
  - **Trade-off:** (B) leaves override-ACTIVE state (0x14000 keeps shadowing the persona — future hk rotations need another
    PROVISION or an erase); (A) restores persona-governed state but needs the human cable dance. Same end TG either way.
- **Blast radius (either option): ZERO collateral.** Flash map, each its own 4 KB sector: persona@0x12000 · board-profile@0x13000 ·
  **TG-override@0x14000 (the only target)** · MASK@0x15000 · SENDTO@0x16000 · RPF1 role@0x17000 · anti-rollback@0x18000 ·
  LBL1 label@0x1B000 · ota_0@0x20000. **NO apiary-role detachment** — role lives @0x17000 + is derivable, fully independent of the
  TG override; hive_id unchanged (persona master_secret). Option A's download-mode entry reboots the board (beats reset — fine,
  these are the dark boards, not the #49 beat-discriminator board).
- **Verify after (safe steady-DTR read):** (A) boot shows NO `PROVISIONED TG restored from NVS` line; (B) `PROVISION-APPLIED` +
  `PROVISION installed live` ACKs, no reboot. Then both: HEALTH decodes tg_hash=04bc57e7, nbrs stops the 0↔1 flap, **dlv increments
  on demo traffic** (the decisive falsifier that the hk now verifies).
- **Conditional branch closed:** the "if target_group already 04bc57e7 AND frames verify → real deliver/LED bug" fork is MOOT under
  the converged mechanism (frames do NOT verify under the stale key) — reopens only if composer's native-frame check refutes.
- **Task #43 (NEW, HELD):** DEPROVISION console verb proposal (clear @0x14000 over console, live-revert to persona hk symmetric with
  the install path). Spec-first via CROSS-HOST-2TG §6 extension; NO firmware change unless Roy explicitly asks.

