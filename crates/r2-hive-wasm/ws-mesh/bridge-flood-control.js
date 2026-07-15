// RESOLUTION (2026-07-04, specs §8.4 ruling fa0ac1f): the "under-reach" this probe surfaced is CORRECT-BY-DESIGN,
// NOT a bug. build_frame's k=3 = an ORDINARY broadcast → spray-and-wait (enforce_ttl_k forwarded_k=k/2=1 →
// build_flood_plan confidence-truncates to 1 next-hop). K is an originator STRATEGY choice (§8.4 item 1), NOT
// derived from target=0; flood (k=15/FLOOD_SENTINEL_K = full-mesh reach) is RESERVED for GROUP_MGMT + critical
// broadcasts (item 4) and must be set EXPLICITLY at build time. A k=15 re-run of this probe floods ALL of C/E/F
// (empirically confirmed the mechanism); k=3 sprays to 1. best_transport was never the issue (core vindicated).
//
// bridge-flood-control — CONTROL experiment to falsify my "relay-targeting excludes the wrong-key
// neighbour (auth-gated)" claim. specs' ruling: relay is UNCONDITIONALLY TG-agnostic and the relay layer
// (r2_route) architecturally CANNOT hold/check a TG key (R2-RUNTIME §13.2). So route_frame (= route_inbound_sync,
// trust-agnostic) cannot be excluding D on auth. Then WHY did the flood target only C, not D?
//
// The discriminator: give the bridge THREE UDP neighbours — C(correct key), E(correct key), F(WRONG key) —
// all viable kind-6 links. Route ONE broadcast and see which get a kind-6 relay send:
//   • {C,E} both targeted, F not  → the exclusion really IS key-correlated (a real anomaly vs canon; investigate
//                                    r2_route) — my "relay-targeting auth-gate" would stand as a bug, not a feature.
//   • only ONE of {C,E} targeted   → the flood emits ONE send per TRANSPORT (a shared-bearer broadcast model), so
//                                    "D excluded" was a MIS-OBSERVATION: it was never about D's key — the flood
//                                    just picks one representative per kind. My finding COLLAPSES → tell specs.
'use strict';
const { HiveUdp } = require('./hive-udp');
const { HiveBridge, UdpBearer } = require('./hive-bridge');

const TG = 0x1234abcd >>> 0;
const HK = Array.from({ length: 32 }, (_, i) => (i * 7 + 1) & 0xff);
const WRONG = Array.from({ length: 32 }, () => 0xaa);
const BR = 0x000000b0, C = 0x000000c3, E = 0x000000e6, F = 0x000000f7, X = 0x0000001a;
const P_BR = 21171, P_C = 21172, P_E = 21173, P_F = 21174, P_X = 21175;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Test body - see the file head for the scenario + pass/fail criteria.
async function main() {
  const bridge = new HiveBridge(BR, { hk: HK, tgHash: TG });
  bridge.addBearer(new UdpBearer({ peers: { [C]: `127.0.0.1:${P_C}`, [E]: `127.0.0.1:${P_E}`, [F]: `127.0.0.1:${P_F}` }, bindPort: P_BR }, 6));
  const c = new HiveUdp(C, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: HK,    tgHash: TG, bindPort: P_C });
  const e = new HiveUdp(E, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: HK,    tgHash: TG, bindPort: P_E });
  const f = new HiveUdp(F, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: WRONG, tgHash: TG, bindPort: P_F });
  const x = new HiveUdp(X, { peers: {}, hk: HK, tgHash: TG, bindPort: P_X });
  await Promise.all([bridge.connectAll(), c.connect(), e.connect(), f.connect(), x.connect()]);
  await sleep(80);

  c.originate(c.buildHeartbeat(1)); // correct key
  e.originate(e.buildHeartbeat(1)); // correct key
  f.originate(f.buildHeartbeat(1)); // WRONG key
  await sleep(200);

  const nbrs = JSON.parse(bridge.hive.neighbours()).map((n) => n.hive_id >>> 0);
  const xFrame = x.buildFrame(0, 0x11223344, [1, 2, 3], 7); // broadcast, correct-key
  const routed = JSON.parse(bridge.hive.route_frame(0, 1, xFrame, 3, 0.5));
  const udpTargets = (routed.sends || []).filter((s) => (s.kind >>> 0) === 6).map((s) => s.target >>> 0);
  bridge.close(); c.close(); e.close(); f.close(); x.close();

  const has = (id) => udpTargets.includes(id);
  console.log(`neighbour links formed: ${nbrs.map((n) => '0x' + n.toString(16)).join(',')}`);
  console.log(`route_frame FULL sends: ${JSON.stringify((routed.sends || []).map((s) => ({ kind: s.kind, target: '0x' + (s.target >>> 0).toString(16) })))}`);
  console.log(`route_frame outcome=${routed.outcome}  kind-6 targets=[${udpTargets.map((t) => '0x' + t.toString(16)).join(',')}]`);
  console.log(`  C(correct)=${has(C)}  E(correct)=${has(E)}  F(wrong)=${has(F)}`);
  console.log('');
  const correctTargeted = (has(C) ? 1 : 0) + (has(E) ? 1 : 0);
  if (correctTargeted >= 2 && !has(F)) {
    console.log('RESULT: both correct-key neighbours targeted, wrong-key NOT → exclusion IS key-correlated.');
    console.log('  Since r2_route cannot check auth (§13.2), this is an ANOMALY vs the TG-agnostic-relay canon →');
    console.log('  investigate r2_route / the wasm route path; my "relay-targeting auth-gate" stands as a BUG.');
  } else if (correctTargeted <= 1) {
    console.log('RESULT: at most ONE correct-key neighbour targeted → the flood emits ONE send per TRANSPORT');
    console.log('  (shared-bearer broadcast model). "D excluded" was a MIS-OBSERVATION — never about the key.');
    console.log('  My relay-targeting-auth finding COLLAPSES; no AB-003/004 tension. Correct the record + tell specs.');
  } else {
    console.log(`RESULT: ambiguous — C=${has(C)} E=${has(E)} F=${has(F)}; investigate further.`);
  }
  process.exit(0);
}
main().catch((e) => { console.error(e); process.exit(1); });
