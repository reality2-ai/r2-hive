# R2 ESP-NOW mesh interop spec (join the leaderless 2-TG heartbeat mesh)

For a non-hive platform (e.g. workshop's esp-idf board) to JOIN the live 9-board
leaderless ESP-NOW heartbeat mesh **byte-correctly**. Source of truth = hive's
`nobt`/`multitg` firmware (`platforms/dfr1195/src/main.rs`) + r2-core's `r2-wire` +
`r2-trust`. Proven on metal 2026-06-22 (cross-host alfred+tuxedo, 2 TGs, clean GroupHmac partition).

## 0. The shortcut — DON'T re-implement the wire format
`r2-wire` and `r2-trust` are `no_std` + portable. **Use them directly** and you are
byte-correct by construction:
- `r2_wire::encode_extended(&ExtendedMessage, &mut buf) -> usize`
- `r2_wire::decode_extended(&[u8]) -> ExtendedMessage`
- `r2_trust::GroupHmac::new([u8;32])`, `r2_wire::{sign_extended, verify_extended}`
Then the ONLY new code is the ESP-NOW transport (esp-idf `esp_now` ⇄ a byte buffer).
That is the one-codebase north-star: shared wire/trust, thin per-platform radio.

## 1. ESP-NOW transport layer
- **Channel: 1, FIXED.** No AP/STA join — set the channel directly
  (`manager.set_channel(1)` on esp-radio; `esp_now_*` + `esp_wifi_set_channel(1, ...)` on esp-idf).
  WiFi in STA mode, NOT associated. Every mesh node MUST be on ch1.
- **Address: broadcast `FF:FF:FF:FF:FF:FF`.** Send + receive on broadcast.
  No ESP-NOW encryption, no LMK/PMK (the R2-WIRE GroupHmac is the auth layer).
  For broadcast RX you don't need to pre-register unicast peers; on esp-idf add the
  broadcast peer to the peer list (channel 1, ifidx STA, encrypt=false) to TX.
- **One R2-WIRE frame per ESP-NOW packet.** No fragmentation — a Heartbeat is 62 bytes
  (well under the 250-byte ESP-NOW payload limit). The ESP-NOW payload IS the encoded
  R2-WIRE extended frame, nothing wrapping it.

## 2. R2-WIRE extended frame = the ESP-NOW payload
Fixed 22-byte header, big-endian, then optional route, payload, optional 32-byte HMAC:
```
byte[0]      = (version<<6) | (msg_type<<3) | flags    // flags = (has_route<<2)|(has_hmac<<1)|mcu_origin
byte[1]      = (ttl<<4) | (k & 0x0F)
byte[2..6]   = msg_id        (BE u32)   // mutable, NOT in the HMAC span
byte[6..10]  = event_hash    (BE u32)
byte[10..14] = payload_len   (BE u32)
byte[14..18] = target_group  (BE u32)
byte[18..22] = target_hive   (BE u32)
[ if has_route: byte[22]=route_len, then route_len × BE u32 ]
payload      (payload_len bytes)
[ if has_hmac: 32-byte HMAC-SHA256 tag, appended last ]
```
- `version = 0`. `MsgType`: **Event = 0, Heartbeat = 5**.
- Extended HMAC tag = **full 32 bytes** (compact format truncates to 8; extended does NOT).

## 3. Heartbeat frame (the sync pulse you emit + couple on)
- version=0, msg_type=**Heartbeat(5)**, flags: `mcu_origin=1, has_hmac=1` (signed), has_route=0
- ttl=1, k=1
- msg_id = your fire sequence (BE u32, ++ per fire; unauthenticated)
- event_hash = 0
- payload_len = 8
- **target_group = the TG id = `fnv1a_32(TG_UUID)` as u32** (live TGs: TG-A=177560432, TG-B=1584099016)
- target_hive = 0 (broadcast within TG)
- payload[0..4] = your hive_id (BE u32); payload[4..8] = your VERSION_FNV (BE u32)
- hmac_tag = 32-byte HMAC-SHA256 (see §4)

## 4. The GroupHmac partition (THIS is what gates coupling)
- **HMAC-SHA256, key = the TG's 32-byte GroupHmac key** (runtime-provisioned per board).
- **MAC span (extended) — exact bytes, in order:**
  ```
  msg_type(1 byte) || event_hash(4 BE) || target_group(4 BE) || target_hive(4 BE) || payload(N)
  ```
  Note it covers the SEMANTIC fields only — NOT byte0/byte1/msg_id/route (the mutable envelope).
- Tag = full 32-byte HMAC output appended at frame end.
- **Coupling gate:** a node couples to a heartbeat ONLY if `verify_extended` passes (recompute
  HMAC over the span with the node's TG key == appended tag). Wrong/absent key → no couple.
  Two TG keys on one shared mesh = two non-coupling clusters = the proven cross-TG isolation.
- To join a TG: hold that TG's 32-byte key, sign your HBs with it, and you'll couple with its members.

## 5. Leaderless coupling behaviour (R2-HEARTBEAT v0.4 §4.1)
- No conductor/election. Every node runs a Mirollo–Strogatz pulse-coupled oscillator: at phase≥1
  it FIRES (broadcast a signed Heartbeat) and resets. On every GroupHmac-VERIFIED heard pulse it
  nudges its phase toward the sender (flat concave coupling) + runs the §4.3 distributed-β rate
  consensus. Same-TG nodes converge to a common phase; spread_ms → 0–few ms.
- hive_id: for consistent identity/dedup use the KS1 canon `derive_hive_id` (HKDF→v4-UUID-string→FNV);
  for basic coupling any unique u32 works.

## 6b. Sender identity: MAC ⇄ hive_id resolution
- **hive_id is carried EXPLICITLY in the Heartbeat payload** (`payload[0..4]`, BE u32) — it is NOT
  derived from the MAC. So `from_hive_id` = the received frame's `payload[0..4]`, full stop.
- The ESP-NOW recv gives the L2 src MAC (`r.info.src_address`). You LEARN the `(hive_id ↔ MAC)`
  mapping by observing Heartbeats: on each recv, `map_peer(payload_hive_id, recv_MAC)`. That table is
  ONLY for UNICAST (r2-route `DirectedHop.transport=EspNow` → look up the target hive's MAC). Broadcast
  HB/coupling needs no lookup — identity is in the frame.
- hive_id derivation (for your own payload): KS1 canon `derive_hive_id` (HKDF→v4-UUID-string→FNV) for
  byte-exact identity with the fleet; any unique u32 works for basic coupling.

## 6c. The PCO/sync engine — ONE-CODEBASE (decision: option A)
The §4 pulse-coupled-oscillator dynamics (the convergence crux) should be a **portable no_std engine
that BOTH hive firmware and other platforms reuse** — like `NegotiationEngine` for the control plane.
Core extracts it from the r2-harness sim (`leaderless.rs`) into a heap-free `f32` struct (no Vec/closures).
Convergence-by-construction; each platform = thin layer (EspNowTransport + run-loop driving the engine).
**Engine contract (mirrors NegotiationEngine):**
- `tick(now_ms) -> Option<Fire>`: advance `phase += rate * dt`; if `phase >= 1.0` → fire, `phase -= 1.0`
  (preserve overshoot), return Fire (caller broadcasts a signed Heartbeat).
- `on_verified_pulse(now_ms)`: called ONLY for a GroupHmac-VERIFIED heard pulse — applies the phase kick
  + rate consensus.
- accessors: `phase()`, `rate()`, `spread_ms()`.

**Exact §4 params + math (hive firmware canon, R2-HEARTBEAT v0.4):**
- `HB_PERIOD_MS = 2000.0`, `CANON_PERIOD_MS = 2000.0` (rate-clamp center), `HB_TICK_MS = 50`.
- `rate` init = `1.0/HB_PERIOD_MS` (cycles/ms); advance per tick: `phase += rate * HB_TICK_MS`.
- Fire: `phase >= 1.0` → fire, `phase -= 1.0`.
- §4.1 phase coupling, `K_PHI = 0.25`, uniform link weight `w = 1.0` (ESP-NOW single-hop):
  - error `e = if phase <= 0.5 { -phase } else { 1.0 - phase }`  (= wrap(0 − phase) ∈ [−0.5, 0.5))
  - kick: `phase += K_PHI * w * phase_response(e)`; then wrap phase into [0,1).
  - `phase_response(e)` = concave Mirollo–Strogatz:
    ```
    m = min(2*|e|, 1)
    resp = ln(1 + 19.085537 * m) / 3      // 19.085537 = e^3 − 1 (b=3); ln via libm::logf
    return 0.5 * resp * sign(e)
    ```
- §4.3 rate consensus, `RATE_BETA = 0.01` (0.0 = refuted control): on a verified pulse, let `interval`
  = ms since last heard pulse from anyone; if `0.5*period < interval < 3.0*period`:
  `r_src = 1/interval; rate += RATE_BETA * (w/sum_w) * (r_src - rate)`; then CLAMP rate to
  `[nom*0.99, nom*1.01]` where `nom = 1/CANON_PERIOD_MS`.
- §4.2 reachback delay-comp: airtime ≈ 0 for single-hop ESP-NOW (transit ≪ period) → delay-comp ≈ 0
  here; the reachback term (frame tx_time) matters for LoRa/multi-hop, not ESP-NOW.
- `spread_ms` metric = `|e| * HB_PERIOD_MS` (the leaderless convergence-tightness telemetry).

## 6. Provisioning (how a board gets its TG key)
Out of band, point-to-point over the board's OWN USB serial (the secret key never goes on the air):
`PROVISION <wire_hex> <tg_id_dec> <grouphmac_key_64hex>` → parse with
`r2_trust::provision::parse_provision` → persist → install into the GroupHmac. See
composer `specifications/CROSS-HOST-2TG.md` §6. For a bench bring-up you can flash-bake one key.
