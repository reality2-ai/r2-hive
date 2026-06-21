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
