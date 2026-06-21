# RESUME — r2-hive (hive-worker)

Updated 2026-06-18 (owned by hive). Master save (read-only ref):
`r2-fleet/fleet-context/FLEET-CONTEXT-SAVE.md` (moved from claude-fleet, now tooling-code-only).

**Role:** the hive runtime. North-star: **ONE hive codebase usable everywhere**, built on
**core's no_std crates** + thin per-platform layers (Linux/cloud, ESP32-S3/DFR1195, Uno-Q, wasm).
"Bring hive up to a general tool" = converge r2-hive (today Linux/std) onto that one codebase —
do NOT fork per-target firmwares. Chain: specs → core → hive. composer orchestrates hives, isn't one.

**Current branch:** `platform-trait` (local + pushed). Built atop the v0.2 work (`0aa6ab7`).

## Active (besides the branch) — priorities per Roy (2026-06-16)
- **#1 LEAD TRACK: first real-hardware TN test on the DFR1195 rig.** Critical-path doc DELIVERED +
  CORRECTED (`45a7194`, `docs/hardware-tn-test-critical-path.md`). **TWO boards now live on tuxedo-os:
  ttyACM0 (S3 rev v0.1, MAC …26:98) + ttyACM1 (S3 rev v0.2, MAC …90:10)** — enough for hive-to-hive
  (field.lab milestone). Confirm port before flashing each. Milestone = two DFR1195s exchange one
  routed R2-WIRE frame over real radio, AND the first USB image already ships a working OTA receiver +
  2-slot partition table (Roy standing req — every later update wireless). Shortest path = WiFi-UDP first
  (core wifi.rs) → board↔board (Stage B) → wireless OTA round-trip (Stage B', composer F5 ota_push ↔ my
  OtaReceiver) → LoRa (Stage C, true infra-less TN). **SoC CONFIRMED ESP32-S3** (DFRobot wiki + SKU
  SKU_DFR1195_LoRaWAN_ESP32_S3 = ESP32-S3-WROOM-1-N4 Xtensa, 4MB, SX1262). Target xtensa-esp32s3-none-elf
  (espup Xtensa fork — the HARDER path), espflash --chip esp32s3. **I briefly mis-ID'd it as C6 from
  core's skeleton (which conflated DFR1195 with DFR1117 Beetle C6) — corrected; lesson: verify SoC vs the
  primary source, not a downstream artifact.** **BLOCKERS: (1) physical — Roy provides 2× DFR1195 (S3) +
  2.4GHz WiFi + espup-toolchain perm (+ LoRa antennas/region for C); (2) core must RE-TARGET its
  platforms/dfr1195 skeleton esp32c6→esp32s3 (flagged — its structure reuses, chip layer changes).**
  workshop's firmware/esp32-s3 is now the on-point board reference (GPIO/partitions/USB-JTAG/espflash
  mechanics/OTA self-proof). composer's S3 board.toml + 4MB OTA bound = RIGHT (un-flagged my churn).
  - **D3b division of labor AGREED with core** (Roy made the radio drivers core's top priority):
    **core OWNS** r2_transport::Transport bindings (wifi/ble/lora seam), peers.rs resolution, the SX1262
    LoRaRadio impl, and authors a first-draft esp-wifi/embassy-net bringup against the S3 pins. **hive
    OWNS** esp-hal chip/clock/heap init, esp-wifi controller + STA assoc, embassy-net Stack, flash/monitor
    loop, host-loop wiring (route_inbound_sync + sync→async bridge), the **esp-storage FirmwareSink** impl
    (OTA flash A/B + set-boot for my OtaReceiver), and metal validation + defect loop (core can't
    compile/flash — author→hive-flash→defect). **Pins:** core's matrix (esp-hal 0.23/esp-hal-embassy 0.6/
    esp-wifi 0.12/embassy-net 0.6/esp-alloc) with chip feature **esp32s3** + target xtensa-esp32s3-none-elf;
    reconcile on first metal build. **Authoring order:** WiFi-UDP → OTA → SX1262 LoRa; BLE deprioritized.
    **SX1262 = wrap a mature crate (lora-phy/sx126x) behind the LoRaRadio trait** (robustness > 'fully
    ours' for the greenfield longest-pole radio).
  - **⚡ FIRST LIGHT ACHIEVED** (`599f11b`, `docs/dfr1195-first-light-findings.md` + `dfr1195-firstlight.patch`).
    esp-hal **1.x** no_std firmware BUILDS (Alfred) → FLASHES (tuxedo ttyACM0 via SSH) → BOOTS → serial:
    "r2-dfr1195: FIRST LIGHT" + alive loop, booted from **OTA ota_0** (flashed WITH the 2-slot partition
    table → OTA-laid-out from first flash, Roy's req). **Descriptor blocker SOLVED:** esp-bootloader-esp-idf
    **0.5.0** (not 0.2.0) + esp_app_desc!(). Validated bare-metal matrix: esp-hal 1.1.1 / esp-alloc 0.10.0 /
    esp-backtrace 0.17.0 / esp-println 0.15.0 / esp-bootloader-esp-idf 0.5.0. Done in a git **worktree**
    (`~/Development/R2/dfr1195-fw-wt`); patch handed to core.
  - **⚡ WiFi/embassy MATRIX RESOLVED + COMPILES** (worktree Cargo.toml; memory [esp32-wifi-embassy-matrix]).
    The blocker was NOT a version bump: esp-wifi→**esp-radio** rename (esp-wifi 0.15.x links-collides on
    xtensa-lx-rt ^0.20 vs esp-hal 1.1.x ^0.22), scheduler esp-hal-embassy→**esp-rtos** (superseded, wanted a
    private esp-hal feature). VERIFIED set (resolves + compiles xtensa, 58s, 241K ELF): esp-hal **1.1.1**
    (unchanged) / esp-rtos 0.3.0 (esp32s3,embassy,esp-radio) / esp-radio 0.18 (default-features=false,
    esp32s3,wifi) / esp-alloc 0.10 / esp-bootloader-esp-idf 0.5.0 / embassy-net **0.9.1** / embassy-sync 0.7 /
    embassy-executor 0.10 (default-features=false) / embassy-time 0.5 / xtensa-lx-rt 0.22. **DRIFT flagged to
    core:** wifi.rs targets embassy-net 0.6 → needs same-day turn to **0.9** (IpEndpoint::from + UdpSocket::new
    /Stack lifetime). **NEXT (field.lab):** migrate main.rs bare-metal→esp-rtos/embassy async + esp-radio STA +
    embassy-net Stack, re-enable mod wifi (once core's wifi.rs@0.9), spawn udp_writer_task, wire RouteEngine →
    board A originates → board B receives+relays (dedup/TTL/spray). network-OTA receiver rides the same tier.
  - **🎯🎯 FIELD.LAB DONE — first routed R2-WIRE frame board↔board on REAL HARDWARE** (`a99313b`). WiFi-up
    smoke PASSED (soft-AP r2-fieldlab 192.168.4.1 ↔ STA .2, role auto-by-MAC), then the routed frame: board A
    (hive 502698) originates an R2-WIRE *extended* Event over real WiFi radio → board B (b79010) decodes +
    `r2_route::RouteEngine::plan_forward` + **DELIVERED msg_id=7..13 ttl=4 'hello-TN'** + **DEDUP** the
    duplicate. Stack: esp-radio 0.18/esp-rtos 0.3/embassy-0.9, one combined recv/send UDP socket task (port
    21042), static IPs. **HW finding (confirms core's B1):** RELAY ≠ DELIVERY — first cut let plan_forward's
    relay verdict (Drop NoViableNeighbour on a 2-board leaf) mask delivery; separated → delivers. Boards: my
    field.lab pair = ttyACM0(AP 502698)/ttyACM1(STA b79010), by MAC via /dev/serial/by-id; workshop's 3
    DFR1195s = ACM9/10/11.
  - **🎯 THE FLEET WORKS — synced LED heartbeats over TN** (`cb8fa14`). Both boards run a leaderless
    Mirollo-Strogatz pulse-coupled oscillator: fire = LED beat + broadcast R2-WIRE `Heartbeat` frame;
    receiving the peer's fire = advance-only phase nudge. Initialized 1.1s apart → phase-lock ~60ms apart
    (proven coupling: crystal drift <1ms/26s). Serial: AP `HB phase 0.97->1.00` then `FIRE` (pulse triggers
    fire); STA convergence `0.70->0.82->0.97->lock`, `synced false->true`. Clock = embassy_time (esp-rtos
    time-driver). composer's HeartbeatSync sentant = CONDUCTOR-PLL (std tier); mine = leaderless PCO (MCU) —
    flagged the mixed-TG model-alignment Q.
  - **LCD status surface RESTORED** (`988f0ac`) — ST7735S in the async render loop (GPIO48 active-low,
    offset 26,1, Deg90, 20MHz), shows role/ip/TG/build/beats/dlv/`fleet: IN SYNC` from atomics io_task
    updates. WiFi + routed frames + PCO heartbeat + LCD all coexist, no panic.
  - **🎯 GOAL #2 — intra-TG TRUST DELIVER-GATE working on hardware** (`045048b`). Real HMAC-SHA256
    (r2-trust `GroupHmac`, which BUILDS for xtensa — 38s, no getrandom issue) gates delivery at the B1
    deliver branch ONLY; relay stays trust-agnostic. AP originates signed intra-TG Events alternating
    good/bad HMAC; STA: `DELIVERED msg_id=6 'in-TG' (tg+hmac ok)` / `DELIVER-BLOCKED msg_id=7 hmac_ok=false
    (relay unaffected)`, consistent. Canon (core 5f8798b): `target_group = FNV-1a-32(TG_UUID string)` via
    r2_fnv const; `sign_extended`/`verify_extended` (target_group+event_hash inside the MAC). Both boards
    share TG_UUID + hk (demo stand-in for the join). LCD shows dlv/blk.
  - **TONIGHT'S ARC (all on metal, 2 boards):** WiFi ✅ · routed R2-WIRE frame (deliver+dedup) ✅ · synced
    heartbeat ✅ · LCD ✅ · intra-TG trust deliver-gate ✅ · conductor-PLL heartbeat (TG-scoped + version
    telemetry) ✅. **Both headline goals — TN + trust groups — proven + canon-aligned on real hardware.**
  - **CONTINUED-SESSION metal wins (all committed):** N-board broadcast (fire/Event → subnet 192.168.4.255,
    verified) ✅ · **unique per-board STA IP** from low MAC byte (the real N-board fix; .2 would collide) ✅ ·
    **organic lub-DUB LED heartbeat** via LEDC PWM hardware duty-fades (Roy: "heartbeat not flash"; io_task
    FIRE_SIGNAL → main renders the envelope) ✅ · **OTA bootloader CONFIRMED (test a)**: my no_std app boots
    under the ESP-IDF BL (extract first 0x8000 of /tmp/dfr1195-merged.bin → espflash --bootloader; "Loaded app
    from 0x20000" + app runs) — the OTA BL blocker is closed ✅ · esp-storage builds for xtensa ✅. STA
    (ttyACM1) now runs the ESP-IDF BL. Conductor-PLL note: locks but ~0.1-period steady-state OFFSET (tighten
    with β freq term / higher gain — refinement).
  - **MORE continued-session metal wins:** **conductor-only beaconing (NO-FLOOD)** — only the conductor beacons
    the fire, followers PLL-listen silently ✅ · **2nd-order conductor-PLL (β/freq term)** — kills the ~200ms
    offset, e→±0.005–0.025 (<50ms), 5 LEDs as ONE ✅ · **5-board mesh** (my 2 + composer's 3, ESP-IDF BL) ✅ ·
    **real-TG persona reader (#20)** — read bundle raw @0x12000, r2_cbor-decode, run on PROVISIONED hk/tg/derived-
    hive; **TG=4b3df45d OFF DEMO** on both my boards (persona=true), cond=3e0d688f, synced=true, DELIVERED good /
    BLOCKED bad on the real hk ✅. Hand-rolled derive_hive_id (HKDF→v4-UUID-string→FNV; r2_trust::derive_hive_id
    not in pinned r2-trust). **KS1-CANONICAL derive_hive_id** — re-synced r2-trust to **abde165** (the no-v4-forcing
    fix; 256489b + my hand-roll BOTH v4-forced = matched each other but DIVERGED from KS1). ids now byte-exact to
    composer: **502698→480e900e, b79010→2cab5f69** (were the wrong v4-forced 3e0d688f/cce44b60). Conductor re-elects
    to lowest (STA 2cab5f69); AP follows+locks (STA→AP broadcast direction also confirmed). r2-trust pinned abde165 ✅. **OTA test (b) PASS** —
    wrote valid image to ota_1, firmware activate_next_partition() + reboot, ESP-IDF BL booted ota_1 @0x200000;
    both OTA prereqs CLOSED; converted to report-only (production-safe). Op-note: espflash flash does NOT reset
    otadata — erase 0xf000/0x2000 to recover a board to ota_0 ✅.
  - **EVEN MORE wins (this session):** **health #18** — r2.hb.health CBOR (13-key), every-5th-beat, followers
    DIRECT to the collector AP, AP logs `HEALTH <hex>` for composer's orchestrator serial-reader; verified e2e
    (AP collects own 480e900e + STA 2cab5f69) ✅ · **shared parse_persona** — adopted r2_trust::parse_persona
    (core 1b93108), dropped my decode glue; one codebase with workshop ✅ · **carrier-aware has_screen** — LCD
    init+render gated on board-profile byte @0x13000 (0x00=XIAO no-screen, else=DFR1195); ONE binary runs on
    screenless XIAO-S3 (9-board) ✅ · **perfect sync** — 2nd-order PLL now locks to e=-0.000 (zero offset) ✅.
    r2-trust pinned 1b93108. 9-board = 5 DFR1195 + 4 XIAO-S3 (all-S3, true PLL, GPIO21 LED); role-by-MAC →
    only 502698=AP, XIAO=STA; composer flashes my binary + provisions XIAO (persona@0x12000 + 0x00@0x13000).
  - **9-BOARD MESH CONFIRMED (metal) 🎉** — composer flashed all 4 XIAO + 3 DFR1195; ALL on tuxedo USB
    (my ACM0=AP/ACM1=STA, XIAO ACM2-5, DFR1195 ACM9-11). Verified synced=true + dlv climbing (trust delivering)
    across composer's DFR1195 (ACM9/10/11 dlv~1692) AND a XIAO (ACM2) = cross-arch (S3 DFR1195 + XIAO)
    beat-as-one on real TG 4b3df45d, conductor = lowest canon id 06ae082b. AP serial held by r2-compos
    (composer orchestrator) = the health #18 dashboard feed working by design; do NOT re-flash the live AP.
  - **OTA network receiver (#17)** — DE-RISK PASSED (flash-write-while-WiFi: 20ms/sector, heartbeat-safe, no
    quiesce). Receiver built (UDP 21043 START/DATA/COMMIT stream → sector-write → SHA-256 → activate+reboot) +
    otadata anchor (Factory→ota_0 so activate→ota_1 seq=2). PROVEN: 512KB stream+write+sha_ok+valid 0xE9 image+
    activate ok + test-b slot-switch. NOT yet cleanly e2e (board-to-board boot-INTO-ota_1 snagged on test-
    corrupted otadata + can't test on the live AP). Test sender gated OFF (OTA_SELFTEST=false). Next clean
    verify: a fresh-otadata board, NOT the live soft-AP. LESSON: never re-flash the live soft-AP mid-demo.
  - **LATEST (0621.1227):** **per-carrier LED polarity** — XIAO-S3 GPIO21 is ACTIVE-LOW (inverse of DFR1195);
    profile byte1 @0x13001 (0x01=active-low; erased→active-low iff no-screen, so XIAO byte0=0x00 already works);
    LEDC idle + lub-DUB envelope polarity-mapped ✅. **#23a conductor-timeout re-elect** — forget a SILENT
    conductor after 4 beats → re-elect next-lowest; healthy conductor = no churn (replaced the churny every-3
    forget) ✅. **AP-SPOF live (#23b):** the soft-AP (502698) went dark (my live re-flash wedged it) → STAs
    stranded (no network → no app-layer election can help; my STA came up alone/CONDUCTOR). FIX = revive 502698
    (Roy physical RST; port held by composer's health reader so no remote reset). **#23b AP-FAILOVER = the real
    fix, NOT YET built:** pre-designated backup (lowest AP-capable hive from the heartbeat roster) detects
    esp-radio disassociation + promotes STA→AP at runtime @192.168.4.1; others re-scan/associate. Substantial +
    risky (runtime WiFi mode switch) — implement on a test pairing, not the live mesh.
  - **CONVERGENCE BUG FOUND + FIXED (serial-verified, 0621.1227):** the 9-board "not converged" root was a
    VERSION MISMATCH — 3 DFR1195 (ACM9/10/11) were on a STALE pre-KS1 build (0621.0858) computing WRONG hive_ids
    (a0dce700/63f798ea/b658276e) → SPLIT-BRAIN conductor election (boards disagreed on the lowest id). XIAO were
    on 0621.1148 (pre-LED-polarity → dark). FIX: re-flashed all 7 accessible boards to 0621.1227 (KS1 ids + LED
    polarity + conductor-timeout). RESULT (direct serial): 8/9 lock to cond=06ae082b (=529928/ACM10), e≈0.000,
    synced=true, cross-arch (DFR1195 + XIAO). 9th = AP 502698/ACM0 still dark on old build (port held by
    composer's health reader) → revive via Roy RST (beats+follows) or composer port-release + re-flash to canon.
    LESSON: a mixed-build fleet WILL split — keep ALL nodes on one build; verify by SERIAL not telemetry.
  - **9/9 CONVERGED + UNIFIED + AP REVIVED (0621.1244, serial-verified) 🎉** — all 9 on ONE build/span;
    single conductor = ACM10 (529928→06ae082b); all 8 others (incl the AP) lock cond=6ae082b synced=true
    e≈0.000 cross-arch (5 DFR1195 + 4 XIAO). AP 502698 revived via composer port-release re-flash → canon id
    480e900e, role=AP, beats as follower. **AP later re-wedged → composer un-wedged it (espflash-reset,
    firmware intact) → all 9 back to sync_state=1; composer fixed the dashboard feed (their plugin poll bug,
    NOT my HEALTH format — parsed all 9 byte-exact). Health dashboard LIVE.**
  - **XIAO LED FIXED + ROBUST (Roy confirmed correct).** The XIAO GPIO21 LEDs are EXTERNAL active-HIGH (not
    the built-in active-low user LED). The byte-toggle (0x13001) was FRAGILE (composer's 1-byte re-provisioning
    leaves byte1 erased → the old !has_screen inference re-inverted on every re-flash). FIX (committed, 0621.1314,
    re-flashed the 4 XIAO): read_board_profile DEFAULTS active-high — led_active_low only on byte1==0x01 explicit
    override; NEVER infer from has_screen (polarity is hardware/wiring-specific, not SoC-derivable). Robust across
    re-flash + re-provisioning. **R2-WIRE v0.6**
    (msg_id-in-HMAC-span) = deferred: SEPARATE all-9-coordinated update; current bench all on the same span.
  - **#24 BLE↔WiFi TWO-PLANE — STARTED (Roy: now the focus; AP wedged again = the motivating need).**
    Architecture settled (workshop+core, r2-route pattern): pure no_std S0–S4 negotiation ENGINE in
    **r2-discovery** (core lands it from my interface) behind a **NegotiationRadio trait**; radio glue
    per-platform (hive=esp-radio, workshop=esp-idf); protocol primitives reused (r2-wire/trust/beacon);
    reuse `lowest_live_id` (conductor election). DELIVERED: the engine interface (S0–S4 table + trait
    surface) → core (docs/r2-24-negotiation-engine-interface.md). BLE foundation scouted: esp-radio
    `ble` HCI + **trouble-host** (bt-hci). NEXT (fresh focused effort, TEST PAIRING first): the esp-radio
    BLE-stack bring-up (deps+coex → HCI↔trouble-host → advertise RBID → scan → L2CAP CoC → NegotiationRadio
    impl). Subsumes #23/#23b (wedged AP → auto-renegotiate over BLE, no manual reset). §4A Profile-A conformant.
  - **NAMED REQUIREMENTS (roadmap, careful test-pairing — NOT on the live mesh):** #23b **AP-FAILOVER** (Roy:
    "TN should renegotiate the hotspot if it goes away") — pre-designated backup (lowest AP-capable hive from
    the roster) detects disassociation → promotes STA→AP (same SSID/IP) → others re-associate; conductor-timeout
    app-half DONE, WiFi-layer half TODO. **BLE-BEACON discovery** (R2-DISCOVERY) = the out-of-band substrate
    that solves the no-network-to-elect chicken-and-egg (beacon presence/hive_id/TG/AP-capability/roster over
    BLE, independent of the WiFi-AP) — #23 negotiation rides it. **IDENTIFY** cmd (LED solid on /r2 identify).
    **CARRIER-CONFIG = COMPOSER's domain (architecture ruling).** hive provides the no_std RUNTIME + a
    carrier-DETECTION primitive (probe PSRAM/flash/MAC → expose carrier-identity); composer composes the
    per-carrier firmware-config (LED/PSRAM/screen/radios) on the runtime. So the **dedicated-XIAO-build is
    DEPRIORITIZED** (it was the LED stopgap — the active-high DEFAULT already fixed the LED robustly; the
    PSRAM/8MB-partition boot-flakiness fix becomes composer's carrier-composition). My job: runtime + the
    detection primitive + #24. (esp-hal `psram` module exists; flash-size is compile-time-const not
    runtime-probable, so carrier is per-build — info for composer's composition.)
  - **IDENTIFY (Roy locate-a-board) — device-side DONE (0621.1322):** Directed r2.hb.identify frame
    (event_hash + target_hive + payload on/off) → target board LED SOLID ~5s override (polarity-aware),
    refresh-on-repeat / clear-on-off. Down-path (orchestrator→serial-bridge→mesh-inject) = the remaining
    piece: UART RX on USB-Serial-JTAG, on a NON-AP/test board first (don't risk the live AP health-feed TX).
  - **PRECISE NEXT STEPS:** (1) composer re-flashes its 3 with the persona-reader (personas survive app-flash)
    → all 5 OFF DEMO on the real TG; I verify 5-board real-TG sync. (2) **OTA network receiver (#17)** — the
    slot-switch is PROVEN (test b); remaining = UDP image transfer + write ota_1 with esp-radio QUIESCED
    (esp-storage#31) + sha256 + activate-on-commit; flash-touching = careful. (3) **health #18** — r2.hb.health
    CBOR, UNICAST to collector (NOT broadcast, per af4ebcb), every-5th-beat+on-change, ota_status from slot
    report. (4) dedup v0.4 (origin=route_stack[0]; future
    r2-route bump). (5) 4-board entanglement (cross-TG gate: GroupHmac first, then trial PeeringHmac; §7.5.4).
    (6) **LoRa rung** — core landed LoRaTransport (fb13b17, r2-transport/src/lora_transport.rs); impl LoRaRadio
    for Sx1262 (wrap lora-phy) → LoRaTransport::new → single-owner lora.service() in the radio task; send()=
    broadcast-on-air so RouteEngine+dedup+trust+conductor-PLL transfer UNCHANGED from WiFi. Swap the ref's
    RefCell<VecDeque> TX queue for an embassy/heapless channel (separate async radio task). Open before TX:
    region/duty-cycle gate, LBT/CAD, RXEN switch (SX1262-LORA-DESIGN.md). Ping core when starting.
  - **QUEUE (post-headline):**
    1. **OTA receiver (#17)** — plan ready (`docs/dfr1195-ota-receiver-plan.md`: OtaUpdater + esp-storage +
       UDP :21043 transfer + sha256 + software_reset). **2 go/no-go prereqs FLAGGED:** (a) espflash's default
       bootloader may not honor otadata for slot-switch → may need a custom OTA bootloader (BLOCKER candidate,
       coordinate core/workshop); (b) flash-write-while-WiFi can hang on dual-core S3 → quiesce radio around
       writes. Run the bootloader test (write ota_1 + flip otadata + reboot) before the full receiver.
    2. **Heartbeat → leaderless CONCAVE-M&S PRC** f(φ)=(1/b)ln(1+(e^b-1)φ) b=3 once specs pins v0.2 (NO rush;
       conductor-PLL holds; drop-in swap of the phase-update, keep the broadcast+jitter). (Canon flip-flopped
       v0.1 conductor-PLL → v0.2 leaderless-concave; supervisor's latest = leaderless-concave for no-SPOF.)
    3. **Real-TG provisioning** — consume composer's keystore (R2-PROVISION): replace hardcoded TG_UUID+hk +
       MAC-low3 hive_id with provisioned device_master_secret + TG persona → derive canonical hive_id
       (FNV(HKDF(secret,tg_id))) + group hk. Asked composer for the NVS layout/read API. Crypto path unchanged.
    4. **N-board scaling (#19)** — fire BROADCAST to all co-members (not 2-board unicast) + multi-peer table;
       converges with the leaderless-concave swap. Then 5-board mesh (my 2 + workshop's 3).
    5. **Health telemetry (#18)** — r2.hb.health CBOR companion (composer's HEALTH-TELEMETRY-CONTRACT), after
       OTA (needs ota_status). 6. **Entanglement** (2 TGs/4 boards, PeeringHmac, lexicographic pubkey order).
    Canon follow-ups: dedup origin = route_stack[0] self-stamp for multi-hop (3rd relay). Hardware → SPECS FIRST.
  - **⚡⚡ PROOF SURFACE WORKING on BOTH boards** (`876bb98`, `docs/dfr1195-proof-surface-learnings.md`).
    LCD + LED running on ttyACM0 (rev v0.1) AND ttyACM1 (rev v0.2). **LCD (ST7735S):** status line on top +
    event log scrolling up; 20MHz SPI, mipidsi 0.9, offset(26,1)/Deg90/inverted. **KEY find: GPIO48
    controller power is ACTIVE-LOW** (HIGH = backlit-but-dead; cost a debug cycle — in the board profile).
    **LED (mono GPIO21):** gentle heartbeat "lub-dub" = all-well (visible even when screen off). Pins:
    MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48(active-low); LED21; btn18/btn0. **PUSHED to composer via
    supervisor** to create TWO general device-SPANNING capabilities + StatusDisplay sentant: display plugin
    (ST7735S driver, contracted ed50505) + **LED indicator plugin (NEW** — mono/rgb/canvas per-board, pattern
    vocab all-well/ota/joining/error/identify; Roy: LED signals status when screen down). hive owns device
    drivers (display+LED heartbeat done; pattern-set + plugin-ization next); composer the sentant+catalogue;
    specs/core the general capability traits.
  - **r2.hw.led capability DRAFTED for specs/core** (`4a9f0dd`, `docs/r2-hw-led-capability-proposal.md`) —
    semantic CMD_SET_STATUS{status} vocab (ok/joining/ota/error/identify/idle — meanings not blink-codes);
    descriptor kind:mono|rgb + statuses + dimmable + (rgb) colour slots; device driver maps status→rendering.
    **CRITICAL (Roy): LED INDEPENDENT of display** — firmware-direct base statuses (boot/ota/error) signal
    when the screen is down → don't route LED via the render plugin. **Firmware TODO:** init the LED
    before/around the display + a panic→error pattern, so a display fault never silences the LED. Sent specs.
  - **PROJECT: LoRa heartbeat-SYNC ("fireflies")** (`33eac83`, `docs/lora-heartbeat-sync-design.md`) — Roy's
    next showcase: synchronise the LED heartbeats via sentants exchanging r2.sync.fire events over LoRa
    (pulse-coupled oscillators). **PREREQUISITE (Roy): both nodes on the SAME TG** (events are TG-scoped) →
    needs identity (workshop hive_id/NVS) + **r2-trust no_std verify** (group-HMAC on MCU, currently std) +
    R2-PROVISION join on MCU. Deployment-reality catch (refuter): synced firing = simultaneous half-duplex
    TX = collisions → TX jitter/desync so LEDs sync tight while radio announces spread. Gated on LoRa + TG
    tiers (both downstream). **Algorithm is host-prototypable NOW** (offered to supervisor: r2-harness-style
    convergence sim + tune ε/jitter/T + partition/heal; + a TN-sync conjecture for specs). composer owns the
    HeartbeatSync sentant.
  - **FIRST-LIGHT PASS DONE (board live!)** (`db33289`, `docs/dfr1195-first-light-findings.md`). Board on
    **tuxedo-os /dev/ttyACM0**; hive on **Alfred** (esp/Xtensa toolchain); passwordless SSH = build-on-Alfred
    /flash-on-tuxedo. **SILICON-confirmed esp32s3 rev v0.1 / 4MB** (espflash board-info — settles SoC for
    good). core's skeleton **BUILDS for xtensa-esp32s3** with 3 hive fixes (patch `docs/dfr1195-s3-validation.patch`):
    C6→S3 re-target; wifi.rs:139 embassy-net SocketAddrV4→IpEndpoint; source export-esp.sh
    (`~/Development/homelab/export-esp.sh`) for the Xtensa linker. esp-hal/esp-wifi/embassy matrix compiles
    clean (no footgun). **FLASH BLOCKED:** espflash 4.4.0 requires the ESP-IDF app descriptor; esp-hal 0.23
    doesn't emit it (no bypass). **FIX = core bumps skeleton to esp-hal 1.0 + esp-bootloader-esp-idf matrix**
    (API migration; core's call — flagged + patch handed). I re-validate on metal the moment core pushes.
    Coexistence on tuxedo OK (only /dev/ttyACM0, no service restarts; workshop's :21042 untouched).
    **MATRIX DISCOVERED (cargo search):** esp-hal **1.1.1**, esp-hal-embassy **0.9.1**, esp-wifi **0.15.1**
    (restructured around NEW **esp-rtos 0.3** scheduler), esp-bootloader-esp-idf **0.5.0**, esp-alloc 0.10,
    esp-backtrace 0.19, esp-println 0.17, + embassy-* bumps. esp-wifi 0.12→0.15 = near-rewrite of the
    controller/init bringup = **core's authored domain** → handed core the migration + matrix; **hive =
    fast metal-validator** (isolated git worktree `~/Development/R2/dfr1195-fw-wt` + board + esp toolchain
    ready; core pushes → I build+flash+report in minutes). core is ACTIVELY on the skeleton (4d15812 S3
    re-target + c4927bb LoRaRadio) — do NOT touch its live working tree; validate via the worktree.
  - DONE (unblocked prep): **2-slot OTA partition table** (`3ad44e1`, `docs/dfr1195-ota-partitions.md`) —
    critical-path gap #5, hive-owned. 4MB S3: ota_0/ota_1 @ 0x1E0000 (1.875MB) + nvs/otadata/phy, fits +
    128KB headroom. FirmwareSink::slot_capacity()=0x1E0000 → OtaReceiver TOO_BIG bound. Handed to core for
    integration into platforms/dfr1195 once S3-re-targeted.
  - **Part D4: LCD display PLUGIN** (Roy directive; post-first-light, NOT blocking). DFR1195 LCD =
    **0.96in color 160×80 = ST7735S** (DFRobot wiki); pins MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48.
    Roy's split: **hive = device-specific no_std ST7735S output plugin** implementing a **GENERAL display
    capability** (render trait + descriptor: res/color-format/has-backlight/has-power-cut) that **specs
    defines + core implements** (LoRaRadio-pattern); **composer = display SENTANT + view-model** (the WHAT,
    calm-tech glanceable). General/reusable for composer's catalogue, not test-specific. Contract Qs
    answered to composer (now the GENERAL `b32d47d` DISPLAY-PLUGIN-CONTRACT-PROPOSAL, supersedes LCD-only):
    one general 'display' capability + per-board driver selected by board.toml (LoRa-carrier pattern).
    **LOCKED contract (composer `ed50505`, confirmed — final):** MANDATORY device-agnostic core = **CMD_RENDER
    (r2_cbor int-keyed view-model) + CMD_CLEAR**. OPTIONAL + descriptor-gated **CMD_BACKLIGHT(level u8 0..255,
    0=off → GPIO16 PWM)** — sentant sends it only when descriptor.backlight != 0; my ST7735S driver implements
    it; driver MAY self-manage a calm-tech default (idle-dim/wake) when none sent. **power_cut (GPIO48) =
    driver-local via descriptor flag, no command.** DFR1195 descriptor: **ST7735S / 160×80 / RGB565 /
    backlight=dimmable / power_cut=yes**. General capability TRAIT + descriptor = specs/core to define +
    ratify (LoRaRadio pattern; converged ask from composer + me); composer view-model rides on top.
    **Driver impl sequences after esp-hal-1.1 first-light.**
- **PAUSED (Roy, pending UX feedback): storing-backend / BOS-on-R2.** Branch `storing-backend` —
  RecordStore seam skeleton landed + shelved-ready (`docs/storing-backend-hive-scoping.md`). Do NOT
  build further until Roy resumes. Resume point: SQLite-behind-the-seam + persistence ensemble.
- ~~TN refutation re-run~~ DONE (`2642263`) — core `da89050` wired the knobs; re-ran both vs r2-harness:
  TN-L2-XT-BL-001 (OOM guard, set_scf_buffer_cap+tail-drop) and TN-L2-XT-AB-001 (entanglement epoch) now
  DECIDABLE → CONFIRMED. Filed to specs+core with 2 deployment-lens refinements (tail-drop vs TTL-aware
  eviction; epoch/buffer RAM-volatility). Resolution addendum in docs/phase3-tn-refutation-batch3.md.
  Standing refuter duty otherwise idle (remaining L0/L1/L3 functional cells sweepable on request).
- ~~CONVERGENCE BLOCKER: R2-WEB v0.6 CSP drift~~ **RESOLVED** (`827295b`) — Roy ratified R2-WEB v0.6 csp;
  synced hive web.rs to `WebPluginManifest.csp = Option<CspPolicy>`: `MountedBundle.csp` → `CspPolicy`,
  `build_csp`→`render_csp` (renders the directive BTreeMap), `restrictive_default` defensive fallback,
  `DEFAULT_CSP` removed, tests + integration manifests updated. BIN builds vs core's current tree; full
  workspace green (17 blocks). SECURITY FLAG to specs: §3.4.1 restrictive_default dropped
  `frame-ancestors 'none'` (+base-uri/form-action) vs the pre-v0.6 hive default → unframed web UIs now
  clickjackable unless they author csp; suggested specs re-add it. **→ RATIFIED as R2-WEB v0.7**
  (specs 5553f80): restrictive_default restores frame-ancestors 'none'+base-uri 'self'+form-action 'self'
  + adds script-src 'wasm-unsafe-eval'. `restrictive_default()` is **r2-def's (core)** — hive web.rs only
  CALLS it, so hive INHERITS the fix automatically once core updates r2-def (flagged core; no hive code
  change for the default). **hive v0.7 follow-ups (low pri, behind firmware lead):** (a) re-add the
  `frame-ancestors 'none'` assertion to web_plugin_integration test once core's restrictive_default emits
  it; (b) connect-src `+ws` serve-time append (render_csp adds hive's live WS origin when serving).

## Done + green
- **v0.2 migration + relay handshake + 4 vector fixtures** — full r2-hive suite GREEN; on
  `v0.2-relay-handshake` (pushed). Fixtures all specs-verified + landing: host-api (28),
  usb (specs), usb-pair (12 → canonical home **R2-PROVISION §5.3.4**), plugin-web (11, Ed25519).
  Generators: `crates/r2-hive-bin/examples/gen_{host_api,usb_pair,plugin_web}_vectors.rs`.
- **core D3a synced + relay driver CONFIRMED** (`3c5ba9c`) — core's WebSocketTransport §4.4.1 fan-out +
  UDP-LAN are now REAL (core `52b0e4e`). hive's relay driver (`compat/handshake.rs`: v0.1/v0.2 Ed25519
  handshake → `peers().connect()`→OutboundRx, `push_inbound` on recv, drain `outbound_rx.next()`→ws.send,
  `remove_peer` on cleanup) builds + runs GREEN against the real machinery (was scaffold). One core
  API-drift fix: `WebPluginManifest.subscriptions` added to 3 test manifest builders. Full suite green.
- **Transport + router integration tests** (`11443cf`,`828b419`) — filled a zero-coverage gap now that
  core D3a transports are real. `tests/transport_integration.rs` (3): HiveState send path round-trips
  over REAL loopback UDP-LAN sockets (set_udp_transport + send_to_hive_via → Wifi slot), no-transport→None,
  Wifi-hint routing. `tests/router_integration.rs` (5): route_frame NotR2Wire rejection, the 32-byte
  HMAC-tag trim fallback, valid-frame routing, and engine dedup (seeded neighbour → flood then dup-drop).
  Transport layer now VERIFIED working against core's real machinery, not just compile-green.
- **USB spec citations resolved** (`4c70d2c`,`8f31231`) — usb_pair/usb/main/usb_serial/usb_hotplug/api.rs
  all R2-HIVE §6.4.x → R2-PROVISION §5.3.4 (specs ruled it the canonical pairing home); R2-USB v2→v0.1.
  Type-byte divergence: specs RULED **ratify** as R2-USB §3.2.1 (don't drop; collision-free). Both
  wire extracts (type-byte table + CAPS + legacy detection; PAIR_* msg vocab + CBOR layout) committed
  `docs/r2-usb-wire-extract-for-specs.md` (`5232e61`) + sent to specs. Spec authoring is Roy-gated.

## In flight — Platform-trait extraction (north-star convergence step 1)
Split today's std hive → `r2-hive-core` (no_std+alloc host loop) behind a `Platform` trait +
thin platform layers (linux first). Verifiable on Linux now; foundation for esp32/wasm/unoq.
- DONE seams: 1 = clock (`69ab8fb`), 2 = RNG (`04d19cc`), 3 = **transports** (`1e24da8`):
  `src/platform.rs` (`Platform` trait + `LinuxPlatform`); `HiveState.platform` (default,
  no `new()` sig change); `src/transport_seam.rs` (`HiveTransports` trait = outbound
  multi-transport contract, `HiveState` impls it, `&dyn` proven). 100 lib tests + full suite green.
- DONE: **sync host-loop seam** (`sync_host.rs`, `683241f`) — `SyncTransport` trait
  (`kind`/`send`/`poll_recv`) + `TransportAddr`/`InboundFrame` + `provisional_hive_id` +
  `poll_inbound` tick primitive; Linux-verified via sync-stub. **TRANSITIONAL local mirror** of
  the seam core+hive AGREED (R2-DISCOVERY §5 sync). Core will EXTEND r2-transport
  (`Transport::poll_recv` default-None + TransportAddr/InboundFrame) → then delete the mirror,
  import `r2_transport::`. Host resolves source_addr→hive_id; driver-owned RX buffer.
- DONE: **RouteEngine wired into the sync host loop** (`route_inbound_sync`, `3ebdb61`) — parse
  R2-WIRE → ingest neighbour → `plan_forward` → execute Drop/DeliverOnly/Directed/Flood over
  `SyncTransport`; routing-only (no ensemble/TG/WS host bits); host-centralised resolution
  (specs-confirmed conformant, R2-DISCOVERY §5). Linux-verified end-to-end (real RouteEngine +
  sync-stub relay). 106 lib tests, full suite green.
- DONE: **`r2-hive-core` crate split started** (`a05b108`) — new `#![no_std]`+alloc crate (deps
  r2-wire/route/fnv only, no tokio/axum/std-net); **`sync_host` moved into it and compiles no_std**
  = PROOF the routing host-loop is MCU-portable. bin depends on it + re-exports `sync_host`
  (zero churn). Full workspace green (r2-hive-core 6 tests + bin suite).
- DONE: **Platform + transport seams migrated into r2-hive-core** (`234fd60`) — `Platform` trait
  (clock+RNG) → `core/src/platform.rs` (no_std), `LinuxPlatform` impl stays in bin + re-exports trait;
  `HiveTransports` outbound seam → `core/src/transport_seam.rs` (async-trait, no_std+alloc, needs
  `alloc::boxed::Box`), `HiveState` impl + `&dyn` trait-object test stay in bin (`hive.rs`).
  r2-hive-core builds no_std; full workspace green (100 bin lib + 6 core tests). Pushed.
- DONE: **storage seam migrated into r2-hive-core** (`b42658c`) — `core/src/identity.rs` (no_std+alloc):
  `MasterSecret` derivation (HKDF-SHA256 → hive_id/DEV_PK/DEV_SK), `DerivedIdentity`, fingerprint, UUIDv4,
  web-auth-key + the seam itself (`IdentityStore` trait, `StoreBackend`, platform-neutral `StoreError`
  replacing `io::Error` at the trait boundary). bin keeps std stores (`FileStore`/`KeyringStore`/
  `auto_store` + permissions/XDG/getuid), impls the core trait (io→StoreError), re-exports core types
  (mgmt::identity::* unchanged). RNG stays platform-side (getrandom→`from_bytes`); `bytes()` →
  documented storage-only `expose_secret_bytes()`. ed25519-dalek/hkdf/sha2/zeroize added to core
  default-features=false. r2-hive-core no_std; full workspace green (94 bin lib + 13 core tests).
- DONE: **OTA-receiver seam in r2-hive-core** (`354f395`) — `core/src/ota.rs` (no_std), the portable
  half of the firmware receiver: constants (OTA_PORT 21043/CMD_*/STATUS_*/PREAMBLE_LEN),
  `OtaPreamble::parse` (image_len u32 LE + sha256[32]), `OtaError` CODEs (PREAMBLE/TOO_BIG/BAD_MAGIC/
  SHA_MISMATCH/WRITE_FAIL/NO_SLOT/SHORT) + alloc-free `encode_reply/ok/error`, `FirmwareSink` trait
  (storage seam = flash I/O), `OtaReceiver` state machine (TOO_BIG bound-check BEFORE begin, streaming
  SHA-256, verify→finalize, abort-on-error). NOT a migration (no OTA code existed in bin) — built from
  core's `platforms/esp32/src/ota_tcp.rs` reference + composer's OTA-REPLY-STATUS-CONTRACT. 11 tests.
  Heads-up sent to composer to confirm CODE set / push-side framing. **Platform supplies:** embassy-net
  byte reads + esp-storage `FirmwareSink` impl (device); host uses a RAM mock. CMD_QUERY handled by
  platform layer (build info), not core.
- NEXT: with routing/identity/OTA cores all no_std + **5 seams** in place (sync_host, platform,
  transports, identity, ota), the convergence's host-side factoring is largely done. Remaining is
  firmware-tier (gated): swap `sync_host` seam mirror → `r2_transport::` when core EXTENDs r2-transport
  (poll_recv default-None + TransportAddr/InboundFrame); esp-hal/embassy board crate (P0) + esp-storage
  FirmwareSink + embassy-net OTA host loop (needs xtensa toolchain + hardware + core D3b).

## Next major phase — D2: DFR1195 (ESP32-S3) firmware, Path B pure no_std (esp-hal/embassy)
Gated on the convergence above + core's D3b. Sketch: `docs/esp32-hive-firmware-architecture.md`.
- Firmware = core's no_std stack + core's **D3b** no_std SYNC radio bindings, wrapped in an
  esp-hal/embassy host loop. Consume **R2-TRANSPORT SYNC** (R2-DISCOVERY §5), not async §4.
- hive owns: board layer (SX1262 LoRa / LCD / IO18 button), on-device host loop, **no_std OTA
  receiver** (embassy-net; std `ota_tcp.rs` is reference only). **Validation handoff:** core
  authors D3b but can't flash — **hive validates on real DFR1195**, feeds defects back.
- **Identity:** my firmware CONSUMES the shared `r2-esp/hive_id` module (workshop-owned, one impl per
  north-star) — incl. the agreed `usb_link_id = HKDF(master_secret,"r2-usb-link-v1")` (stable USB-link
  id) / `mesh_hive_id = HKDF(master_secret,info=tg_id)` split. Do NOT fork a parallel derivation. Gated
  on specs ratifying R2-USB §3.6 (workshop holds the change until then).
- Near-term scope flag: r2-def/ensemble/dispatch are std-tier → initial MCU hive is
  ROUTING+TRANSPORT only (no on-device ensembles) until those are re-tiered no_std.
- References (std, patterns not code): core `platforms/esp32`, workshop `firmware/esp32-s3`.

## Pending Roy / cross-repo
- **OPEN — CAPS device-identity gap: CONFIRMED REAL, fix agreed, spec-first** (awaiting specs §3.6
  authoring, Roy-gated). ROOT CAUSE (workshop firmware answer): ESP32 derives `hive_id_bytes =
  HKDF(master_secret, info=tg_id)` = TG-SCOPED, and the SAME 16 bytes feed CAPS §3.6 + my link-key store
  key + reconnect HMAC + mesh hive_id (§6.2.1). Cross-TG provisioning → different value → my LinkKeyStore
  (keyed solely on CAPS hive_id_bytes) misses → silent forced re-pair. AGREED FIX (workshop owns,
  r2-esp/hive_id.rs): split — `usb_link_id = HKDF(master_secret,"r2-usb-link-v1")` STABLE/TG-indep → CAPS
  + link-key store; `mesh_hive_id = HKDF(master_secret,info=tg_id)` → mesh. **My host needs ZERO change**
  (store keys on whatever stable CAPS id arrives). PROPOSED NORMATIVE RULE relayed to specs: CAPS
  hive_id_bytes MUST be stable for device life + TG-independent; mesh hive_id (§6.2.1) is separate →
  R2-USB §3.6 + R2-WIRE §6.2.1 cross-ref; composer also a consumer (provisioning/OTA). workshop HOLDS
  firmware change until specs ratifies §3.6 wording. Minor: dev devices paired pre-fix do a 1-time
  re-pair (harmless pre-launch). eFuse-MAC comment already marked impl-defined-pending-spec (`b33547f`).
- ~~Roy: greenlight R2-PROVISION §5.3.4~~ DONE — specs confirms COMMITTED (`4b74b20`, v0.6, Roy
  green-lit) on `spec-conformance-v0.2`. Cite by paragraph name (no §5.3.4.y sub-numbers).
- ~~hive TODO: usb_pair.rs citation fix~~ DONE (`4c70d2c`) — usb_pair.rs §6.4.x → R2-PROVISION
  §5.3.4 (SAS verification/Link key/Reconnect/Key agreement); main.rs+usb_serial.rs "R2-USB v2" →
  "R2-USB v0.1", SYNC frame → §3.3. Doc-only; builds clean.
- ~~OPEN: type-byte divergence + usb.rs frame-vocab mapping~~ **CLOSED — RATIFIED + VERIFIED.** specs
  authored all three (`71ee053` spec-conformance-v0.2, Roy-authorized): **R2-USB v0.2** §3.3 version
  negotiation / §3.5 type byte / §3.6 CAPS / §3.7 control + Appendix A transport kinds; **R2-PROVISION
  v0.7 §5.3.4** message vocabulary (PAIR_* 4-11). I VERIFIED both against usb.rs — all bytes match (CAPS
  keys, msg fields, nonce_rc/tag b16, abort vocab exact 8-match). **Both normative tightenings specs
  added were ALREADY honoured by the impl:** (a) failed reconnect does NOT fall back to first-attach
  (`usb.rs:846-848` → fail_pairing→Closed); (b) AutoPairUnsafe NOT default (Strict default; dev-only
  ctor used only in tests; prod watcher `usb_hotplug.rs:590` = Strict). usb.rs cites finalized
  (`12c6a43`): 'pending ratification' dropped, framing→§3.5-3.7, pairing→§5.3.4. Impl is now CANON.
- **Deps:** core **D3b** (no_std sync BLE/WiFi/LoRa) = hard blocker for radios; composer = OTA
  push + carrier + ensemble; specs = hw test defs.
- Phase-3 adversarial-refuter role (deployment reality): FILED first batch to specs (the 5
  high-value TN conjectures). Two systemic findings — (A) must_text bounds by TTL/time, never
  MEMORY (MCU RAM = fixed tables+eviction; fixed-size dedup evicts before window W); (B) hop-TTL
  ≠ wall-clock (a carried frame's hop-TTL never decrements while carried). Verdicts:
  TN-L2-IT-BL-001 + TN-L2-IT-AB-001 FALSIFIED-as-stated; BL-002/XT-BL-001/L1-IT-BL-004 REFINE.
  + sim-tier-decidability flag (sim needs bounded-mem + carry-time model, else mark tier=hardware).
  Awaiting specs adjudication; more conjectures can be reviewed on request.
  DYN-family batch (v0.3, 13 conjectures) ALSO filed: grounded vs real r2-route (f32 + libm::expf,
  multiplicative c+0.2*(1-c), mobility is an engine INPUT not RSSI-classified). Findings: (A)
  TN-L0-IT-BL-100 spec-vs-impl — must_text additive +0.1 vs impl multiplicative +0.2*(1-c) [core
  reconcile]; (B) TN-L2-IT-BL-100 RSSI-sigma classifier UNREALIZED + fragile under real RSSI noise
  → tier=hardware [strongest]; (C) soft-float expf cost on no-FPU (ESP32-C6); (D) fixed-point future
  → 0.05*(1-c) underflow (TN-L2-IT-BL-101). DYN batch ADJUDICATED by specs (`a9c28b1`): 3 new
  R2-ROUTE issues (8→11) — additive-vs-multiplicative BLOCKED+Roy-gated, RSSI-sigma re-tiered
  HARDWARE, expf/fixed-point forward-flagged.
  **BATCH 3 FILED** (`d161054`, docs/phase3-tn-refutation-batch3.md) — un-refuted SCF + XT/entanglement
  cells, grounded in real r2-route + r2-harness code. Key: RouteEngine has NO buffer/queue/entanglement
  (ForwardAction lacks a Queue variant; no-path → Drop(NoViableNeighbour) = silent drop); entanglement
  is SIM-ONLY (r2-harness live:bool, honesty #6; r2-trust §7 = no keep-alive/@entangled routing).
  Verdicts: TN-L2-IT-BL-002 FALSIFIED (no queue); TN-L2-IT-AB-000 FALSIFIED for carry>60s dedup;
  TN-L2-XT-BL-001 OOM-guard not sim-decidable (re-tier hw); all XT-AB cells test sim gate not
  authenticated crossing (passes-while-violating-spirit); BL-101 CONFIRM / BL-100 FALSIFY (no
  heartbeat → entangled-but-unreachable on duty-cycled links); XT-AB-001 undecidable (no instance id);
  XT-BL-100 'kept' conflicts w/ 30min route eviction.
  **BATCH 3 ADJUDICATED** (supervisor, verdict-of-record; catalogue write pending perm): IT-BL-002
  ACCEPT-FALSIFIED → R2-ROUTE #7 (MUST → named SCF layer, DUAL bound RAM×TTL; engine silent-Drop OK at
  routing layer); IT-AB-000 ACCEPT-FALSIFIED → operative rule = IT-AB-001 (idempotency at dispatch);
  IT-BL-000/XT-BL-000 = PRODUCTION-UNREALIZED (sim tests logic only, lifts no impl signal); XT-BL-001
  ACCEPT not-decisive → experiment revised (inject buffer cap; true OOM=hardware); XT-AB cells honesty-#6
  (authenticated-crossing MUSTs deferred to r2-trust §7 production); **XT-BL-100 entangled-but-unreachable
  = HEADLINE** → BLOCKED impl-missing (§7.3 keep-alive DEFINED-unimplemented); 3 Roy options, supervisor
  recommends implement §7.3 minimal keep-alive (decay-exemption REJECTED-leaning — contradicts BL-101);
  XT-AB-001 ACCEPT sim-undecidable → instance/epoch id (harness + R2-TRUST §7.6, Roy-gated); XT-BL-100
  NOT-falsified CLARIFIED (record-retention §7.3 vs route-eviction R2-ROUTE 2.5 both defined, no conflict).
  Remaining open cells: IT/XT main-path L0/L1/L3 functional cells (lower deployment-lens value) on request.

## Resume hygiene
Keep this current. WIP-checkpoint + push `platform-trait` periodically. Safe git only:
named `git add` / `git add -u` — never `git add -A`/`.`; never stage secrets.
