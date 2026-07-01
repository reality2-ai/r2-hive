// hive-ws — wire a WasmHive's route inbound/outbound to a REAL WebSocket (#26, option B:
// JS-carried binding). The wasm route core stays pure-sync; JS owns the async socket:
//   inbound  WS message (R2-WIRE bytes) → hive.route_frame(...)  → deliver / relay decision
//   outbound each relay `send` + any originated frame           → ws.send(bytes) to the bearer
// The gateway rebroadcasts to the other hives = a real multi-hive mesh over WebSocket, the
// SAME route core the ESP32/Linux run — no in-process relay. Uses Node's built-in global
// WebSocket (client); no npm deps.
'use strict';
const path = require('path');

// WS bearer routing-kind: 1 = Wifi (closest existing TransportKind for an IP/WS bearer;
// the §2.7 profile is what's carrier-independent — the kind only tags the link for the
// transport-aware routing math). source_hive = 0 = "unknown immediate sender" (the frame's
// route_stack[0] origin is what (msg_id,origin) dedup keys on, so 0 here is fine).
const WS_KIND = 1;

class HiveWs {
  constructor(hiveId, url, opts = {}) {
    const pkg = opts.pkgDir || path.join(__dirname, 'wasmhive-node');
    this.wh = require(path.join(pkg, 'r2_hive_wasm.js'));
    this.hive = new this.wh.WasmHive(hiveId >>> 0);
    if (opts.hk && opts.tgHash != null) {
      this.hive.setGroupHmac(Uint8Array.from(opts.hk), opts.tgHash >>> 0);
    }
    this.id = hiveId >>> 0;
    this.url = url;
    this.onDeliver = opts.onDeliver || (() => {});
    this.onRoute = opts.onRoute || (() => {});
    this.seq = 0;
    this._t0 = Date.now();
    this.ws = null;
  }

  _now() { return Math.floor((Date.now() - this._t0) / 1000) >>> 0; }

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
    // Two SEPARATE layers, per the route contract:
    //  (1) DELIVER-GATE: verify_frame = the real r2_trust deliver-gate (tg_ok/hmac_ok/deliver).
    //      This is how a hive ACCEPTS a frame for its trust-group (the local-delivery decision).
    //      route_inbound_sync deliberately omits classify/keys, so delivery lives here, not in
    //      route_frame's outcome (a self-addressed frame → route_frame Dropped = "don't forward").
    let gate;
    try { gate = JSON.parse(this.hive.verifyFrame(bytes)); } catch (e) {
      process.stderr.write(`# hive ${hex(this.id)}: verifyFrame threw: ${e}\n`); gate = { deliver: false };
    }
    if (gate.deliver) this.onDeliver(this.id, bytes, gate);
    // (2) FORWARDING: route_frame = relay/flood decision; push its sends[] back onto the bearer.
    let out;
    try {
      out = JSON.parse(this.hive.route_frame(0, WS_KIND, bytes, this._now(), 0.5));
    } catch (e) {
      process.stderr.write(`# hive ${hex(this.id)}: route_frame threw: ${e}\n`);
      return;
    }
    this.onRoute(this.id, out);
    for (const s of out.sends || []) {
      this.ws.send(Uint8Array.from(Buffer.from(s.frame, 'hex')));
    }
  }

  // Originate a frame from THIS hive onto the bearer (the gateway broadcasts to the others).
  originate(bytes) { this.ws.send(Uint8Array.from(bytes)); }

  buildFrame(targetHive, eventHash, payload, seq) {
    return this.hive.build_frame(targetHive >>> 0, eventHash >>> 0, Uint8Array.from(payload), (seq ?? this.seq++) >>> 0);
  }
  buildHeartbeat(seq) { return this.hive.build_heartbeat((seq ?? this.seq++) >>> 0); }

  close() { if (this.ws) this.ws.close(); }
}

function bytesOf(data) {
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return Uint8Array.from(Buffer.from(data)); // string fallback
}
function hex(n) { return (n >>> 0).toString(16).padStart(8, '0'); }

module.exports = { HiveWs, WS_KIND };
