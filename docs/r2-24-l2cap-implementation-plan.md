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
