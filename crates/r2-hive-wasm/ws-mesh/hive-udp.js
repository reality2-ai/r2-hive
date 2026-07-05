// hive-udp — wire a WasmHive's route in/out to a REAL UDP socket (Node dgram). This is the
// UNICAST-per-peer R2-DISCOVERY §4.4 model core's Linux `UdpLanTransport` uses, byte-for-byte
// interoperable: each frame is a raw R2-WIRE datagram unicast to a resolved peer addr. There is NO
// multicast/broadcast in core (grep-empty) — a "broadcast-style" send from the route core becomes N
// unicasts to resolved addrs. The wasm route core stays pure-sync; JS owns the async socket + the
// `hive_id -> "ip:port"` PeerTable.
//   inbound  UDP datagram (R2-WIRE bytes) -> verifyFrame (deliver-gate) + route_frame (forward)
//   outbound each route_frame `sends[]` entry -> resolve `s.target` hive_id -> unicast to its ip:port
//
// PeerTable population: core has NO discovery beacon, so first-contact needs either a peer unicasting
// us first (inbound-first: recv learns the source addr) or a CONFIG SEED (opts.peers). LAN auto-discovery
// is out-of-scope until specs reconciles §2.6.1-multicast vs §4.4-unicast (flagged). The unicast data
// path here is exactly what a Linux r2-hive UDP peer speaks.
'use strict';
const path = require('path');
const dgram = require('dgram');

// UDP bearer routing-kind (LOCAL routing-math tag ONLY — core confirmed the r2-wire frame is
// TRANSPORT-AGNOSTIC, carries NO transport id, so the kind never goes on the wire and browser⇄board frames
// match regardless). Per core's authoritative Transport taxonomy: generic/global connectionless UDP =
// Udp(6) [default here]; a SoftAP/infrastructure UDP-LAN bearer maps to the EXISTING Wifi(1) ("UDP events
// over SoftAP or infrastructure") — pass `opts.kind: 1` for that topology. Do NOT invent a new UDP-LAN
// variant (that's a specs Transport-enum change, not a local fork). source_hive=0 = "unknown immediate
// sender" (the frame's route_stack[0] origin is what (msg_id,origin) dedup keys on).
const UDP_KIND = 6;

class HiveUdp {
  // opts: { peers: {<hiveId decimal>: "ip:port"}, hk, tgHash, bindPort, bindAddr, pkgDir, onDeliver, onRoute }
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
        `# ⚠ hive ${hex(hiveId >>> 0)}: NO GroupHmac — TG-AGNOSTIC pure-routing sim (§7.5.4 opt-in accept-all).\n`
      );
    }
    this.id = hiveId >>> 0;
    // PeerTable: hive_id (u32) -> "ip:port". Seeded from opts.peers; recv also learns source addrs.
    this.peers = new Map();
    for (const [k, v] of Object.entries(opts.peers || {})) this.peers.set(parseInt(k) >>> 0, v);
    this.bindPort = opts.bindPort || 0;
    this.bindAddr = opts.bindAddr || '127.0.0.1';
    // Routing-math kind: Udp(6) generic connectionless [default]; Wifi(1) for a SoftAP UDP-LAN topology.
    this.kind = opts.kind != null ? (opts.kind >>> 0) : UDP_KIND;
    this.onDeliver = opts.onDeliver || (() => {});
    this.onRoute = opts.onRoute || (() => {});
    this.seq = 0;
    this._t0 = Date.now();
    this.sock = null;
  }

  _now() { return Math.floor((Date.now() - this._t0) / 1000) >>> 0; }

  /** Add/update a peer's addr (the config-seed / §3-resolution surface; recv also calls this). */
  addPeer(hiveId, addr) { this.peers.set(hiveId >>> 0, addr); }

  connect() {
    return new Promise((resolve, reject) => {
      const sock = dgram.createSocket('udp4');
      this.sock = sock;
      sock.on('error', (e) => reject(e));
      sock.on('message', (msg, rinfo) => this._onFrame(new Uint8Array(msg), rinfo));
      sock.bind(this.bindPort, this.bindAddr, () => resolve(this));
    });
  }

  _sendTo(addr, bytes) {
    if (!addr || !this.sock) return;
    const i = addr.lastIndexOf(':');
    this.sock.send(Buffer.from(bytes), parseInt(addr.slice(i + 1)), addr.slice(0, i));
  }

  _onFrame(bytes, rinfo) {
    // (0) drop our OWN echo (a relaying peer may bounce our originated frame back).
    try { if (this.wh.frame_origin(bytes) === this.id) return; } catch (_) { /* undecodable → fall through */ }
    // Inbound-first learning: remember the source addr (a full stack would upgrade the id via §3 resolution).
    if (rinfo) this._learn(rinfo);
    // (1) DELIVER-GATE (local delivery for my TG) — verifyFrame is the real r2_trust gate.
    let gate;
    try { gate = JSON.parse(this.hive.verifyFrame(bytes)); }
    catch (e) { process.stderr.write(`# hive ${hex(this.id)}: verifyFrame threw: ${e}\n`); gate = { deliver: false }; }
    if (gate.deliver) this.onDeliver(this.id, bytes, gate);
    // (2) FORWARDING — route_frame relay/flood; unicast each send to its resolved next-hop addr.
    let out;
    try { out = JSON.parse(this.hive.route_frame(0, this.kind, bytes, this._now(), 0.5)); }
    catch (e) { process.stderr.write(`# hive ${hex(this.id)}: route_frame threw: ${e}\n`); return; }
    this.onRoute(this.id, out);
    for (const s of out.sends || []) this._sendTo(this.peers.get(s.target >>> 0), hexToBytes(s.frame));
  }

  _learn(rinfo) {
    // Provisional inbound addr — kept keyed by "ip:port" so an unknown sender is at least reachable back.
    // (Canonical hive_id resolution is §3's job; this mirrors udp_lan.rs recv's provisional-id fallback.)
    this._lastSrc = `${rinfo.address}:${rinfo.port}`;
  }

  /** Originate a frame from THIS hive: N unicasts to all known peers (the unicast analogue of a shared-
   *  bearer broadcast; each peer's route core dedups + delivers/relays). */
  originate(bytes) {
    for (const addr of this.peers.values()) this._sendTo(addr, bytes);
  }

  /** Drive one ensemble TICK and originate every frame the node's sentants emit (HB, sensor reading, …). */
  tick(seq) {
    const out = JSON.parse(this.hive.tick((seq ?? this.seq++) >>> 0));
    for (const f of out.frames || []) this.originate(hexToBytes(f));
    return out;
  }

  buildFrame(targetHive, eventHash, payload, seq) {
    return this.hive.build_frame(targetHive >>> 0, eventHash >>> 0, Uint8Array.from(payload), (seq ?? this.seq++) >>> 0);
  }
  /** Build a CRITICAL broadcast (k=15/FLOOD_SENTINEL_K) — full-mesh flood per §8.4, set EXPLICITLY.
   *  Every relay floods to ALL viable neighbours (vs buildFrame's k=3 spray-to-1). */
  buildCriticalFrame(targetHive, eventHash, payload, seq) {
    return this.hive.build_critical_frame(targetHive >>> 0, eventHash >>> 0, Uint8Array.from(payload), (seq ?? this.seq++) >>> 0);
  }
  buildHeartbeat(seq) { return this.hive.build_heartbeat((seq ?? this.seq++) >>> 0); }
  /** Enable the SENSOR ensemble role (emits an r2.tn.routetest reading each tick). */
  enableSensor() { this.hive.enableSensor(); }

  close() { if (this.sock) { try { this.sock.close(); } catch (_) {} this.sock = null; } }
}

// hex string -> Uint8Array.
function hexToBytes(h) {
  const out = new Uint8Array(h.length >> 1);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.substr(i * 2, 2), 16);
  return out;
}
function hex(n) { return (n >>> 0).toString(16).padStart(8, '0'); }

module.exports = { HiveUdp, UDP_KIND };
