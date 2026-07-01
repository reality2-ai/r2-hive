# ws-mesh — r2-hive-wasm over a REAL WebSocket (#26, WASM-WS binding)

Makes r2-hive-wasm mesh over an actual socket instead of the in-process virtual-mesh relay — the
browser half of the "production no-radio hive" (host/UDP is core's binding). This is the **option B
(JS-carried)** reference: the wasm route core stays pure-sync; JS owns the async WebSocket and drives
`route_frame` (forwarding) + `verifyFrame` (deliver-gate) per inbound frame.

## Pieces
- **`gateway.js`** — a zero-dep WebSocket **broadcast relay** = the WS analogue of the ESP-NOW shared
  bearer. Rebroadcasts each frame to every OTHER connected hive; it does NOT route (loops bounded by the
  route core's dedup + TTL). Layer-agnostic infra (works for option A too). `node gateway.js [port]`.
- **`hive-ws.js`** — wires a `WasmHive`'s route in/out to a real `WebSocket` (Node's built-in client):
  inbound → `verifyFrame` (deliver-gate) + `route_frame` (relay `sends[]` back onto the bearer);
  `originate(bytes)` / `buildFrame` / `buildHeartbeat` to emit.
- **`test-mesh.js`** — end-to-end proof: 3 hives over the gateway; A+B share a TG key, C has the wrong
  key (same tg_hash). A fires a SIGNED heartbeat over WS → B delivers (`hmac_ok`), C rejected (TG
  isolation) — the real deliver-gate over a real socket.

## Build + run
The wasm-node package is generated (gitignored). Build it first:
```
cd ..                     # crates/r2-hive-wasm
wasm-pack build --target nodejs --out-dir ws-mesh/wasmhive-node
cd ws-mesh && node test-mesh.js     # expect 3× PASS
```
Run a live mesh: `node gateway.js 21055` then point hive clients (or composer's browser app) at
`ws://<host>:21055`.

## Status / open seam (see ../../../docs/WS-TRANSPORT-BINDING.md)
This is the JS-carried (option B) reference. Core owns the host-UDP `ConnectionlessRadio` binding and the
shared `TransportProfile` struct (§2.7); the one open decision is whether the browser binding stays JS-carried
(B, recommended) or becomes a Rust `web_sys` `WsRadio` (A, full symmetry). The gateway + wiring survive either.
Gateway ownership (hive infra vs composer's bench server) — to confirm with composer.
