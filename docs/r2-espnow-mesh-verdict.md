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
