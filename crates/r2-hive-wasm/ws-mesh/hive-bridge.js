// hive-bridge — a HETEROGENEOUS cross-transport R2 mesh node (#26): ONE WasmHive route core
// speaking MULTIPLE bearers at once (WS + UDP + … carrier), relaying frames ACROSS transports.
// This is R2-ROUTE §5.4 multi-transport-relay + §5.2 per-neighbour DIRECTED egress — NOT a gateway
// construct (specs: there is no gateway; a "bridge" is an ordinary node whose route engine happens
// to know neighbours on >1 transport). dedup + TG GroupHmac survive BY CONSTRUCTION: the frame-carried
// origin (§3.3) is transport-agnostic so (msg_id,origin) dedup is unaffected by the hop; the GroupHmac
// signed span is frame CONTENT (route_stack is mutable relay metadata, excluded), so relay-append does
// not invalidate it; the deliver-gate fires only at the final destination.
//
// Mechanism: route_frame(arrival_kind, bytes) → sends[] each tagged with the NEXT-HOP's transport
// `kind` (the wasm route core captures every chosen hop on the CaptureTransport matching that
// neighbour's learned transport). The bridge dispatches each send to the bearer whose kind matches —
// so a frame heard on WS is relayed out UDP to a UDP-only neighbour, and vice versa. A neighbour is
// learned on a transport only after an inbound frame arrives with that bearer's kind, so downstream
// peers must announce themselves before the route core will target them.
//
//   inbound on bearer X → deliver-gate (verifyFrame = the real r2_trust TG gate, local delivery)
//                       → route_frame(0, X.kind, bytes) → for each send: bearerByKind[send.kind].sendTo(send.target, frame)
//   originate (own broadcast: HB / sensor tick) → broadcast on EVERY attached bearer
'use strict';
const path = require('path');
const dgram = require('dgram');

// ── Bearers ────────────────────────────────────────────────────────────────────────────────────
// A bearer is a transport link the bridge speaks. It is socket-ONLY (holds no WasmHive) — the bridge
// owns the single route core. Each bearer declares its routing `kind` (the §2.2 medium id the route
// core tags neighbours with) and exposes: connect(), broadcast(bytes), sendTo(target, bytes), close(),
// plus an `onFrame(bytes)` callback the bridge assigns on addBearer().

// WS bearer = the shared-broadcast bearer (gateway rebroadcasts to every other hive). Directed and
// broadcast sends both put the frame on the one socket; the gateway fans it out and each peer's route
// core dedups. `target` is unused (there is no per-peer WS socket). Kind 1 = Wifi (the closest existing
// TransportKind for an IP/WS bearer; matches hive-ws.js WS_KIND).
class WsBearer {
  constructor(url, kind = 1) {
    this.url = url;
    this.kind = kind >>> 0;
    this.ws = null;
    this.onFrame = () => {};
  }
  connect() {
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(this.url);
      ws.binaryType = 'arraybuffer';
      this.ws = ws;
      ws.addEventListener('open', () => resolve(this));
      ws.addEventListener('error', (e) => reject(e.error || new Error('ws error')));
      ws.addEventListener('message', (ev) => this.onFrame(bytesOf(ev.data)));
    });
  }
  broadcast(bytes) { if (this.ws) this.ws.send(Uint8Array.from(bytes)); }
  sendTo(_target, bytes) { this.broadcast(bytes); }
  close() { if (this.ws) { try { this.ws.close(); } catch (_) {} this.ws = null; } }
}

// UDP bearer = the UNICAST-per-peer R2-DISCOVERY §4.4 model (byte-interoperable with a Linux r2-hive
// UDP peer). A directed send resolves target hive_id → addr via the PeerTable; a broadcast becomes N
// unicasts. Kind 6 = Udp (generic connectionless; pass kind 1 for a SoftAP UDP-LAN topology, per core's
// taxonomy). Mirrors hive-udp.js.
class UdpBearer {
  // opts: { peers: {<hiveId decimal>: "ip:port"}, bindPort, bindAddr }
  constructor(opts = {}, kind = 6) {
    this.kind = kind >>> 0;
    this.peers = new Map();
    for (const [k, v] of Object.entries(opts.peers || {})) this.peers.set(parseInt(k) >>> 0, v);
    this.bindPort = opts.bindPort || 0;
    this.bindAddr = opts.bindAddr || '127.0.0.1';
    this.sock = null;
    this.onFrame = () => {};
  }
  connect() {
    return new Promise((resolve, reject) => {
      const sock = dgram.createSocket('udp4');
      this.sock = sock;
      sock.on('error', (e) => reject(e));
      sock.on('message', (msg) => this.onFrame(new Uint8Array(msg)));
      sock.bind(this.bindPort, this.bindAddr, () => resolve(this));
    });
  }
  addPeer(hiveId, addr) { this.peers.set(hiveId >>> 0, addr); }
  _sendAddr(addr, bytes) {
    if (!addr || !this.sock) return;
    const i = addr.lastIndexOf(':');
    this.sock.send(Buffer.from(bytes), parseInt(addr.slice(i + 1)), addr.slice(0, i));
  }
  broadcast(bytes) { for (const addr of this.peers.values()) this._sendAddr(addr, bytes); }
  sendTo(target, bytes) { this._sendAddr(this.peers.get(target >>> 0), bytes); }
  close() { if (this.sock) { try { this.sock.close(); } catch (_) {} this.sock = null; } }
}

// ── The bridge ─────────────────────────────────────────────────────────────────────────────────
class HiveBridge {
  // opts: { hk, tgHash, pkgDir, onDeliver, onRoute }
  constructor(hiveId, opts = {}) {
    const pkg = opts.pkgDir || path.join(__dirname, 'wasmhive-node');
    this.wh = require(path.join(pkg, 'r2_hive_wasm.js'));
    this.hive = new this.wh.WasmHive(hiveId >>> 0);
    if (opts.hk && opts.tgHash != null) {
      this.hive.setGroupHmac(Uint8Array.from(opts.hk), opts.tgHash >>> 0);
      this.keyed = true;
    } else {
      // Keyless pure-routing sim: R2-TRUST §7.5.4 fail-closes an unkeyed hive by default; opt in explicitly.
      this.keyed = false;
      this.hive.setUnkeyedOpen(true);
      process.stderr.write(
        `# ⚠ bridge ${hex(hiveId >>> 0)}: NO GroupHmac — TG-AGNOSTIC pure-routing sim (§7.5.4 opt-in accept-all).\n`
      );
    }
    this.id = hiveId >>> 0;
    this.onDeliver = opts.onDeliver || (() => {});
    this.onRoute = opts.onRoute || (() => {});
    this.seq = 0;
    this._t0 = Date.now();
    this.bearers = [];
    this.bearerByKind = new Map();
  }

  _now() { return Math.floor((Date.now() - this._t0) / 1000) >>> 0; }

  /** Attach a bearer (WsBearer/UdpBearer/…). Wires its inbound to the bridge and indexes it by kind. */
  addBearer(bearer) {
    if (this.bearerByKind.has(bearer.kind)) {
      // Two bearers on the SAME routing kind is a misconfig: the route core cannot tell them apart, so
      // a directed send is ambiguous. Warn loudly; last-registered wins for dispatch.
      process.stderr.write(
        `# ⚠ bridge ${hex(this.id)}: two bearers share kind=${bearer.kind} — directed egress is ambiguous.\n`
      );
    }
    bearer.onFrame = (bytes) => this._inbound(bearer, bytes);
    this.bearers.push(bearer);
    this.bearerByKind.set(bearer.kind, bearer);
    return bearer;
  }

  /** Connect every attached bearer. */
  async connectAll() { await Promise.all(this.bearers.map((b) => b.connect())); return this; }

  _inbound(bearer, bytes) {
    // (0) drop our OWN echo: a broadcast bearer bounces a frame we sent back to us; origin==self ⇒ ours.
    try { if (this.wh.frame_origin(bytes) === this.id) return; } catch (_) { /* undecodable → fall through */ }
    // (1) DELIVER-GATE — the bridge is itself a TG member; verifyFrame is the real r2_trust gate.
    let gate;
    try { gate = JSON.parse(this.hive.verifyFrame(bytes)); }
    catch (e) { process.stderr.write(`# bridge ${hex(this.id)}: verifyFrame threw: ${e}\n`); gate = { deliver: false }; }
    if (gate.deliver) this.onDeliver(this.id, bytes, gate, bearer.kind);
    // (2) CROSS-TRANSPORT FORWARD — route_frame tags each send with the next-hop's transport kind; the
    //     bridge dispatches each to the bearer of that kind (unicast bearers resolve send.target → addr).
    let out;
    try { out = JSON.parse(this.hive.route_frame(0, bearer.kind, bytes, this._now(), 0.5)); }
    catch (e) { process.stderr.write(`# bridge ${hex(this.id)}: route_frame threw: ${e}\n`); return; }
    this.onRoute(this.id, out, bearer.kind);
    for (const s of out.sends || []) {
      const dst = this.bearerByKind.get(s.kind >>> 0);
      if (dst) dst.sendTo(s.target >>> 0, hexToBytes(s.frame));
      else process.stderr.write(
        `# bridge ${hex(this.id)}: no bearer for send.kind=${s.kind} (target ${hex(s.target >>> 0)}) — dropped\n`
      );
    }
  }

  /** Originate a broadcast frame (HB / sensor reading) onto EVERY attached bearer. */
  originate(bytes) { for (const b of this.bearers) b.broadcast(bytes); }

  /** Drive one ensemble TICK and originate every frame the node's sentants emit. */
  tick(seq) {
    const out = JSON.parse(this.hive.tick((seq ?? this.seq++) >>> 0));
    for (const f of out.frames || []) this.originate(hexToBytes(f));
    return out;
  }

  /** Enable the SENSOR ensemble role (emits an r2.tn.routetest reading each tick). */
  enableSensor() { this.hive.enableSensor(); }
  buildHeartbeat(seq) { return this.hive.build_heartbeat((seq ?? this.seq++) >>> 0); }

  close() { for (const b of this.bearers) b.close(); }
}

// ── helpers ──────────────────────────────────────────────────────────────────────────────────────
function bytesOf(data) {
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  return Uint8Array.from(Buffer.from(data)); // string fallback
}
function hexToBytes(h) {
  const out = new Uint8Array(h.length >> 1);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.substr(i * 2, 2), 16);
  return out;
}
function hex(n) { return (n >>> 0).toString(16).padStart(8, '0'); }

module.exports = { HiveBridge, WsBearer, UdpBearer };
