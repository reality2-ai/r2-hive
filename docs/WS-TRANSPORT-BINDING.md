# #26 — r2-hive-wasm real-socket transports (WS + UDP): design proposal

Status: **PROPOSAL / spec-first** (awaiting core's transport-seam confirmation). Author: hive.
Spec anchor: R2-TRANSPORT v0.16 §2.7 Transport Profile (specs `bcb1a37`). Task #26.

## Goal
Today r2-hive-wasm routes over an **in-process virtual-mesh** bearer (composer's `router.js` relays
`route_frame` sends between in-memory hive instances). #26 makes it the **production no-radio hive**:
multiple wasm hives (browser, over **WebSocket**) + host processes (over **UDP**, first-class L1) mesh over
**real sockets**. Then the topological isomorphism becomes a real network.

Division (supervisor + core scoping): **core** leads the host-UDP native binding (`ConnectionlessRadio` over
`UdpSocket`, d0f1864). **hive** owns the **WASM-WS** binding + wiring both into r2-hive-wasm's route in/out +
the §2.7 physics exports.

## §2.7 Transport Profile (the shared, carrier-independent param-set)
Gathers EXISTING canonical per-transport params (only `range→loss` is new/provisional):

| field | meaning | source | status |
|---|---|---|---|
| `max_payload` (MTU) | max R2-WIRE frame | §2.2 / R2-LORA §5.2 | existing |
| `power_cost` | §5.2 selection-score denominator | §2.2 | existing (`TransportId::default_power_cost`) |
| `wire_format` | Compact / Extended | §2.2 / §2.4 | existing |
| `rssi→quality` | RSSI→[0,1], −50→1.0 / −80→0.0 | §2.5 / R2-ROUTE §2.6 | **exported** `quality_from_rssi` (v0.4.7) |
| `decay_rate` (λ) | per-transport staleness rate | R2-ROUTE §2.4 | existing (guard `LoRa.λ<WiFi.λ<BLE.λ`) |
| `jitter` (min,max ms) | keepalive/emit de-correlation | R2-HEARTBEAT §1A.2 | existing |
| `staleness_timeout` | silence horizon | **DERIVED** `-ln(min_conf)/λ` | derived (not stored) |
| `range→loss` | distance→synthetic RSSI atten. | radio-sim | **PROVISIONAL** — exported `range_to_loss` (v0.4.7), caller-supplied steepness |

**Q for core:** the shared `TransportProfile` struct should be single-sourced — propose it lands in
`r2-transport` (core owns) so both the host-UDP binding and r2-hive-wasm import it (no fork). For a REAL
WS/UDP socket the profile is **routing metadata** (power_cost/MTU/λ feed transport-aware `best_transport`);
loss/jitter/range are NOT simulated on a reliable socket — they matter for the radio-SIM path only.

## WS binding — two candidate architectures

### A. Rust `web_sys::WebSocket` `WsRadio` (symmetric with core's host `UdpRadio`)
A `ConnectionlessRadio`/transport impl INSIDE r2-hive-wasm holding a `web_sys::WebSocket`; the SAME trait
core's host-UDP binding impls → one abstraction, two bindings.
- **+** true symmetry (one seam, UDP + WS both `ConnectionlessRadio`); the "two bindings of one profile" ideal.
- **−** `web_sys::WebSocket` is **async/event-driven**, but the route core is **sync** (`route_inbound_sync`).
  Needs an async↔sync bridge: inbound WS `onmessage` must land in a queue the sync `route_frame` drains;
  outbound `send()` is fire-and-forget (OK sync). Workable (a `RefCell<VecDeque>` inbound queue + JS event
  wiring) but adds `web-sys`/`js-sys` deps + wasm-only event plumbing.

### B. JS-carried (host owns the WS; wasm stays pure sync route in/out)
JS holds the `WebSocket`; `onmessage` → `hive.route_frame(bytes)`; the returned `sends[]` → `ws.send()`.
Matches the CURRENT model (router.js already drives route_frame + reads sends) — the "in-process mesh" just
becomes "over a WS gateway."
- **+** zero new wasm deps; browser-idiomatic; minimal change (route_frame-in/sends-out is already
  transport-agnostic); ships fastest.
- **−** asymmetric with core's Rust UDP binding (the "binding" for WS lives in JS glue + a gateway, not a
  Rust trait). The profile still rides as metadata.

**hive recommendation:** **B for the browser** (the wasm→sync boundary makes A's async bridge friction not
worth it in-browser; the existing route_frame/sends API already IS the binding surface), **A-style for host**
(core's Rust `UdpRadio`). i.e. accept a deliberate asymmetry: host = Rust `ConnectionlessRadio`; browser =
JS-carried over the same wire + same profile. The **profile** is what's unified, not the socket-holding layer.
If core prefers full symmetry (Rust `WsRadio` via web_sys), that's option A — I'll build it; just adds the
async-inbound-queue bridge. **This is the one decision I need from core.**

## WS relay gateway (needed regardless of A/B)
Browser wasm can't be a server; hives mesh through a **broadcast relay** (the WS analogue of the ESP-NOW
shared-broadcast bearer): each hive's frame → gateway → rebroadcast to all OTHER connected hives → each calls
`route_frame`. Dedup (msg_id,origin) + TTL already prevent loops/storms. This gateway is layer-agnostic infra
(works for both A and B). Open: is the gateway hive's infra or composer's bench server? (composer runs the
browser bench + likely already has a WS server for the dashboard — confirm to avoid a duplicate.)

## Sequence (once core confirms the seam)
1. core lands `TransportProfile` in r2-transport + the host-UDP `ConnectionlessRadio` (reference/UDP-first).
2. hive: WS binding per the chosen option + wire r2-hive-wasm route in/out to it (replace the in-process relay
   default with the real-socket path; keep in-process as a test bearer).
3. hive: attach the §2.7 profile to each transport link (routing reads power_cost/MTU/λ).
4. Peer-refute the transport seam (spoof/replay across the real socket; dedup/deliver-gate hold) →
   commit/push/hosted-green.

## Done so far (v0.4.7, commit 6df4060)
`quality_from_rssi` (§2.5 clamp) + `range_to_loss` (§2.7 provisional) exported from r2-hive-wasm — the shared
sim/field physics; 9/9 host tests green.
