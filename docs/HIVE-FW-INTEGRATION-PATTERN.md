# Hive firmware integration pattern — for the nRF54-LR2021 (embassy-nrf) platform

Proactive support for composer's new-arch platform lead (supervisor directive). The proven DFR1195/SX1262 hive
loop adapted to embassy-nrf + the LR2021. **Target: tuxedo↔pi5 faster-LoRa.** Built greenfield = the *simplified*
(post-Occam) architecture from the start — no PCO to migrate.

## The big win: the data-plane loop is GENERIC over `LoRaRadio` → it ports directly

`r2-lr2021` implements the **same `r2_transport::lora::LoRaRadio` seam** as `r2-sx1262` (per its own docs — same
structure, shared `lora_mtu`/`LoRaConfig`). The hive io_task is written against that seam, so **the loop code is
identical across both chips** — you swap only the radio *construction*, not the loop.
- `Lr2021::new(spi, busy, reset, delay)` (embassy-nrf SPIM + GPIO) instead of the SX1262 ctor.
- Wrap in an RF-switch newtype IF the LR2021 EVK has a TX/RX switch (the DFR uses `RxenRadio` = GPIO HIGH-RX/LOW-TX around `transmit`/`listen`; check the EVK's RF-switch pin in NRF54-LR2021-SCOPE.md).
- **GATING TODO (core's):** `r2-lr2021`'s `LoRaRadio` methods currently return `Lr2021Error::Unimplemented` — the LR2021 SPI command transcription isn't done. The loop will compile + run but the radio is dark until core transcribes. (Flagging core.)

## (1) The io_task data-plane loop structure (the proven hive loop)

One embassy task owns the radio + the routing. Each tick:
```
loop {
    // (a) TX: drain the carrier-agnostic outbound channel -> radio, with loose TX-jitter
    while let Ok((frame, len)) = DATA_TX.try_receive() {
        radio.send(&frame[..len], now_ms);          // LoRaRadio::send; TX-jitter to dodge half-duplex
    }
    // (b) SERVICE: one non-blocking radio service step (continuous-RX + TX-pacing + airtime-gating)
    match radio.service(now_ms) {                    // LoRaRadio::service(now) — the proven SX1262 shape
        RadioEvent::Rx(buf, info) => {
            let msg = decode_extended(buf)?;          // r2-wire (no_std, ports)
            let origin = msg.route[0];                // ROUTE-ORIGIN-1 (A): route_stack[0]; route=None -> drop
            if dedup.seen(origin, msg.header.msg_id) { continue; }   // (origin,msg_id) dedup, transport-agnostic
            let advice = engine.plan_forward(req);    // r2-route (no_std, ports)
            // RELAY: Directed/Flood -> ttl-1 -> DATA_TX (carrier re-broadcast); deliver-gate untouched on relay
            // DELIVER (for_me): §7.5.4 classify_extended_full(msg, group_hmac, &peering) -> SameGroup -> DATA_RX
            //                   + RECEIPT_SIGNAL (the ~400ms event-flash)
        }
        _ => {}
    }
    // (c) neighbour liveness: ingest_observation on received heartbeats (feeds the route engine)
    // (d) keepalive emit (see (2))
    Timer::after(tick).await;                         // embassy time — common to esp-hal-embassy + embassy-nrf
}
```
Carrier-agnostic: the loop talks `DATA_TX`/`DATA_RX` channels; the radio is injected. On nRF54 the carrier set is
**LoRa (r2-lr2021) + BLE** (no ESP-NOW/WiFi) — the channels absorb the different carrier set behind the same loop.
All the routing/dedup/deliver-gate/wire logic is the no_std r2-core crates (r2-route/wire/trust/cbor) — ports as-is.

## (2) The keepalive — greenfield v0.5 (build it RIGHT, no PCO)

The nRF54 is greenfield → implement R2-HEARTBEAT v0.5's kept-liveness directly, **no PCO machinery at all**:
- **Tier-aware, tunable rate (config knob, not a constant):** always-on tier = a deployment-chosen low rate
  (websocket-like, ~tens of s); **duty-cycled SENSOR tier = NO separate keepalive timer — liveness piggybacks
  the sense-wake** (the rate IS the sense period; §1A.1). Event-driven wake = sentinel hierarchy (timer/threshold/
  radio; a flood threshold → immediate wake+TX+alert).
- **Jitter = loose**, reuse the TX-jitter (no new constant).
- **Scopes:** intra-TG member multicast NOW; per-live-entanglement later (needs the entanglement table — same dep
  as §7.5.4 cross-TG peering).
- Emit it as a plain frame on the beat/wake in step (d) — no phase-coupling, no rate-consensus, no spanning-tree
  precondition. This is the *whole* heartbeat on nRF54.

## (3) Peripheral bindings (embassy-nrf)

- **LED P2.00 active-HIGH** — two roles: (canon) **light-now** via the IDENTIFY TG-directive (composer's
  IdentifyBridge: `IDENTIFY <wire> <1|0>` over serial → set P2.00); (test-only) a **brief ~400ms event-flash**
  on message receipt (`RECEIPT_SIGNAL` → an `recv_flash` counter, the TN-FR-1 LED-on-receipt). **Active-HIGH = no
  inversion** (vs the DFR's active-low — just drive HIGH = on). NOT the retired HB-beat flash.
- **OLED I2C** — embassy-nrf TWIM; gate the render like the DFR board-profile flag (one binary, screen-or-not).
- **Grove / SAADC sensor input** — embassy-nrf SAADC for the SENSOR tier: sense-wake → read value → emit a
  `wairoa.reading` (origin in route_stack[0]) → piggyback the keepalive (no separate timer). This is the
  custom-sensor near-reference path (3-stage sleep/wake fits the low-power M33).

## Division of labour
- **composer (new-arch lead):** the embassy-nrf platform layer — HAL init, SPIM/TWIM/SAADC/GPIO bindings, the
  embassy executor, the radio construction + RF-switch, wiring r2-lr2021 into the loop.
- **core:** transcribe the LR2021 SPI commands in r2-lr2021 (the `LoRaRadio` methods, currently `Unimplemented`).
- **hive (me):** this loop/keepalive/peripheral-role pattern; I help map the platform-shim + review the
  data-plane wiring when composer's at the fw layer.
