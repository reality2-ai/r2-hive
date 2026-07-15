// bridge-neighbour-probe — DISCRIMINATOR for specs' AB-004 tension (#26). The bridge test showed a
// wrong-key node D is never RELAYED to. specs (r2-transient-networking-conjectures TN-L0-XT-AB-004,
// Roy-locked): neighbour FORMATION must NOT require GroupHmac/TG verification — a below-L5 node forms a
// link from ANY heard frame; GroupHmac gates only above-L5 delivery/dispatch. So "D never relayed to"
// is ambiguous:
//   (A) D FORMED a link entry but relay-selection applies a SEPARATE auth check → NEW security mechanism.
//   (B) D got NO link entry at all → AB-004-class formation-auth-gating regression in the wasm path = a BUG.
// The discriminator: does D appear in the bridge's neighbour table (a last_seen/link entry) at all?
'use strict';
const { HiveUdp } = require('./hive-udp');
const { HiveBridge, UdpBearer } = require('./hive-bridge');

const TG = 0x1234abcd >>> 0;
const HK = Array.from({ length: 32 }, (_, i) => (i * 7 + 1) & 0xff);
const WRONG = Array.from({ length: 32 }, () => 0xaa);
const BR = 0x000000b0, C = 0x000000c3, D = 0x000000d4;
const P_BR = 21161, P_C = 21162, P_D = 21163;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Test body - see the file head for the scenario + pass/fail criteria.
async function main() {
  const bridge = new HiveBridge(BR, { hk: HK, tgHash: TG });
  bridge.addBearer(new UdpBearer({ peers: { [C]: `127.0.0.1:${P_C}`, [D]: `127.0.0.1:${P_D}` }, bindPort: P_BR }, 6));
  const c = new HiveUdp(C, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: HK,    tgHash: TG, bindPort: P_C });
  const d = new HiveUdp(D, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: WRONG, tgHash: TG, bindPort: P_D });

  await Promise.all([bridge.connectAll(), c.connect(), d.connect()]);
  await sleep(80);

  // Both announce over UDP: C with the CORRECT TG key, D with the WRONG key. Same frame shape
  // (build_heartbeat), same transport — the ONLY difference is the GroupHmac key.
  c.originate(c.buildHeartbeat(1));
  d.originate(d.buildHeartbeat(1));
  await sleep(200);

  // STEP 1 — FORMATION: does each announce form a link entry (regardless of key)?
  const nbrs = JSON.parse(bridge.hive.neighbours());
  const ids = nbrs.map((n) => n.hive_id >>> 0);
  const hasC = ids.includes(C), hasD = ids.includes(D);
  console.log('bridge neighbour table:', JSON.stringify(nbrs));
  console.log(`STEP1 FORMATION: C(correct-key) link=${hasC}  D(wrong-key) link=${hasD}`);

  // STEP 2 — RELAY-TARGETING: route a broadcast frame from a 4th TG member X through the bridge (arriving
  // as WS/kind 1). Both C and D are viable kind-6 links → a TG-agnostic flood would target BOTH. Who does
  // route_frame actually emit a kind-6 send to?
  const X = 0x000000e5;
  const x = new HiveUdp(X, { peers: {}, hk: HK, tgHash: TG, bindPort: 21164 });
  await x.connect();
  const xFrame = x.buildFrame(0, 0x11223344, [1, 2, 3], 7); // broadcast (target 0), TG-signed
  const routed = JSON.parse(bridge.hive.route_frame(0, 1, xFrame, 3, 0.5));
  const udpTargets = (routed.sends || []).filter((s) => (s.kind >>> 0) === 6).map((s) => s.target >>> 0);
  const relayC = udpTargets.includes(C), relayD = udpTargets.includes(D);
  console.log(`STEP2 RELAY: route_frame(X broadcast) outcome=${routed.outcome} udp-targets=[${udpTargets.map((t) => '0x' + t.toString(16)).join(',')}]`);
  console.log(`             relayed-toward C=${relayC}  D=${relayD}`);

  bridge.close(); c.close(); d.close(); x.close();
  console.log('');
  if (!hasD) {
    console.log('VERDICT (B — BUG): D has NO LINK ENTRY → the wasm route path gates FORMATION on authentication,');
    console.log('   violating AB-004 (formation must be TG-agnostic). AB-004-class regression → flag core/hive.');
  } else if (relayC && !relayD) {
    console.log('VERDICT (A — NEW MECHANISM): D FORMED a link (AB-004 ok) but relay-TARGET selection excludes it →');
    console.log('   a SEPARATE additive auth check at relay-targeting, distinct from §2.1.3 formation. Catalogue it.');
  } else if (relayC && relayD) {
    console.log('VERDICT (C — MIS-OBSERVED): D forms a link AND is relayed toward (flood is TG-agnostic at BOTH');
    console.log('   formation and relay). So the bridge-test D=0 was a DELIVER-GATE rejection after all (D received');
    console.log('   the frame, its r2_trust gate rejected the wrong key) — my "neighbour-exclusion" label was WRONG.');
  } else {
    console.log(`VERDICT (?): unexpected — relayC=${relayC} relayD=${relayD}; investigate.`);
  }
  process.exit(0);
}
main().catch((e) => { console.error(e); process.exit(1); });
