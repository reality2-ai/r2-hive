# RESUME — r2-hive (hive-worker)

> Older closed arcs live in RESUME-archive.md (rotated 2026-07-06; this file holds LIVE state only — keep it readable in one pass).

> **🚨 OPS CONTAINMENT (supervisor-codex 2026-07-12): DO NOT use `fleet ask` or `fleet refute` against any live writer** — the isolation is BROKEN (responder.sh only changes shell cwd; provider.sh resumes the provider without an isolated cwd / hard read-only, so a resumed off-thread responder CAN mutate the ORIGINAL checkout — it already mutated specs' checkout despite a temp worktree). **`fleet send` remains ALLOWED** (one-way, no resume-mutation). Use only `fleet send` until the fleet-fix lands + passes isolation regression tests. My r2-hive checkout verified clean @HEAD (not mutated). Fix owed fleet-side (pass isolated cwd + hard RO + worktree-fail-closed + a mutation-attempt regression test).

> **⚠ OPS (supervisor 2026-07-12): HOSTED CI is DOWN org-wide** (GitHub billing block, Roy-side) — so the ONLY gates right now are **local builds/tests + this RESUME** (the durable failover source). Do NOT claim "hosted-CI-green" for anything this window; say "local-verified" distinctly ([[local-check-vs-hosted-ci]]). This is why session docs-commits are PUSHED to origin/platform-trait (durability off-box matters more with CI down). **✅ LOCAL-GREEN BASELINE (2026-07-12, CI-down failover datapoint): r2-hive-bin lib 110/110; r2-wasm-host conformance 4/4 (post abi_hash-split); RAK blespike staged-image recipe (`--no-default-features --features dev,blespike,demomember,uf2`) compiles GREEN (2 benign unused-const warnings BRINGUP_HIVE_ID/TG_HASH under this feature combo — pre-existing, not a regression); Cargo.lock synced + `--locked` resolve OK. RAK monolithic MCU image UNAFFECTED by core's abi_hash-split (no wasm/abi_hash refs).**

> **✅ CANONICAL SPEC REF — FINALIZED (supervisor 2026-07-11, supersedes the interim branch rule):** specs PUSHED origin/main (Roy-authorized, `8c8310d→e4b818e`, all 27 session commits); supervisor VERIFIED **origin/main = R2-UPDATE v0.60** (reject codes 16/17/18 present in origin/main:R2-UPDATE.md), board/chip_profile schema v0.34, TG-lane v0.59. **origin/main is canonical + current again — verify/build against origin/main as normal.** The interim "track spec-conformance-v0.2, avoid origin/main" rule is RESCINDED. specs adopts push-after-every-merge, so origin stays honest. **My threads: PIECE-B reject codes 16/17/18 now on origin/main (still deferred until the CoC receiver); tn_base board_profile tracks schema v0.34 on origin/main.**

## 🔬 XIAO BENCH decode-support (android, 2026-07-12) — golden frames handed + board-health cleared (commit 95d5148)
android hit two things decoding the XIAO USB egress: (1) InvalidRouteLen on live captures, (2) the stream went QUIET.
**Both answered via fleet send + persisted to docs/BENCH-BOARD-FACTS.md.** (1) ROOT CAUSE = capture drop, NOT a
decoder bug: `dd bs=1` byte-drops the egress (27–30B vs true fixed 31B); byte0=0x06 sets has_route, so ONE dropped
byte reads data[12] as rlen=0x00 = InvalidRouteLen exactly. Handed a byte-exact GOLDEN frame from canonical
`r2_wire::encode_compact` (round-trips decode_compact): compact 31B = `0653000164cedbf305fe0701011234a10018eaa101182a0102030405060708`;
R2-USB DATA record 33B = `1f00…` (1f00 = payload_len 31 LE). Emitter LANDED = `crates/r2-hive-bin/examples/gen_golden_compact_frame.rs`
(regen: `cargo run -p r2-hive --example gen_golden_compact_frame`). 0xA1 sighting golden already byte-exact in
dfr1195 `USB-BEACON-SIGHTING-FORMAT.md` KAT. (2) QUIET = peer-silence (egress is PURE forwarded LoRa RX, NO
keepalive); board still enumerated at same MAC = NOT in ROM download mode; forward-task can't wedge on a DTR toggle
(USB-JTAG egress drops-when-unread, never blocks). Did NOT touch the board.
**★ BEARER-OWNER DECISIONS LOCKED (composer eb0bc75 + android, 2026-07-12):** LoRa REAL + WiFi REAL + BLE dark.
**(a) fw inject-per-transport harness CANCELLED** (composer #2 — no no-RF sim leg needed; removed from open items).
**(b) NEW hive task #66 (composer #1), GATED on supervisor GO:** stand a 2nd LoRa node at SF7 (`benchsf7`) as the
R2-PROVISION §3.2 JOIN counterparty to the XIAO (android is phone-provisioner ONLY). Deps on GO: 2nd SX1262 board
attached (RAK DISCONNECTED — need Roy hw) + a JOIN-role build (not keyless xiaobridge). **BLE-REAL FINALIZED (composer 0b2d0bd)** — not skip. Reflash `xiaobridge,ble` still HELD but ADDITIVE (keeps LoRa
bridge + adds ble_task); trigger = AFTER (a) WiFi leg proven AND (b) android BlueZ central built, at a convenient
break in android's live capture. **✅ compile-verified locally (xtensa esp toolchain): `xiaobridge,ble,benchsf7`
builds GREEN (warnings-only) = no feature-unification blocker.** **⚠ compile-green ≠ coex-proven** — BLE+WiFi+LoRa on
the one S3 radio is metal-only at reflash time; full ELF owed at the reflash window (NOT staged now — fw may change).
**Do-not-assume:** do NOT reflash until composer/android give the break signal. boot-banner hive_id/TG/persona/build_id
owed on next reset catch.

## 📡 RAK BLE — ★★ PIVOTED to (A-mcuboot) 2026-07-13 (bare-metal nrf-sdc + MCUboot; blespike-under-s140 REFUTED) — task #68
**★ THE BLESPIKE-UNDER-DORMANT-s140 SPIKE (below, b7691c0) IS REFUTED — the coexistence it was testing FAILED.** Roy flashed b7691c0 on TUXEDO (known-good env, Meshtastic booted there): "starts to load, then loops." Env-independent ⇒ the s140↔nrf-sdc architecture conflict is REAL, not Alfred's USB env. nrf-mpsl (bare-metal MPSL) cannot run BEHIND a resident s140 (both own RADIO/RTC0/TIMER0/CLOCK). **⛔ DO-NOT-ASSUME / DO-NOT-REFLASH: do NOT re-stage or re-flash `~/rak-blespike/…b7691c0.uf2` — it loops. The staged-UF2-@0x26000-behind-s140 model is dead.**
**PIVOT (supervisor-confirmed 2026-07-13) = (A-mcuboot): bare-metal nrf-sdc/nrf-mpsl + MCUboot, NO resident SoftDevice.** Two falsifiers, same direction: (1) the only crates.io s140 bindings `nrf-softdevice-s140 0.1.2` bind **SD_VERSION=7.0.1** but the RAK's resident SD is **6.1.1** (verified: registry src bindings.rs:855; corroborated RAK-MCUBOOT-OTA-PLAN §4.3 — stock ships s140 6.1.1) = undefined SVC/struct ABI landmine, so the (B') "raw s140 bindings through the resident SD" path is ALSO refuted; (2) Roy's HARD REQ "last firmware flash ever, then OTA" needs dual-bank **MCUboot** (task #60 choice) + an R2-native TG-gated OTA receiver — and **MCUboot precludes a resident SoftDevice** (MCUboot@0x0 boots a self-contained app owning the bare radio = bare-metal nrf-sdc). So Roy's OTA req FORCES bare-metal, which ALSO removes the s140 that made blespike loop — Occam AND less work, AND it's the pre-existing task#44 target (the s140 path was a blespike-loop detour). **FOUNDATION (bare-metal nrf-sdc BLE + full-TN + embassy 0.7, reusing the blespike code main.rs:296-871) COMPILE-GREEN post-pivot** (`cargo check --no-default-features --features dev,uf2,blespike,demomember --target thumbv7em`, 2 benign dead-code warns).
**HELD on Roy's bootloader pick (embassy-boot-nrf vs C-MCUboot)** — that gates the whole bootloader-verify seam: RAK-MCUBOOT-OTA-PLAN.md §2 slot map + §3 anti-rollback are HOLD-not-RESOLVED pending the pinned MCUboot version/swap-algo/config; `activate()` = fail-closed `ActivateNotWired` stub; VTOR/FLASH-ORIGIN re-link (0x26000→primary-slot) waits on the map. GREEN-2a metal (RAK boots + BLE scanner sees beacon + LoRa/TN live) + #49 (ONE proven OTA cycle on THIS board) BOTH gate on removing the resident s140 = the bootloader change itself; #49 is the acceptance gate before the cable is declared retired. Gates STAND: spec-first + core-coordinated on the swap/ImageSink seam; codex refuter on the OTA/bootloader trust seam before done. **Ledger: rak4630-fw docs/ledger-blespike-boot-failure.md @5ae8d8f (pushed).** OTA write-half already built (src/ota.rs FlashSink, `ota` feature, a6a3526). **OTA co-design agreed with core (2026-07-13):** core writes the RECEIVER (drives SignedOtaApply::feed, partition-INDEPENDENT — proceeding NOW) + owns the ImageSink CONTRACT (trait + §5.5 invariants in r2_update); HIVE owns the metal ImageSink impl (FlashSink) + MCUboot partition/swap/confirmed-boot/flash. I send core FINAL bank offsets + the rollback mechanism (seq-sole-floor vs HW-security-counter — DGP unavailable under swap/revert) the moment Roy picks the bootloader + I pin the MCUboot config. Also verified 2026-07-13: core's tgid=ZERO change (aea0e9f) is a NO-OP for all hive trees — r2-hive has no join-envelope path; the esp32 fw JoinRequest already sends zero envelope tgid + handle_join_response ignores the response tgid (R2-SEC-01), doesn't call verify_group_mgmt_authority.
**★ ROY PICKED embassy-boot-nrf (Rust-native, 2026-07-13) — bootloader-verify seam UNBLOCKED, building.** Analysis done (RAK-MCUBOOT-OTA-PLAN §1.5+§2.embassy-boot, pushed): (a) VERSION landmine CONFINED — embassy-boot-nrf 0.12.0 pulls embassy-nrf 0.11+embassy-sync 0.8 (>app's 0.7), but the bootloader is a SEPARATE binary so the app stays 0.7+nrf-sdc-0.3 and signals swap via plain-Nvmc write of STATE SWAP_MAGIC=0xF0 (embassy-boot kept OUT of the app); (b) TRUST-MODEL — stock embassy-boot verifies APP-side, Roy wants BOOTLOADER-side R2/TG verify → CUSTOM bootloader: read anchored TG_PK from sealed persona partition, r2_update-verify staged DFU + enforce seq>floor before embassy-boot swap, else reject; (c) provisional flash map BOOTLOADER48K/STATE/PERSONA-8K-outside-slots/ACTIVE==DFU-493K, sizes PENDING a bootloader-size measurement (chicken-and-egg). **⚠ ROY DECISION PENDING (flagged to supervisor): RECOVERY posture** — embassy-boot removes ALL stock recovery (UF2 drag-drop + Nordic serial-DFU + UART2_TX→GND tier-2); bricked-bootloader → SWD tier-3 ONLY (REVERT auto-revert covers a bad OTA, so #49 stays safe). Roy: accept SWD-only, or I add a serial-recovery leg. **Verify seam Q1-5 fleet-sent to core (spec-first): image layout, bootloader one-shot verify(dfu,tg_pk) entry, TG_PK-from-persona read, anti-rollback ACTIVE=confirmed/DFU=pending, app-side early-reject split.** NEXT (gated on core Q1-5): build the custom bootloader binary (with verify gate) → measure size → pin the map → re-link app (FLASH ORIGIN 0x26000→ACTIVE, VTOR) → activate()=write SWAP_MAGIC → metal GREEN-2a + one OTA cycle (#49) → codex refuter on verify/swap seam. Core building the owned-sink receiver in parallel (§4.5 split).
**★ 2026-07-13 latest:** (1) core LANDED OwnedOtaApply @27a9e6f (by-value sink, finish→(AppliedUpdate,S) no-activate, abort→S, all paths release Nvmc) — my FlashSink+activate plug in; split CONFIRMED (hive=FlashSink+metal+activate, core=contract+OwnedOtaApply+receiver; confirming GO-wording w/ supervisor to close core's flag). (2) GROUNDED flash map (DFU≥ACTIVE+1page source-verified vs boot_loader.rs:158 — corrected earlier 'equal'): BOOTLOADER 128K@0x0 / STATE 8K@0x20000 / SEALED-PERSONA 8K@0x22000 / ACTIVE 436K@0x24000 / DFU 444K@0x91000; sent to core (receiver is offset-independent). (3) Roy chose (b) SIGNED serial-recovery leg, DEV-MODE gated: OTA + serial-recovery CONVERGE on ONE bootloader R2/TG verify gate (TG_PK-anchored sig + seq>floor), two transports (BLE-CoC/LoRa + USB-CDC); prod EXCLUDES the serial leg (SWD/APPROTECT only); verify ON even in dev (never blind). Codex-refute-before-done adds the 4 serial-leg surfaces (unsigned-accept/seq-rollback/anchor-bypass/DoS). embassy-boot-nrf 0.12 stack COMPILE-VERIFIED on thumbv7em. NEXT build (custom bootloader: embassy-boot BootLoader + R2/TG verify gate + dev-gated signed serial-recovery) gated on core's Q1-5 verify-seam answers → measure size → pin map → re-link app → activate()=STATE-magic → GREEN-2a + OTA cycle #49 → codex refute.
**★★ 2026-07-13 — core answered Q1-5 + CUSTOM BOOTLOADER SKELETON BUILT + LINKS CLEAN.** Core's verify seam (all 5): Q1 image layout = [UpdateHeader 137B §2.2.1 v3 | payload | detached Ed25519 sig 64B], sig over the HEADER by the TG key, verify = Ed25519(sig,header,tg_pk) + SHA256(payload)==header.payload_hash; Q2 core ADDING alloc-free `verify_staged_slot(header,payload,sig,ctx)->VerifiedHeader` + Q3 alloc-free `persona_tg_pk(sealed)->[u8;32]` (parse_persona uses Vec, not bootloader-safe) — BUILDING NOW; Q4 seq-sole-floor, CONFIRMED=ACTIVE/PENDING=DFU, enforce seq>floor before swap; Q5 app-verify=early-reject, BOOTLOADER=authoritative. **NEW crate `platforms/rak4630-bootloader` (rak4630-fw, workspace-excluded): embassy-boot-nrf 0.12 A/B swap + load, LINKS CLEAN thumbv7em (.vector_table@0x0, 3.7KB/128K region), dev+prod both green.** build.rs memory.x = grounded map symbols; R2/TG verify gate = PLACEHOLDER (wire core's verify_staged_slot+persona_tg_pk when they land — enable the commented r2-update dep). **DEV/PROD posture folded in:** dev=`Debug::Allowed`(open)+serial-recovery; prod(`--no-default-features`)=`Debug::Disallowed`(APPROTECT LOCK — the takeover-requires-physical linchpin; bootloader owns UICR now that there's no SoftDevice)+no-serial-recovery; prod ultimate recovery=destructive ERASEALL+recommission. Entanglement NON-FORECLOSURE (Roy canon): bootloader=simple anchored check (own-TG persona, seq>floor), NOT entanglement-eval (that lives app/base staging layer); anchor read kept extensible (no single-key-forever). ⛔ DO-NOT-FLASH until verify wired + codex-refuted. LAST-MILE gated on core's verify_staged_slot+persona_tg_pk: wire gate + dev serial-recovery + app re-link (0x26000→0x24000, VTOR) + activate()=SWAP-magic → GREEN-2a + #49 → codex refute (incl 4 serial-leg surfaces).
**★ 2026-07-13 latest: verify_staged_slot LANDED (core 7093c9d, r2-update 79/79) + core-codex adversarial review on the seam started.** Sig: `verify_staged_slot(header, sig, payload_chunks: impl IntoIterator<Item=&[u8]>, ctx) -> Result<VerifiedUpdate,VerifyError>` = verify_header(Ed25519 header-sig + seq>floor + targeting) + streaming SHA256(payload)==header.payload_hash, streamed from DFU flash pages (no RAM buffer), alloc-free, serves OTA + serial-recovery. Layout [header 137B][payload][sig 64B] confirmed. STILL OWED (core, unblocked by my answer): Q3 `persona_tg_pk` — I sent core the PERSONA LAYOUT (compact CBOR map, tg_pk = value of int key 5, 32B bstr strict; alloc-free walk with dup-key-5 reject per R2-SEC-06; self-delimiting, trailing 0xFF ignored; r2-trust persona.rs:8-10/79-128) + asked for the DeviceContext ctor (live fields tg_pk+current_seq vs locked defaults host_abi_hash/core_abi_version/certs/battery). GATE WIRING READY the moment persona_tg_pk + ctx-ctor land: read STATE magic → persona_tg_pk(0x22000) → DeviceContext(tg_pk,floor) → verify_staged_slot(DFU) → Err⇒erase DFU+clear SWAP magic (fail-closed) → embassy-boot prepare. Then dev serial-recovery + app re-link + activate() → GREEN-2a + #49 + codex-refute.
**★★ 2026-07-13 — BOTH verify primitives LIVE + DeviceContext ctor GIVEN; gate wiring = the next focused block.** persona_tg_pk LANDED (core 3e15d75, r2-trust 81/81; in r2_trust::persona, alloc-free CBOR key-5 walk, dup-key-5 reject, fail-closed None). verify_staged_slot LANDED (7093c9d). **DeviceContext ctor for the bootloader own-TG verify (core-specified):** tg_pk=persona_tg_pk(0x22000); current_seq=ACTIVE-slot floor; update_authority_certs=&[]; revocation_gset=&[]; now=None (clockless); authority_epoch_floor=0; battery_pct=100; class_hash/carrier_hash=board-FNV OR 0-if-image-minted-target0; tg_prefix/device_id_prefix=[0;8] (target_device=0); host_abi_hash=[0;8]; core_abi_version=0 (PT_FIRMWARE_FULL=abi wildcard). ⚠ MUST match how the base image is MINTED — sent composer the all-wildcard bench mint proposal (target_class/carrier/device=0, abi_hash=0, min_core_abi=0, min_battery=0, sign=bench TG_SK 0xF305FE07 whose PK=persona tg_pk); then core confirms ctx maps. **⛔ BLOCKER for wiring: the fw's VENDORED crates/r2-update + crates/r2-trust are STALE** (have verify_header+DeviceContext but NOT verify_staged_slot/persona_tg_pk) → **RE-VENDOR r2-update→7093c9d + r2-trust→3e15d75 FIRST** (diligence-clean per norms), then enable both deps in platforms/rak4630-bootloader/Cargo.toml, wire the gate (embassy-boot BlockingFirmwareUpdater get_state for the SWAP check + clear-on-reject, NOT a hand-rolled magic read), compile-verify. NEXT BLOCK (fresh, security-critical — don't rush): re-vendor → wire gate → compile → confirm mint params (composer+core) → dev serial-recovery + app re-link + activate() → GREEN-2a + #49 + codex-refute.
**★ 2026-07-13 — mint params CONFIRMED + a NEW byte-layout seam raised (gates the gate).** Composer (from minter code): its `ota-sign`/`build_signed_ota_stream` ALREADY emits my all-wildcard proposal (137B v3 header, target_class/carrier/tg/device=0, abi_hash=0, min_core_abi=0, ensemble_semver=0, PT_FIRMWARE_FULL=0x01, seq=CLI default1, min_battery=0, issuer_pk=held TG tg_pk, detached Ed25519 over the 137B header ONLY, payload auth via payload_hash) — NO minter change. tg_pk PATH = composer's PATH 2 (RAK persona@0x22000 NOT yet anchored): composer mints a FRESH bench TG → sends me the 32B tg_pk → I anchor @0x22000 at commissioning → composer ota-signs with its tg_sk. **★ OPEN DESIGN SEAM (raised to core, who owns verify_staged_slot + staging): BOOTABLE-AFTER-SWAP** — embassy-boot swaps the WHOLE DFU into ACTIVE, so ACTIVE[0] MUST be the app vector table ⇒ the raw bootable payload must sit at DFU[0], header CANNOT be prepended in the bootable region. AND header+sig (held in RAM app-side by OwnedOtaApply) must be PERSISTED to flash for the bootloader to re-verify. PROPOSED: DFU=[payload@0 (bootable, app@ACTIVE_ORIGIN)] + [header137+sig64 FIXED TRAILER @DFU_end-201]; composer's 0x03-header-sig-payload STREAM stays (receiver re-stages to this on-flash layout). Asked core: (a) payload=raw-bootable assumption? (b) trailer-vs-metadata-region? (c) FlashSink stages payload@0 + receiver writes trailer? **Gate wiring + app-link + receiver all wait on core's layout ruling.** composer fixing its 123→137 stale doc comments.
**[⤵ HISTORICAL / SUPERSEDED below — the blespike-under-s140 authoring arc that led to the refutation; kept for the bring-up boilerplate (Irqs/MPSL/SDC/beacon identity) which (A-mcuboot) REUSES, NOT for the flash model]**

## 📡 RAK BLE bring-up (P4 = task #58/#44) — PRIORITY (Roy LIFTED do-not-flash 2026-07-11); AUTHORING the 2a spike
**do-not-flash-RAK LIFTED** — Roy de-risks BLE NOW on the connected RAK (iterative dev flashes; final sealed image stays flash-once). **Profile RESOLVED = LEGACY 31B** (core+supervisor aligned, specs ruling incoming; extended/bloom DEFERRED — no consumer, ext-adv not universally phone-scannable, legacy→extended is additive). Step-1 (embassy 0.7 migration) DONE (dc6e3ed). Weight-anchor (main.rs:814-830, extended placeholder) to be REPLACED by the real legacy path in the spike.
**⚠ SAFETY — FLASH ttyACM1 ONLY** (verified on Alfred: ttyACM1 = `r2-rak4630` VID **1209** Reality2 serial rak4630-dev = RAK nRF52840; ttyACM0 = Arduino Leonardo VID 2341 = **DO-NOT-TARGET**). Verify board identity BEFORE every flash. Roy runs one-liners unless he delegates with strict ttyACM1-only verify.
**Binding contract (r2-ble @d80ebf7, `binding` feature = nrf-sdc 0.3.0/nrf-mpsl 0.3.0/bt-hci 0.4, NO trouble-host for 2a — raw HCI advertise):** BleHost ALREADY exists — `BleConfig::new(BleAddress).with_name("rak4630")`, `BleHost::new(sdc: &'static SoftdeviceController, cfg)`, `set_advert(&beacon_ad)`, `advertise_start()/stop()`. **`set_advert` takes the FULL 28-byte `build_legacy_beacon` output VERBATIM** (raw-HCI needs the complete AD element len+type+company+payload) — NOT `encode_advert`'s 24B inner (that's the bluer/esp32 path); binding prepends FLAGS_AD `[02 01 06]` → exactly 31B.
**HIVE OWNS (steps 3+5):** the MPSL/SDC bring-up = fw boilerplate — `bind_interrupts!` the **6 IRQs** (RNG, EGU0_SWI0, CLOCK_POWER, RADIO, TIMER0, RTC0), StaticCells for MPSL+SDC, `mpsl_task`/`sdc_task` spawns, `sdc::Builder::new().support_adv().support_peripheral().build(sdc_p,&mut rng,mpsl,mem)` → `&'static SoftdeviceController`; PARTITION per descriptor.rs (PPI app=0..16, LFCLK=LFXO, RNG seed-before-SDC-build then hand to SDC, DROP the manual HFXO under ble — MPSL owns clock). Then `BleHost::new` + build LEGACY beacon (identity = SAME as LoRa: `compute_rbid(derive_beacon_session_key(hk,hive_id),epoch)`+class_hash BE, NOT esp32's `rbid[0]=0xFF` hack) → `build_legacy_beacon`→28B → `set_advert` → `advertise_start`; LoRa loop stays live (green-2a = scanner sees advert + LoRa runs). **Compile-verify LOCALLY possible** (LIBCLANG_PATH→esp-clang; nrf-sdc-sys vendors the nrf52840 blob — [[rak-ble-binding-compile-env]]).
**PROGRESS 2026-07-11:** ✅ `blespike` feature + nrf-sdc 0.3.0/nrf-mpsl 0.3.0/bt-hci 0.4 deps committed (5ce28d8, match r2-ble binding, lock-clean); ✅ nrf-sdc 0.3.0 API read (full bring-up example in its lib.rs); ✅ core unblocked the advertise leg + gave the LOCKED LegacyBeacon identity (rbid 7fce111165325a9a, class_hash bafe8ac1, tx_power 20, build_class 2) + is pinning a byte-exact BLE KAT (I picked flags=BeaconFlags::default()=0x00, anti_collision=0x0000, told core); ✅ **§11 integration plan COMMITTED (432ba25, BLE-PLAN.md)** — full bring-up pattern + the TWO mapped conflicts: (1) RNG-sharing (fp_seed@599 consumes p.RNG but SDC needs it → under blespike make the persistent SDC Rng first, fill fp_seed from it, then &mut to build), (2) Irqs needs a blespike variant (+SWI0_EGU0/POWER_CLOCK/RADIO/TIMER0/RTC0), + LFXO-not-RC lfclk, drop-HFXO. **NEXT (mechanical per §11): author main.rs bring-up under #[cfg(feature="blespike")] (do NOT touch the proven default/persona/ota path) → cargo check --features blespike (LIBCLANG_PATH→esp-clang) + default green → stage → ttyACM1-only flash one-liner for Roy.** Core co-authors the CoC seam (2b) next.
**✅ BLE KAT PINNED (core, r2-discovery canonical_rak_bench_ble_legacy_beacon_vector) — assert on-metal byte-exact:** `1bffffffb201007fce111165325a9abafe8ac1140000020000000000` (28B AD: Len1B FF CID-FFFF magicB2 ver01 flags00 rbid-7fce111165325a9a class-bafe8ac1 txpwr14=20 ac0000 buildclass02 +5×00). Identity == LoRa §8.1 beacon. encode_advert→24B inner=AD[4..28]; set_advert prepends 02 01 06. **✅ r2-discovery RE-VENDORED @core bc3c633 (133547c)** — core confirmed build_class is a real §7.3 struct field; my vendored copy predated it. Diligence-clean (Cargo.toml identical, only cosmetic doc deltas, NO local RAK divergence — hive discovery features all upstreamed). LegacyBeacon now carries build_class + the canonical BLE KAT test is vendored. **✅ blespike deps (nrf-sdc/nrf-mpsl/bt-hci) IN + LOCKED (5ce28d8+6154693); firmware default + persona compile-GREEN thumbv7em.** ⚠ FALSE-ALARM NOTE (2026-07-11): a serde_core "break" panic was a WRONG-DIR artifact — `cargo check --target thumbv7em` from the workspace ROOT checks std host-tools (r2-forge/bluer) that fail for no_std; ALWAYS check from platforms/rak4630 [[mcu-cargo-check-from-package-dir]]. The firmware was never broken. **✅ BLE 2a BRING-UP AUTHORED + COMPILES GREEN (rak4630-fw 526c815) — the milestone supervisor asked for.** main.rs MPSL/SDC bring-up + advertise per §11/§12: cfg-split Irqs (blespike +6 IRQs), mpsl_task/sdc_task, RNG-shared fp_seed (SDC owns RNG via new_blocking), MPSL(LFXO 50ppm)+SDC(PPI 0..=16, Mem<8192>) before the LoRa loop, identity-gated advertise (LegacyBeacon{build_class:2, identity==LoRa beacon} → build_legacy_beacon 28B → BleHost::new(opaque §7.4.0 addr from session key) → set_advert → advertise_start). **GREEN on thumbv7em: default (proven path untouched) + blespike,demomember + blespike,persona** (LIBCLANG_PATH→esp-clang). First-metal nits fixed from real sources: Irqs EGU0_SWI0/CLOCK_POWER, Rng<_,_,Blocking> via new_blocking, Mem<8192> literal, accuracy literal 50ppm (no enum), 'static-mut rng passed BY VALUE (build ties SDC 'd to it). **STAGING IN PROGRESS:** release UF2 build needs **CARGO_PROFILE_RELEASE_LTO=false** (nrf-mpsl C-bitcode can't LTO-merge with esp-clang's LLVM); board-verified Alfred map (ttyACM1=RAK VID1209, ttyACM0=Arduino, ttyACM2=XIAO esp32s3 D8:3B:DA:75:C3:3C — DO-NOT-TARGET). NEXT: elf2uf2.py → stage on alfred → ttyACM1-verified flash one-liner (double-tap→INFO_UF2.TXT offset 0x26000/0x27000→drag-drop). **⚠ DO-NOT-ASSUME (the metal risk the spike TESTS): dormant-s140 + nrf-sdc-takes-radio-directly coexistence is UNPROVEN — compile proves nothing about it; if no advert/crash on metal, that's the likely cause (BLE-PLAN §2 claims it works, metal decides).** Core co-authors the CoC seam (2b) after green-2a.
**✅ STAGED ON ALFRED + board-verified flash helper (2026-07-11):** `~/rak-blespike/r2-rak4630-blespike-demomember-b7691c0.uf2` (sha256 **ae79d44b45b18ef3...c9f944**, family 0xADA52840, links @0x26000, 134KB image). Roy flashes via `~/rak-blespike/flash-rak-blespike.sh` — REFUSES unless ttyACM1=VID1209 (RAK), sha-checks, waits for the RAK4631 drive after double-tap, confirms INFO_UF2 offset, drag-drops. SAFE: XIAO esp32s3 (ttyACM2) + Arduino (ttyACM0) never touched (UF2→RAK4631 drive only). **BUILD CMD (reproducible):** `CARGO_PROFILE_RELEASE_LTO=false LIBCLANG_PATH=<esp-clang> cargo build --release --no-default-features --features dev,blespike,demomember,uf2` (--no-default-features drops cs-cortexm → nrf-mpsl's priority-aware CS wins, fixes duplicate _critical_section_1_0 link err; LTO=false: nrf-mpsl C-bitcode can't merge) → `python3 tools/elf2uf2.py`. **WAITING: Roy flashes + reports (LED floor=boot; external BLE scanner sees the 31B advert + LoRa still runs = GREEN-2a).** ⚠ dormant-s140/nrf-sdc coexistence = the unproven metal risk (compile proves nothing). Commits: 526c815 (bring-up) + b7691c0 (CS-feature fix).
**🔧 FLASH-HELPER BUG FIXED (2026-07-11, rak4630-fw 5e750d0; NOW TRACKED at platforms/rak4630/tools/flash-rak-blespike.sh + re-staged ~/rak-blespike/ on Alfred):** Roy's flash STALLED at step 2 — the helper waited for an OS auto-mount (/run/media,/media) but **Alfred does NOT auto-mount**; the RAK4631 UF2 drive appears as `/dev/disk/by-label/RAK4631` (block dev) but stays UNMOUNTED → 60s timeout. Double-tap/bootloader entry worked (ttyACM1 1209→239a). FIXES: (1) key detection on the by-label BLOCK DEVICE not an auto-mount; (2) **self-mount via `udisksctl mount -b` + `findmnt`**, cp, sync (best-effort — board auto-resets on UF2 receipt), `udisksctl unmount`; (3) **already-in-bootloader fast-path** (identify RAK by VID 239a + WisBlock_RAK4631, skip the double-tap). Flash target is ALWAYS the by-label device (RAK-unique; Arduino=2341/XIAO=303a never match) — safety anchor holds regardless of the app/bootloader VID flip. Verified syntax + dry-run detection vs live state. **⚠⚠ LIVE STATE UPDATE (2026-07-12 11:43): the RAK is now DISCONNECTED from Alfred.** A USB re-enumeration shuffled the ports — `ttyACM1` is now the **XIAO esp32s3** (303a USB_JTAG_serial_debug_unit), `ttyACM0`=Arduino (2341); NO VID 1209/239a device present; `/dev/disk/by-label/RAK4631` is a **DANGLING** symlink (RAK gone). So **green-2a is BLOCKED on RAK RECONNECTION**, not merely on Roy running the helper. **🔧 HELPER HARDENED (rak4630-fw 74a9764, re-staged): step 1 no longer hardcodes ttyACM1** — the shuffle proved that stale — it now **SCANS every /dev/ttyACM* for the RAK's unique VID** (1209 app / 239a+WisBlock_RAK4631 bootloader) wherever it enumerates; refuses on 0 matches (RAK absent) or >1 (ambiguous); falls back to a LIVE by-label device if no CDC tty. Flash target UNCHANGED = the by-label RAK4631 device (RAK-unique) — XIAO(303a)/Arduino(2341) can NEVER be flashed. **Verified: real run aborts SAFELY with the RAK absent (XIAO-on-ttyACM1 + Arduino NOT mis-identified; no mount, no cp, exit 1).** Green-2a verdict still owed from Roy once the RAK is reconnected.

## 🔨🚧 BUILDING — P3 USB-PAIR simple-secure PERIPHERAL SM (task #67) — supervisor GO 2026-07-12 against R2-PROVISION v0.50 @0f61c81
**★ GO #4 = BUILD ORDER (supervisor 2026-07-12):** build the simple pairing PERIPHERAL SM INTO the xiaobridge fw against twin-clean **v0.50 @`0f61c81`** (verified: ancestor of specs HEAD aff328a on spec-conformance-v0.2), regenerate vectors, re-verify terminal MACs, THEN reflash the XIAO so pairing lights over the live USB link (android runs PairingHost over ttyACM1 = the bench-critical reflash). BLE-beacon reflash stays DEFERRED behind WiFi; keep LoRa/USB bridge live; fold BLE into the same reflash if convenient. OK to install pyserial in a venv for a **DTR-LOW (no-reset) banner read** for hive_id/TG.
**✅ SPEC READ @0f61c81 (scratchpad/R2-PROVISION-0f61c81.md §5.3.4 lines 448-798).** Frame vocab (R2-USB §3.7 control `[0xFF]‖CBOR({0:msg_type,1:fields})`): 4 PAIR_HELLO_HOST host→p `{}` · 5 PAIR_COMMIT p→host `{1:commit(32)}` · 13 PAIR_HOST_REVEAL host→p `{1:eph_pk_host(32),2:nonce_host(32)}` · 6 PAIR_REVEAL p→host `{1:eph_pk_p(32),2:nonce_p(32)}` · 7 PAIR_CONFIRM host→p `{1:confirm_mac(16)}` · 8 PAIR_DONE p→host `{1:done_mac(16)}` · 14 PAIR_ACK host→p `{1:ack_mac(16)}` · 9 RECONNECT_CHALLENGE host→p `{1:nonce_rc(16)}` · 10 RECONNECT_RESPONSE p→host `{1:tag(16)}` · 11 PAIR_ABORT either `{1:reason tstr}`. (msg_type 12=observation, 15=retired.) **PERIPHERAL SM:** recv HELLO(4)→RETAIN persisted key, gen fresh eph+nonce, send COMMIT(5); recv HOST_REVEAL(13); send REVEAL(6); Z=X25519, reject all-zero→ABORT{bad_key}; recv CONFIRM(7) verify confirm-host MAC; send DONE(8); recv ACK(14) verify ack-host MAC→**PERSIST K**. Reconnect: recv CHALLENGE(9)→tag=HMAC(link_key,"r2-usb-reconnect-v1"‖nonce_rc‖usb_link_id)[..16]→RESPONSE(10); activation host-local (peripheral proves to host only). Idempotent in-session dup (RAM, 60s terminal timeout): dup CONFIRM→resend DONE, dup ACK→no-op. Key preservation: persisted key retained until final ACK-verify; re-pair on any doubt (no journal/gen/rotation/crash-recovery). transcript = eph_pk_host‖eph_pk_p‖nonce_host‖nonce_p (128B). usb_link_id = CAPS hive_id_bytes (16B) = HKDF("r2-usb-link-v1",device_master_secret,"")[..16].
**✅ TERMINAL MACs RE-VERIFIED byte-exact vs v0.50 (supervisor's re-verify check DONE):** usb_pair.rs labels + asserted hex == spec UP14/UP18 (confirm `4e4c5ff2…`/done `08ba274f…`/ack `1ec03c3d…`/reconnect `2f62edaa…`); KDF domains did NOT move (spec line 570: byte-exact constructions unchanged, naming-only). 14 usb_pair tests green. Crypto restored @5fc3a20 is spec-valid — no revector needed.
**⚠ SCOPE BOUND (spec §5.3.4 lines 456-460, 573-602):** pairing establishes `link_key` ONLY. Key-bearing USB-SAS join (§3 JoinInvite/JoinResponse group-key install) is SPEC-GATED **fail-closed** until the owed `JoinInvite`↔`link_key` invite-MAC binding lands — do NOT implement join key install; the SM ends at link_key + reconnect.
**✅ BUILT + BYTE-PROVEN (2026-07-12) — reflash PENDING android integration contract + supervisor ping.**
**dfr1195-fw commits:** 238a6de (crate) + b54915b (fw wiring). **NEW crate `dfr1195-fw/crates/r2-usb-pair`**
(host-testable no_std lib, root-ws member, xtensa-buildable via fw path-dep): crypto + §3.7 control-frame
codec + peripheral SM. **11 host KATs GREEN incl. the refutation-grade ones:** full choreography emits
UP14.sequence `frame_hex` BYTE-EXACT (HELLO→COMMIT→HOST_REVEAL→REVEAL→CONFIRM→DONE→ACK), reconnect ==
UP18.frame_response_10, bad_key/protocol_error aborts byte-exact, non-contributory + premature-reveal +
idempotent-dup all pass. Crypto == UP1-8/13/14/18. **Wired into xiao_bridge_task** (0xFF control-frame demux
in the `[len u16 LE][payload]` bridge RX loop; link_key persisted @0x1C000 magic 'R2LK'; usb_link_id derived
via derive_usb_link_id). **Both xtensa builds GREEN: `xiaobridge,benchsf7` (pairing wired) AND default
(proven path untouched — all pairing code xiaobridge-cfg-gated).**
**⛔ REFLASH BLOCKED on 2 android integration-contract answers (fleet-sent; do NOT flash until resolved):**
(1) FRAMING — does PairingHost wrap §3.7 control frames in the same `[len u16 LE][payload]` bridge framing
(my demux keys on payload[0]==0xFF)? (2) CAPS/usb_link_id — does PairingHost require a CAPS frame advertising
hive_id_bytes before opening, and does it bind link_key to whatever the peripheral advertises? **usb_link_id
BENCH stand-in = HKDF(r2-usb-link-v1, efuse-MAC)[0:16]** (spec IKM = persona master_secret, not surfaced by
parse_persona — owed when the XIAO carries a persona). **CAPS emission NOT yet built** (pending whether android
needs it). **Refutation status:** byte-exact vector conformance IS the adversarial check for crypto/protocol;
opposite-provider fleet-refute is UNAVAILABLE (isolation containment — send-only), so no twin pass on the
integration wiring yet — carrying that as an explicit gap. **Also owed:** DTR-LOW pyserial banner read (supervisor
authorized) for the XIAO hive_id/TG. **Vectors:** UP1-8,13,14,18 ACTIVE; UP9-12,15-17 NON-ACTIVE (durability).
**Vector generator (r2-hive gen_usb_pair_vectors.rs) still emits the OLD UP1-12 set — coverage-floor regen owed
(the LANDED vector file @specs is already v0.47 simple-secure + is the contract; generator is stale hygiene).**
**✅ REFLASH STAGED (2026-07-13, supervisor 4-point resolve) — dfr1195-fw platforms/dfr1195/REFLASH-XIAO-PAIRING.md (pushed).**
Complex-hive reframe CONFIRMS scope (phone+XIAO=1 hive, USB=internal bus, no group-key-over-USB; specs will REMOVE
the USB-SAS key-bearing path entirely, Roy-gated). **Artifact transfer = GIT reproducible-build-on-Alfred** (release
VERIFIED here: compiles+LINKS clean once export-esp.sh sourced = 1.12MB ELF; NO binary in git). **Flash = 1 human-gated
espflash cmd** keyed on STABLE by-id serial D8:3B:DA:75:C3:3C (never bare ttyACM; ttyACM0=Arduino=NEVER), Roy runs,
sequenced vs android capture. **⚠ BANNER-READ MOOT** (supervisor authorized DTR-LOW read, but xiaobridge esp-println/no-op
= NO banner on the USB stream); identity = MAC-derived usb_link_id (pairing binds this) + likely-unprovisioned mac_low3
hive; **CAPS emission = the clean identity+usb_link_id exposure path — PROPOSED, pending android confirm they want it.**
**STILL THE reflash gate: android framing/usb_link_id/CAPS answer (fleet-sent, pending) → then build CAPS if needed →
Alfred build+flash by Roy.**
**★ android ANSWERED (design-intent, their host SM HELD/reverted @7a2c950 pending specs+ping): (1) FRAMING confirmed
== my build ([len][0xFF‖CBOR]); (2) CAPS REQUIRED — host expects §3.6 CAPS advertising hive_id_bytes(=usb_link_id)
BEFORE opening, binds link_key to the CAPS value.** **⚠ CONFORMANCE FINDING (grow-strong-ideas, before building CAPS
— ledger docs/ledger/xiaobridge-pairing-framing.md):** R2-USB §3.5 = a v2 link MUST type-byte EVERY frame
(0x00-0xFB local_id R2-WIRE, 0xFE CAPS, 0xFF control); §3.5:313-315 "no dev-mode shortcut" = advertising v2 while
sending untyped/legacy frames OR skipping CAPS is NON-CONFORMANT. The xiaobridge SYNC advertises v2 (`02`) but
forwards RAW untyped LoRa frames → non-conformant. My 0xFF control demux IS §3.5-clean; the GAPS = no CAPS (0xFE) +
untagged LoRa frames. **Superseding conjecture (v2): converge the bridge to a FULLY §3.5-conformant v2 link
(prepend local_id byte to LoRa frames + emit §3.6 CAPS + 0xFF pairing) — spec-clean + north-star, BUT changes the
egress format android's built parse_bridge_stream (d8696fd) + its LIVE capture depend on → cross-repo, MUST be
sequenced.** **PIVOTAL OPEN ATTACK: does the complex-hive reframe (USB=INTERNAL bus) EXEMPT the bridge from full
§3.5 conformance? Only specs/supervisor can rule.** ESCALATED to supervisor for the conformance call + sequencing;
android heads-up sent. usb_link_id: canonical = HKDF(r2-usb-link-v1, device_master_secret)[0:16] (§3.6 normative,
UP13); host takes it from CAPS so NO unprovisioned-fallback decision needed (my refutation of android's "needs a
fallback"). **★ android DE-RISKED path (A) (2026-07-13):** its host ALREADY has the §3.5 type-byte demux built
(core-ffi/src/usb.rs 0xFE/0xFF/encode_local_id_frame); 0xA1 sighting home ALREADY canon = **0xFF msg_type=12
OBSERVATION** (retires 0xA1, resolves my open attack); **NO live LoRa capture running** (XIAO quiet, no 2nd SX1262)
so sequence-free; usb_link_id refutation ACCEPTED. Ledger v2 conjecture rose **0.5→0.85**. **KEY: path (A) DOMINATES**
— even if the reframe would exempt the bridge, (A) is never wrong (more conformant + north-star), so the ruling only
decides effort-timing. **✅ BUILT the §3.6 CAPS encoder (r2-usb-pair encode_caps, host KATs green); android CONFIRMED byte-exact @363a39d.**
**★ supervisor GO'd path (A) + ruled DEFAULT-TO-CONFORMANT (no complex-hive exemption) 2026-07-13 → CONVERGENCE LANDED
(dfr1195-fw 06a4dab):** the xiaobridge is now a fully §3.5-conformant v2 link — CAPS(0xFE) emitted after SYNC; LoRa
egress frames tagged with local_id 0x02 ([len][02][compact]); sightings re-encoded 0xA1→§3.7.1 OBSERVATION
[len][0xFF][{0:12,1:{0:beacon,1:bearer,2:rssi,3:snr}}] (encode_observation, **byte-exact vs TV27**); ingress
local_id-tagged frames stripped+TX'd; pairing on 0xFF unchanged. **13 crate KATs green; BOTH xtensa builds green;
release LINKS clean = 1.125MB ELF (reflash artifact ready, recipe unchanged).** Pivotal open attack (reframe-exemption)
= RESOLVED (supervisor: conformant, no exemption). **REFLASH now READY on my side** — coordinate: android aligns
bridge.rs to the conformant framing (they said they'd do it once I land the egress = DONE) + un-holds+builds its host
PairingHost SM (gated on specs+ping) → Roy runs the Alfred flash (recipe REFLASH-XIAO-PAIRING.md, sequenced). The
reflash puts the conformant peripheral on the board; the pair LIGHTS once android's host SM is built.
**✅ VERSION-DRIFT CHECK CLEAN (grow-strong-ideas, 2026-07-13): peripheral CONFIRMED byte-conformant to android's build
target.** Diff my pin (v0.50 @0f61c81) → specs origin/main: r2-usb-pair-vectors.json UNCHANGED (all UP1-8/13/14/18 +
frame_hex identical); TV27 observation on main = byte-identical to my encode_observation KAT; CAPS/local_id framing
unchanged; main's §5.3.4 change = USB-SAS key-bearing REMOVAL (a path I never built — scope was link_key only) +
§3.4(b) glance-SAS fix; "no byte drift". **REFLASH is now purely PHYSICAL-gated (nothing owed from hive):** android's PairingHost SM BUILT + byte-proven
@ff649da (11 KATs, host frames vs UP14 + reconnect UP18); **VECTOR-TRANSCRIPT INTEROP PROVEN BOTH SIDES** (my
choreography KAT consumes the exact UP14 host frames + emits correct responses; android's KATs mirror; shared
constructions ⇒ interop for any keys). **SOLE remaining un-run attack = METAL interop** (real random keys + real
USB-JTAG link + live SYNC→CAPS→pairing) — will NOT claim done until a real pair lights (ledger conjecture 0.95, not
1.0). Gated on: Roy reconnects the XIAO to Alfred's bus (OFF-bus, RAK holds ttyACM1) + runs the by-id espflash recipe
(REFLASH-XIAO-PAIRING.md, reproducible-from-source, release-verified) → android wires FFI + drives pairing over
ttyACM1 → first metal light. My CAPS + msg shapes are FINAL (version-drift-checked vs specs main).

## 📦 ARCHIVE — P3 Profile-A/B durability saga (BENCH-DROPPED, superseded by the v0.50 simple-secure GO above)
The v0.34→v0.44 Profile-A/B refutation arc (whipsaw A→retract→B→simple; STAGE-1 crypto built @9114254, reverted @3fff533, restored @5fc3a20; hive-codex/supervisor-codex durability blockers = REVEAL-crash split, simultaneous-power-loss split, lineage/target_gen, v1-fallback-bypass-gate) is **no longer the active path** — Roy dropped full durability for the bench (USB link is transitional→on-board). Full crash-durability is a parked FIELD track (`docs/proposals/USB-PAIRING-DURABILITY-REWRITE-2026-07-12.md`). **Two durability findings still worth carrying into the simple SM build:** (i) my host `usb.rs` negotiates down to v1 (`negotiates_down_to_v1_when_peripheral_responds_v1`) — simple-secure doesn't gate on a durable activation, but keep the pairing carried over the v2 control-frame path; (ii) the hive_id-vs-usb_link_id input to link_key = the CAPS `hive_id_bytes` (usb_link_id, TG-independent device-life-stable), NOT the mesh hive_id (spec §5.3.4 lines 556-571 + UP13). Detail lives in git history / RESUME-archive if the field track revives.

## (was 🔒 HOLD) — P3 USB-PAIR X25519 security finding (android-codex cross-impl, 2026-07-12) — CONFIRMED real in hive; fix CANON-GATED + cross-impl-aligned
android-codex flagged that android P3a AND hive share two P3 (R2-PROVISION §5.3.4 USB pairing) gaps. **BOTH CONFIRMED against hive ground truth:**
1. **No contributory-key rejection (defense-in-depth MUST).** `usb_pair::shared_secret` (usb_pair.rs:68-73) returns `*z.as_bytes()` with NO `z.was_contributory()` check — a zero/low-order `peer_pk` forces Z=0. **⚠ THREAT NARROWED (android-codex + specs advisory 2026-07-12 — my earlier "MITM defeat / attacker-known SAS" framing was TOO STRONG):** the SAS is TRANSCRIPT-BOUND (usb_pair.rs `sas_code` HKDFs Z over eph_pk_host‖eph_pk_periph‖nonce_host‖nonce_periph — verified), so classic MITM key-SUBSTITUTION is ALREADY caught by the human SAS comparison. Contributory-reject is a **defense-in-depth MUST** (specs-confirmed), NOT the sole MITM guard. Still LIVE-REACHABLE (consumed at usb.rs:812 inside the `UsbSession` P3 state machine) + a real gap to close. Negative vector = **UP14** (UP13 is already usb_link_id — specs typo flagged); planned abort reason = dedicated **bad_key** (pending canon).
2. **zeroize hygiene gap.** `x25519-dalek` in Cargo.toml:43 is `default-features=false, features=["static_secrets"]` — NO `zeroize` feature, so StaticSecret/SharedSecret lack ZeroizeOnDrop despite R2-PROVISION's destroy-eph_sk/Z MUST. **⚠ NUANCE: enabling the feature alone is INSUFFICIENT** — `shared_secret` returns a plain `[u8;32]` copy and usb.rs stores Z/link_key/eph_sk as plain arrays (never zeroized); proper destruction needs `Zeroizing<[u8;32]>` on the STORED secrets in usb.rs, done together with the feature.
**DECISION = HOLD, do NOT patch unilaterally (spec-first + cross-impl parity).** The rejection semantics need canon — specs/android escalation ACTIVE to pin was_contributory-rejection in R2-PROVISION §5.3.4 + a negative KAT. Patching hive's `shared_secret` API (→Result/Option) now would DIVERGE from Android's approach + pre-empt the canon. **When canon lands: (a) reject `!was_contributory()` at the primitive with the dedicated `bad_key` abort, aligned with Android, (b) add UP14 (low-order peer_pk → bad_key reject), (c) `Zeroizing<>` the stored Z/link_key/eph_sk + enable the x25519-dalek zeroize feature — ALL in one coordinated pass, both impls together.** ⚠ **The actual adaptive-SAS-grind blocker is SEPARATE (android-codex leading): first-pair `HELLO_HOST` exposes the host contribution BEFORE the peripheral COMMIT — HOLD the P3 state-machine CHOREOGRAPHY until that ordering + the canon land.** **✅ Android's PRIMITIVE is already hardened (a03d949); so the remaining cross-impl coordination = state-machine choreography + Zeroizing + the exact abort bytes, AFTER the ordering fix + canon land (per android-codex 2026-07-12).** Reject prevents unsafe Z=0 reuse; a malicious endpoint only ever knows its OWN session (NOT the honest endpoints' keys) — do NOT overframe as classic-MITM key-recovery. Confirmed both gaps to android-codex; holding P3 hardening. Folds around the P2 v2-conformance work (same usb.rs surface).
**➕ msg7/8 KEY-CONFIRMATION / DESYNC finding (hive-codex, specs ruling design-only 2026-07-12 — v0.34 NOT LANDED) — CONFIRMED against ground truth, HELD:** (a) `handle_pair_done` (usb.rs:843-859) ignores the DONE body (`_body`) + persists `link_keys.store(&lk)` with NO MAC check; (b) `user_confirms` (usb.rs:898) emits an EMPTY PAIR_CONFIRM (no MAC); (c) `pending_link_key` derived BEFORE the operator prompt (usb.rs:832, eager). **Threat = availability/key-confirmation desync (an active USB attacker blocks CONFIRM + injects DONE → host-only persistence, or injects CONFIRM at peripheral → inverse desync), NOT key recovery.** **SPECS-DRAFTED FIX (design only, apply in the coordinated pass when v0.34 lands):** PAIR_CONFIRM(7)=`HMAC-SHA256(link_key, 'r2-usb-pair-confirm-host-v1' ‖ transcript)`; PAIR_DONE(8)=`HMAC-SHA256(link_key, 'r2-usb-pair-confirm-peripheral-v1' ‖ transcript)`; `transcript = eph_pk_host‖eph_pk_peripheral‖nonce_host‖nonce_peripheral`; verify CONSTANT-TIME; fail→`bad_key`+destroy pending; **each side persists ONLY after a valid peer MAC.** Eager pending key = conformant IFF destroyed-on-decline (specs; canon wording changes derive→persist/retain) — current wipe (usb.rs:274-281) is BEST-EFFORT only (real zeroize owed = same Zeroizing pass). **Also CONFIRMED-current + CONTESTED-canon: msg4 sends host contribution before commit (usb.rs:723-745, commit:None) — specs working tree has an uncommitted alternate empty-msg4/msg13; choreography still contested → HOLD.** **Also: `run_session` lacks an ENFORCED 60s deadline (documented at usb.rs:126/904 but no Instant/Duration timer) — retain for the coordinated rewrite.** hive-codex asked specs to rule; I do NOT patch locally (specs v0.34 mid-edit/uncommitted; Android confirms no persist path today + is holding its terminal path). **ONE coordinated P3 rewrite when v0.34 lands: was_contributory-reject + MAC'd CONFIRM/DONE + persist-after-peer-MAC + destroy-on-decline Zeroizing + enforced 60s deadline + UP14 negative KAT — all cross-impl-aligned with Android byte-for-byte.**
**✅ STATUS RESOLVED (android-codex 2026-07-12): v0.34 IS clean-landed on specs origin/main `d6f13b0` (semantic commit `c57d568`).** Android's poisoned 271132f was REVERTED (`971acae`; origin/master recovered at `2633e26+`) — no revert pending; still don't mirror that reverted content. **⚠ HOLD GATE (updated 2026-07-12, android-codex): the CONFIRM/DONE MAC BYTES now LANDED + independently verify (v0.35). Coordinated P3 STILL HELD on a DEEPER blocker = finalization crash/loss key-desync + a spec self-contradiction:** (1) **the peripheral replaces K0→K1 on CONFIRM BEFORE the host verifies DONE** — a lost DONE or a crash mid-finalization strands the sides at **K1 (peripheral) vs K0 (host)** with NO single-key reconnect recovery. Needs a **dual-slot / atomic-finalization crash-loss canon + vectors** (keep both keys until DONE verified, or a defined recovery). (2) **specs self-contradiction:** "discard stale key on HELLO" appears in TWO sequence lines, contradicting the key-PRESERVATION MUST — needs resolving before impl. **Await the dual-slot/finalization crash-loss canon + vectors.** Until then: do NOT implement the CONFIRM/DONE MAC yet (the bytes verify but the state-machine finalization semantics are unsafe as written) and do NOT split out was_contributory/Zeroizing early — land the whole pass once, byte-aligned with Android (which is holding its terminal path too). v0.34 base = specs origin/main d6f13b0; verify canonical text directly on specs, never mirror a reference commit.
**⚖️ specs-UP14-request vs supervisor-HOLD reconciled (2026-07-12) → SUPERVISOR HOLD WINS:** specs (v0.36 origin 297d811) asked me to lock 2 impl-source constants I own (like UP1-12) + supply/verify UP14 bytes (provisional confirm=`4e4c5ff2…`, done=`08ba274f…`) + implement the dual-key reconnect recovery. I CONFIRMED the 2 low-risk DESIGN constants (they help regardless of refutation outcome): **(1) truncation = 16B** (matches `reconnect_tag` [0..16]); **(2) labels = `r2-usb-pair-confirm-host-v1` (PAIR_CONFIRM/7) + `r2-usb-pair-confirm-peripheral-v1` (PAIR_DONE/8)** — DONE label is `confirm-peripheral-v1` NOT `done-peripheral-v1`. **BUT supervisor (same turn) HELD the pairing mirror: canon NOT freeze-ready — v0.36 still under ACTIVE refutation (first-pair lost-DONE / one-sided key desync / reconnect crash-ACK / retry semantics); mirroring/freezing now replicates a wrong+moving target.** So I **did NOT land** the confirm_tag MAC helper or lock/byte-verify UP14 (I drafted the helper in usb_pair.rs then REVERTED it — freezing impl-source against a non-frozen MAC/transcript is premature). **HELD until supervisor pings that the pairing canon CONVERGED + Android's first-pair impl settled — THEN, once against the final version: land confirm_tag helper + byte-verify UP14 + wire persist-after-peer-MAC + dual-key reconnect recovery + was_contributory + Zeroizing + 60s deadline, cross-impl-aligned.** Redirect per supervisor = RAK BLE spike / MCUboot slot-map / recovery ladder (all currently gated on Roy). Do-not-mirror-android-state-machine stands.
**✅ MY 2 CONSTANTS VALIDATED (android-codex 2026-07-12): its independent impl REPRODUCES the v0.36 MAC bytes `4e4c…`/`08ba…`** — so my confirmed 16B truncation + labels (`confirm-host-v1`/`confirm-peripheral-v1`) + transcript order are CORRECT (byte-agreement across impls). **Still HELD — the REMAINING gap is the dual-key RECONNECT wire, not the MAC:** the current shape (one UNKEYED challenge + one response tag) gives the responder NO K0/K1 SELECTION signal → responder can't tell which key the peer is reconnecting under (K1 fixes lost-DONE but K0 is needed to finalize a re-pair; a TTL can't resolve unknown DONE delivery). Needs a **selector / dual-tag / final-ACK wire + exact state+crash vectors** (canon owes this); stale discard-on-HELLO text still persists. Byte-lock/impl still HELD until that converges + supervisor pings.
**➡️ v0.37 pinned the FIX = ratified 3-MESSAGE durable-journal atomic key-commit terminal:** PAIR_CONFIRM(7)→PAIR_DONE(8)→**PAIR_ACK(14)**, three K1-auth HMAC-SHA256[0..16] tags over the 128B transcript; bilateral durable JOURNAL {transcript,K0?,pending=K1,phase∈{CONFIRM_SENT,DONE_SENT,PROMOTED}} written BEFORE each step + crash-resumed; monotonic promotion (peripheral keeps K0 until it verifies ACK, host promotes on DONE-verify); idempotent dups; K0 retired ONLY on a successful K1-reconnect, NEVER on TTL; authenticated reconnect (RECONNECT_CHALLENGE carries a K1-preferring key-slot SELECTOR + host authenticated acceptance) = the msg 9/10/15 reconnect domain.
**↻ v0.37→v0.40 = 7 rounds of twin refutation on the RECONNECT/durability domain (full per-version audit in git log 2026-07-12; condensed here):** each round found a fresh real defect — CONFIRM/DONE bytes verified (v0.35) → design-partial reconnect selector (v0.36) → key-RETIREMENT permanent K1/K0 split (supervisor-codex, §5.3.4 L558-561/L580-583, DONE_SENT-after-PROMOTED) → `host_gens` list NOT MAC-bound = MITM strip-K1/DoS (v0.39) → host key-RETENTION deadlock (v0.40, host must retain K1 from CONFIRM send, not drop pre-DONE). Throughout: I EPHEMERALLY verified my 3 terminal MAC constants for specs (confirm/done/ack; 3-way byte-agreed with specs' provisional + android's independent OpenSSL) then REVERTED each time — **NEVER landed against the moving target; tree always clean.** Do-NOT-mirror Android 271132f (reverted retired-v0.33 flow). Verify canonical text directly on specs, never a reference commit.
**✅ TERMINAL MACs RE-LOCKED (specs v0.41 §5.3.4 + pair-vectors 0.9, origin c24cfa9, 2026-07-12) — MY VALUES STABLE, no longer provisional:** confirm=`4e4c5ff2…`/done=`08ba274f…`/ack=`1ec03c3d…` re-locked in canon because I confirmed (impl-owner) the **terminal domain binds NO gen/phase/host_gens** (transcript = eph_pk_host‖eph_pk_peripheral‖nonce_host‖nonce_peripheral, 128B, K1 — mirrors sas_code). The 7-round v0.34→40 churn was ALL in the SEPARATE reconnect domain (msg 9/10/15), never the terminal MAC — my ephemeral-verified constants were the stable anchor that let specs re-lock. **⏳ IMPLEMENTATION still HELD pending a ROY A/B SCOPING CALL (supervisor paused the crash/rotation-durability grind):** **Option A = ship happy-path first-pair + SIMPLE same-key reconnect NOW** (single link_key, the original gen-free reconnect_tag, NO generations/rotation/journal), defer durability to a field-hardening track → **this is ~all the pairing crypto I ALREADY HAVE (usb_pair.rs + usb.rs) + the 3 terminal MAC helpers, MINUS the rotation state machine** (small, low-risk); **Option B = hold for full convergence** (dual-key/journal/rotation). specs pings the MOMENT Roy rules → I implement to the PINNED profile, not a guess. usb_link_id/UP13 (R2-KEYSTORE info-independent device identity) already aligned. **Terminal MACs now 4-WAY validated** (my impl + specs provisional + android OpenSSL + hive-codex OpenSSL, all byte-exact).
**➡️ OPTION A CHOSEN (supervisor, per specs inbox ts1783826113 — relayed by hive-codex; STILL HOLD CODE until specs lands the buildable carve-out + supervisor pings ME directly):** secure BENCH profile = first-pair `HELLO→COMMIT→HOST_REVEAL→REVEAL→CONFIRM→DONE→ACK` + basic SAME-KEY reconnect; **NO generations/rotation/crash-journal**. Field durability stays required/parallel (separate track). **⚠ OPTION-A IMPLEMENTATION GUARDRAILS (hive-codex adversarial — bake into the P3 pass, do NOT skip):** (1) **scope MUST explicitly name lost-DONE/ACK/reboot recovery + persistence behavior** — else "deferred durability" silently recreates the one-sided-state desync; (2) **update the VECTOR pipeline, not just constants:** `gen_usb_pair_vectors.rs` today emits only UP1-12 gen-free + vendored fixture is v0.1 + `vector_coverage.rs:62-65` min_referenced=0 (not replayed) — specs v0.9 metadata FALSELY claims UP1-17 come from the helper; regenerate + vendor + real JSON replay/coverage in the SAME pass (do NOT hardcode the 3 terminal bytes in a unit test + claim vector conformance); (3) **basic reconnect msg9/10 CONFLICTS with v0.41 generation reconnect msg9/10/15** → pin explicit profile/version/CAPS negotiation OR a distinct wire so a later field upgrade can't reinterpret frames; do NOT claim full v0.41 conformance; (4) **SAS timeout applies ONLY to the pre-CONFIRM operator window — after CONFIRM, NO destructive timeout/wipe** (peer may have acted); (5) **profile-DOWNGRADE guard:** the SAS/link_key/terminal-MAC transcript does NOT bind the profile, so an unauthenticated A/B negotiation + field fallback lets a MITM downgrade B→A with no drift — **NO opportunistic fallback; use a LOCAL REQUIRED policy (bench requires A; field requires B + rejects A/mismatch)**; dynamic negotiation would need profile-transcript-binding + new KATs. **2 canon hygiene blockers specs must fix first:** (i) R2-PROVISION:514-521 "SAS decline → no link_key derived" contradicts the eager-pending-conformant-iff-destroyed ruling → wording to derive→**persist/retain** (hive currently derives pending pre-prompt); (ii) v0.41:702-713 conformance prose still calls UP14 provisional/v0.40 vs the 669-678 RE-LOCK → remove stale language. Reconnect vectors/KATs stay provisional.

## ⏸ HOLD (P2 USB-framing conformance blocker — supervisor-codex CONFIRMED; fold AFTER the BLE window)
The XIAO 0xA1 beacon-sighting producer (dfr1195-fw) + the Android decoder advertise R2-USB **v2** (SYNC 02) but forward LEGACY frames + a top-level 0xA1 — which is INVALID: v1 allows only one R2-WIRE payload; v2 §3.5 treats 0xA1 as `local_id` and REQUIRES a compact R2-WIRE body (the sighting is not) + CAPS. So the current Android+XIAO green is a **BILATERAL VIOLATION = FALSE green**. **DO:** HOLD the P2 conformance verdict; preserve the current artifact as REGRESSION INPUT (not a conformant release); do NOT mint replacement bytes locally — **specs provides the canonical observation-encoding home** for the raw beacon that does NOT collide with v2 `local_id`. Coordinate android + specs on (framing version + the canonical 0xA1-sighting encoding); my XIAO producer matches whatever is agreed. **Priority: fold when I surface from the BLE metal spike — NOT urgent over the BLE window, but P2 is UNTRUSTED until this lands.** Gates the P2 two-bearer bench (live green on drifted framing = false green).
**✅ SPECS RULED the fix DIRECTION (supervisor-codex relay):** current 0xA1 is non-conformant in BOTH v1 and v2. Canonical home = a **NEW v2 `0xFF` control subtype** for link-local radio observation — likely **CBOR body** {bearer, local_id, raw beacon, optional RSSI/timestamp}. REJECTED: 0xFC/0xFD, and R2-WIRE-wrapping the sighting. Intent = link-local telemetry ONLY. **STOP shipping 0xA1** (preserve hex as regression input). **NEEDS: a hive proposal + Roy/supervisor GO before implementing** (author the 0xFF-subtype proposal after the BLE window). Full v2 alignment ALSO needs (separate, larger): type-tags on ordinary frames + CAPS after SYNC + pairing/reconnect or explicit dev-mode status. **FOLD-PLAN: post-BLE → author the v2 0xFF control-subtype proposal (spec-first) → Roy/supervisor GO → implement in the XIAO producer (dfr1195-fw) + coordinate android to match.**
**SCHEMA (specs pinning, still settling — bind to specs' proposal-of-record `r2-specifications/docs/proposals/R2-USB-OBSERVATION-SIGHTING-2026-07-11.md`, UNCOMMITTED):** v2 0xFF control frame, **msg_type=12** (observation), link-local MUST-NOT-relay by frame class. Body `0xFF || CBOR({0:12, 1:{0:beacon-bstr-verbatim, 1:bearer, 2:rssi, 3:snr-optional}})`. **Bearer = R2-TRANSPORT §2.2: BLE=0/WiFi=1/LoRa=2** (specs corrected the relayed variants; my LoRa producer already emits Transport::Lora=2 ✓). ⚠ relayed schemas CONFLICTED (supervisor-codex had local_id@0/quarter-dB; android hop4 had bearer 1=LoRa/2=BLE) — specs' latest DIRECT ruling (bearer BLE=0/LoRa=2, beacon@key0) is authoritative; android asked specs to confirm final. **CAPTURE SHAPE CONFIRMED to specs (from producer main.rs:4835): beacon = R2-BEACON-from-0xB2-magic VERBATIM (NOT full 7.3 AD, NOT CID/header); LoRa=16B prod/17B dev; BLE=build_legacy_beacon[4..].** I OWN the proposal (android repositioned its doc to consumer-side INPUT, one proposal-of-record = mine). android needs a real PROD 16B LoRa capture (I generate at author-time). HARD PREREQ both sides: v2 type-tag framing + CAPS-after-SYNC (legacy framing = the drift root); android usb.rs lacks both today. One-pass phone+XIAO+dfr1195 vs a shared vector, AFTER Roy/supervisor GO.
**✅ ROY AUTHORIZED (supervisor 2026-07-11): msg_type=12 sighting + FULL V2 conformance — NO dev-mode shortcut.** XIAO producer must implement FULL v2 framing (type-tag every non-SYNC frame + CAPS-after-SYNC + pairing/reconnect) + emit the msg_type=12 observation control frame (canonical CBOR, bearer BLE=0/LoRa=2) REPLACING 0xA1. **SEQUENCE: fold AFTER the BLE-metal window (BLE keeps priority in the connected RAK window); P2 v2-conformance is queued behind it.** Then byte-exact converge phone+XIAO+dfr1195 + the real P2 bench.
**✅ R2-USB v0.8 CANON LANDED (specs, merged main 1931cc7) — build the producer TO it (spec-first, spec leads):** (1) §3.7.1 observation (msg_type=12) canonical; TV27 pinned from my dev LoRa beacon. (2) CAPS kind unified to R2-TRANSPORT §2.2 (BLE=0/LoRa=2/WiFi=1); Appendix-A's conflicting LoRa=1/BLE=2 retired; non-R2 transports (zigbee/thread) use the tstr-kind form; TV21/TV22 re-issued. **⚠ my XIAO producer has NO v2 CAPS emitter yet (legacy framing) — so it's BUILD-to-§2.2-from-scratch, not a migrate/re-emit;** I build every bearer emission §2.2-correct. (✅ CONFIRMED r2_route::Transport IS §2.2 byte-exact — transport.rs:43: Ble=0/Wifi=1/Lora=2/Internet=3/Usb=4/WifiMesh=5/Udp=6; so (Transport as u8) emits §2.2 IDs natively, zero mapping.) OWED in the fold: real PROD LoRa 16B capture w/ MEASURED rssi/snr (deterministic bytes b201007fce111165325a9abafe8ac114 given) + net-new BLE producer + real 24B BLE-core capture. All AFTER the BLE window. **BEACON SIZE SET (supervisor-codex, cross-spec fix): valid LoRa = {15B prod-core no-tx_power, 16B prod+tx_power, 17B dev} per R2-BEACON §8.1.1; producer forwards magic-first VERBATIM (size-agnostic, len self-distinguishes); build consumer/vector acceptance = {15,16,17}. Bench emits 16B prod/17B dev (uses Some(20) tx_power).**

## 🛰 RAK SEALED-OTA (task #44 P3, Roy chose B=MCUboot) — write-half BUILT; anti-rollback seam HARDENED, §2/§3 HELD-not-RESOLVED
Cross-repo pointer (full state: rak4630-fw platforms/rak4630/RAK-MCUBOOT-OTA-PLAN.md + RESUME). **✅ Fork-independent OTA write-half BUILT** (rak4630-fw a6a3526, `ota` feature; FlashSink impls r2_update ImageSink for nRF flash: begin/write/finalize_write-0xFF-pad/abort; activate()=fail-closed ActivateNotWired stub). r2-trust + r2-update re-vendored @core a83b167 (e6ca444, diligence-clean superset). **★ Anti-rollback = COMPOSED two-layer (r2_update signed seq AUTHORITATIVE + MCUboot boot-confirm backstop); HARDENED after a 2026-07-11 adversarial review (hive-codex + core-codex twin, both correct):** (1) STAGE-TIME EQUALITY GATE — core adding an additive `staged_rollback_value()` ImageSink hook my sink overrides (parse inactive-slot MCUboot metadata → core enforces == AppliedUpdate.seq in finish() before activate, fail-closed RollbackValueMismatch); NOT a build-trust promise; (2) SINGLE DERIVED FLOOR — current_seq_floor() DERIVES from the CONFIRMED slot (no non-atomic r2-mirror-write; confirm strictly before advance; boot-time max-reconcile if mirrored); (3) pending_seq() SWAP-STATE-AWARE; + crash-point KATs. **★ CONFIG GAP (hive-codex, correct): MCUBOOT_DOWNGRADE_PREVENTION needs OVERWRITE_ONLY, NOT swap — so with swap/revert the signed r2 seq is the SOLE floor (HW_ROLLBACK_PROT is a distinct security-counter path).** §2 flash map NOT ground-truthed (swap-move-vs-offset/scratch/trailer depend on config). **NEXT GATE: pin exact MCUboot version + swap algorithm + anti-rollback config BEFORE Roy approves the map or activate() is wired.** Core landing the hardened hook (I override when it lands; interim-gate inside activate()). Core raising R2-UPDATE §9.2 (bootloader value derived-from-seq) with specs.
**★ SEALED-RECOVERY LEG design input CAPTURED (RAK-MCUBOOT-OTA-PLAN.md §4.1-4.4, commits f7a2c75 + 37cbcc2; supervisor-relayed Roy's RAK DFU doc + WisBlock README 2026-07-11; NO action on the live spike):** stock RAK4631 = Adafruit UF2 (double-tap) + Nordic serial DFU (AT+BOOT→nrfutil usb-serial signed zip). **✅ VALIDATES the live spike's link offset: WisBlock confirms stock = bootloader V0.4.3 + SoftDevice S140 6.1.1 (v6, NOT v7) → app region @0x26000 — exactly where the staged blespike UF2 links** (INFO_UF2.TXT still self-checks per-board at flash time regardless). **3-TIER RECOVERY LADDER (design the slot map against all 3): (1) app-DFU (double-tap / AT+BOOT / UART2_TX→GND); (2) HW-forced DFU (`UART2_TX`→GND + RESET, app-independent); (3) bricked-BOOTLOADER restore over J-Link/SWD (.hex reflash).** Sealed design MUST preserve ≥ tier-2 (UART2_TX→GND) + tier-3 (SWD) = the computer/bootloader-independent un-brick floors. **OPEN DESIGN Q (tier-1 transport only): KEEP stock bootloader as recovery vs REPLACE with MCUboot serial-recovery (mcumgr/SMP).** adafruit-nrfutil form: `dfu serial --package X.zip -p /dev/ttyACM1 -b 115200 --singlebank --touch 1200`. board-verify always (RAK doc says ttyACM0 but Alfred RAK=ttyACM1). ⚠ NOTE: supervisor called this "task 60" but local #60 = hive-bin trail gap; the RAK recovery leg is under **#44** (MCUboot OTA) — filed there.

## 🔧 ESPUP / ESP-IDF TOOLCHAIN STAND-UP (supervisor-directed) — IN PROGRESS
The `platforms/esp32` build (core's OTA-critical target) is **ESP32-C6 = RISC-V** (`riscv32imac-esp-espidf`, `MCU=esp32c6`), NOT Xtensa — per its own `rust-toolchain.toml` it uses **stock nightly + build-std + rust-src** (the espup "esp" Xtensa toolchain is only for S3/orig-ESP32; do NOT source export-esp.sh's PATH for it, though LIBCLANG_PATH from there feeds bindgen). Pins **ESP_IDF_VERSION=v5.3.3**, global tools (`~/.espressif`). Found already present: espup, ldproxy, nightly+rust-src, `~/.espressif/{esp-idf,tools/cmake,tools/ninja,python_env/idf5.3_py3.14_env}`. **First build LAUNCHED (bg, bootstraps IDF v5.3.3 C framework — long)**; log: scratchpad/esp32c6-idf-build.log. GREEN build of the CURRENT esp32 firmware = "espup up + ready" proof (BEFORE core's diff lands). ⚠ NOT-YET-VERIFIED until that build finishes green; do not report ready until then.

## 📥 INCOMING (core-authored, I build+flash-verify): esp32 OTA hardening patch — 2 CONFIRMED CRITICALs, ONE diff (core-codex)
esp32/IDF build = HIVE-owned (ESP32-C6, see toolchain note above); core writes it blind, hands me ONE diff (the r2-sx1262 split). Full picture: OTA-HARDENING-PLAN.md on the core branch. Also FIXED in that plan (supervisor): #5 cert-version+TG gates. **AWAITING core's single patch; then I build + flash-verify on real esp32-C6.**

## 🤝 tn_base recipe modeling — SPECS RULED (§3.3.1 v0.30 @8c8310d); SHA-fidelity = remaining open item for core
Specs ratified the TWO-AXIS model (matches what I told composer): **tn_base@ver** = shared core-TN-base@1.0.0 (board-INDEPENDENT ABI_VERSION/CORE_ABI_VERSION = abi_hash/min_core_abi) + **board_profile@ver** = platforms/rak4630 (pinmap/flash/OTA); "pin the platform" pins the board_profile axis, NOT tn_base; the flashed r2-rak4630 binary = BUILD OUTPUT (compose of both + ensembles + switches), provenance ref not a pin. Confirmed to specs: my model matches, tn_base does NOT need to name the binary. **REMAINING OPEN (build-FIDELITY, separate from the settled modeling):** the RAK realizes core-TN-base by composing ~9 individually-VENDORED core crates (path-deps) at MIXED shas (r2-trust/r2-update @a83b167, r2-ble @d80ebf7, others older) — so `tn_base=core-TN-base@1.0.0` is only TRUTHFUL if those SHAs satisfy 1.0.0's coupling-KAT; piecemeal re-vendor tends to LAG. **Awaiting: CORE certifies the RAK's SHA set == 1.0.0 coupling-KAT (or names the version it does realize).** ✅ composer BUILT recipe/v2 with the two axes (tn_base=core-TN-base@<certified> + board_profile=nrf52840-rak4630@0.1.0; owner-resolver wired; did NOT hardcode 1.0.0 — my caveat captured). ✅ HANDED core the honest SHA manifest: CERTAIN = r2-trust/r2-update @a83b167 + r2-ble @d80ebf7; UNCERTAIN (no reliable cite) = r2-wire/r2-transport/r2-route/r2-discovery/r2-heartbeat/r2-dataplane/r2-sx1262 — offered to CONTENT-DIFF those from my side (I hold both the vendored crates AND the local r2-core checkout; core's sandbox can't) against whatever base shas core names. **✅ CERTIFIED + CLOSED (core content-diffed the RAK crates itself, specs certified):** all 9 pins content-matched to OLD upstream shas, **ZERO RAK-local edits** (nothing to upstream): r2-wire@ffc7dc7, r2-transport@1673691 (pre-8508309), r2-route@b483ef3, r2-discovery@9fc56aa (now bc3c633 post my re-vendor), r2-heartbeat@bdda130, r2-dataplane@aff9928, r2-sx1262@1275732 (pre-8508309), r2-trust/r2-update@a83b167, r2-ble@d80ebf7. **tn_base=core-TN-base@1.0.0 CERTIFIED TRUTHFUL** (ABI_VERSION=1/MAJOR, CORE_ABI_VERSION=0/MINOR, abi_hash c37f504d — stable since M1; no sha crosses a version boundary). Caveat: the 1.0.0 coupling-KAT postdates my SHAs (never RUN on my set) → specs ruling: pin @1.0.0 ALONGSIDE a coherent core git SHA (HEAD **1b630d4** realizes 1.0.0 KAT-green); on next re-vendor pull all TN-base crates from ONE sha + run the KAT. composer flipped recipe/v2 to core-TN-base@1.0.0 (6359a22). **Confirmed to core: RAK firmware does NOT dep r2-engine (KAT lives there); 8508309 duty-cycle = pull-the-code-OK (harmless, repeater stays continuous-RX, duty-cycle is a field-power option not a default).** **ota_ensemble (MCUboot) = the SOLE remaining recipe pin** (owed when my MCUboot version+geometry lands). [[tn-base-is-mixed-sha-assembly]].
**⚠ CERT-FRAMING CORRECTION (supervisor-codex, core accepted): the provenance manifest is a PRE-CERT INVENTORY, not a sealed cert** — it proves zero-local-forks + clean-re-vendorability + ABI-compat + MAJOR.MINOR=1.0 resolvability, but NOT one exact tn_base@ver REALIZATION (a PATCH commit can alter runtime BEHAVIOUR while preserving the ABI+band). So the **uniform-SHA re-vendor is the REQUIRED GATE to close the SEALED-IMAGE cert (not optional hygiene): re-vendor ALL vendored crates to ONE certified core SHA (HEAD or a Roy cert-tag) → REBUILD → pin the resulting image DIGEST.** MY RAK-firmware build action (+ Roy supplies the cert-SHA). The 8508309 unify I agreed = THIS re-vendor (to one SHA across all crates). Do the SEAL build from a single certified SHA, NOT the current mixed 06-23..07-05 pin set (now even more mixed: r2-discovery @bc3c633 for build_class, others older) — so the cert is reproducible-exact. **NO ACTION NOW** (gated behind BLE + MCUboot config + Roy cert-SHA; espup lead time). Folds into the sealed-field-image build.

## 📻 RADAR (core, no action — pending specs): R2-UPDATE v0.54 §2.4 abi_hash-nonzero-for-CODE
A CODE-bearing OTA payload MUST carry nonzero abi_hash. RAK monolithic firmware OTA = firmware_full (0x01) with abi_hash=0 + host_abi_hash=[0;8]. Rule AMBIGUOUS: reading X (strict) → starts rejecting; reading Y (exempts full-base-replacement, which a monolithic MCU image arguably IS) → gate only plugin_module 0x07 (doesn't exist yet) → **zero RAK impact**. Core LEANS Y, flagged specs, HOLDING the core gate — nothing changes for RAK OTA now. IF specs rules X: set RAK firmware abi_hash to a real host-ABI value + host_abi_hash to match. Core confirms on ruling.
- **CRITICAL #1 — anti-rollback FAIL-OPEN (ota_tcp.rs):** load_anti_rollback() returns (0,0) on an NVS OPEN/READ ERROR (not just absent) → a corrupt/unavailable NVS floors anti-rollback to 0 → an OLD signed seq is ADMITTED (downgrade); and commit/stage/clear discard every NVS write error (let _ = set_u32) → a failed floor advance is SILENT. FIX (agreed, fail-CLOSED): NVS ops return Result + refuse-to-START an OTA if the floor is unreadable + durably stage {seq,payload_hash} BEFORE esp_ota set_boot + reconcile pending after mark-valid (same discipline as the RAK FlashSink).
- **✅ CRITICAL #1 (dup early ota_health_check) LANDED @core 42ff54a** — removed the early confirmer that marked the OTA partition VALID on weak checks (FNV/CBOR + random-key RBID) BEFORE WiFi/identity/trust (cancelled auto-rollback pre-real-health + left non-PENDING so confirm_or_rollback cleared pending_seq without advancing current_seq = stale replayable floor). Now confirm_or_rollback_on_boot() (after real §5 health) is the SOLE confirm owner + only esp_ota_mark_app_valid callsite. Blind-authored (core verified by reasoning); **I compile + metal-verify on espup.**
- **✅ BOTH CRITICALS LANDED — combined platforms/esp32 state BUILDABLE (core, 2026-07-11):** #1 @42ff54a + #2 @56945dd. ★ #2 ALSO fixed a compile-unblock: the esp32 signed-OTA DeviceContext was MISSING host_abi_hash + core_abi_version (drifted since M2 v0.50) → the OTA path (ota_tcp handle_start_signed) didn't compile vs workspace r2-update; #2 adds them ([0;8]/0, esp32 runs no dynamic modules). #2 = 6 NVS fns→Result (ABSENT=Ok(0) vs FAULT=Err); load_anti_rollback Err→REFUSE OTA (never gate vs a faulted floor); stage_pending_seq DURABLY before esp_ota set_boot; confirm fail-SAFE toward not-bricking. Blind-authored, verified-by-review. **MY BUILD+FLASH-VERIFY (r2-sx1262 split) — after the BLE window.** TWO metal items MINE: (1) power-cut crash-point tests (NVS = hardware, core can't unit-test blind; invariants in-code); (2) partitions.csv NVS_OTA_NAMESPACE + OTA/otadata partitions must exist — ping core if the partition CONFIG needs a core-side touch. Core iterates blind against my compiler output — ping on any build error.
- **⚠ STOP-SEAL VERDICT (supervisor-codex 2026-07-11): this pair is NOT sealing-complete — residuals sent to core:** (1) confirm-path marks app VALID on unreadable-pending/floor-commit-failure then clears pending = stale anti-rollback floor; (2) boot_health_ok is CONSTANT-TRUE + confirmation runs BEFORE real §5.2 radio+TG-sign+wire checks (and before BLE init); (3) authority_epoch commit is AFTER set_boot + nonfatal = violates atomic pre-activation eviction. Metal power-cut plan must cover a durable CANDIDATE identity/hash JOURNAL + confirm/floor reconciliation, NOT seq-only. **So my IDF build = a COMPILE DIAGNOSTIC only (surface compile issues meanwhile); do NOT flash-CERTIFY the pair as seal-complete until core corrects the residuals.** Build-to-diagnose OK; seal-cert BLOCKED pending core's next pass.
- **⏳ #2 REOPENED (core accepts core-codex's 4 residuals, 2026-07-11) — FIX PLAN folds into the SAME esp32 pass:** (a) confirm-side floor-commit FAILURE only logged+cleared pending = permanent-valid+stale-floor (not fail-closed); (b) seq-only journal can't reconcile candidate-valid-vs-old after power-cut; (c) boot_health_ok() is literally `{true}` STUB + confirm runs BEFORE BLE/trust init = NO real §5.2 (core's "#1 = after real §5" claim was WRONG); (d) authority_epoch commits AFTER set_boot (should be BEFORE, §9.4a floor-before-activation). Core fixes (d)+authors the durable journal {seq,payload_hash,candidate-partition-identity}+boot-reconcile+block-OTA-while-unresolved; provides the core encode/decode/hash self-test in boot_health_ok. **(c) needs FIRMWARE CO-DESIGN with me (my esp32 state): what boot_health_ok can assert (operational-radio + TG-credential-sign self-test + core self-test, AFTER BLE/L2CAP+trust load) + WHERE in main() confirm belongs (after receiver+trust up). Gave core preliminary input; full co-design when core pings (d)+journal landed.** esp32 = compile-diagnostic-only until #2 survives metal.
- **⏳ CRITICAL #2 (NVS fail-open) — LANDED @56945dd (see above)** — was: coming in the SAME platforms/esp32 pass (core authoring carefully — security-critical Result refactor: Result/fail-closed load_anti_rollback + propagate commit/stage/clear write errors + stage {seq,payload_hash} BEFORE set_boot + reconcile after mark-valid). Both criticals land in the r2-core state BEFORE I build (espup lead time). **NO ACTION until #2 lands + the combined state is ready; then I build platforms/esp32 (C6) + resolve the partitions.csv staging gap + flash-verify on real esp32-C6.** ⚠ partitions.csv = TWO distinct things (core clarified): (a) the esp-idf-sys custom-partition STAGING gap = MY IDF domain (build-config, not affected by #1 which only changed confirm logic); (b) if #2's anti-rollback NVS namespace needs the NVS/otadata partition DEFINITION touched, that's a CORE-side partitions.csv coordination — ping core alongside #2. Distinguish (a) vs (b) at build time. Core pings when #2 lands + combined platforms/esp32 buildable; no urgency either side (espup/board/partitions lead time).

## ✅ DONE: anti-rollback hook + DeviceContext.now COMBINED re-vendor @core d8e09a2 (rak4630-fw a2b6a73)
Clockless A′ landed @d8e09a2, carrying BOTH breaks I held for → ONE combined re-vendor: (1) FlashSink `staged_rollback_value` override ADDED = `Ok(RollbackBinding::ExplicitlyNotApplicable)` (swap/revert, r2 seq = sole floor; core owns ==seq equality in finish()); (2) `DeviceContext.now:Option<u64>` + 3 new VerifyError codes (16 CertExpired/17 CertNotYetValid/18 ClocklessDelegatedCert) = **DEFERRED to PIECE-B** — ground-truth: the RAK does NOT construct a DeviceContext yet (no CoC receiver) + doesn't map reject bytes, so no site until PIECE-B (then now:None since RAK=TG-direct clockless=accepted). Clean forward re-vendor (a83b167 ancestor of d8e09a2, Cargo.toml identical, zero divergence). Verified `--features ota` + `persona,ota` green. **This closes the OTA re-vendor thread.**
<!-- superseded header retained below for grep-history -->
## (was) ⏳ RE-VENDOR HELD: anti-rollback `staged_rollback_value` hook LANDED @core 7d89516 (specs v0.55)
Hook LANDED — specs v0.55 RATIFIED the no-default `fn staged_rollback_value(&self) -> Result<RollbackBinding, Self::Error>` (RollbackBinding = {ExplicitlyNotApplicable | Value(u64)}; core-codex's refutation of Option-default-None prevailed). **REQUIRED trait method** → re-vendoring r2-update ≥7d89516 forces the override. **Override PREPARED** (rak4630-fw ota.rs 23a9d69, exact code recorded): swap/revert (Roy chose B) returns `Ok(RollbackBinding::ExplicitlyNotApplicable)` — conscious opt-out (r2 seq = sole floor), NOT a silent None. Core now OWNS the equality (SignedOtaApply::finish, after finalize_write, before activate) — no in-activate() interim gate. **⏳ RE-VENDOR HELD to COMBINE with the imminent DeviceContext `now: Option<u64>` break** (core landing clockless cert-expiry A′ next; RAK = TG-direct clockless → pass `now: None`, stays accepted). ONE combined re-vendor + both edits when core pings the ctx field/sha. **Safe to hold** — activate() is a fail-closed stub, rollback gate not live yet (gated on MCUboot config + Roy slot map). DO-NOT-ASSUME: told core I'm holding; if core wants @7d89516 re-vendored NOW it'll say so.

## 🧩 WASM HOST (r2-wasm-host, M3 / task #64) — ABI mechanism PROVEN, but NOT security-complete (c709 review 2026-07-11)
hive-codex + supervisor-codex adversarial review (both correct) — do NOT record this workstream
security-complete: the M3 mechanism (byte-exact ABI conformance, [[wasm-host-linkable-base]]) is
proven, but the HOST is **signature-gate UNIMPLEMENTED + UNMETERED** (open gates). Confirmed gaps: (1) src/lib.rs L22-25 signed-
envelope gate is UNIMPLEMENTED — the __r2_abi_hash load-gate calls UNVERIFIED guest scratch funcs; must parse
SIGNED custom-section metadata PRE-instantiate, structurally reject start/unapproved-imports/excessive-
memory, THEN instantiate. (2) src/lib.rs L179-201 = Engine::default + Store<()> with NO fuel/epoch/
ResourceLimiter → no CPU/mem/event/log DoS bound; need HostContext(Clock/Rng/Log deterministic) +
per-call fuel/deadline + memory/table/instance limits + output/emit/log quotas + trap containment. (3)
capability authz absent: plugin.toml capabilities.requires is a DEPENDENCY DECLARATION (R2-PLUGIN §12.3),
NOT authorization — broker must gate declared ∩ signed-envelope-authorized ∩ device-available ∩ policy.
(4) ptr+len = a distinct **Wasm ABI projection**, NOT repr(C) (M1 @73af517's native L3 vtable does NOT
unblock the L2 Wasm ABI — keep the spike on named exports + checked i32/offsets; don't bind prod host to
PluginVTableV1; await specs split). (5) the final TG/authority gate + Plugin trampoline are ABI-bound →
keep provisional WAT test-private + the prod Rust wrapper GATED; expose only an abstract EnvelopeVerifier
→ VerifiedModule until the UpdateHeader v3 envelope lands. (6) browser nested-guest needs a real JS
WebAssembly-API Worker spike (existing r2-hive-wasm glue is NOT proof). **Signed provenance does NOT
neutralise buggy/compromised modules — the mechanics spike MUST falsify loop/trap/import-at-hash +
allocation-bomb modules before any untrusted use.** Do NOT auto-start hardening; Roy-gated next phase.

## 🔥 XIAO EDGE-BRIDGE POWER-STANDBY (heat fix, task #65) — ✅ FLASHABLE + STAGED on alfred, Roy-OPERATOR-gated (radio RX-duty-cycle only; MCU light-sleep deferred)
Roy: the XIAO edge bridge runs hot. **PLAN: docs/XIAO-BRIDGE-STANDBY-PLAN.md (updated 803ad66).** Root cause GROUND-TRUTHED vs LIVE dfr1195-fw 8022c2e: SX1262 CONTINUOUS RX (SetRx RX_CONTINUOUS, r2-sx1262:800) = always-hot; STEP-4 duty-cycle NOT landed (advertised-not-enforced main.rs:388/1288/2816).
- **PATH 1 (fastest heat win, no phone-coupling):** (1a) add SetRxDutyCycle 0x94 to r2-sx1262 [⚠ CORE-OWNED crate — author + hand core]; (1b) lora_route_task RX-duty-cycle not continuous listen(), OFF-BY-DEFAULT `standby` feature; (1c) MCU light-sleep between DIO1 wakes (esp-hal light_sleep, DIO1+USB-resume wake). **PATH 2 (phone-coupled, separable):** phone-gone (USB-suspend/BLE-disc/app-closed)→Intermittent+deeper sleep; wake on USB-resume/beacon. **SCOPE: XIAO pure EDGE bridge (D4→phone) only; mid-mesh-transit OUT.**
- **✅ SPEC CONTRACT RATIFIED — R2-RUNTIME v0.25 §3.2.6 @4072063 (specs, merged main, gates green), NO NEW WIRE, landed VERBATIM.** Mirror: docs/R2-RUNTIME-EDGE-BRIDGE-STANDBY-CANDIDATE.md (updated 803ad66; §3.2.6 is the normative source). Carve-out: pure EDGE bridge (sole downstream = a presence-driven sink) MAY standby→dc=Intermittent. DISCRIMINATOR INVARIANT: standby legal IFF no dependent downstream expects it awake. TRANSIT bridge (LoRa island) MUST stay AlwaysOn; Intermittent-transit = conformance violation (discriminator = topology not hardware). Phone-presence transition = self-asserted §3.2.x power-state, re-advertise dc, off-by-default.
- **⚠ TWO DRAFT BUGS specs caught + I corrected (803ad66):** (1) **dc is CLASS-ONLY** — wake_cadence/wake_window are LOCAL config knobs, NOT wire fields (my draft wrongly said "advertised on the dc field"; specs held the ratified §3B.1 ruling R2-ROUTE.md:693 "dc class-only, §12.6 needs no dc period field"). (2) **SIZING invariant REFINED:** NOT literal wake_cadence < 120s, but **wake_cadence < the UPSTREAM node's scf_ttl_s** (§3.2.2 policy, F2-default 120s) — scf_ttl_s is an independent upstream-tunable knob; literal 120s would silently drop if operator set upstream scf_ttl_s=60s. SCF reuses R2-ROUTE §3B.1 UNCHANGED (D4 is XIAO direct neighbour → hop-by-hop custody covers destined-through frames; no new §3B.x rule).
- **GATES ALL CLEARED: (1) spec ratify ✅ (§3.2.6); (2) Roy scope ✅; (3) core accept ✅ — Diff 1/2 @1bbb32b + Diff 3 @8508309 (verified in-tree: rx_duty/arm_rx/set_rx_standby/set_rx_continuous + both re-arm sites routed + dispatch KAT). ✅ PATH-1 FLASHABLE.**
- **✅ RE-VENDOR + BUILD DONE (dfr1195-fw bd67669 + 4af9b97):** SURGICAL cherry-pick of core@8508309's 3 duty-cycle diffs into the vendored r2-sx1262 + r2-transport (NOT wholesale — the vendored crates are a pinned base 172 lines diverged from core; wholesale would drag ~130 lines unrelated churn + risk clobbering local divergence). Byte-identical to core's landed code. **Both `cargo +esp check` green: `--features xiaobridge` (standby off, default image) AND `--features xiaobridge,standby` (on), exit 0 no new warnings.** core's 1<<5→1<<2 PreambleDetected prose fix folded (609c11e); PreambleDetected NOT armed (RxDone-wake sufficient, agreed).
- **✅ STAGED ON ALFRED (Roy GO 2026-07-11, stage-only — I did NOT flash):** built on alfred canonical box (rsynced source→~/dfr1195-fw-build, cargo build --release --features xiaobridge,standby, NOT cargo run). **Standby image: alfred:~/xiao-standby-04ce0049.elf sha256 04ce00491a2a6c2bb2997bbb6f7195cfb140c90fd3e3ece1421a141686198183 (1106124B).** Fallback (standby-OFF, same tree, clean revert): **alfred:~/xiao-fieldfallback-a6114724.elf sha256 a6114724...eb09 (1105432B).** Verify-then-record: the two DIFFER by 692B = standby arm genuinely compiled in. ⚠ XIAO NOT on alfred yet (alfred has ttyACM0=Arduino, ttyACM1=RAK-nRF52840); Roy plugs XIAO (esp32s3, MAC d8:3b:da:75:c3:3c) → new Espressif-USB-JTAG ttyACM. ★ DFR1195 is ALSO esp32s3 → MAC is the ONLY safe discriminator. VERIFY: `espflash board-info --port PORT` must show esp32s3 + MAC d8:3b:da:75:c3:3c. FLASH (Roy): `espflash flash --chip esp32s3 --port PORT ~/xiao-standby-04ce0049.elf` (no --monitor — binary pipe). REVERT: same with xiao-fieldfallback-a6114724.elf.
- **THEN (Roy-gated bench):** Roy flashes → idle-draw/heat + duty-cycle-engages check (est. SX1262 warm-sleeps ~62% of each 8ms cycle; the WIN is metal-measured, NOT yet confirmed) + D4→XIAO→phone path stays green → path-1 CLOSES → I ping core the bench result. **BENCH SIZING (core-verified, see docs/XIAO-BRIDGE-STANDBY-PLAN.md §2 "Bench sizing"):** detection rule (rxPeriod+sleepPeriod) <= TX preamble airtime; bench 3ms/5ms + 8-sym SF7 preamble (8.19ms) FIRST — rx=3ms is MARGINAL (~2.9 symbols). On high miss-rate, core's lean = (b) lengthen TX preamble to 12-16 sym BEFORE (a) bumping rxPeriod — **(b) is CORE-OWNED (KAT-locked profile, anti-mutual-deafness): I ping core → core lands preamble_len bump + extends the profile-lock KAT to assert preamble_len (unguarded today = silent-drift, core closing) → I re-vendor. NOT a firmware edit.** **★ SF12 re-size before field (CORRECTED 2026-07-11, supervisor-codex): 8-sym SF12 preamble = 262 MILLIseconds = the DETECTION budget (rx+sleep ≤ 262ms); the 24-bit sleepPeriod hardware cap = 262 SECONDS (0xFFFFFF×15.625µs), 1000× larger + NON-binding. SF12 thus ALLOWS a LONGER sleep than SF7 — don't carry 3/5ms because it WASTES SF12's ~250ms headroom, not because a cap blocks it; re-size from preamble/detection-margin+SCF/power policy** ([[setrxdutycycle-preamble-sizing]]). PATH-2 (phone-coupled + deep MCU light-sleep) queued behind.
- **✅ DRIVER DIFFS AUTHORED + HANDED CORE (r2-hive docs/SX1262-SETRXDUTYCYCLE-DIFF-PROPOSAL.md, commits 733d82d + c7fc7a8).** ⚠ ARCHITECTURAL FINDING: path-1 = **TWO** core-owned diffs, NOT one — because `LoRaTransport` OWNS the radio + RX arming (`service()` re-issues continuous `listen()` at new:61/TxDone:154/RxTimeout:166), so firmware can't duty-cycle RX alone. **Diff 1/2** = `LoRaRadio::listen_duty_cycle(rx_us,sl_us)` seam default (falls back to `listen()`, non-breaking) + Sx1262 `0x94` override (`us_to_steps` 15.625µs grid, RXEN-HIGH-once, both core correctness pts folded). **Diff 3** = `LoRaTransport` RX-arming MODE (`rx_duty` Cell None=continuous default + `arm_rx` helper + `set_rx_standby`/`set_rx_continuous`). core busy → queued.
- **✅ FIRMWARE STANDBY ARM AUTHORED + COMMITTED (dfr1195-fw 810573e), off-by-default `standby=["loraroute"]`.** RxenRadio overrides `listen_duty_cycle` (RXEN-high-once); lora_route_task calls `lora.set_rx_standby(3ms rx/5ms sleep)` sized for benchsf7 SF7 (symbol ~1.02ms, 8-sym preamble ~8.2ms, ~62% warm-sleep; SF12 needs re-size; sleepPeriod caps ~262ms). MCU light-sleep (1c) = precise documented DEFERRAL / open gate (esp-hal light_sleep+DIO1/USB-wake untestable off-metal; radio-duty-cycle alone = fastest heat win). **DEFAULT-image UNBROKEN: `cargo +esp check --features xiaobridge` (standby OFF) exit 0, no new warnings.** Standby arm compiles ONLY post-revendor.
- **FLASHABLE GATE = core accepts BOTH diffs → hive re-vendors r2-sx1262 + r2-transport into dfr1195-fw → `cargo +esp check --features xiaobridge,standby` green → build+stage on alfred (NEVER flash — Roy-only).** Then bench idle-draw measurement + keep D4→XIAO→phone delivered-path green. PATH-2 (phone-coupled `set_rx_standby`↔`set_rx_continuous` on USB-suspend/resume + deep light-sleep) queued behind path-1.


## ✅✅ RAK DEMOMEMBER BAKE — DONE + STAGED (2026-07-10). Full detail → dfr1195-fw/RESUME.md sibling → **rak4630-fw/RESUME.md** (e4e8334 source + 20792d9 resume).
The RAK is now a REAL member of demo TG `0xF305FE07` (no-TG-less canon realized on this board): DataPlane carries `Some(GroupHmac::new([0x5C;32]))` (deliver-enabled to own TG, §7.5.4 preserved via own HK) and emits the real §8.1 LoRa discovery beacon → the phone sees it (Roy: all hives beacon on every bearer they have HW for). `demomember` feature; both feature sets cargo-check clean. Identity LOCKED byte-exact **4 ways** (hive r2_discovery crate + composer 5c094bf + core KAT 6a91799 + fresh independent Python HKDF/HMAC): hive_id `0x1aa20ab7` (KS1-derived), rbid `7fce111165325a9a` @ epoch 0. Emitted frame `b201007fce111165325a9abafe8ac11402` (17B dev; rbid @ LoRa offset [3..11]; class_hash `bafe8ac1` = role-0 hive/repeater, NOT composer's `c60dd3a9` placeholder). Staging `.hex` sha `f2c9e17…3603e940` @0x26000 (serial-DFU). Sent composer the frame, core the metal-verify handoff, supervisor image-ready. **GATE: Roy/human flashes (hive never flashes).** This realizes canon §4 (no-TG-less) concretely for the RAK ahead of the scheduled type-level refactor.

## 🧩 WASM HOST — LINKABLE BASE (NEW workstream, Roy GO 2026-07-10, task #64; parallel to RAK)
Roy ruled BUILD the linkable base; hive owns the **wasm HOST** (base HOSTS wasm ensemble modules = Level 2, greenfield — NOT r2-hive-wasm, which is R2 compiled TO wasm). Ref: r2-specifications docs/proposals/R2-LINKABLE-BASE-AND-VARIANT-MANAGEMENT-2026-07-10.md (D.3 steps 4-5). **Prep design committed: docs/WASM-HOST-LINKABLE-BASE-PREP.md (c709e23).**
- **Two ABI surfaces** (both = the §12.4 Plugin trait lowered to repr(C) per proposal A.5.3): module-EXPORTS (execute/init/poll → wasm ptr+len) + host-IMPORTS (the capabilities.requires §12.3 syscall surface = "the host-import surface"). A wasm module becomes a Rust Plugin impl trampolining into its exports → plugs into existing engine dispatch.
- **Phasing (D.3 4-5, core A.5 binding):** std host (wasmtime) + browser (native WebAssembly) FIRST; then esp32s3+PSRAM wasm3; **nrf52840 = ONE bounded (<=32KB) boot-reserved wasm3 slot only** (A.5.1 — general multi-module REFUTED; binding constraint = 64KB-page CONTIGUITY on MMU-less heap, not total RAM). **Level-0 hot-path/crypto NEVER modularized** (A.5.4 — route/dedup/wire/CBOR/FNV, radio io_task, HMAC/HKDF/Ed25519). Host = orchestration-class only.
- **✅ M1 LANDED — gate LIFTED.** R2-PLUGIN v0.6 §12.4.3 Frozen ABI RATIFIED (specs 73af517, in my local checkout; read fully). Froze: AbiResponse[128]/AbiError[64]/AbiResult (repr(C,u8)); PluginVTableV1 (repr(C), abi_version FIRST, id, execute/init/poll/name extern C, inst=*mut c_void); ONE export `__r2_plugin_vtable_v1`; init/poll required (r2-forge no-op); abi_hash = SHA-256 over canonical schema, load gate order = verify sig → abi_hash EXACT-match → instantiate. 4 rulings: 8B abi_hash trunc in UpdateHeader / exact-match v1 / one-vtable-per-module / init-poll-required-noop. core implements the repr(C) types + r2-forge shim-gen + vtable KAT.
- **✅ WASM-PROJECTION CO-DESIGN — CANDIDATE DRAFTED (d2d8302), handed to specs for ratification as §12.4.3.1.** §12.4.3 froze the NATIVE repr(C) vtable; the L2 boundary needs a byte-for-byte WASM PROJECTION (host + r2-forge module must agree). Per core's spec-first ask it lands NORMATIVE (specs-ratified §12.4.3.1), not a bilateral handshake. Draft: `docs/WASM-PROJECTION-12.4.3.1-CANDIDATE.md`. **core's f866c3f adversarial review ACCEPTED IN FULL** (2 corrections):
  - **8 module exports** (CONFIRMED sound by core): memory / global __r2_abi_version / func __r2_abi_hash(out_ptr writes FULL 32B) / global __r2_plugin_id / r2_init(result_ptr) / r2_execute(cmd,dptr,dlen,result_ptr) / r2_poll(ev_out,buf,cap)->i32 / r2_name(buf,cap)->i32. inst DROPPED (one module=one plugin, Ruling 3).
  - **CORRECTION accepted — AbiResult = BYTE-EXACT native repr(C,u8) 136B IMAGE** (NOT my earlier 132B len-prefixed guess): tag@0, payload@4 (AbiError u32 forces union-align 4), field order data[128]-then-len u16 (Ok) / code u32,desc[64],desc_len (Err). ⇒ core's EXISTING native AbiResult KAT pins the wasm buffer too — ONE layout, zero separate encoding. Host supplies >=136B result_ptr.
  - **full 32B abi_hash v1 = c37f504d4c2a9d8c1f5bc214aa229b4ae8c0d88897a49cce519814d8915a817e** (core provided; first 8B = c37f504d4c2a9d8c). TWO forms (targeting≠authz, B.2.0): FULL 32B = load-gate exact-match (module exports, host embeds, Ruling 2); 8B trunc = UpdateHeader/recipe compat, header-sig-authenticated (Ruling 1). Monolithic images = all-zero.
  - load-gate order confirmed: verify sig → abi_hash 32B EXACT → instantiate+read exports.
- **✅ specs ENDORSED the §12.4.3.1 design IN FULL + ruled it UNGATED** (version bump only, no vector co-bump; core supplies the wasm-image KAT). Endorsed: native-image-136B AbiResult (the right Occam call — one canonical layout, two projections), the two abi_hash forms, inst-drop (Ruling 3). specs could NOT read my cross-repo candidate doc from its worktree → **I handed specs the FULL §12.4.3.1 text INLINE** (fleet, plain-text no-backticks). **specs is write-blocked this turn** (read-only sandbox mirror) → will LAND R2-PLUGIN §12.4.3.1 (from my inline text) + R2-UPDATE v0.47 when write access returns.
- **M2 context (specs ruling, NOT my edit — OTA-header layer, ZERO impact on my wasm host-import surface):** ⚠ specs REVERSED the earlier drop-it call — **min_core_abi is RESTORED; UpdateHeader v3 = 137 B three-field** (abi_hash[8] + ensemble_semver[4] + min_core_abi). This is the OTA-header/recipe layer only; the wasm projection (§12.4.3.1) is unaffected — do NOT touch the host on account of it. (Earlier RESUME note said "drop min_core_abi / 135B" — superseded by this reversal.)
- **✅ core CO-PINNED §12.4.3.1 — BOTH confirmed:** (a) native-image-136B acceptable + PREFERRED (r2-forge emits the byte-exact native image; its existing native AbiResult 136B KAT pins the wasm buffer too, zero drift); (b) the 8 export names/sigs match r2-forge codegen. **ONE refinement folded (r2-forge asserts on emit):** all INTER-FIELD PADDING bytes MUST be written ZERO (offsets [1..4] after tag + tail-to-136) — native repr(C,u8) leaves padding uninit, so pinning zero makes the 136B buffer fully-determined + KAT-pinnable + host reads deterministic (unused BRANCH region need not be zeroed — host reads only tag-selected fields). Plus: HOST MUST BE LITTLE-ENDIAN (native-LE image; wasm+x86/ARM all LE). Candidate updated r2-hive 4c1f4f8; sent specs both additions as the ratification addendum. **core's r2-forge NATIVE export codegen is UP (7a3a501: const vtable_for + r2_plugin_module! macro); the wasm-emit shares those thunks + the padding-zero assertion and lands the MOMENT specs ratifies §12.4.3.1** → then core pings me for the joint interop-test (a real r2-forge-emitted module vs my r2-wasm-host).
- **✅✅ §12.4.3.1 RATIFIED (specs 6ccd656, R2-PLUGIN v0.7) — candidate + core co-pin folded VERBATIM. r2-wasm-host BOUND 1:1 (r2-hive 6a13ec8), 4/4 conformance tests PASS.** Swapped provisional_abi → the real `abi` module: 8 exports, full-32B abi_hash v1 (c37f504d...817e), 136B native-image AbiResult (tag@0/payload@4, Ok data[128]@4+len@132, Err code@4+desc[64]@8+desc_len@72). WasmHost: load→instantiate applies the LOAD GATE (read __r2_abi_hash 32B, Ruling-2 EXACT-match refuse-to-instantiate) →r2_init(must be Ok)→serve execute/poll/name; module-instance IS plugin-instance (no host imports in v1). **✅✅ §12.4.3.1 v0.8 RATIFIED @853f233 (LIVE specs writer) — my func-read (99660fb) is now CANON + the load-gate coherence note is in verbatim (verify-sig→inert instantiate→__r2_abi_hash 32B exact→refuse-r2_init-unless-match — which my host already does; conformant module has no start func). GENERAL RULE pinned: every scalar export in §12.4.3.x = value-returning func, never a global. NOTE: earlier specs replies were a read-only fleet-ask FORK that overstepped (self-caught + stood down); the LIVE writer @853f233 is authority — durable spec state through ae5e818→853f233.** **⚠ v0.8 FIX (core wasm32 bug 5447034, r2-hive 99660fb): __r2_abi_version + __r2_plugin_id are VALUE-RETURNING FUNCS () -> i32, NOT globals — a Rust pub-static global exports the value's ADDRESS not the value (my old global-read only worked because hand-WAT globals hold the value; a real r2-forge Rust module exports a global-holding-address). Host switched to call_i32_func for both; conformance WAT exports them as funcs; 4/4 still pass. CONFIRMED to specs → specs ratifies §12.4.3.1 v0.8 (hive+core converged).** Tests (crates/r2-wasm-host/tests/conformance.rs, conformant WAT guest): binds+executes (id7/ver1/name spike/echo), 128B truncation, **wrong-abi_hash REFUSED (fail-closed Ruling 2)**, malformed-reject. Clean build.
- **⚠→✅ §12.4.3.2 OPENED (specs, from my §12.4.3.1 binding gap) — CANDIDATE DRAFTED (r2-hive 75cb102), co-pinning with core.** The host↔guest MEMORY-REGION convention (WHERE the host places data_ptr/result_ptr) — §12.4.3.1 fixed signatures not placement. specs' lean = module-reserved host-scratch region; open Q = one-convention-vs-tiered. **My answer (drafted): ONE unified baseline + optional escalation, NOT tiered:** REQUIRED = module exports __r2_scratch_ptr + __r2_scratch_len globals delimiting a host-owned region (host places all §12.4.3.1 buffers there; deterministic, bounded, NO allocator on MCU per A.5); OPTIONAL = __r2_alloc/__r2_free for std/browser execute-inputs exceeding the region (MCU omits). Host algo: fixed buffers always in-region; input in-region-else-alloc-else-fail-closed. Spans MCU→std→browser without tiering the host. **✅ specs APPROVED the unified-baseline design (not tiered)** + flagged a CONSISTENCY issue: scratch_ptr/len are more metadata globals hitting the same address-not-value bug. **RESOLVED UNIFORMLY (584aa05): ALL FOUR metadata exports are value-returning funcs () -> i32** — abi_version/plugin_id (host-proven 99660fb) + __r2_scratch_ptr()/__r2_scratch_len(). Added specs' pins: bounds (abi_hash 32/AbiResult 136/NAME_CAP 64/POLL_CAP 256/INPUT_MIN 512 → scratch_len floor 1000B), owner+lifetime (host-owned, transient-per-call), sandbox-safe note (wasm bounds-checks → bad ptr TRAPS not corrupts). Co-pinning funcs+region-emit with core.
- **✅ core CO-PINNED §12.4.3.2 (95b6bd7) — CONVERGED** (core independently landed the same funcs-not-globals fix; verified global=1048612=address on wasm32). Pinned NAME_CAP 64/POLL_CAP 256 normative; INPUT_MIN >=512 per-target (host always reads __r2_scratch_len()); net rule = every scalar export a value-returning func, memory the only non-func. r2-forge emits the region + funcs + alloc-on-std; MCU omits alloc.
- **📦 COMBINED v0.8 PACKAGE HANDED TO SPECS (inline, both parts):** PART A = §12.4.3.1 globals→funcs amendment (abi_version/plugin_id, DONE+host-proven); PART B = full §12.4.3.2 text. specs ratifies ONE R2-PLUGIN v0.8. **HELD on rebinding r2-wasm-host to __r2_scratch_ptr()/len() until specs ratifies v0.8 (spec-first).**
- **core FULLY CONFIRMED co-pin (wasm32 evidence):** the static-mut-SCRATCH + extern-C-fn pattern exports funcs returning real offset/len; cfg-gated alloc; transient-per-call = normative r2-forge emit guarantee. **HOST-LAYOUT DECISION (mine, core left it to me): 1000B NON-OVERLAPPING sum** (5 distinct fixed sub-buffers: abi_hash/AbiResult/name/poll/execute-input — matches my host with zero special-casing; NOT the 648B one-buffer-reuse; ~350B is nothing on nrf52840). On rebind the host places its 5 sub-buffers within [scratch_ptr(), scratch_ptr()+scratch_len()).
- **✅✅✅ §12.4.3.2 RATIFIED @aa4d826 (R2-PLUGIN v0.9) — my convention VERBATIM. M3 WASM PROJECTION IS SPEC-COMPLETE (§12.4.3.1 + v0.8 + §12.4.3.2). r2-wasm-host REBOUND (45bb098), 4/4 green.** Host now reads __r2_scratch_ptr()/__r2_scratch_len() (funcs), validates the 1000B floor, places 5 non-overlapping sub-buffers relative to region base (hash@0/result@32/name@168/poll_ev@232/poll_buf@236/input@492); execute input_cap = scratch_len-492, fail-closed on overflow (__r2_alloc escalation = follow-on). Conformance WAT exports scratch funcs (region [16384,+2048)). **✅ ERRATUM FIXED @9d3846b (R2-PLUGIN v0.10): floor 1000→1004 (poll sub-buffer 4+256=260; sizing invariant 32+136+64+(4+256)+512 >= 1004; INPUT_MIN stays 512). Host bumped MIN_LEN→1004 (fe69c70), 4/4 green, specs CONFIRMED my rebind conformant to v0.10.**
- **✅✅✅ TRUE-E2E INTEROP GREEN (f992eaf) — M3 CAPSTONE, L2 MECHANISM PROVEN END-TO-END.** Built core's real r2-forge Echo (WASM-INTEROP.md e1838ab, r2_plugin_wasm_module! → wasm32) with explicit 1004 scratch; ran it through r2-wasm-host: emit → load → inert-instantiate → read __r2_scratch_ptr()/len() → __r2_abi_hash 32B EXACT load-gate → r2_init(Ok) → r2_execute → 136B AbiResult **BYTE-EXACT** vs the KAT. 7/7 vectors green (abi_version 1 / plugin_id 0x2a / scratch_len 1004 / execute(0x01,[AA,BB]) image 0000000001aabb...03000000 / parsed Ok([01,AA,BB]) / name echo / poll None); wrong-abi_hash refuse-to-instantiate still holds (conformance 4/4). Runner = `examples/interop.rs` (cargo run --example interop -- <echo.wasm>); added PluginInstance::execute_raw + scratch_len. **SHAS:** hive f992eaf/45bb098/fe69c70; spec R2-PLUGIN 853f233(v0.8)+aa4d826(v0.9)+9d3846b(v0.10); core 34acb2d+a35009c+e1838ab. Reported supervisor+core+specs. **The two-impl loop caught+fixed 3 ratified-spec bugs (globals-address / memory-region / 4B floor), all spec-first.**
- **M3 CORE-MECHANISM COMPLETE.** REMAINING (Roy-gated NEXT PHASE, not this): browser backend (native WebAssembly, reuse r2-hive-wasm glue) + esp32s3+PSRAM wasm3 (nrf52840 = 1 bounded slot A.5). ABI-independent build-now available: B.4 TG-gated module-load gate (load-gate step 1) + the __r2_alloc escalation (std/browser large inputs).
- **[SUPERSEDED] earlier v0.8-ratify plan:** (1) rebind r2-wasm-host from provisional fixed `scratch` offsets → __r2_scratch_ptr()/len() reads (place 5 non-overlapping sub-buffers within the region); (2) core emits the real r2-forge .wasm (scratch region + funcs + alloc-on-std) + the AbiResult image KAT → run the INTEROP-TEST (real r2-forge module vs my host) = the true end-to-end. **core READY: r2-forge v0.8 funcs LANDED + pushed (34acb2d, cdylib export = memory + 7 funcs); 1000B non-overlapping layout accepted; core holds only the scratch-region emit until §12.4.3.2 ratifies, then emits the Echo .wasm + KAT.** (3) build-now: B.4 TG-gated load gate; (4) later: browser + esp32s3+PSRAM wasm3.
- **🔴 INTEROP BLOCKER + RESOLUTION (core 5447034: r2-forge L2 wasm-emit UP, verified on real wasm32).** core built a real r2-forge module and found: Rust `#[no_mangle] pub static i32` lowers to a wasm GLOBAL holding the linear-memory ADDRESS of the value, NOT the value (e.g. __r2_abi_version global = 1048612, value 1 lives at that offset). My host reads globals as VALUES (get_global i32) → would read 1048612 not 1 → interop FAILS. **CO-PIN DECISION (agreed w/ core): switch __r2_abi_version + __r2_plugin_id from GLOBALS → VALUE-RETURNING FUNCS** (`__r2_abi_version()->i32`, `__r2_plugin_id()->i32`): unambiguous cross-language (deref-address is a Rust-lowering leak a hand-WAT/non-Rust conformer wouldn't satisfy), cleanly Rust-emittable, consistent with __r2_abi_hash already a func (→ all metadata reads funcs, only `memory` non-func). Deviates from ratified 'global' → **route to specs as a §12.4.3.1 AMENDMENT (co-pinned w/ core).** FUNC exports + the 136B LE padding-zero AbiResult image are byte-locked (core confirmed on wasm32) — no change there; core supplies the AbiResult image KAT as the shared vector. **SPEC-FIRST HOLD: keep host at ratified global-read until specs amends, then flip get_global→get_typed_func (trivial) + update the conformance WAT (globals→funcs) → interop with core's real .wasm + KAT.**
- **⛔ FLEET SENDS BLOCKED THIS TURN (permission lockdown — same as specs/core hit).** OWED when approval returns: (1) fleet send core = co-pin funcs + request the real r2-forge .wasm + AbiResult KAT; (2) fleet ask specs = the globals→funcs §12.4.3.1 amendment (interop-blocking, expedite). Decision is recorded here so nothing is lost.
- **NEXT:** (1) report specs BOUND + 4/4 (specs asked: ping when bound + core's wasm-image AbiResult KAT passes end-to-end). (2) INTEROP-TEST: core pings when its r2-forge wasm-emit (7a3a501 native codegen up) is ready → run a REAL r2-forge module + core's wasm-image KAT against my host (the true end-to-end). (3) resolve the memory-region follow-on with specs/core. (4) Build-now (ABI-independent): B.4 TG-gated module-load gate (load-gate step 1, the documented hook). (5) later: browser backend + esp32s3+PSRAM wasm3.
- **✅ M3 scoping STARTED — wasmtime runtime-mechanics DE-RISK SPIKE PASSES 3/3 (commit ca2b341).** New crate `crates/r2-wasm-host` (EXCLUDED from default workspace like r2-hive-wasm; wasmtime is heavy → out of no_std/host CI). WasmHost (wasmtime Engine/Linker/Store): load(wasm|wat)→instantiate→execute, fresh Store per call (no guest-state leak). Spike (tests/mechanics_spike.rs, inline WAT guest, no external toolchain) proves BOTH surfaces end-to-end: #1 module `execute(cmd,dptr,dlen,optr,ocap)->written` export + #2 host `host_emit(ptr,len)` import + linear-memory I/O both ways; also out_cap truncation (independent paths) + malformed-module reject. **⇒ D.1 std runtime choice (wasmtime) DE-RISKED.** provisional_abi module is a clearly-marked provisional stub (open gate), swapped 1:1 for the ratified §12.4 lowering at M1.
- **✅ ABI_HASH SPLIT PRE-STAGED (r2-wasm-host 031f32a, R2-PLUGIN §12.4.3.3 v0.14 = core's M1 abi_hash split):** core commit-1 @ebe7bf7 (additive, still emits legacy ABI_HASH_FULL) does NOT break me; commit-2 flips wasm modules' exported hash to **ABI_HASH_WASM = 5b6c9317beca30c553a0bb2ff0fd69c5ba704097957d0a75cf8a6b20161c9717** (universal wasm32; ≠ per-ISA ABI_HASH_NATIVE — the D2 fix so the §7 load-gate tells wasm vs native apart). r2-wasm-host is SELF-CONTAINED (own embedded host hash + own .wat test modules, NO Rust-symbol import from core — core grep-confirmed + I verified), so I **PRE-STAGED** rather than same-day-sync: renamed `ABI_HASH_V1`→`ABI_HASH_WASM` (src/lib.rs) + updated the load-gate compare + the 2 conformance .wat modules (conformant bakes ABI_HASH_WASM; wrong = same value first-byte-flipped 5b→00, preserves the refuse-test). **LOCAL GATE (CI down): `cargo test --test conformance` = 4/4 GREEN.** Told core it can push commit-2 anytime — my crate already expects ABI_HASH_WASM + stays internally green (no deployed modules → zero interop risk). Native \x7fR2N container header (commit-2, native-only) does NOT touch wasm-host. **⚠ M1 NOT PRODUCTION-COMPLETE (supervisor-codex 2026-07-12, core+specs own the fixes — do NOT treat the abi_hash/load-gate as done):** (1) WASM preflight (static-section / no-start / import-allowlist / fuel-metering) STILL UNIMPLEMENTED in r2-wasm-host; (2) single `host_abi_hash` can't represent wasm+native before a payload-format sniff; (3) native `target_arch` registry lacks ABI distinctions (xtensa call0/windowed, x86_64 Windows/SysV); (4) no production native-container consumer. The abi_hash exact-match gate is REAL but is the load-gate FLOOR, not the finished preflight. Aligns with the already-recorded "signature-gate UNIMPLEMENTED + UNMETERED" open gates. Core+specs drive; I sync when they land.
**★ REQUIRED HARDENING FLOOR for the wasm-host loader (supervisor-codex 2026-07-12; the DESIGN TARGET when M1/M3 unblocks — current raw loader = pre-auth executable surface, KEEP HELD):** (1) a **`VerifiedModuleArtifact` newtype produced ONLY by auth + static validation over immutable binary-Wasm bytes** — the type system makes "unverified bytes reaching instantiate" unrepresentable; (2) **NO production raw/WAT load** (WAT stays test-only — my conformance .wat modules are tests, not a prod path); (3) **NO `Module::new` / `instantiate` / `start` before section/import/export/feature/limits checks** (preflight BEFORE compile+instantiate, not after); (4) **cache/load bound to the verified bytes** (can't swap bytes post-verify). (5) **Native PIC needs real isolation OR an explicit trusted-native model — NOT manifest confinement** (manifest ≠ a sandbox). This supersedes the current lib.rs flow (which instantiates then reads __r2_abi_hash — i.e. runs guest code before the gate). Build to this floor when the preflight work unblocks; core+specs+supervisor-codex own the sequencing.
**★ SCOPE = API-REPLACEMENT, NOT ADDITIVE + CO-OWNED (core heads-up 2026-07-12, HELD pending specs' C1 native-L3 ruling):** core-codex's adversarial pass independently confirmed my ground-truth (public raw wasm_or_wat load lib.rs:23-25/199; Module::new compiles BEFORE any gate :191-193; Instance::new runs start :203-206; abi_hash read POST-instantiate from the guest :231-242; no imports/fuel/limits/custom-section gate). **v2 (F4+F3+F5+errata#7 unified): a non-forgeable `VerifiedModuleArtifact` produced ONLY by full sig → payload-hash → static-metadata (`r2.meta` custom section, read PRE-instantiate) → policy/preflight; host compile+instantiate accept ONLY that newtype — NO public raw-`&[u8]` in prod; WAT/text REJECTED in prod (WAT→wasm makes compiled bytes ≠ payload-hashed bytes); static checks (byte-size/features/imports/start-section/metadata) BEFORE Module::new; compile+instantiate on the SAME immutable bytes; instantiate = quarantine + trap-safe init + atomic swap.** **CO-OWNED: core defines the newtype + pipeline contract; HIVE replaces the r2-wasm-host load surface (an API replacement, my current load API goes away — plan for it, not a helper add).** core hands me the RATIFIED v2 contract once specs rules C1 + core-codex re-refutes; nothing to do until then.
**✅ THREAD CLOSED: core commit-2 LANDED (r2-core d9b3b3f) — `wasm_abi_hash` now emits EXACTLY my pre-staged 5b6c9317… value, ZERO drift; a real core-macro wasm module's `__r2_abi_hash` load-gates against my host by construction. Value LOCKED (only changes on a SCHEMA_WASM algorithm_version bump; core pings if so). r2-engine 44/44 green core-side.**
- **specs assignment (M3):** report milestones to SPECS (specs rolls up to supervisor). M1 (§12.4 repr(C) ABI) = core-drafting/specs-ratifies; specs will ping the ratified ABI. Core open offer: SDC-inclusive co-resident RAM measurement needs MY Nordic-blob link (non-blocking, only if Roy asks in review).
- **NEXT (M3 tail, gated/ordered):** (a) HELD-for-M1 → bind surfaces #1/#2 to the ratified §12.4 repr(C); (b) build-now → the B.4 TG-gated module-load gate (verify module sig under TG update root BEFORE instantiate; ABI-independent) + wrap LoadedModule as an r2-engine::Plugin; (c) later → browser backend (native WebAssembly, reuse r2-hive-wasm glue), then esp32s3+PSRAM wasm3.
- Reported supervisor (c709e23) + specs (ca2b341 milestone). Coordinating under specs; separate from RAK.

## 🏛️ FOUNDATIONAL — Roy architectural canon: DOC AUTHORED (c4e3fe7) + §4 DRIFT-FIXED (5d96e6d, specs-codex); structural invariant SCHEDULED (2026-07-10)
**✅ §4 CORRECTION (5d96e6d, specs-codex review of c4e3fe7):** the birth derivation NO LONGER chains HK through tg_id — corrected to the two SEPARATE paths off the TG keypair (verified vs r2-trust): `derive_group_keys(TG_SK)`→DEK+HK (R2-TRUST §3.1, hkdf.rs:55); `TG_PK`→tg_id (R2-WIRE §6.2.1); `device_master_secret+tg_id`→hive_id (derive_hive_id) + mesh_sk/mesh_pk (derive_mesh_key); RBID session_key=HKDF-Expand(HK, r2-beacon-rbid-v1‖hive_id_be32). Added self-issued key-holder cert at singleton birth (cert.rs::issue, lifecycle.rs:95; membership iff valid cert). Repo-qualified build_lora_beacon refs to dfr1195-fw/rak4630-fw (NOT r2-hive tree); noted the RAK demomember bake e4e8334 = first on-metal no-group-None realization. **✅ §4 CLOSED (dcfa1ff): specs-codex line-by-line PASS (DEK/HK R2-TRUST:107, tg_id R2-WIRE:552, hive_id/mesh R2-WIRE:538, RBID R2-BEACON:284, membership-iff-cert R2-TRUST:72 + self-cert R2-PROVISION:94) + final nit enforced (cert SUBJECT = mesh_pk, causal-order reordered). §4 is refactor-authority-ready.** Structural refactor stays SCHEDULED (multi-repo, core-coordinated, gated on R2-PROVISION re-persona); demomember bake = its first on-metal proof.
**✅ docs/HIVE-ARCHITECTURE-CANON.md AUTHORED + committed c4e3fe7 (Roy directive "now"):** the hive impl-view MIRROR (not fork) of the 4 rulings, every claim ground-truth-verified vs hive code, with version anchors: (1) all devices run the core TN hive role-agnostic (R2-ARCH v0.15/R2-RUNTIME v0.24); (2) composition layering core-MUST→OTA+dub-dub→dev-report-TN→role (R2-RUNTIME v0.24/R2-INDICATOR v0.5); (3) dual-bearer beacon, HB≠beacon (R2-BEACON v0.41/R2-HEARTBEAT v0.17); (4) no-TG-less singleton-TG-of-one no-group-None (R2-TRUST v0.40 §2.3/R2-PROVISION v0.30). TYPE-INVARIANT home NAMED = r2-dataplane::DataPlane.group (lib.rs:145/212, Option<GroupHmac>→non-Optional; CORE-owned, coordinate) + r2-hive-bin (hive.rs:255/router.rs:276). Pinged supervisor to verify. **Structural refactor still SCHEDULED** (deferred, multi-repo, gated on the §2.3 pin [landed] + core coordination for r2-dataplane's type).

Roy DIRECTS: make 4 architectural rulings CANON IN HIVE (not just specs) AND enforce as STRUCTURAL INVARIANTS. Priority: **NO TG-LESS DEVICE** — no group None/keyless; every device instantiates with ≥ a singleton TG-of-one, ALWAYS; TG-less must be UN-REPRESENTABLE (a TYPE/CONSTRUCTOR invariant, not a runtime check). The 4 rulings: (a) no-TG-less; (b) all-devices-run-core-TN-hive (role-agnostic substrate); (c) all-hives-dual-bearer-beacon (beacon=discovery primitive); (d) device-composition baseline (core-TN MUST + OTA + dub-dub + dev-only report-TN + role).
**WHERE it lands — DECIDED (hive-codex): `docs/HIVE-ARCHITECTURE-CANON.md`** (NOT AGENTS.md) = the hive-implementation VIEW of spec-canon, MIRRORING (not forking) specs R2-TG / R2-ARCH / the provisioning spec. **PHASING (hive-codex): create the DOC now (specs-mirroring impl-view); DEFER the TG-less-unrepresentable structural refactor until specs/core PIN singleton-TG-of-one vs §7.5.4 deliver-gate + self-hk/born-TG + enrolment re-key/merge (sent specs).** Keep the RAK SF7-only and provisioned-member bench-beacon paths SEPARATE.
**INVARIANT SCOPE (no-TG-less), by layer:** r2-hive-core (ROOT: group/membership type NON-Optional, singleton TG-of-one born default, constructor REQUIRES a TG); r2-hive-bin (remove the keyless-dev Option<TG>=None path — router.rs:276, hive.rs source_tg:Option<TG>@631, R2_DELIVER_UNKEYED_OPEN → always-a-TG, unkeyed_open→relay-only-within-TG); firmware (RAK main.rs:535 group=None, DFR persona-fallback → always Some(TG)).
**BLOCKER to pin FIRST (sent specs):** born singleton-TG-of-one vs §7.5.4 deliver-gate — today group=None=fail-closed; a singleton-of-one is a REAL TG (own hk) so gate semantics change. Need R2-TG to pin: self-hk-from-birth? deliver-to-self? enrolment=re-key/merge. 
**PHASING:** foundational-tier, SCHEDULED post-demo (multi-repo refactor), does NOT disrupt the beacon/demo. Directly enables the RAK beacon (RAK gets a TG → conformant rbid). Coordinating with specs so hive-canon == spec-canon verbatim. NEXT: doc-location confirm + specs deliver-gate pin → draft the canon doc → schedule the invariant refactor.

## ✅ COMPACT-ON-LoRa DEMO — codec-conformance GO, both ELFs staged, operator-load-ready (2026-07-10)
> Firmware detail + full trail live in **dfr1195-fw/RESUME.md** (commit 0e4b25f). Pointer here because this checkout is read by hive-codex.
Core delivered the §5.1 compact codec-conformance gate: authoritative canonical D4 frame = commit **4e0e72a (29B, real apiary values)** — `06 53 00 01 64CEDB11 F305FE07 01 0052 000000010185 +HMAC` — verify_compact byte-exact (r2-trust 76/76). (The earlier 28B 9262d2e was core's placeholder.) My D4 emit-compact validated byte-exact incl payload; payload encoding confirmed RAW seq(u32 BE)+value(i16 BE) from apiary.rs ground truth (not CBOR). Peers locked to 4e0e72a: composer 12/12, android APK 1b180a65. Both ELFs (re)built + staged on alfred, byte-exact reproduced from the recorded shas: **D4** `~/dfr-sensor-compact-c4cce18.elf` `888411581eae1c49908108e0519ed564d67ff4e27c756184d25082c5408bd454` · **XIAO** `~/xiao-bridge-compact-17a2377.elf` `d99eeebdb50240a0e68289ac2019cf6a36caed7672ac2ae20dff59c9b1402e14`. NEXT = Roy operator-loads both to metal (hive does NOT); then composer+android joint delivered-path verdict. Composer pinged (byte-checks its tool vs core vector first); supervisor reported.

## 🔦 RAK↔DFR R2 LoRa-mesh pair (2026-07-08, supervisor Roy-ruling) — DFR artifact built, topology call HELD
- **RAK e8b5cd6 (calm-LED) ALREADY runs live LoRa RX/TX + R2-ROUTE relay** — no combined rebuild needed (4d69f5a Phase-2 event-RX is an ancestor). **BUT it is a KEYLESS REPEATER** (group=None, rak main.rs:535): relays+keepalives, delivers nothing, no TG membership.
- **DFR LoRa-mesh artifact** `--features loraroute,loratcxo,dev`. **loratcxo MANDATORY** (SX1262 PLL locks only w/ TCXO 1.8V; plain loraroute = RF-dark). ⚠️ **SUPERSEDED sha c0a4e762 (SF7) — first-light neighbours=0 root cause: DFR loraroute OVERRODE as923_nz's SF12 down to SF7 while the RAK uses as923_nz VERBATIM (SF12); SF7≠SF12 = mutually deaf.** My "as923_nz verbatim so PHY can't drift" was WRONG — the loraroute path mutated cfg.spreading_factor. **FIXED (dfr1195-fw fcba238): SF7→canonical SF12; new artifact calm-lora-sf12 ELF sha256 7b0ae958…4df874f1.** Full PHY now matches RAK (916.8/BW125/SF12/CR45/sync 0x21).
- **2-board RAK↔DFR = cross-chip LoRa PHY + R2-ROUTE RELAY + mutual RX** (dev event-blip on both LEDs each RxDone) — **NOT two-way app delivery** (RAK keyless; DFR originates to absent dest f91c8911). Delivered-e2e needs supervisor's topology call: (a) RAK=relay-B between two demo-TG DFRs (loraroute default A→B→C, no RAK change, +1 DFR) or (b) rebuild RAK as demo-TG endpoint. **Confirmation + decision sent to supervisor (hop 4/50); HELD on reply.**
- BLE-onto-RAK (Roy): acknowledged, sequenced AFTER the LoRa test; r2-ble already wired under RAK `ble` feature (a0feb69); 2b-2d gated on GREEN 2a. Not started.


## 🔴 PUBLIC-LEAK HARD-GATE CLEARED (2026-07-07) — pilot-site name scrubbed from public main
- **Roy HARD-GATE:** public r2-hive default branch (main) HEAD had the bare pilot-site name live on GitHub
  (LORA-FIRSTLIGHT.json + ~10 more files) — my platform-trait scrub never reached public main. **CLEARED:**
  pushed **main fdfc4bd..c48eb1c** (commit **c48eb1c**), surgical IN-PLACE scrub of 11 files (RESUME.md,
  dfr1195-firstlight.patch, HEARTBEAT-SIMPLIFICATION.md + 8 field-results) with the reviewed platform-trait
  convention (the place-name + TG-name string → pilot-site; the two te-reo terms → site / guardian). Pure
  substitution (36/36 symmetric); JSON re-validated; public-content-hygiene gate + macron sweep pass. VERIFIED
  from ground truth: git grep origin/main (whole tree) after push = ZERO non-allowlisted hits. ALLOWLIST
  preserved: the wairoa_as923_nz + wairoa.reading code identifiers (Roy's pending identifier ruling). **DO-NOT-ASSUME / open items:**
  (1) public **main is 584 commits BEHIND platform-trait** (main ⊆ platform-trait); I did a surgical scrub NOT
  a fast-forward — advancing/curating main is Roy's/supervisor's call (flagged). This scrub DIVERGES main from
  platform-trait by 1 commit (future reconcile = merge not ff). (2) root cause: CI hygiene gate is NOT on main +
  CI billing-blocked → no push-guard. (3) Roy still to rule whether wairoa_as923_nz/wairoa.reading get renamed.

## 🟢 DFR1195 CALM-LED — R2-INDICATOR v0.2 ported (dfr1195-fw 6792f98), artifact reported for flash
- Supervisor: apply the SAME calm signature to the DFR1195 (canonical cross-board), sequenced AFTER the RAK flash.
  DONE (code): dfr1195-fw **6792f98** — `led_signature` module (heartbeat 20 BPM / strobe 0.18s via libm) + loop
  rewrite to §7 priority Identify(solid) > Updating(OTA strobe) > Healthy(calm dim ~30% heartbeat), dev event-blip;
  esp-hal LEDC on GPIO21, polarity-aware; supersedes the old OTA-breathe/no-idle-heartbeat scheme. Compiles clean
  prod+dev. ARTIFACT (built on my box — xtensa gcc is in the rustup esp toolchain, add its bin to PATH: ~/.rustup/
  toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin): ELF at platforms/dfr1195/target/xtensa-esp32s3-none-elf/release/r2-dfr1195,
  **sha256 b4ddefa1b94548d589de47cef1a24aa4aaf81a15afd8e2ef1b93f7d303553d91**, built --features dev / R2_BUILD_ID=calm-6792f98 /
  CREDS-LESS. Reported to supervisor (they copy+verify+write via the README esp32-s3 runner over ACM1). FLAGS: (1) creds-less
  (LED renders regardless; WiFi mesh needs R2_WIFI creds from Roy → rebuild); (2) RAK↔DFR inter-device transport OPEN/undecided (RAK=LoRa,
  this DFR build=WiFi/ESP-NOW; confirm a common transport/feature). **OTA #49 pin note:** dfr1195-fw HEAD is now 6792f98
  (= b807bb5 + calm-LED); the coex OTA payload was pinned coex-b807bb5 — re-confirm 6792f98-vs-b807bb5 at OTA build time.

## 🔴 D4 DELIVER-BLOCKED — I REFUTED the 0x14000-wipe (do NOT wipe; gates a Roy-only destructive op)
- supervisor-codex proposed a Roy-only erase of the 0x14000 sector to fix D4 hmac_ok=false (despite provision-applied
  + hk-byte-verified). REFUTED against dfr1195-fw code (I own #42): 0x14000 IS the SINGLE credential slot
  (write_provisioned_tg→0x14000 main.rs:4866/2345; boot→hk @349; deliver-gate group_hmac from it @1026/2088;
  live-swap via PENDING_PROVISION @1177) → no "stale override" to outrank; a WIPE DELETES the verified-correct key
  (→ unkeyed fail-closed, rejects MORE, needs full re-install). NON-destructive per the fw's own §7.5.4 note
  (main.rs:53 — hmac_ok=false 3-way ambiguous, check TAG BYTES first): tag ZERO → #39 sender origination (fix sender,
  D4 fine, matches earlier verify-true verdict); tag REAL + reject → stale in-RAM group_hmac (alt provision bypassed
  PENDING_PROVISION) → plain REBOOT re-reads 0x14000 (@349/1026). Tell: absence of 'provision installed live' print
  (@1181) = the reboot case. Sent to supervisor; 4-board GO held for Roy. **DO-NOT: wipe 0x14000.** [[hmac-false-triage]]

## 🟡 OTA #49 coex payload — HIVE-OWNED, escalated (gated on Roy #49 GO + inputs)
- Provenance RESOLVED (composer byte-proof; my task#35 note was WRONG): cb87c8aa-app.bin = NON-coex (predates coex
  3aae196 by 2 days). Board-side stays 29e250cf coex. ACTION on hive: produce a COEX version-marked app.bin (composer
  can't build fw) from dfr1195-fw HEAD 9631761 (has coex; newer than 29e250cf → distinguishable), version-mark =
  R2_BUILD_ID env at build (the OTA-landed verifier), composer signs (seq-based anti-rollback set at sign-time).
  ESCALATED to supervisor (fleet firmware/key GATE fired → do NOT auto-run). Supervisor CLEARED 2 of 3 inputs:
  (1) SOURCE = dfr1195-fw **b807bb5** (supervisor ENDORSED HEAD-with-audited-fix over the stale 9631761 pin; verified
  delta = the one scanner-fix commit, +8/-2); (2) R2_BUILD_ID = **coex-b807bb5**. STILL HELD on: (3) WiFi STA creds (Roy-only secret — points me at an Alfred wifi_config.toml
  OR drops creds into a file I read; NEVER over fleet) + Roy's #49 GO. Do NOT compile/produce until both land. esp
  toolchain IS on this box (rustup esp + xtensa). When both in: build coex app.bin → verify coex+build-id → hand
  composer path+sha → composer signs (anti-rollback SEQ). Dry-run framing-proof stays fine with cb87c8aa meanwhile.

## 🟢 LATEST (2026-07-07 pm) — #40 weak-trail acceptance LOCKED + LED reconcile #59 + trail-triage #60
- **#59 LED reconcile — ✅ COMPLETE (all loops CLOSED); only Roy's metal read remains.** specs ruled my strobe-vs-window
  nit + landed **R2-INDICATOR v0.3 (c6290b8)** adopting my scope-the-window fix (transient envelopes reduce by own period)
  → "your RAK firmware is conformant as-is", NO change. Core VERIFIED e8b5cd6, review loop CLOSED. Marked #59 done
  (residual = Roy's metal read of polarity/panic-strobe, isolated + 1-line-fixable, not hive-controllable).
- **#58 BLE 2a — FULLY UNBLOCKED:** core landed the BleHost contract (c845257, acted on all my review points) + PHY_BLE=0b100
  ingress arm (eb4f6b6) for the 2c wire. Ready for my first advertise-only 2a cut whenever I flip r2-ble/binding on — a
  focused joint-with-core on-metal pass (add nrf-sdc/mpsl/bt-hci, apply §8 partition, advertise R2-BEACON). FLASHING = Roy.
- **#59 details (superseded above; kept for the artifact sha):** rak4630-fw HEAD now **e8b5cd6**
  (was 281461f). Core adversarial pass acted on: (F1) strobe 0.18s does NOT divide the 60s window → once/min
  OTA-strobe glitch → FIXED by reducing the strobe phase by its OWN period (STROBE_PERIOD_MS=180; heartbeat keeps
  the 60s lockstep window, 3.0s divides exactly); host-verified off-cadence edges old=2→new=0; §4 result byte-identical.
  (C2) panic PSEL release HARDENED — pwm0_disable now also disconnects PSEL.OUT[0] (0x4001_C560=0xFFFFFFFF) so a
  latched PWM pin can't silence the no-probe panic strobe. Core triaged my 3 concerns: polarity SOUND, 30Hz refuted,
  panic-release plausible(hardened). All 5 variants green+warning-free. **Artifact:** out/r2-rak4630-usbserial.uf2
  (0x26000, 53008 B) **sha256 88b377595036918c78c72219963020e6a59ab0a6cc89da11ec39c2db18d40763** (SUPERSEDES a9a2239d).
  Flagged 0.18-vs-60s to specs. DO-NOT-ASSUME: PWM polarity dim-vs-bright + panic-strobe-visibility still metal-untested
  (isolated + 1-line-fixable; Roy confirms first read).
- **§4.6 SETTLED (b66f887, R2-ROUTE v0.64):** reply-retrace + recorded-successor strengthening only, overheard-TX
  reinforcement REMOVED, viability-aware selection required; R2-WIRE §8.5 route-stack append now load-bearing MUST
  (v0.35). → **#60 UNBLOCKED** (the fused-path trail behaviour is now stable to converge onto). Core acked #40 as
  "a no-op that regresses" (matches my disposition). Core's LED 281461f adversarial pass (polarity/panic-disable/30Hz)
  is QUEUED (not yet returned) → keep #59 held pending it + Roy's metal read.
- **task #40 (§4.3.4 weak-trail in wasm) — RE-VERIFIED + acceptance LOCKED (NOT a bump).** Core/supervisor
  relayed a "proceed with #40 wasm bump to 572650e/ace0d6d/7ac5e1f" ACTION. Ground truth REFUTED the bump: the
  wasm pin **41adbd1 already CONTAINS all three** (merge-base(41adbd1,7ac5e1f)=7ac5e1f → 7ac5e1f is OLDER; all
  three are ancestors of 41adbd1). Bumping "to 7ac5e1f" would REGRESS the wasm to older core. trail.rs at the pin
  is already u32 (note_forwarded/on_received) + reply_msg_id_ext present. The empty wasm paths() symptom = BUILD-LAG
  (stale pkg), exactly as core's RESUME said. Fix: added host test `forward_frame_lays_weak_origin_trail_in_paths`
  (r2-hive **525806b**) proving a plain broadcast forward lays a weak origin-ward trail (destination=origin,
  next_hop=sender) in paths() — passes at 41adbd1; full wasm suite 21/0; wasm32 clean; rebuilt pkg (gitignored,
  sha 34fc187d → composer rebuilds from hive HEAD). CAVEATS answered to supervisor: (1) double-fire — dfr1195-fw
  wires TrailReinforcer on its OWN hand-rolled RX (handle_rx_frame DCE-unused there), no double-fire today; only on
  the #32 fused migration after re-vendor (non-urgent); RAK fw = fused path, zero manual trail, clean. (2) hive-bin
  = converge on fused path (#60). **DO-NOT-ASSUME:** do NOT bump the wasm core pin backward to 7ac5e1f — 41adbd1 is newer.

- HEADs: r2-hive `5ee75f5` (clean); rak4630-fw `281461f` (LED reconcile committed, clean). Branch platform-trait / rak4630-fw.
- **task #59 (combined RAK build a+b+c) — CODE DONE, artifact ready, HELD on peer-refute + Roy metal-read.**
  rak4630-fw `281461f`: replaced the bespoke dub-dub state machine with the specs **R2-INDICATOR v0.2** state→signal
  core (reused r2-workshop `dfr1117/led.rs` mono ref byte-for-byte). (a) heartbeat+strobe §4 envelope math via **libm**
  (full-precision expf); phase = uptime % 60s window (lockstep-ready). (b) Roy calm tuning: **20 BPM + dim ~30% peak duty
  via REAL PWM** on P1.03 — LED moved GPIO Output → `SimplePwm<PWM0>` (a 1-bit LED can't be dim). (c) CDC dfu-token
  already in (23f90d0). Overlay priority §7 Updating>Healthy, dev event-blip §6 max-ed on top. **Fault floor preserved**:
  boot/panic/diag stay RAW GPIO + `pwm0_disable()` (raw PWM0 ENABLE=0 @ 0x4001_C500) first so a running PWM never masks
  the no-observer strobe. Service loop select3→select4 (+33ms LED arm, ~30Hz render). ALL 5 variants green+warning-free
  (~52KiB/480KiB). Envelope math HOST-VERIFIED (20 BPM→3.0s divides 60s evenly, lub 1.0/dub 0.70, strobe 50% of 0.18s).
  **Artifact:** `platforms/rak4630/out/r2-rak4630-usbserial.uf2` (offset 0x26000, 53024 B) **sha256
  a9a2239d8ce43d9a079f4a740b7a1ba36c8b0c218b18fdaf8ab9a6bce3d4e002**. Reported to supervisor (scps to tuxedo; hive never scps).
  **DO-NOT-ASSUME:** PWM polarity is a metal-untested claim — embassy-nrf masks the polarity bit ⇒ duty INVERTED for
  active-high (brightness=(max−duty)/max); isolated to `LedPwm::set_brightness` (one-line flip if a metal read shows it
  inverted). Fault floor intact ⇒ a bad polarity read is COSMETIC, not a risk. Asked **core** for an adversarial pass on
  polarity + panic-PWM-disable-release + the 30Hz arm (reply pending in inbox). Keep #59 open until that clears.
- **task #60 (§4.3.4 trail reinforcement) — TRIAGED against pinned source; hive-bin gap is real but caller-wiring REFUTED.**
  supervisor relayed core's bounded check ("weak/reply trail not wired in hive-wasm or hive-bin"). Ground truth: **wasm
  half REFUTED** — BOTH wasm rx paths already reinforce (route_inbound_sync takes `self.reinforcer` as a param, r2-hive-wasm
  lib.rs:464; the fused `handleRx` reinforces INTERNALLY inside core's `handle_rx_frame` per r2-dataplane lib.rs:229-237 doc
  "BY CONSTRUCTION", hooks 780/914). Wiring on_received into handleRx would DOUBLE-reinforce → do NOT. **hive-bin half is the
  real gap** — its async `router::route_frame` drives `engine.plan_forward` directly (router.rs:397) with only §3.6
  `reinforce_delivery`, no §4.3.4 TrailReinforcer. BUT core's own dataplane doc says the caller-duty on_received recipe is the
  footgun they deliberately RETIRED. Spec-aligned fix = converge route_frame onto the fused handle_rx_frame (#36 pattern), a
  real async-daemon refactor. **BLOCKED on core's intent** (asked via fleet). Do NOT hand-wire on_received. Not blocking
  composer's TG-boundary demo. task #40 (wasm) stands legitimately complete.

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
- ✅✅✅ MIGRATION METAL-CHECK PASSED (supervisor flashed d2bbe502@a3ddff9 on tuxedo-os 2026-07-07): embassy 0.7
  BOOTS + runs first-light on REAL HARDWARE (diag on new stack, node 0x52414b34 dev), Roy SAW the dub-dub. Migration
  did NOT regress boot — the bisection-before-BLE passed AND earned its keep (caught the CDC bug below). → READY for
  the BleHost co-author on the on-metal 2a spike (r2-ble wired blob-free; flip r2-ble/binding for the spike).
- ✅ CDC CONNECT-BOUNCE FIXED (rak4630-fw 23f90d0, metal-check finding): embassy-0.7 CDC spurious byte on host-connect
  was bouncing the board to DFU (my single-byte reader acted on it). Fix: (1) discard RX ~300ms after connect; (2)
  reboot trigger is now the explicit multi-byte token 'dfu' (NOT bare 'b') — stray byte = harmless dump only. Serial
  connect no longer bounces; 't' works; reboot = send 'dfu'. All variants green. TRIGGER CHANGED 'b'→'dfu'.
- 📋 COMBINED BUILD PENDING (supervisor wants ONE flash, a+b+c): (c) CDC fix DONE. (a) LED reconcile to specs'
  R2-INDICATOR v0.1 canonical envelope + (b) Roy's CALM TUNING (25 BPM slower + ~25-35% peak-duty dim glow via PWM)
  = task #59, the next focused pass (micromath + PWM + reconcile raw-GPIO paths). Hand ONE build when all 3 in+green.
- ✅ #57 fn LANDED (core 5b22368 r2-core-consolidation): r2_trust::hkdf::trust_group_uuid(tg_pk)→[u8;36]. My grep hit
  STALE vendored r2-trust. ACTION = re-pin r2-hive's r2-trust to >=5b22368, then the bounded per-TG change (KAT:
  trust_group_uuid(0x42*32)=425ed4e4...; master 0xAA*32→hive_id c19eac1d...). Focused pass, peer-refute the self-check.
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
- **✅ RE-VERIFIED vs a stale specs off-thread conformance verdict (2026-07-11):** specs flagged that an OFF-THREAD fork (citing a STALE §3.2=HMAC-SHA256 pre-v0.2 defect) had wrongly called the Ed25519 device-first handshake conformant, and warned "stop claiming Ed25519 conformant / don't build on it." **GROUND TRUTH on the live tree = NO-OP for hive: there is NO Ed25519 relay handshake to retract.** `compat/handshake.rs` is already AUTH-FREE v0.11 (subscribe() = `{version:3, trust_group, timestamp}`, no device_id/signature/challenge; versions 1/2 → close 4401). The Ed25519 device-first path was REMOVED by task #56 (Roy ruling 2026-07-07). Only HISTORICAL removal-comments + the DISTINCT R2-WEB §4.2 served-`/r2` per-message Ed25519 (a legitimately-authed trusted-serving surface, `gen_plugin_web_vectors.rs`) mention Ed25519. **4 auth-free compat tests GREEN** (incl `retired_versions_parse_but_are_not_v3`). AUTH-FREE stands (Roy final-confirm pending, non-blocking); I hold any handshake-crypto change + will NOT reintroduce Ed25519. **✅ SPECS ADOPTED the ground truth (2026-07-11): its non-conformance flag was downstream of a stale 01:15 inbox describing the PRE-removal state; redone verdict points at handshake.rs HEAD = CONFORMANT (specs told supervisor). Thread CLOSED — no hive change.** [[handshake-relay-v011-auth-free]] — the exact stale-thread trap that memory flags; live-tree-over-stale-artifact discipline confirmed by specs.
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

