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

## Params for core
T_fallback (Profile A WiFi/BLE) = 5s (documented per §4A.4(A)). T_negotiate ~10s (R2-WIFI §3.3.1
#wifi_offer timeout). Send back: confirm the module home + the trait names, and whether the
existing PeerTable/BeaconObservation cover the roster needs or the engine needs a thin roster of
its own.
