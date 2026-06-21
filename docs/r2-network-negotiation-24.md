# #24 — Network-negotiation protocol (BLE control plane ↔ WiFi data plane, self-healing)

Roadmap item, **after the 9-board cleanup** (now done). SPEC-FIRST: implement FROM the canon
(below), referencing workshop's simpler conforming impl (location pending from supervisor).
**Subsumes #23 / #23b** (AP-failover = phase 4 — BLE survives WiFi loss, so no chicken-and-egg).

## The canonical two-phase model (transport-general)
Per **R2-WIFI §1.1** ("Discover, Then Talk", universal across ALL transports) + **R2-BLE §15
ADR-BLE-001** ("BLE is the control plane, WiFi is the data plane"; validated: WiFi 176× BLE
throughput). BEACON = BLE (short) or LoRa (long); DATA = WiFi (local) or LoRa (long).

## State machine (4 phases)
1. **DISCOVER** — BLE beacon advertise + scan. Declare RBID / hive_id / TG / capabilities /
   supported-transports / **AP-capability** / roster. Canon: **R2-BEACON** (BLE §7, LoRa §8.1),
   **R2-BEACON §3/§3.1** (discovery→connection flow), **R2-DISCOVERY §3** (RBID→hive_id) + **§4.6**
   (beacon discovery API).
2. **NEGOTIATE** — over BLE, agree the data plane: who's AP, SSID/creds, **AP IP**, roster.
   Canon: **R2-BLE §12** (negotiate_transport, flags-based) + **R2-WIFI §3.3/§3.4** (#wifi_req →
   #wifi_offer(creds) → #wifi_done; pseudocode §3.3.1) + **R2-TRANSPORT §2.4** (wire-format
   selection) + **R2-ROUTE §5** (transport selection scoring).
3. **TALK (data plane)** — WiFi SoftAP + UDP R2-WIRE (my existing mesh: routing + conductor-PLL
   heartbeat + trust). Canon: **R2-WIFI §3-4** / **R2-LORA §5** / **R2-TRANSPORT §3** (bindings).
4. **DISRUPTED → FALLBACK → RENEGOTIATE** (self-healing) — a failed data transport
   (**R2-TRANSPORT §2.3** Transport State, incl. FAILED) triggers routing reselection
   (**R2-ROUTE §5.6**: routing MUST respect transport state) → fall back to the always-on BLE
   beacon plane → re-negotiate → re-form WiFi. **This is #23 AP-renegotiation, done right.**

## Firmware impact / prerequisites (no_std, esp-radio)
- **BLE stack** on the S3/C6 — NEW (not yet touched). Need a no_std BLE controller+host
  (esp-radio BLE feature, or trouble/bleps). The biggest lift. Beacon advertise + scan + GATT/
  L2CAP for the WIFI_REQ/OFFER/DONE signalling.
- **R2-BEACON** beacon format (BLE §7) + R2-DISCOVERY RBID→hive_id resolution.
- The WiFi data plane already exists (this firmware) — wire it as the negotiated phase-3 transport.
- **Transport-state fallback** — adopt R2-TRANSPORT TransportState (FAILED) + R2-ROUTE §5.6
  reselection so a dead AP/WiFi triggers the BLE re-negotiate.
- **AP-IP must NOT be hardcoded** (R2-WIFI v0.6 §3.2/§4.3, hw-confirmed workshop+hive): the
  SoftAP IP is stack-dependent (embassy 192.168.4.1, esp-idf 192.168.71.1, …). **CURRENT FIRMWARE
  DIVERGES** — it hardcodes 192.168.4.1 (AP), the collector hive @.1, and the .255 broadcast.
  OK for the all-embassy 9-board (AP=.1), but for #24/interop the AP IP comes from the #wifi_offer
  (and a joining STA uses its DHCP default-gateway). Fix as part of #24.

## Implementation order
1. **BLE↔WiFi (local)** FIRST — reference workshop's simpler impl (location pending) + the canon.
2. **LoRa-as-beacon + LoRa-as-data** follow the SX1262 driver (#22, core leading) — same state
   machine, transport-generalized (BEACON=BLE|LoRa, DATA=WiFi|LoRa).

## Status
Roadmap. The 9-board (WiFi data plane + conductor-PLL + trust + health) is the proven phase-3
substrate. #24 adds the BLE discovery/negotiation control plane around it + the self-healing
fallback. Big fresh effort — needs the BLE stack bring-up + workshop's reference + a test pairing.
