// hive-ws-browser — the BROWSER (ESM) build of the #26 WASM-WS binding (option B, JS-carried).
// Same contract as ws-mesh/hive-ws.js (Node) but for the browser: ES module, global WebSocket,
// Uint8Array/hex (no Buffer), and the caller passes the already-initialised wasm module (so this
// stays decoupled from the pkg path — you import the wasm-pack --target web build yourself).
//
// USAGE (composer's webapp):
//   import initWasm, * as wasm from './wasmhive/r2_hive_wasm.js';   // wasm-pack --target web pkg
//   await initWasm();                                               // one-time wasm load
//   const hive = new HiveWs({ wasm, hiveId: 0x0a, url: 'ws://127.0.0.1:21055',
//                             hk: nodesHkBytes, tgHash: 0xTG,       // omit for TG-agnostic sim
//                             onDeliver: (id, bytes, gate) => …,    // deliver-gate accepted (for my TG)
//                             onRoute:   (id, out)        => … });  // {outcome, sent, sends[]}
//   await hive.connect();
//   hive.originate(hive.buildHeartbeat());                          // emit onto the bearer
//
// WS MESSAGE SHAPE (client ↔ gateway): a WebSocket **binary** frame whose payload is the raw R2-WIRE
// frame bytes — nothing wraps it (the gateway is a dumb broadcast bearer; it rebroadcasts the bytes to
// every OTHER connected hive). No JSON envelope on the wire; JSON is only the local route_frame return.
//
// RECEIVE PATTERN (mirror on any client): (1) wasm.frame_origin(bytes)===self ⇒ drop own echo;
// (2) hive.verifyFrame(bytes) → the real r2_trust deliver-gate ({keyed,tg_ok,hmac_ok,deliver}) = local
// delivery; (3) hive.route_frame(0, WS_KIND, bytes, now, dice) → forwarding, push each sends[].frame back
// onto the bearer. Delivery (verifyFrame) and forwarding (route_frame) are SEPARATE layers — a
// self-addressed frame returns route_frame outcome "Dropped" (nothing to forward) yet still delivers.

// WS bearer routing-kind: 1 = Wifi (closest existing TransportKind for an IP/WS bearer; the §2.7 profile
// is the carrier-independent part — this only tags the link for the transport-aware routing math).
export const WS_KIND = 1;

export class HiveWs {
  constructor(opts = {}) {
    if (!opts.wasm) throw new Error('HiveWs: pass the initialised wasm module as opts.wasm');
    this.wasm = opts.wasm;
    this.id = (opts.hiveId >>> 0);
    this.url = opts.url;
    this.hive = new this.wasm.WasmHive(this.id);
    if (opts.hk && opts.tgHash != null) {
      this.hive.setGroupHmac(Uint8Array.from(opts.hk), opts.tgHash >>> 0);
      this.keyed = true;
    } else {
      // MISCONFIG GUARD (refuter Angle-3): no GroupHmac ⇒ keyless pure-routing sim. R2-TRUST §7.5.4 makes an
      // unkeyed hive FAIL-CLOSED by default (default-OPEN is FORBIDDEN); we EXPLICITLY opt this bench hive into
      // the legacy accept-all deliver so the sim still works. DANGEROUS for a real trust mesh — never on a
      // production node; pass {hk,tgHash} for the real deliver-gate.
      this.keyed = false;
      this.hive.setUnkeyedOpen(true); // §7.5.4 explicit dev-only opt-in (else unkeyed → deliver:false)
      console.warn(`⚠ hive ${hex(this.id)}: NO GroupHmac — TG-AGNOSTIC pure-routing sim (§7.5.4 opt-in accept-all). Pass {hk,tgHash} for the real deliver-gate.`);
    }
    this.onDeliver = opts.onDeliver || (() => {});
    this.onRoute = opts.onRoute || (() => {});
    this.seq = 0;
    this._t0 = (typeof performance !== 'undefined' ? performance.now() : 0);
    this.ws = null;
  }

  _now() {
    const ms = (typeof performance !== 'undefined' ? performance.now() : 0) - this._t0;
    return (Math.floor(ms / 1000) >>> 0);
  }

  connect() {
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(this.url);
      ws.binaryType = 'arraybuffer';
      this.ws = ws;
      ws.addEventListener('open', () => resolve(this));
      ws.addEventListener('error', (e) => reject(e.error || new Error('ws error')));
      ws.addEventListener('message', (ev) => this._onFrame(bytesOf(ev.data)));
    });
  }

  _onFrame(bytes) {
    // (0) drop our OWN echo (broadcast bearer bounces a relayed copy back; unauth frames aren't
    // dedup-RECORDED so we'd re-relay). frame_origin is a MODULE free-fn (wasm.frame_origin), not a method.
    try { if (this.wasm.frame_origin(bytes) === this.id) return; } catch (_) { /* undecodable → fall through */ }
    // (1) DELIVER-GATE (local delivery for my TG) — verifyFrame is the real r2_trust gate.
    let gate;
    try { gate = JSON.parse(this.hive.verifyFrame(bytes)); } catch (e) { console.warn(`hive ${hex(this.id)}: verifyFrame threw`, e); gate = { deliver: false }; }
    if (gate.deliver) this.onDeliver(this.id, bytes, gate);
    // (2) FORWARDING — route_frame relay/flood; push each send back onto the bearer.
    let out;
    try { out = JSON.parse(this.hive.route_frame(0, WS_KIND, bytes, this._now(), 0.5)); }
    catch (e) { console.warn(`hive ${hex(this.id)}: route_frame threw`, e); return; }
    this.onRoute(this.id, out);
    for (const s of out.sends || []) this.ws.send(hexToBytes(s.frame));
  }

  originate(bytes) { this.ws.send(Uint8Array.from(bytes)); }
  buildFrame(targetHive, eventHash, payload, seq) {
    return this.hive.build_frame(targetHive >>> 0, eventHash >>> 0, Uint8Array.from(payload), (seq ?? this.seq++) >>> 0);
  }
  buildHeartbeat(seq) { return this.hive.build_heartbeat((seq ?? this.seq++) >>> 0); }
  close() { if (this.ws) this.ws.close(); }
}

// Normalise a WS message (Blob/ArrayBuffer/typed) to Uint8Array.
function bytesOf(data) {
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return new TextEncoder().encode(String(data)); // string fallback
}
// hex string -> Uint8Array.
function hexToBytes(h) {
  const out = new Uint8Array(h.length >> 1);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.substr(i * 2, 2), 16);
  return out;
}
function hex(n) { return (n >>> 0).toString(16).padStart(8, '0'); }
