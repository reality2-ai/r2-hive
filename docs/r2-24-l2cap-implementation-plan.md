# #24 S1→S4 implementation plan — L2CAP CoC → NegotiationRadio → network-forming

**Status:** S0 DISCOVER DONE on metal (advertise+scan+resolve, 2-board). S1 design 100% CLOSED
(core's 5 answers + workshop's interop spec). All trouble-host APIs gathered. This doc = the
instant-resume build plan. Build env: r2-discovery @1496916 (engine + beacon + reform-hardening).
Firmware: `dfr1195-fw-wt/platforms/dfr1195/src/main.rs`, `ble_task`, behind `#[cfg(feature="ble")]`.
Build: `cargo build --release --features ble`. 2-board test on ACM1 (b79010=2cab5f69) + ACM11 (0dcadbf8).

## Design (CLOSED — core + workshop)
- **PROVIDER-STAR.** Engine elects ONE provider (lowest eligible hive_id). Joiners each open ONE control
  CoC to the provider; provider ACCEPTS N. `send_control(HiveId,msg)` peer-addressed; radio maps HiveId→Conn.
- **BOTH adv-sets, always.** Non-connectable RBID beacon runs CONTINUOUSLY (never silent, S0–S4) +
  AP-capable nodes ALSO run a **connectable** adv (same RBID payload). Don't switch the beacon. (R2-BLE §10)
- **Connect-addr from the live connectable-adv scan** (carries the same resolvable RBID). Radio owns
  hive_id→CURRENT connectable addr (rotates with RBID — never store static). Engine never sees an addr.
- **PSM = 0x00D2** (R2-WIRE event PSM; OTA=0x00D3). Listener on 0x00D2 while advertising. BT_SECURITY_LOW.
- **Radio owns HiveId→Connection map + lifecycle**: open-on-demand (send_control to unconnected peer →
  establish ACL+CoC), buffer-until-connected, teardown on S3/provider-switch. Trait UNCHANGED
  (send_control/poll_control only). Engine tolerates async connect (WifiReq → wait data_plane_state==Available,
  else T_negotiate timeout → S0 retry). If CoC still opening when send_control fires → buffer.
- **Framing (R2-BLE §6.4):** SDU = `[len_lo, len_hi, payload]` 2-byte LITTLE-ENDIAN prefix. One ControlMsg/SDU.
- **ControlMsg codec = SHARED `r2_discovery::ControlMsg` encode/decode** (NOT hand-rolled per platform —
  workshop's call, north-star like beacon+engine: one codec → esp-idf↔esp-radio byte-exact, no drift).
  Proposed layout (core to land + own the bytes): `[tag]` 0x01=WifiReq / 0x02=WifiOffer + `ssid[32]‖psk[64]‖
  ap_hint_be32` / 0x03=WifiDone. ≤101 B ≪ MTU 512 → no fragmentation. l2cap.rs is OPAQUE transport (strips the
  LE frame, hands raw payload up); the decode is in NegotiationRadio via the shared codec. PENDING: core adds it.

## trouble-host APIs (gathered, v0.6.0)
- `HostResources<DefaultPacketPool, CONNS, CHANNELS, ADV_SETS>` — size CONNS≥2, CHANNELS≥2, ADV_SETS≥2.
- Provider: connectable `Advertisement::ConnectableScannableUndirected{adv_data, scan_data}` →
  `Connection::accept(&stack).await` → `L2capChannel::accept(&stack, &conn, &[0x00D2], &L2capChannelConfig::default()).await`.
- Joiner: `central.connect(&ConnectConfig{scan_config, connect_params}).await` → `Connection` →
  `L2capChannel::create(&stack, &conn, 0x00D2, &config).await`.
- Channel I/O: `channel.send::<C>(&stack, &frame).await` / `channel.receive::<C>(&stack, &mut buf).await`.
- Scan already yields (hive_id, rep.addr) in `R2ScanHandler.on_adv_reports` — store both for the map.

## Build milestones (incremental, metal-verify each)
- **M7 — CoC connectivity (2-board).** Add ADV_SETS=2: keep the non-conn beacon + add a connectable adv
  (provider/AP-capable only — gate on a role flag, interim "lowest of KNOWN_HIVE_IDS present" or a const).
  Provider: accept-Connection loop → L2capChannel::accept on 0x00D2 → store. Joiner: on resolving the
  provider in scan, central.connect to its addr → L2capChannel::create 0x00D2. Exchange one framed test
  SDU each way. PROVE: a byte crosses the CoC between ACM1↔ACM11 while beacon+scan+WiFi all still run.
  Watch: HostResources sizing, concurrent adv-sets+scan+conn airtime, the shared-stack borrow across
  runner/peripheral/central/channel (likely needs the stack ref threaded; trouble examples use it across joins).
- **M8 — NegotiationRadio impl** (struct over the stack + maps). advertise/poll_scan = existing beacon+scan.
  send_control(hid,msg): map hid→addr→(connect if needed)→channel.send(frame(encode(msg))); buffer if opening.
  poll_control(): drain channels → (addr→hid, decode). bring_up_provider = existing SoftAP start;
  join_provider = existing STA connect (AP-IP via gateway, not hardcoded); data_plane_state ← WiFi link;
  teardown_data_plane; now_ms = embassy Instant. HiveId→Connection map = heapless::FnvIndexMap or a small array.
- **M9 — run the engine.** `NegotiationEngine::<16>::new(my_hive, NodeCaps::new(ap_capable, power_state),
  5000, 10000)`; each tick `eng.poll(&mut radio)`; `eng.request_data_plane()` on demand;
  `eng.set_power_state(..)` on transitions. DISCOVER→NEGOTIATE→FORM on 2-board (joiner gets WifiOffer →
  joins the provider's SoftAP).
- **M10 — fallback→reform + telemetry.** Disruption (AP lost / silence>T_fallback / unreachable) → S3→S4→
  re-elect→S2. Emit health key13=forming_phase, key14=neighbor_count, key15=role for composer's proof surface.

## All-hands (ready)
core: engine+beacon ready @1496916, lockstep on any trait gap (none expected). workshop: byte-compatible
l2cap.rs (PSM 0x00D2, LE framing) + will mirror framing + offers esp-idf NegotiationRadio for cross-platform
proof (greenlight when my CoC sends). composer: proof surface prepped for key13/14/15 + flashes on ready.

## M8 ARCHITECTURE — sync↔async bridge (trait is SYNC; codec landed @53c1e58)
NegotiationRadio is SYNC (fn advertise/poll_scan/send_control/poll_control/bring_up_provider/
join_provider/data_plane_state/teardown_data_plane/now_ms) + `engine.poll(&mut radio)` SYNC. BLE
(trouble) is ASYNC. So the radio impl is a sync façade over async BLE via STATIC shared state:
- **Async background tasks** (spawned in ble_task / its own task): runner.run_with_handler(&handler);
  advertise loop (non-conn RBID beacon + connectable adv on provider_capable nodes); scan loop
  (handler pushes observations + populates HiveId↔addr); a CONNECTION-MANAGER loop (drains CTRL_OUT →
  connect-if-needed via HiveId↔addr → L2capChannel::create → frame+send; CoC receive → strip frame →
  push CTRL_IN with the src hive).
- **Static shared state** (embassy_sync blocking Mutex<RefCell<…>> or heapless queues):
  SCAN_OBS: Deque<(hive_id, provider_capable, power_state)> — handler push / poll_scan pop → NegObservation::new.
  HIVE_ADDR: map hive_id→(AddrKind,[u8;6]) from scans (current connectable addr; rotates).
  CTRL_OUT: Deque<(hive_id, [u8;MAX_ENCODED_LEN], len)> — send_control push / conn-mgr drain.
  CTRL_IN: Deque<(hive_id, ControlMsg)> — conn-mgr push / poll_control pop.
  DATA_PLANE: atomic state (Available/Failed) ← WiFi link.
- **Sync NegotiationRadio impl** (the façade): advertise = no-op (async loop already advertises the beacon);
  poll_scan = SCAN_OBS.pop → NegObservation::new(hive,provider_capable,power); send_control = msg.encode →
  CTRL_OUT.push; poll_control = CTRL_IN.pop; bring_up_provider = start SoftAP (existing wifi); join_provider
  = STA connect (existing wifi, AP-IP via gateway); data_plane_state = DATA_PLANE atomic; teardown; now_ms
  = embassy Instant ms.
- **Engine task:** `let mut eng = NegotiationEngine::<16>::new(my_hive, NodeCaps::new(provider_capable, power),
  5000, 10000);` loop { eng.poll(&mut radio); eng.request_data_plane() when app needs it; Timer tick }.
- **ControlMsg codec (landed @53c1e58):** `let mut b=[0u8;ControlMsg::MAX_ENCODED_LEN(=103)]; let n=msg.encode(&mut b);`
  send &b[..n] (wrapped in [len_lo,len_hi]); `ControlMsg::decode(payload)->Option` (after stripping the frame).
  Wire: WifiReq=[0x01] / WifiOffer=[0x02][ssid_len][ssid][psk_len][psk][ap_hint:4 BE] / WifiDone=[0x03]. Both
  platforms identical (workshop folded the same API → zero-drift).
- **Provider/joiner roles now ENGINE-DRIVEN** (not the M7 const): the engine elects (lowest provider_capable
  hive); the conn-mgr opens CoC to the elected provider. M7's M7_PROVIDER_HIVE const retires.

## M8c — REAL WiFi data plane (runtime reconfig) — TRACTABLE (APIs confirmed)
M9 network-forming NEGOTIATION is proven on metal; M8c swaps the stub (DATA_PLANE_AVAIL=true) for real
runtime WiFi. Confirmed tractable — NO stack recreation:
- `esp_radio::wifi::WifiController::set_config(&Config)` (mod.rs:2540) — runtime reconfig (AP↔STA, ssid/psk).
- `embassy_net::Stack::set_config_v4(ConfigV4)` (lib.rs:541) — runtime IP change on the LIVE stack.
- `WifiController::connect_async()` (mod.rs:2860).
Current WiFi is STATIC-at-boot (main.rs:127 is_ap by MAC → wifi_cfg → wifi::new → stack w/ fixed IP;
controller owned by wifi_task @1541). M8c architecture:
- Static `DATA_PLANE_CMD` (embassy Signal): bring_up_provider/join_provider (sync trait) store the cmd
  (mode + ssid + psk from DataPlaneParams) + signal; a WIFI-CONTROL task (refactor: owns the controller,
  replaces/extends wifi_task) picks it up → set_config(AP|STA) → start/connect_async → stack.set_config_v4
  (AP: 192.168.x.1 static; STA: gateway/DHCP) → set DATA_PLANE_AVAIL on link-up (clear on loss = the S3
  disruption signal for M10).
- bring_up_provider(p): AP cmd (ssid=p.ssid(), psk=p.psk(), be the SoftAP). join_provider(p): STA cmd
  (connect to p.ssid()/p.psk(), AP-IP via gateway — workshop wifi_sta::get_gateway pattern, not hardcoded).
- data_plane_state: DATA_PLANE_AVAIL (Available when associated+IP; Failed on loss).
- On the 2 test boards: the boot mesh STA is REPLACED by the formed data plane (provider→its SoftAP,
  joiner→that SoftAP). BLE control plane keeps running underneath (coex proven). composer sees the REAL B→W flip.
- DELICACY: the controller is in wifi_task; refactor to a control task that handles reconfig cmds; sequence
  the set_config/connect vs the existing io_task using the stack; keep BLE coex. Then M10: lose-AP →
  DATA_PLANE_AVAIL=false → engine S2→S3→S4→reform + emit key13/14/15 telemetry.

## ACCEPTANCE TEST (Roy's canonical TN success) — remaining dynamic behaviors after M8c
M8c = real form DONE. The full success demo + the 4 remaining behaviors (each substantial):
1. **FORM → SYNC** (next): the 2 formed boards lub-dub-sync over the formed WiFi. FINDING (metal): they
   FORM but DON'T sync yet — joiner `synced=false` (no HB<- received), provider beats stuck ~8. The io_task
   heartbeat needs formed-net work: (a) socket lifecycle — io_task spawns before the joiner's stack is up
   (wait_config_up skipped for ble) → bind/rebind after the data plane is up; (b) SoftAP broadcast AP↔STA
   (192.168.4.255 over r2-tn-form); (c) conductor-PLL on the formed net (provider 0dcadbf8 = lowest = conductor;
   note is_ap≠serve_ap mismatch — io_task uses is_ap[MAC] not serve_ap[elected], so the WiFi-AP runs STA-role).
2. **TG-GATE (real resolve)** — CONFIRM forming is TG-gated. CURRENTLY BYPASSED: M8b/M8c inject a SYNTHETIC
   peer obs (push_scan_obs(peer_hive)) + deterministic addr — NOT via resolve. Must replace with the REAL
   S0 scan→decode_advert→resolve_rbid_windowed (proven in S0 DISCOVER) feeding push_scan_obs: same-TG peers
   resolve (shared hk via derive_beacon_session_key) → obs; cross-TG can't resolve → no obs → no elect → no
   form (the negative test). CENTRAL CONFLICT: provider scans freely (it accepts, doesn't connect); joiner
   must scan-to-resolve THEN connect (time-share central: Scanner → resolve → into_inner → connect on join_provider).
3. **N-DEVICE-JOIN** — a later same-TG board AUTO-joins the EXISTING provider (don't form new). The engine
   elects the lowest; a higher new board elects the existing lower provider → join_provider → joins. Needs the
   real scan (sees the existing provider) + the provider accepting N CoCs (multi-channel, vs the M8c single).
4. **AP-FAILOVER/REFORM (M10, subsumes #23)** — provider off → joiner data_plane_state→Failed (wait_for_disconnect
   → AVAIL=false) → engine S2→S3→S4 → re-elect next-lowest provider_capable → NEW provider brings up its AP →
   others re-join. HARD PART: the re-elected provider was a STA (booted station interface) → must become AP at
   runtime (the interface-binding issue — needs the access_point interface; M8c pre-assigns by boot role). Real
   runtime AP-bring-up on re-election = the delicate piece (set_config AP + the AP interface/stack).
Each is a focused milestone; report each. core's canonical contract: docs/R2-24-NEGOTIATION-BRIEF.md (631b758).

## CORRECTION (Roy, reverses TG-gate-on-forming) — FORMING is TG-AGNOSTIC
"TG doesn't matter below the trust layer." So:
- **FORM/JOIN = TG-AGNOSTIC** — any devices (any TG) discover → negotiate → form/join the SHARED transient
  network (the transport substrate). Do NOT gate forming on TG. Elect/form among ALL discovered peers.
- **SYNC = per-TG** (trust overlay): same-TG devices heartbeat-sync together (Goal#2 per-TG heartbeats), over
  the shared formed net. Cross-TG boards on the same net sync SEPARATELY.
- **DELIVER = per-TG** (GroupHmac; cross-TG via entanglement). Already have the deliver-gate.
- **RBID-resolution = trust-recognition** (is-this-my-TG-peer, for sync/deliver scoping), NOT a forming-gate.
- So the M8b synthetic-obs/inject EFFECT (form regardless of TG) is ALIGNED with TG-agnostic forming — but it
  should be REAL discovery (see all peers, any TG). The earlier "TG-gate the form / cross-TG don't form" = WRONG;
  cross-TG DO form/join, they just don't sync/deliver together.
- **OPEN (flag core/specs): the TG-agnostic ELECTION mechanism.** The engine elects lowest hive_id, but hive_id
  comes from resolve_rbid (same-TG only) — you CAN'T resolve a cross-TG peer's hive_id. So how to elect/join
  across TGs? Options: same-TG elects via hive_id + cross-TG JOINS the provider (provider_capable flag + BLE addr,
  no election); OR a TG-agnostic election key (RBID/addr-based). Needs a core/specs ruling for the cross-TG case.
  (For 2 SAME-TG boards — the immediate demo — the current hive_id election works; this only affects cross-TG.)

## Revised next milestones
- **FORM→SYNC** (2 same-TG boards): form (TG-agnostic, M8c) + per-TG heartbeat-sync over the formed net. The
  io_task formed-net fix (provider stall ~beat 8 / joiner no-HB) is the blocker — NOT a TG-gate.
- TELEMETRY (composer M10): designate ONE r2-tn-form board to print "r2-...: HEALTH <hex>" (key13/14/15) on
  its USB-serial; composer's health-reader is now MULTI-SOURCE (--ap-port repeatable) → add it as a 2nd source
  → the forming boards' phase-strip shows alongside the mesh (distinct wire_ids).

## DIAGNOSIS — "two same-IP hotspots" (Roy bug) + the exactly-one-elected-AP fix
ROOT (structural): the WiFi-AP role is decided at BOOT, DECOUPLED from the engine's dynamic election.
- (1) **Static boot-AP ≠ dynamic election.** serve_ap = a BOOT const (M7_PROVIDER_HIVE test value, or MAC for
  the mesh) — not the engine's elected provider (lowest-PRESENT provider_capable). The stack interface
  (AP vs STA) is BOUND at stack-creation (boot) → can't change to match the election without a refactor. So
  the board that serves the AP can be one the engine did NOT elect.
- (2) **is_ap[boot-MAC] != serve_ap** (role mismatch — FIXED 434a972): io_task used is_ap (boot-MAC=502698) for
  AP-relay/IP/health, diverging from serve_ap (WiFi role) → a board acting AP in heartbeat logic while STA in
  WiFi (or vice versa) = a second AP identity. Fixed: for ble, is_ap = serve_ap (exactly the AP board acts AP).
- (3) **Trigger candidates for the actual two-AP/same-IP** (need Roy's specifics): (a) a MIX of builds — a
  default board (is_ap=502698 → r2-fieldlab @192.168.4.1) + a ble board (serve_ap → r2-tn-form @192.168.4.1) =
  two APs same IP, different SSID; (b) the serve_ap test-const not matching the fleet's true lowest → engine
  elects X but the const board serves → divergence; (c) >2 boards / the ACM-renumbering scrambling roles.

THE FIX (Roy/supervisor ask = election-driven single-AP, supersede ALL static boot-AP):
- NO static boot-AP. The engine elects the lowest-PRESENT provider_capable; ONLY that elected provider brings
  up the SoftAP at RUNTIME (bring_up_provider → real AP-bring-up); ALL others are STA-joiners (no AP, no IP
  conflict). Guarantees exactly ONE AP for ANY subset — no static-boot divergence.
- IMPL = the M10 runtime AP-bring-up: needs the access_point interface available at runtime (boot APSTA or a
  dual-stack, since the stack-interface is bound at creation) → on bring_up_provider, set_config AP + AP-IP
  (.1) + stack.set_config_v4; joiners get distinct IPs (no collision). M8c pre-assigned by boot const (the bug);
  M10 makes it election-driven.
- ENV-BLOCK: metal-verify blocked by the ACM device RENUMBERING (test boards unflashable at their paths) →
  needs a rig replug + the DFR1195 by-id (MAC) paths.
