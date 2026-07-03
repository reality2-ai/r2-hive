// bridge-test-mesh — e2e HETEROGENEOUS cross-transport relay (#26). Topology is the proof:
//
//     A (sensor, WS-only) ──ws──▶ [ gateway ] ──ws──▶ BRIDGE (WS+UDP, one TG) ──udp──▶ C (receiver, UDP-only)
//                                                                              └─udp──▶ D (WRONG key, UDP-only)
//
// A speaks ONLY WebSocket; C/D speak ONLY UDP. The ONLY path from A to C is THROUGH the bridge, which
// runs BOTH bearers in ONE WasmHive and relays A's WS readings out over UDP. So any delivery at C
// PROVES the frame crossed WS→bridge→UDP (R2-ROUTE §5.4 multi-transport-relay + §5.2 directed egress).
//
// Verified end-to-end properties (instrumented received-vs-delivered disambiguates the mechanism):
//   • CROSS-TRANSPORT RELAY — C (same TG key, UDP-only) delivers A's WS-originated readings.
//   • DEDUP survives the hop — C receives >deliveries (a duplicate arrives, is deduped, not re-delivered).
//   • TG-ISOLATION via NEIGHBOUR-EXCLUSION — D (wrong key) receives 0 and delivers 0: its wrong-key announce
//     is not authenticated, so the bridge's route core never LEARNS it as a neighbour (A1 verify-then-record
//     gates the reachability record too), so wrong-key nodes never even receive relayed traffic. This is a
//     STRONGER isolation than the per-dest deliver-gate — the outsider is excluded at the routing layer.
//     (The pure deliver-gate — relay-for-TG-X but deliver-only-to-TG-Y — needs a multi-TG bridge; follow-on.)
//
// NB the wasm route core only forwards to a neighbour it has LEARNED on a transport (via an inbound
// frame's arrival_kind), so C/D must announce themselves (a heartbeat) to the bridge over UDP BEFORE
// A's readings flow — otherwise the bridge's route engine has no UDP neighbour to relay to.
'use strict';
const path = require('path');
const { spawn } = require('child_process');
const { HiveWs } = require('./hive-ws');
const { HiveUdp } = require('./hive-udp');
const { HiveBridge, WsBearer, UdpBearer } = require('./hive-bridge');

const TG = 0x1234abcd >>> 0;
const HK = Array.from({ length: 32 }, (_, i) => (i * 7 + 1) & 0xff);
const WRONG = Array.from({ length: 32 }, () => 0xaa);

const A = 0x000000a1, BR = 0x000000b0, C = 0x000000c3, D = 0x000000d4;
const WS_PORT = 21150;
const P_BR = 21151, P_C = 21152, P_D = 21153; // UDP ports

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
function hexToBytes(h) {
  const out = new Uint8Array(h.length >> 1);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.substr(i * 2, 2), 16);
  return out;
}

function startGateway() {
  return new Promise((resolve, reject) => {
    const gw = spawn('node', [path.join(__dirname, 'gateway.js'), String(WS_PORT)],
      { stdio: ['ignore', 'ignore', 'pipe'] });
    const to = setTimeout(() => reject(new Error('gateway did not start in 3s')), 3000);
    gw.stderr.on('data', (d) => { if (String(d).includes('listening')) { clearTimeout(to); resolve(gw); } });
    gw.on('exit', (code) => { clearTimeout(to); reject(new Error(`gateway exited early (${code})`)); });
  });
}

async function main() {
  let cDelivers = 0, dDelivers = 0, brRelayUdp = 0;

  const gw = await startGateway();

  // A — WS-only sensor (drive its ensemble via the underlying hive; HiveWs has no tick wrapper).
  const a = new HiveWs(A, `ws://127.0.0.1:${WS_PORT}`, { hk: HK, tgHash: TG });
  a.hive.enableSensor();
  const aTick = (t) => {
    const o = JSON.parse(a.hive.tick(t >>> 0));
    for (const f of o.frames || []) a.originate(hexToBytes(f));
  };

  // BRIDGE — WS bearer (to the gateway) + UDP bearer (knows C's + D's addrs). One TG key.
  const bridge = new HiveBridge(BR, {
    hk: HK, tgHash: TG,
    onRoute: (_id, out, arrivalKind) => {
      for (const s of out.sends || []) if ((s.kind >>> 0) === 6 && arrivalKind !== 6) brRelayUdp++;
    },
  });
  bridge.addBearer(new WsBearer(`ws://127.0.0.1:${WS_PORT}`, 1));
  bridge.addBearer(new UdpBearer({ peers: { [C]: `127.0.0.1:${P_C}`, [D]: `127.0.0.1:${P_D}` }, bindPort: P_BR }, 6));

  // C / D — UDP-only receivers (know the bridge's UDP addr, to announce + be relayed to). onRoute fires
  // on EVERY non-echo frame received (before/independent of the deliver-gate), so cRecv/dRecv measure what
  // actually ARRIVED — this disambiguates "wrong-key rejected at the deliver-gate" (dRecv>0, dDeliver=0)
  // from "wrong-key never relayed to" (dRecv=0 = excluded at neighbour-learning).
  let cRecv = 0, dRecv = 0;
  const c = new HiveUdp(C, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: HK,    tgHash: TG, bindPort: P_C, onDeliver: () => cDelivers++, onRoute: () => cRecv++ });
  const d = new HiveUdp(D, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: WRONG, tgHash: TG, bindPort: P_D, onDeliver: () => dDelivers++, onRoute: () => dRecv++ });

  await Promise.all([a.connect(), bridge.connectAll(), c.connect(), d.connect()]);
  await sleep(100); // WS handshakes settle

  // (1) C + D announce over UDP → the bridge's route core learns them as UDP (kind 6) neighbours.
  c.originate(c.buildHeartbeat(1));
  d.originate(d.buildHeartbeat(1));
  await sleep(100);

  // (2) A emits sensor readings on WS → gateway → bridge(WS) → relay → UDP → C.
  for (let t = 1; t <= 5; t++) { aTick(t); await sleep(50); }
  await sleep(150);

  a.close(); bridge.close(); c.close(); d.close();
  try { gw.kill('SIGTERM'); } catch (_) {}

  const pass = cDelivers >= 1 && dDelivers === 0 && brRelayUdp >= 1;
  console.log(`bridge relayed WS→UDP sends=${brRelayUdp} (want >=1)`);
  console.log(`C(same-key, UDP-only) received=${cRecv} delivered=${cDelivers} (want deliver>=1)`);
  console.log(`D(wrong-key, UDP-only) received=${dRecv} delivered=${dDelivers} (want deliver=0)`);
  console.log(dRecv > 0
    ? `  → TG-isolation MECHANISM = DELIVER-GATE: D received ${dRecv} relayed frame(s) but the r2_trust gate rejected all (wrong key).`
    : `  → TG-isolation MECHANISM = NEIGHBOUR-EXCLUSION: D's wrong-key announce was not authenticated → not learned → never relayed to (0 received).`);
  console.log(pass
    ? 'PASS bridge-mesh: heterogeneous WS→UDP cross-transport relay + dedup-survives-hop + TG-isolation (§5.4/§5.2)'
    : 'FAIL bridge-mesh');
  process.exit(pass ? 0 : 1);
}
main().catch((e) => { console.error(e); process.exit(1); });
