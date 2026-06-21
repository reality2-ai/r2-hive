# #24 — Network-negotiation protocol (two-plane, self-healing)

Roadmap item, after the 9-board cleanup (done). **CANON = R2-DISCOVERY v0.2 §4A** (baa0a94) — the
consolidated two-plane state machine; implement FROM it (not re-derived). Reuses the existing
R2-WIFI §3.3/§3.4 handoff (#wifi_req/#wifi_offer/#wifi_done) — **no new wire format**. Workshop's
simpler impl + building-blocks are the reference. **Subsumes #23/#23b** (AP-failover = S3→S4→S1).

## Two-plane model (R2-WIFI §1.1 + R2-BLE ADR-BLE-001)
- **Control plane (beacon)** — presence/discovery/negotiation-signalling/fallback-rendezvous;
  always-on, low-power. BLE beacon (R2-BEACON §7) or LoRa beacon (§8.1). **MUST stay active while
  the data plane is up** (reduced duty OK per R2-BEACON §7.5, but never silenced).
- **Data plane (negotiated)** — bulk event/payload (R2-WIRE). WiFi SoftAP/UDP (my existing mesh)
  or LoRa-long. Established on demand over the control plane; may be lost without losing control.

## State machine (R2-DISCOVERY §4A — canonical S0–S4)
- **S0 DISCOVER** — advertise+scan control plane (§4.6); resolve peers (§3). → S1 when peer(s)
  found + a data-plane need arises.
- **S1 NEGOTIATE** — agree the data-plane transport+params over the control plane; for a
  shared-medium plane, ELECT the provider (§4A.3). WiFi = R2-WIFI §3.3/§3.4 handoff. → S2 on
  established; → S0 on fail/timeout.
- **S2 DATA** — data plane up; R2-WIRE flows; control plane at reduced duty. → S3 on disruption;
  → S0 on graceful teardown (#wifi_done).
- **S3 DISRUPTED** — data plane lost/degraded. Triggers: assoc/link loss; provider control-plane
  **silence > T_fallback**; provider advertises power_state Critical/Survival (R2-BEACON §7.2.1);
  data-plane address unreachable (R2-WIFI §4.3). → S4.
- **S4 FALLBACK + RENEGOTIATE** — return to the control plane; re-enter NEGOTIATE excluding the
  failed provider/transport. → S2.
- **Self-healing loop = S2→S3→S4→S1→S2.** Control plane never went down → recovery is automatic.

## §4A.4 conformance (confirmed to specs)
1. **Control plane active while data plane up** — YES (BLE beacon always-on underneath; reduced
   duty, never silenced).
2. **Disruption→beacon-fallback→renegotiate (S2→S3→S4→S1→S2)** — YES (the self-healing loop).
3. **Shared-medium provider election (§4A.3) = lowest eligible hive_id (AP-capable + power_state
   Normal/Eco) + silence-failover** — YES, and it directly REUSES my proven mechanisms: the
   conductor election (lowest hive_id) + canonical derive_hive_id + the conductor-TIMEOUT
   (silence-failover, already built #23a). Add the AP-capable + power_state Normal/Eco eligibility
   filter + exclude Critical/Survival.
4. **Documented T_fallback + triggers** — adopt §4A triggers (assoc/link loss; silence>T_fallback;
   power_state Critical/Survival; addr unreachable). **T_fallback (Profile A, WiFi/BLE) = provisional
   5 s** control-plane silence (≈ a few BLE beacon intervals). The FAST path is the immediate
   WiFi-disassociation trigger; T_fallback only covers *silent* degradation (no disassoc event).
   §4A leaves it transport-profiled/unpinned — this is the documented Profile A value per §4A.4(A),
   to be pinned alongside the #21/#22 LoRa profile (LoRa T_fallback will be much larger).

## BEACON PRIVACY (correction — my earlier note was wrong)
The beacon carries **RBID only** — `RBID = HMAC-SHA256(session_key, epoch_counter)[0:8]`
(R2-BEACON §6.1), rotating + privacy-preserving. **NO hive_id / TG / roster in the beacon**
(R2-DISCOVERY §3.2: hive_id MUST NOT be derivable from RBID). RBID→hive_id is a LOOKUP against
known peers' precomputed RBID schedules (needs their session_key); unknown advertisers stay
unknown until **post-connect + auth**. TG-recognition is post-connect+auth, not in the beacon.

## NET-NEW orchestration (the gap — build on workshop's building-blocks)
Workshop has the pieces (beacon / l2cap / wifi_prov / connect_static / wifi_ap + boot-fallback +
docs/BLE-WIFI-NEGOTIATION.md). My net-new no_std orchestration:
1. **Peer-AP election over BLE** (= §4A.3) — lowest eligible hive_id; reuse the conductor logic +
   derive_hive_id + the silence-failover (#23a).
2. **Roster + PSK generated + distributed peer-to-peer over L2CAP** (wifi_prov codec, peer-sourced
   — no central provisioner).
3. **Runtime WiFi-health → BLE-renegotiate closed loop** (S2→S3 triggers → S4 → S1).

## Firmware prereqs
- **no_std BLE stack** (the big lift) — beacon advertise/scan + L2CAP CoC for signalling. New.
- R2-BEACON (RBID schedule) + R2-DISCOVERY §3 RBID↔hive_id lookup.
- Transport-state fallback: R2-TRANSPORT §2.3 (FAILED) + R2-ROUTE §5.6 reselect.
- **FIX hardcoded AP IP** (R2-WIFI v0.6 §3.2/§4.3): current fw hardcodes 192.168.4.1 — for #24 the
  AP IP comes from the #wifi_offer (joining STA uses its DHCP gateway). OK for the all-embassy
  9-board now; fix for interop.

## Order
BLE↔WiFi (local) FIRST (workshop ref + canon) → then LoRa-as-beacon + LoRa-as-data per #22 (core's
SX1262 driver). Same state machine, transport-generalized.
