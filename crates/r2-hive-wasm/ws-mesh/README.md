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

## Security boundary (WS-seam refuter Angle-2)
The gateway has **no connection auth or rate-limit** — any client that can open a socket floods
`route_frame()` O(N×M). It therefore binds **localhost-only (127.0.0.1) by default** — that isolation
IS the boundary. Binding a routable interface is an explicit opt-in (`WS_MESH_HOST=0.0.0.0`) that MUST
be paired with an auth token before exposure (the code warns on a non-local bind). A keyless hive client
(`HiveWs` without `{hk,tgHash}`) runs TG-agnostic (accepts all frames) and logs a loud warning — pass a
GroupHmac for the real deliver-gate.

## Browser module (composer's webapp) — `hive-ws-browser.js` + `.d.ts`
The ESM build of the binding for the browser (Node's `hive-ws.js` is the same contract for Node). Composer's
webapp wires its wasm hives to the gateway with it:
```js
import initWasm, * as wasm from './wasmhive/r2_hive_wasm.js';  // wasm-pack --target web pkg (this crate's pkg/)
await initWasm();
import { HiveWs } from './hive-ws-browser.js';
const hive = new HiveWs({ wasm, hiveId: 0x0a, url: 'ws://127.0.0.1:21055', hk: nodesHk, tgHash: TG,
                          onDeliver: (id, bytes, gate) => …, onRoute: (id, out) => … });
await hive.connect();
hive.originate(hive.buildHeartbeat());
```
**WS message shape (client ↔ gateway):** a WebSocket **binary** frame whose payload is the raw R2-WIRE frame
bytes — no JSON envelope on the wire (the gateway is a dumb broadcast bearer). JSON appears only in the local
`route_frame` return. **Receive pattern** (baked into `HiveWs._onFrame`): `frame_origin`(echo-drop) →
`verifyFrame`(deliver-gate) → `route_frame`(forwarding, relay `sends[]`). Delivery and forwarding are SEPARATE
layers — a self-addressed frame yields `route_frame` outcome `Dropped` yet still delivers via `verifyFrame`.
Types in `hive-ws-browser.d.ts`; the `WasmHive`/free-fn types are in the wasm-pack `pkg/r2_hive_wasm.d.ts`.

## Status / open seam (see ../../../docs/WS-TRANSPORT-BINDING.md)
This is the JS-carried (option B) reference — **RATIFIED B** (core confirmed; reserve A). Core owns the host-UDP
`ConnectionlessRadio` binding (`r2_transport::host_udp::HostUdpRadio`) and the shared `TransportProfile` (§2.7);
the browser binding rides the SAME profile over WS. **Gateway = HIVE infra** (composer confirmed its bench runs no
general WS broadcast bearer). Composer connects its browser wasm hives to this gateway via `hive-ws-browser.js`.
