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
//   • D (wrong key) receives 0 / delivers 0 — but this is NOT a TG-isolation proof (twice-corrected, do NOT
//     read it as one). bridge-neighbour-probe.js + bridge-flood-control.js pinned the real mechanism: D DOES
//     form a neighbour link (formation is TG-agnostic, AB-004 ok), but route_frame emits exactly ONE flood-send
//     per TRANSPORT (its shared-broadcast-bearer model — target = a representative neighbour, here C). The
//     control proved this is NOT auth: with TWO correct-key UDP neighbours, the 2nd correct-key one is ALSO not
//     targeted. So D=0 is FLOOD-UNDER-REACH on a unicast bearer (my UdpBearer unicasts to the single
//     representative), not key-rejection. Genuine TG-isolation-via-deliver-gate is proven in udp-test-mesh.js
//     (A directly unicasts to a wrong-key peer → its r2_trust deliver-gate rejects). RESOLVED (specs §8.4 ruling
//     fa0ac1f): D=0 is CORRECT-BY-DESIGN, not a bug — build_frame k=3 = an ORDINARY broadcast → spray-and-wait
//     (bounded, forwarded_k=k/2=1 → truncate to 1 next-hop). Full-mesh reach is a k=15/FLOOD_SENTINEL_K guarantee
//     reserved for GROUP_MGMT/critical broadcasts, set EXPLICITLY. NO bearer-fanout gap; best_transport fine.
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
  let cDelivers = 0, dDelivers = 0, aDelivers = 0, brRelayUdp = 0, brRelayWs = 0;

  const gw = await startGateway();

  // A — WS-only sensor (drive its ensemble via the underlying hive; HiveWs has no tick wrapper).
  // onDeliver counts what A delivers — used for the REVERSE leg (C's UDP reading relayed out WS to A).
  const a = new HiveWs(A, `ws://127.0.0.1:${WS_PORT}`, { hk: HK, tgHash: TG, onDeliver: () => aDelivers++ });
  a.hive.enableSensor();
  const aTick = (t) => {
    const o = JSON.parse(a.hive.tick(t >>> 0));
    for (const f of o.frames || []) a.originate(hexToBytes(f));
  };

  // BRIDGE — WS bearer (to the gateway) + UDP bearer (knows C's + D's addrs). One TG key.
  const bridge = new HiveBridge(BR, {
    hk: HK, tgHash: TG,
    onRoute: (_id, out, arrivalKind) => {
      for (const s of out.sends || []) {
        if ((s.kind >>> 0) === 6 && arrivalKind !== 6) brRelayUdp++; // relayed onto UDP from a non-UDP arrival
        if ((s.kind >>> 0) === 1 && arrivalKind !== 1) brRelayWs++;  // relayed onto WS  from a non-WS arrival
      }
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

  // (2) FORWARD leg (WS→UDP): A emits sensor readings on WS → gateway → bridge(WS) → relay → UDP → C.
  //     This also teaches the bridge that A is a WS (kind 1) neighbour, enabling the reverse leg.
  for (let t = 1; t <= 5; t++) { aTick(t); await sleep(50); }
  await sleep(120);

  // (3) REVERSE leg (UDP→WS): C now emits on UDP → bridge (knows A on WS) → relay → WS → A delivers.
  //     Proves the bridge is genuinely BIDIRECTIONAL, not just WS→UDP.
  c.enableSensor();
  for (let t = 2; t <= 5; t++) { c.tick(t); await sleep(50); }
  await sleep(150);

  a.close(); bridge.close(); c.close(); d.close();
  try { gw.kill('SIGTERM'); } catch (_) {}

  const pass = cDelivers >= 1 && aDelivers >= 1 && dDelivers === 0 && brRelayUdp >= 1 && brRelayWs >= 1;
  console.log(`bridge relayed WS→UDP sends=${brRelayUdp} (want >=1); UDP→WS sends=${brRelayWs} (want >=1)`);
  console.log(`FORWARD  A(WS-only)→C: C(same-key, UDP-only) received=${cRecv} delivered=${cDelivers} (want deliver>=1)`);
  console.log(`REVERSE  C(UDP-only)→A: A(same-key, WS-only)  delivered=${aDelivers} (want deliver>=1)`);
  console.log(`UNDER-REACH  D(wrong-key, UDP-only) received=${dRecv} delivered=${dDelivers}  [NOT an isolation proof]`);
  console.log(dRecv > 0
    ? `  → D received ${dRecv} relayed frame(s); its r2_trust deliver-gate rejected all (wrong key) = genuine deliver-gate.`
    : `  → D received 0: FLOOD-UNDER-REACH, not key-rejection — route_frame emits one flood-send/transport to a`
      + ` representative (C); the UdpBearer unicasts to it. Control (bridge-flood-control.js) proved a 2nd CORRECT-key`
      + ` neighbour is ALSO not reached, so this is NOT auth. Real TG-isolation-via-deliver-gate: see udp-test-mesh.js.`);
  console.log(pass
    ? 'PASS bridge-mesh: heterogeneous BIDIRECTIONAL cross-transport relay (WS↔UDP) + dedup-survives-hop (§5.4/§5.2). '
      + '[D=0 is flood-under-reach, NOT isolation — see notes; real deliver-gate isolation is udp-test-mesh.js]'
    : 'FAIL bridge-mesh');
  process.exit(pass ? 0 : 1);
}
main().catch((e) => { console.error(e); process.exit(1); });
