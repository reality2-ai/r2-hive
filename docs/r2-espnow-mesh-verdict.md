# ESP-NOW true-mesh — feasibility VERDICT (hive, 2026-06-21)

## ⚠ CORRECTION (core, 2026-06-21) — (A) star-over-ESP-NOW vs (B) TRUE mesh are DIFFERENT code paths
My "ESP-NOW reuses S0–M9 unchanged" conflated two things. specs ruled Mode 2 true-mesh has NO provider election:
- **(A) STAR-over-ESP-NOW (Mode 1b transport-swap):** the #24 engine DOES reuse (election picks ONE provider;
  bring_up/join_provider swap SoftAP→ESP-NOW peering, no trait change). BUT it's STILL A STAR — the elected
  provider is a SPOF; it moving out of range = full reform. Does NOT solve the mobile AP-SPOF.
- **(B) TRUE ESP-NOW MESH (Mode 2 — the mobile general case Roy wants, THE TARGET):** NO provider, NO election,
  NO bring_up/join_provider. Every device enables ESP-NOW + relays peer-to-peer via **R2-ROUTE** (multi-hop +
  dedup + TTL + decay + flood). This is **r2-route territory, NOT the #24 negotiation engine.**
  REUSES from the proven stack: the BLE **BEACON/S0 discovery**, **RBID/resolve** (peer identity + WiFi MAC),
  the **conductor-PLL heartbeat-sync** (over ESP-NOW broadcast), **R2-ROUTE** relay (proven on the 9-board mesh),
  **GroupHmac** per-TG delivery. Does NOT use the provider election or the WifiReq/Offer/join handshake (no
  provider). The NegotiationRadio trait's bring_up/join_provider are N/A for Mode 2.
- **M7/M8/M9 (L2CAP CoC + negotiation engine + SoftAP form) = the STEPPING-STONE** (proved the BLE control
  plane + the form→sync logic on metal). The TRUE-mesh demo doesn't run the provider election for forming.
- **TARGET (Roy/supervisor): rebuild the demo on (B)** — discover → enable ESP-NOW mesh (no AP) → R2-ROUTE relay
  → heartbeat-SYNC. Mobility-native, no SPOF, no two-AP. Infra-SoftAP (Mode 1b) kept LIGHT for fixed/workshop.
- SEQUENCE: specs canon (Mode 2 / reality2-mesh) → core **Transport::EspNow** → hive rebuilds the demo on it.

---


**VERDICT: FEASIBLE + STRONGLY FAVORED** for the mobile-wearable true-mesh data plane. It SUPERSEDES the
SoftAP-star for the general (mobile) case; keep SoftAP-join for the INFRASTRUCTURE case. R2-ROUTE selects.

## Feasibility (esp-radio 0.18, no_std — confirmed in-tree)
- `esp-radio` has the **`esp-now` feature** + `mod esp_now`: `EspNow` / `EspNowManager` (`add_peer(PeerInfo)`)
  / `EspNowSender` (`send(dst:&[u8;6], data)`) / `EspNowReceiver` (`receive()`), `BROADCAST_ADDRESS=[0xff;6]`,
  and **async** `send_async`/`SendFuture` + `receive_async`/`ReceiveFuture` (embassy-compatible).
- **Connectionless**: direct peer-to-peer frames by MAC (+ broadcast). NO AP, NO STA association, NO IP/DHCP.
- **Coex**: uses the WiFi radio without association → coexists with the always-on BLE control plane (esp-radio/coex).

## Why it beats SoftAP-star for MOBILE wearables (continual reform = normal mode)
1. **No AP → no star to break when a node moves out of range.** ESP-NOW is symmetric P2P; the mobility failure
   mode of SoftAP (the AP-wearable leaving kills the net) VANISHES.
2. **The two-same-IP / AP-role bug is MOOT** — no AP, no IP assignment, no boot-AP-vs-election divergence.
3. **The hard M10 part (runtime AP-bring-up for a re-elected provider) VANISHES** — reform = RE-ROUTE (R2-ROUTE)
   as nodes move, not AP-failover. Fast + robust under constant churn.
4. **The conductor-send-stall (SoftAP-no-STA) is MOOT** — ESP-NOW broadcast needs no associated STA. FORM→SYNC
   over ESP-NOW broadcast is simpler.

## What REUSES (only the data-plane transport changes)
- **S0/M7/M8/M9** — the BLE control plane (discover + negotiate) is unchanged; it remains how peers find each
  other + agree to use the mesh (and exchange WiFi MACs for ESP-NOW unicast).
- **Heartbeat-sync** (conductor-PLL) — over ESP-NOW broadcast instead of UDP-broadcast.
- **R2-ROUTE** — multi-hop mesh routing over ESP-NOW (P2P-by-MAC + the RouteEngine for multi-hop/dedup/TTL).
- **Trust layer** (GroupHmac delivery) — per-TG over the shared ESP-NOW mesh (forming TG-agnostic, deliver TG-scoped).
- The whole negotiation/election/discovery investment carries; **only the SoftAP/UDP data-plane is swapped.**

## Dual-mode data plane (Roy's refinement) — R2-ROUTE transport SELECTION
- **INFRASTRUCTURE**: a fixed AP (e.g. the WORKSHOP computer) — devices JOIN it (keep the SoftAP-STA-join path).
- **MESH (general/mobile)**: BLE + **ESP-NOW** (+ LoRa later) true-mesh, no fixed infra = the reality2 mesh.
- The negotiation / **R2-ROUTE selects**: prefer infra-AP when available/preferable; else form the ESP-NOW mesh.
  Do NOT drop infra-AP; ADD ESP-NOW-mesh as the general case.

## Integration plan (the pivot)
- ADD an **ESP-NOW data-plane transport**: EspNow init (coex with BLE) → add_peer(discovered peers' WiFi MACs)
  → async send/receive → wire to R2-ROUTE (data) + the heartbeat (broadcast). MAC exchange: broadcast needs no
  MAC; unicast peer MAC comes from the negotiation (carry the WiFi MAC in the WifiOffer / a beacon field).
- NegotiationRadio data-plane for mesh: "bring_up/join" = enable ESP-NOW + add the peer MACs (no AP/IP);
  data_plane_state = ESP-NOW ready. KEEP the SoftAP-join impl for infra mode.
- **DROP the deep SoftAP AP-role investment** (per supervisor — moot for mesh). Keep the role-align (harmless).
- Cross-TG (core's ruling): election is within-TG (hive_id); cross-TG = JOIN (provider_capable flag readable
  without resolve + the below-L5 control plane) — a platform association path beside the engine (no engine change).

## ESP-NOW true-mesh REALIZATION PLAN (what hive builds when core lands Transport::EspNow)
Sequence: specs reality2-mesh canon → core Transport::EspNow (+ prefer-infra StrategyVector preset) → hive realizes.
1. **ESP-NOW init** — esp-radio EspNow on the STA interface (PeerInfo.interface = Station); coexists with WiFi +
   the BLE control plane (esp-radio/coex). No AP, no association for the data plane.
2. **hive_id↔MAC peer-map (LEARN from recv-src — core's design call, privacy-preserving):** recv ESP-NOW
   broadcast → (src_MAC, payload) → decode_advert(payload) → resolve_rbid → hive_id → map[hive_id]=src_MAC.
   NO MAC in the beacon (would re-leak a trackable id vs the rotating RBID). Map lives in hive's Transport impl
   (platform), exactly like the BLE HiveId↔addr map.
3. **Transport::EspNow trait impl (hive):** enable + the peer-map + send (unicast→map[hive_id] / broadcast) +
   recv → feed r2-route. core gives the Transport routing-tag/metadata + r2-route forwarding-by-hive_id.
4. **r2-route relay over ESP-NOW** — "forward to hive_id N" → map[N] → ESP-NOW unicast; multi-hop/dedup/TTL/decay
   (the RouteEngine, already proven on the 9-board). NO provider election (Mode 2 = true mesh).
5. **heartbeat-SYNC over ESP-NOW broadcast** — reuse the conductor-PLL (the FORM→SYNC work; broadcast addr, no MAC).
6. **GroupHmac per-TG delivery** — reuse (forming TG-agnostic, deliver TG-scoped).
7. **BLE S0 discovery** feeds peer identities (beacon/scan/resolve) — the only #24 piece the true mesh reuses
   for discovery; NO M7-M9 provider-election/WifiReq-Offer-join (no provider).
DEMO TARGET: 2+ boards → BLE-discover → enable ESP-NOW mesh (no AP) → R2-ROUTE relay + heartbeat-SYNC → mobile,
no SPOF, no two-AP. Infra-SoftAP (Mode 1b, criterion#1 PROVEN on metal) kept LIGHT for fixed/workshop.

## ESP-NOW mesh demo — BUILD STATUS (2026-06-21)
- **M-ESPNOW-1 DONE on metal:** ESP-NOW true-mesh FORMS — board A recvs board B's connectionless broadcast
  (`ESP-NOW RECV peer_hive=2cab5f69 src=f4:12:fa:b7:90:10`), no AP, src-MAC captured, coex w/ WiFi+BLE.
- **Canon:** Transport::EspNow = **id 5 / 0x20** (R2-TRANSPORT §2.2; USB owns 4) — core re-landing at id 5.
  Affects core's transport tag, NOT my esp-radio ESP-NOW mechanics; I wire r2-route against id 5.
- **NEXT — sync over ESP-NOW (the heartbeat):** route io_task's PROVEN conductor-PLL over ESP-NOW via a BRIDGE
  (don't re-impl the PLL; don't risk the proven io_task). Static embassy Channels ESPNOW_TX/ESPNOW_RX between
  io_task (PLL) + espnow_task (ESP-NOW): io_task conductor-broadcast → ESPNOW_TX.send → espnow_task ESP-NOW
  broadcast; espnow recv → ESPNOW_RX.send → io_task recv-select reads it (into scratch → existing decode/PLL).
  cfg-gated: `#[cfg(feature="ble")]` uses the bridge, default uses the UDP socket UNCHANGED (infra-mode safe).
  The heartbeat frame carries the originator hive_id → on recv, map[hive_id]=src_MAC (M-ESPNOW-2 peer-map for
  unicast). Reuses the exact PLL (phase/fire/lock/conductor-timeout) — zero PLL re-impl risk.
- **M-ESPNOW-2:** the recv frame's originator hive_id + src_MAC → hive_id↔MAC peer-map (in espnow_task).
- **M-ESPNOW-3:** feed r2-route neighbour-observations (hive_id reachable on Transport::EspNow + RSSI link-q if
  esp-radio ReceiveInfo exposes it) + r2-route forward-by-hive_id → map[hive_id] → ESP-NOW unicast; GroupHmac
  per-TG delivery above. Then the demo: discover → ESP-NOW mesh → SYNC, no AP, mobile.
- WHY the bridge (not an io_task transport-swap or a PLL re-impl): the io_task PLL is PROVEN (criterion#1 on
  metal); a bridge reuses it intact + cfg-keeps the infra-mode UDP path untouched. The recv-select restructure
  is the one careful part — do it deliberately, not rushed.

## ✅ NO-AP MESH SYNC — DONE ON METAL (2026-06-21)
The demo target HIT: 2 boards, NO AP, discover(BLE) → ESP-NOW mesh → SYNC over ESP-NOW broadcast.
Follower (2cab5f69): `HB<-esp-now cond=dcadbf8 e=0.249→0.057→-0.084 (lock)` + `synced=true`. The conductor
(0dcadbf8) broadcasts beats over ESP-NOW; the follower PLL-locks over ESP-NOW (e→0). The PROVEN conductor-PLL
reused INTACT via the bridge (io_task↔espnow_task static channels, cfg-gated; default UDP path untouched) —
zero re-impl. Fixes that made it sync: (1) NO AP for the mesh (serve_ap=false ble; M8c SoftAP-star dropped —
it forced ch6 + diverged the radios; AP-SPOF gone); (2) EspNow.set_channel(1) — all mesh nodes align without
an AP-join. RSSI absent in esp-radio ReceiveInfo → seed link_quality 0.7 (M-ESPNOW-3). Mobility-native, no SPOF.
REMAINING for the routed-data mesh: M-ESPNOW-2 (hive↔MAC map from recv src) + M-ESPNOW-3 (r2-route
forward-by-hive_id over Transport::EspNow id5 + GroupHmac per-TG delivery). The SYNC half is proven on metal.
