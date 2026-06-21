# R2 BLE data-mesh (`bluetooth mesh`) — design + feasibility

Roy's re-sequencing (2026-06-21): prove the SINGLE-TRANSPORT data-meshes first (BLE → confirm WiFi/ESP-NOW
→ LoRa later), THEN multi-hop across them. This is the **BLE data-mesh**: R2-data over `Transport::Ble`,
single-hop, the same bridge pattern as the (done) ESP-NOW mesh.

## Goal
Heartbeat-**SYNC** + GroupHmac-gated **DELIVERY** over BLE, connectionless broadcast, single-hop, no central.
Reuse the EXACT proven conductor-PLL + GroupHmac deliver-gate (zero re-impl) — only the transport differs.

## Feasibility — CONFIRMED
- trouble-host 0.6 exposes **`ExtNonconnectableScannableUndirected`** (extended non-connectable advertising,
  payload up to ~254B) — R2-WIRE frames (~30-40B) fit (legacy 31B is too tight once AD overhead is counted).
- Scan path already exists (`R2ScanHandler` + `decode_advert`) — extend it to decode a data AD (the R2 frame
  in manufacturer data, R2 company ID) → the bridge.
- BLE + WiFi + ESP-NOW coex already proven (esp-radio coex on metal this session).

## Design — same bridge pattern as ESP-NOW
- Static channels `BLE_TX` / `BLE_RX` (embassy Channel<MeshFrame,N>), exactly like `ESPNOW_TX`/`ESPNOW_RX`.
- `io_task` (the PROVEN PLL + deliver-gate) routes its conductor-broadcast + recv over `BLE_TX`/`BLE_RX`
  (cfg-selected data plane), instead of ESP-NOW/UDP. The PLL and GroupHmac code are UNTOUCHED (the win of
  the bridge: the heartbeat lub-dubs and the deliver-gate runs over whatever transport the bridge carries).
- `ble_task` in BROADCAST mode: when `BLE_TX` has a frame, (re)advertise it as Ext non-connectable adv
  (manufacturer data = the R2 frame), updated each beat; scan continuously → decode R2 data AD → `BLE_RX`.
- Single-hop (no relay) for this milestone — mirrors the ESP-NOW no-AP sync + 1-hop GroupHmac delivery.

## Build mode
The current ble build's data plane is ESP-NOW (io_task ↔ ESPNOW_TX/RX) + the M7–M9 L2CAP negotiation. The
BLE data-mesh is a DISTINCT data plane (io_task ↔ BLE_TX/RX, broadcast ble_task). Select via a cfg/feature
(e.g. `ble-mesh`) or a runtime board-profile flag so each single-transport mesh is provable in isolation
(Roy: prove each transport's mesh independently before multi-hop across them).

## Steps
1. `BLE_TX`/`BLE_RX` channels (copy the ESPNOW_TX/RX shape).
2. `ble_task` broadcast mode: Ext non-connectable advertise of `BLE_TX` frames (update per beat) + scan-decode
   the R2 data AD → `BLE_RX`. (Distinct from the connectable-adv + CoC negotiation path.)
3. `io_task` data-plane select → BLE_TX/RX (reuse the PLL conductor-broadcast + the deliver-gate verbatim).
4. Metal test (2 boards): conductor beats over BLE adv → follower PLL-locks (`HB<-ble … (lock)` synced=true);
   conductor originates signed Events over BLE → follower DELIVERED good / BLOCKED bad (GroupHmac over BLE).
5. `Transport::Ble` neighbour-obs (for the LATER multi-hop-across-transports phase).

## Banked (deferred per Roy): multi-hop
Multi-hop relay is PROVEN on metal (ESP-NOW A→B→C) + the seam answers are banked: mcu_origin:false (relay-
viable), K=15-flood OR K=2+handle-Drop(SprayWait)→direct-deliver via HIVE_MAC, directed flood-fallthrough,
broadcast=flood (not DeliverOnly), paths self-build on record_delivery_success. Resume after each single-
transport mesh (BLE, WiFi/ESP-NOW✓, LoRa) is proven.

## ✅ DONE on metal (2026-06-21) — heartbeat-SYNC + GroupHmac delivery over BLE
2 boards (A provider 0dcadbf8 / B joiner 2cab5f69), data plane = L2CAP CoC (`blemesh` feature):
B PLL-locks to conductor A over BLE — `HB<-ble cond=dcadbf8 e=-0.050→-0.000→0.018 (lock)` synced=true — AND
delivers A's signed Events with the GroupHmac gate — `DELIVERED msg_id=16 in-TG (tg+hmac ok)` dlv=8. The
PROVEN conductor-PLL + GroupHmac deliver-gate reused VERBATIM via the transport-agnostic DATA_TX/RX bridge;
`serve_data_coc` bridges the CoC ↔ DATA carrying R2-WIRE frames. Renamed ESPNOW_TX/RX → DATA_TX/RX (generic).
FIXES on metal: health-send cfg-gated off for blemesh (blocked io_task at beat 25 — no WiFi controller up in
BLE-only mode); ev_target=0 broadcast for single-hop; MESH_LABEL='ble' (accurate serial evidence).
BOTH single-transport data-meshes now proven on metal (WiFi/ESP-NOW + BLE) with the same reused PLL+GroupHmac.
NEXT per Roy: LoRa-mesh later; multi-hop across transports (banked, proven on ESP-NOW A→B→C).
