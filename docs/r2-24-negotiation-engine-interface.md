# #24 negotiation engine — interface for core (r2-discovery `negotiation` module)

Per workshop+core: **pure no_std state machine (r2-route pattern)**. core lands the engine in
**r2-discovery** (new `negotiation` module over the existing discovery traits, no-alloc tier);
hive (esp-radio) + workshop (esp-idf) each impl the per-platform **radio trait**. Reuses
`lowest_live_id` (shared with R2-HEARTBEAT conductor election, core's conductor.rs). Conforms
R2-DISCOVERY §4A (S0–S4). Platform BLE on hive = esp-radio `ble` HCI + trouble-host (bt-hci).

## S0–S4 transition table (canonical §4A)
| State | Event (input) | Guard | Action (via radio trait) | Next |
|---|---|---|---|---|
| **S0 DISCOVER** | poll tick | — | `advertise(rbid_beacon)` + `poll_scan()` | S0 |
| | beacon observed | — | roster.upsert(peer, caps, power_state, last_seen) | S0 |
| | data-plane need | peers present | elect provider = `lowest_live_id`(eligible); if self→`bring_up_provider`; else→`send_control(provider, WifiReq)` | S1 |
| **S1 NEGOTIATE** | WifiOffer(creds) rx | joiner | `join_provider(params)` | S1 |
| | data_plane_state==Available | — | — | S2 |
| | timeout `T_negotiate` / fail | — | backoff; drop offer | S0 |
| **S2 DATA** | (steady) | — | data flows (R2-WIRE); beacon at reduced duty (never silent) | S2 |
| | disruption | link-loss \| silence>`T_fallback` \| peer power_state Critical/Survival \| addr-unreachable | mark provider FAILED | S3 |
| | WifiDone (graceful) | — | `teardown_data_plane()` | S0 |
| **S3 DISRUPTED** | entry | — | exclude failed provider from election set | S4 |
| **S4 FALLBACK+RENEGOTIATE** | entry | — | re-enter NEGOTIATE over the (always-on) beacon plane, excluding the failed provider → re-elect | S1 |

Self-healing loop = **S2→S3→S4→S1→S2**. Control plane (beacon) stays up across S2–S4 (the
whole point — recovery needs no out-of-band human). `lowest_live_id` eligibility = AP-capable
+ power_state Normal/Eco; silence-failover = peer silent > T_fallback dropped from the live set.

## Radio/discovery trait surface (what the engine drives; platform impls per-side)
Built on r2-discovery's existing traits (BeaconAdvertiser, PeerMap/PeerTable, AsyncTransport,
TransportState, LinkQuality, BeaconObservation). The negotiation engine needs:
```rust
trait NegotiationRadio {
    // CONTROL PLANE (BLE beacon; RBID-only per R2-BEACON §6.1 / R2-DISCOVERY §3.2)
    fn advertise(&mut self, beacon: &BeaconAd);                 // S0 advertise own RBID
    fn poll_scan(&mut self) -> Option<BeaconObservation>;       // S0 observed peer (RBID/caps/power)
    fn send_control(&mut self, peer: PeerRef, msg: &ControlMsg);// S1 WifiReq/Offer/Done over L2CAP CoC
    fn poll_control(&mut self) -> Option<(PeerRef, ControlMsg)>;
    // DATA PLANE (WiFi SoftAP/UDP — already exists in this firmware)
    fn bring_up_provider(&mut self, p: &DataPlaneParams) -> bool; // SoftAP (the elected AP)
    fn join_provider(&mut self, p: &DataPlaneParams) -> bool;     // STA join (AP-IP from offer/gateway)
    fn data_plane_state(&self) -> TransportState;                // Available/Failed = disruption-detect
    fn teardown_data_plane(&mut self);
    fn now_ms(&self) -> u64;                                     // T_fallback / T_negotiate deadlines
}
```
- **ControlMsg** = WifiReq / WifiOffer{ssid, psk, ap_hint} / WifiDone — reuses R2-WIFI §3.3/§3.4
  (#wifi_req/offer/done), no new wire format.
- **Engine state** (heap-free): `state: S0..S4`, `roster: PeerTable` (existing), `my_hive`,
  `my_caps` (AP-capable, power_state), `provider: Option<HiveId>`, `failed: excluded set`,
  timer deadlines (now_ms-based). Pure transitions; `poll(&mut self, radio: &mut impl
  NegotiationRadio)` drives it. No alloc.
- **Reuse:** `lowest_live_id` helper (the conductor election primitive) — share, don't duplicate.

## Hive's per-platform radio glue (the (a) layer)
esp-radio `ble` HCI + trouble-host → advertise/scan + L2CAP CoC for control; the existing
esp-radio WiFi SoftAP/UDP for the data plane (wire `data_plane_state` to TransportState
Available/FAILED). AP-IP via gateway discovery (workshop `wifi_sta::get_gateway()` pattern),
NOT hardcoded.

## Hive BLE-stack bring-up plan (the platform layer — fresh focused effort, test pairing first)
Scouted-feasible: esp-radio `ble` feature exposes an HCI interface (`read_hci`/`write_hci`);
pair with **trouble-host** (no_std BLE host, uses `bt-hci` — installed). Steps:
1. **Deps + coex** — add esp-radio `ble` + `coex` features (WiFi+BLE coexist on one radio — the
   init/controller-sharing is the main risk), trouble-host, bt-hci. esp-rtos already schedules.
2. **Controller↔host wiring** — bridge esp-radio HCI (read_hci/write_hci) to trouble-host's
   Controller. Verify on a TEST PAIRING (2 boards), NOT the live 9-board.
3. **ADVERTISE** — R2-BEACON RBID beacon (rbid = HMAC(session_key, epoch)[0:8]; reuse r2-core
   beacon build). First milestone: board advertises, observable by a phone/another board scan.
4. **SCAN** — observe peers → BeaconObservation (caps/power_state). RBID→hive_id via known-peer
   schedule lookup (R2-DISCOVERY §3).
5. **L2CAP CoC** — control channel for WifiReq/Offer/Done.
6. **NegotiationRadio impl** — wire 1–5 + the existing WiFi data plane to the trait; run the
   shared engine (once core lands it). data_plane_state ← WiFi TransportState (Available/FAILED).
RISK: WiFi+BLE coex init + trouble-host↔esp-radio HCI version compat — expect iteration; do it
fresh, on the test pairing, before touching the live mesh.

## BLE-stack scout (concrete, 2026-06-21)
- esp-radio 0.18 HAS the BLE controller (`src/ble/btdm.rs`, BLEController) + HCI (`read_hci`/`write_hci`).
- `bt-hci` 0.8 present (Controller/Transport traits) — the trouble↔controller bridge.
- **trouble-host is NOT yet a dep** — must add (verify version compat vs bt-hci 0.8 + esp-radio 0.18).
- **coex** (WiFi+BLE on one radio) = the real complexity — fw runs WiFi (esp-radio+esp-rtos); BLE needs
  esp-radio `coex` + careful controller init order alongside the live WiFi.
- core READY (e1963b8): engine + ctors (NegObservation::new / NodeCaps::new / DataPlaneParams::new +
  ssid()/psk() / ControlMsg / lowest_live_id / NegotiationEngine::<16>::new); `r2_core::beacon::compute_rbid`
  exists + vector-tested. Gated = the beacon CODEC + power/provider_capable flags (NOT the radio bring-up).
- **Structure: feature-gated `ble`** (optional deps, off by default) so the live 9-board firmware keeps
  building while the BLE path develops on a TEST PAIRING. Order: deps resolve → esp-radio BLE controller
  init (+coex) → bt-hci bridge → trouble-host advertise (RBID via compute_rbid) → scan → L2CAP CoC →
  NegotiationRadio impl over the ready engine. New-stack integration = a focused dive (iterates on
  versions/coex), not a marathon-tail rush.

## BLE bring-up STATUS (2026-06-21) — 4 metal milestones DONE; SCAN is next (fully researched)
DONE on b79010 (--features ble), all metal-verified + externally scan-confirmed:
1. deps resolve+compile (esp-radio ble+coex + bt-hci 0.8.1 + **trouble-host 0.6.0** = the bt-hci-0.8 pin;
   0.2=bt-hci0.3 / 0.7=bt-hci0.9 mismatch). Feature-gated `ble` OFF by default; live fleet still builds.
2. BLE controller inits + WiFi+BLE **COEX holds** (mesh stays synced).
3. trouble-host **ADVERTISE** up + external bluetoothctl confirms `C0:52:2C:AB:5F:69`.
4. **REAL R2-BEACON codec** — `ble_task` uses r2_discovery::beacon::{compute_rbid, encode_advert,
   LegacyBeacon, BeaconFlags, PowerState}; 24-byte canonical payload in 0xFF mfg AD; external scan
   confirms `ManufacturerData 0x01b2` (the encode_advert output). Built vs core's r2-discovery @7b4666e.

### SCAN — the EXACT implementation path (researched, ready to code next session)
trouble-host surfaces adv reports via an **EventHandler**, NOT a return value:
- `impl trouble_host::prelude::EventHandler for MyHandler { fn on_adv_reports(&self, reports: LeAdvReportsIter) {...} }`
  (host.rs:696). For ext-adv use `on_ext_adv_reports` + scan_ext. Each report: addr + `data` (raw AD bytes).
- Drive: `runner.run_with_handler(&handler)` (NOT `run()`) so the handler fires; concurrently
  `Scanner::new(central).scan(&ScanConfig{active:false, interval, window, timeout, phys:PhySet::LE_1M, filter_accept_list:&[]})`
  in a loop (scan() returns a ScanSession per report-batch; call repeatedly).
- In on_adv_reports: walk the AD structures in `report.data`, find the 0xFF manufacturer element, take its
  payload → `r2_discovery::beacon::decode_advert(payload) -> LegacyBeacon` → `resolve_rbid(&beacon.rbid,
  &peers, epoch) -> Option<HiveId>` → build `NegObservation::new(hive_id, ap_capable, beacon.flags.power_state)`.
  Store into a fixed-cap observed-roster (Signal/Mutex<heapless::Vec>) for the engine to poll.
- CONCURRENCY Q to verify on metal: one Host doing BOTH advertise (hold Advertiser) AND scan — likely needs
  `join(runner.run_with_handler(h), advertise_hold, scan_loop)`. If the controller won't do simultaneous
  adv+scan cleanly, alternate (advertise N ms / scan N ms) or split roles for the first test.
- TEST: flash a 2nd DFR1195 with --features ble OFF the live mesh (or a spare); the two observe each other's
  R2-BEACON → decode_advert logs the peer's rbid → resolve_rbid_windowed → hive_id. Then L2CAP CoC →
  NegotiationRadio over the engine.

### RBID session_key + epoch — CANON RESOLVED (core+specs, 2026-06-21; specs blessed the interim)
§6.1's literal "random per-session key" CONTRADICTS §3.3 (resolver derives the peer's key from the trust
relationship); specs ruled §6.1 the outlier. So the RESOLVABLE, canon-aligned model (replace my hk[..16]/
epoch=0 placeholders with this in BOTH advertise + resolve):
- **session_key = `r2_discovery::beacon::derive_beacon_session_key(&hk, hive_id)`** — IMPORT it (shipped
  @9996fa3, no-alloc, no new dep), do NOT hand-roll (drift risk). Construction (for reference): HKDF-SHA256
  **Expand-ONLY** (PRK=hk directly — hk is already a PRK per R2-WIRE §10.3, so `Hkdf::from_prk(hk)`, NOT
  `Hkdf::new(salt, hk)`), **info = b"r2-beacon-rbid-v1" || hive_id_be32**, L=16. **hive_id is MANDATORY** →
  PER-MEMBER distinct RBIDs (a TG-wide key makes all members' RBIDs identical — core's correction). Every TG
  member shares hk (from the join) → can derive ANY peer's key from (hk, peer_hive_id) → resolve; strangers
  (no hk) can't. INTERIM root=hk (R2-KEYSTORE §2 members hold {cert,dek,hk}, not TG_SK); canonical r2-trust
  version + byte vector land on Roy's §6 ruling — already mesh-consistent. ALREADY WIRED in the advertise
  (metal rbid = baf64d9d for hive 2cab5f69).
- **epoch = floor(shared_coarse_time_ms / 900_000)** (T_rotate=900s) from ANY shared time base — heartbeat
  beat_seq when present (do NOT hard-couple; R2-HEARTBEAT is OPTIONAL per specs), else RTC/NTP/GPS.
- **resolve_rbid_windowed(observed, &[(hive_id, session_key) per known peer], epoch, 1)** — the ±1 window
  (R2-DISCOVERY §3.3) absorbs clock skew, so tight time-sync is NOT required; a synced clock just narrows it.
  Since the key is TG-derived-identical, every peer entry uses the SAME derived key.
  Both functions already shipped (core 3420389) — no core change, just call windowed with window=1.
- FIX noted: the local crates index was stale (showed trouble 0.2.4 as max) — `cargo search trouble-host`
  refreshes it; then `cargo update -p trouble-host --precise 0.6.0` pins the bt-hci-0.8 version.

## Params for core
T_fallback (Profile A WiFi/BLE) = 5s (documented per §4A.4(A)). T_negotiate ~10s (R2-WIFI §3.3.1
#wifi_offer timeout). Send back: confirm the module home + the trait names, and whether the
existing PeerTable/BeaconObservation cover the roster needs or the engine needs a thin roster of
its own.

## S1 L2CAP CoC — researched; DESIGN GATE (core lockstep) before implementing
S0 DISCOVER is DONE on metal (advertise+scan+resolve cross-board). S1 = the bidirectional control
channel for WifiReq/Offer/Done. trouble-host pieces located: `L2capChannel` (create/accept) +
`L2capChannelWriter/Reader`; connection via `central.connect(&ConnectConfig)` (central side) +
connectable advertise → `Connection::accept` (peripheral side). It's CONNECTION-ORIENTED — a real
subsystem, not a beacon tweak. Open DESIGN questions for core (the trait/engine side) before I build:
1. **Role model.** Proposal: elected provider (lowest hive_id) = BLE **peripheral** (connectable-advertises
   + accepts L2CAP); joiners = **central** (connect → L2CAP). Does the engine's `send_control(peer, msg)`
   assume ONE connection to the provider, or a connection-per-peer mesh? (Provider-star is simplest + matches
   §4A.3 election.)
2. **Connectable adv vs the RBID beacon.** The RBID beacon is non-connectable (discovery). The CoC needs
   connectable advertising. Provider runs BOTH (non-conn RBID beacon + connectable control adv = 2 adv-sets),
   or switches to connectable during NEGOTIATE? (2 adv-sets = always-discoverable; mode-switch = simpler radio.)
3. **Address-for-connect.** Beacon address rotates (privacy). `central.connect` needs a target address —
   from a separate connectable-adv scan, or carry the connectable addr in/derived-from the resolved identity?
4. **PSM** for the R2 control CoC (pick a dynamic PSM, e.g. 0x0080?).
5. **NegotiationRadio.send_control/poll_control over L2CAP** — exact trait shape: does the radio own the
   HiveId→Connection map + the connection lifecycle (open-on-demand, teardown on S3), and just expose
   send/poll? Confirm so I impl the glue while the engine stays transport-agnostic.
Platform bits I'll own (decide + surface): the 2-adv-sets vs switch, the central/peripheral driver, the
connection map. core owns the trait semantics. Then: NegotiationRadio impl → run the S0–S4 engine →
BLE→WiFi network-forming (S2 = the existing SoftAP/UDP data plane) → disruption→S3→S4→reform.

## S1 DESIGN CLOSED (all-hands 2026-06-21) — workshop's proven L2CAP spec + interop contract
workshop's esp-idf/NimBLE l2cap.rs is the cross-platform reference; esp-radio side MATCHES for interop:
- **PSM = 0x00D2** (R2_PSM), **MTU = 512**.
- **Framing (R2-BLE §6.4):** each SDU = `[len_lo, len_hi, payload...]` — 2-byte **LITTLE-ENDIAN** length prefix
  (NB: LE, unlike R2-WIRE's BE). MAX_FRAME 4096. One ControlMsg per SDU.
- **HiveId↔BLE-addr MAP** (the key bridge): NegotiationRadio is HiveId-addressed; L2CAP is BLE-addr-addressed.
  Populate the map from SCANS — the scan report carries BOTH `rep.addr` AND the resolved hive_id (I already
  resolve RBID→hive_id; just also store rep.addr). Then `send_control(hid,msg)` = map hid→addr → l2cap
  send_to(addr, frame(encode(msg))); `poll_control()` = drain_received→(payload,addr) → map addr→hid →
  (hid, decode(payload)).
- **ControlMsg wire encoding** (mine; ≤101 B, fits MTU 512 → NO fragmentation): `[tag:u8]` where
  tag 0x01=WifiReq (no body), 0x02=WifiOffer + body = `ssid[32] || psk[64] || ap_hint_be32` (100 B), 0x03=WifiDone.
  Wrapped in workshop's `[len_lo,len_hi]` frame. (Confirm byte-compat with workshop before first interop.)
- **Roles:** provider (lowest hive_id) = BLE peripheral (connectable adv + accept CoC); joiners = central
  (connect → CoC). Provider runs 2 adv-sets (non-conn RBID beacon for discovery + connectable for control) OR
  switches during NEGOTIATE — platform decision, leaning 2-sets for always-discoverable.

## Network-forming TELEMETRY (for composer's proof surface) — emit when the engine runs
Extend r2.hb.health: **key13 = forming_phase** (0=discover 1=negotiate 2=form 3=fallback 4=reform, from the
engine S0–S4 state), **key14 = neighbor_count** (resolved peers in roster), **key15 = role** (0=none 1=provider
2=joiner). composer renders the discover→negotiate→form→heal sequence live (alongside the existing key10
transport set B→W). Populated once the NegotiationRadio + engine are wired; static/absent until then.

## Engine: reform-hardening adopted
Bumped r2-discovery to core's 1496916 (Fallback resets exclusions on full candidate-set exhaustion → a node
that disrupts through every provider RECOVERS instead of stranding; test reform_after_all_providers_excluded).
No API change.
