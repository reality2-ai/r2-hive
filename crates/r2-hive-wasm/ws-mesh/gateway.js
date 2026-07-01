// r2 ws-mesh gateway — a zero-dep WebSocket BROADCAST relay = the WS analogue of the
// ESP-NOW shared-broadcast bearer (#26). Each connected wasm-hive sends an R2-WIRE
// frame; the gateway rebroadcasts it to every OTHER connected hive, which feeds it to
// its route core. Loops/storms are prevented by the route core's (msg_id,origin) dedup
// + TTL — the gateway is a dumb bearer, it does NOT route.
//
// Layer-agnostic infra: works whether the hive client is JS-carried (option B) or a
// Rust web_sys WsRadio (option A). No npm deps — hand-rolled RFC6455 (Node ships a WS
// *client* but no server). Binary frames only; small payloads (<64 KiB).
//
//   node gateway.js [port]     # default 21055
'use strict';
const http = require('http');
const crypto = require('crypto');

const WS_GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
const PORT = parseInt(process.argv[2] || process.env.WS_MESH_PORT || '21055', 10);
// SECURITY BOUNDARY (WS-seam refuter Angle-2): the gateway has NO connection auth or rate-limit — any
// client that can open a socket can flood O(N×M) route_frame() calls. So it binds LOCALHOST-ONLY by
// default (127.0.0.1); the closed-bench 127.0.0.1 isolation IS the boundary. Binding a routable
// interface (WS_MESH_HOST=0.0.0.0) is an EXPLICIT opt-in that you MUST pair with a real auth token /
// rate-limit before exposing. (Node's `listen(port)` with no host would bind 0.0.0.0 — do NOT rely on
// the default; we pin the host below.)
const HOST = process.env.WS_MESH_HOST || '127.0.0.1';

const clients = new Set(); // net.Socket, one per connected hive

function accept(key) {
  return crypto.createHash('sha1').update(key + WS_GUID).digest('base64');
}

// Encode one server→client binary frame (unmasked, FIN=1, opcode=0x2).
function encodeFrame(payload) {
  const len = payload.length;
  let header;
  if (len < 126) {
    header = Buffer.from([0x82, len]);
  } else if (len < 65536) {
    header = Buffer.from([0x82, 126, (len >> 8) & 0xff, len & 0xff]);
  } else {
    throw new Error('ws-mesh: frame too large (>64 KiB)');
  }
  return Buffer.concat([header, payload]);
}

// Pull complete frames out of a per-socket buffer (client→server, always masked).
// Returns {frames:[Buffer], rest:Buffer, close:bool}.
function drainFrames(buf) {
  const frames = [];
  let off = 0;
  let close = false;
  while (off + 2 <= buf.length) {
    const b0 = buf[off];
    const b1 = buf[off + 1];
    const opcode = b0 & 0x0f;
    const masked = (b1 & 0x80) !== 0;
    let len = b1 & 0x7f;
    let p = off + 2;
    if (len === 126) {
      if (p + 2 > buf.length) break;
      len = (buf[p] << 8) | buf[p + 1];
      p += 2;
    } else if (len === 127) {
      if (p + 8 > buf.length) break;
      // only the low 32 bits (our frames are small)
      len = buf.readUInt32BE(p + 4);
      p += 8;
    }
    const maskLen = masked ? 4 : 0;
    if (p + maskLen + len > buf.length) break; // incomplete — wait for more
    let payload = buf.slice(p + maskLen, p + maskLen + len);
    if (masked) {
      const mask = buf.slice(p, p + 4);
      const out = Buffer.allocUnsafe(len);
      for (let i = 0; i < len; i++) out[i] = payload[i] ^ mask[i & 3];
      payload = out;
    }
    off = p + maskLen + len;
    if (opcode === 0x8) { close = true; break; } // close
    if (opcode === 0x1 || opcode === 0x2) frames.push(payload); // text/binary
    // opcode 0x9 ping / 0xA pong ignored for this bearer
  }
  return { frames, rest: buf.slice(off), close };
}

const server = http.createServer((_req, res) => {
  res.writeHead(426, { 'Content-Type': 'text/plain' });
  res.end('ws-mesh gateway — WebSocket only\n');
});

server.on('upgrade', (req, socket) => {
  const key = req.headers['sec-websocket-key'];
  if (!key) { socket.destroy(); return; }
  socket.write(
    'HTTP/1.1 101 Switching Protocols\r\n' +
    'Upgrade: websocket\r\nConnection: Upgrade\r\n' +
    `Sec-WebSocket-Accept: ${accept(key)}\r\n\r\n`
  );
  clients.add(socket);
  process.stderr.write(`# ws-mesh: hive connected (${clients.size} on the bearer)\n`);
  let buf = Buffer.alloc(0);
  socket.on('data', (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    const { frames, rest, close } = drainFrames(buf);
    buf = rest;
    for (const f of frames) {
      // BROADCAST to every OTHER hive (shared-bearer semantics; sender excluded).
      const wire = encodeFrame(f);
      for (const c of clients) if (c !== socket && !c.destroyed) c.write(wire);
    }
    if (close) socket.end();
  });
  const drop = () => {
    if (clients.delete(socket)) {
      process.stderr.write(`# ws-mesh: hive left (${clients.size} on the bearer)\n`);
    }
  };
  socket.on('close', drop);
  socket.on('error', drop);
});

if (HOST !== '127.0.0.1' && HOST !== 'localhost' && HOST !== '::1') {
  process.stderr.write(`# ⚠ ws-mesh gateway binding NON-LOCAL host ${HOST} — NO AUTH/RATE-LIMIT. Add a token before exposing.\n`);
}
server.listen(PORT, HOST, () => {
  process.stderr.write(`# ws-mesh gateway listening ws://${HOST}:${PORT} (broadcast bearer, ${HOST === '127.0.0.1' ? 'localhost-only' : 'EXPOSED'})\n`);
});

module.exports = { encodeFrame, drainFrames, accept }; // for unit tests
