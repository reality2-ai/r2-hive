// bridge-deliver-gate — the GENUINE TG-isolation proof that bridge-test-mesh.js explicitly is NOT.
//
// bridge-test-mesh showed D(wrong-key) DELIVERED 0 — but D also RECEIVED 0 (flood-under-reach: an
// ORDINARY k=3 broadcast sprays to ~1 next-hop, so the wrong-key neighbour was never even reached).
// D-delivered-0 there proves NOTHING about the deliver-gate. To exercise the r2_trust deliver-gate you
// must first REACH the wrong-key neighbour, THEN observe it REJECT. That needs FULL-MESH reach — a
// CRITICAL broadcast with k set EXPLICITLY = FLOOD_SENTINEL_K (15) per R2-ROUTE §8.4 (K is by-CRITICALITY,
// never derived from target). At k=15, enforce_ttl_k enters flood_mode and build_flood_plan floods ALL
// viable neighbours (skips the confidence-truncation), so D RECEIVES — and its deliver-gate rejects the
// wrong key. This is the sanctioned companion to build_frame (k=3); see WasmHive::build_critical_frame.
//
// Two proofs, on ONE topology — bridge (correct key) with UDP neighbours C(correct key) + D(WRONG key):
//   (1) FLOOD-PLAN DISCRIMINATOR (synchronous, deterministic): route the SAME broadcast at k=3 vs k=15
//       through the bridge's route core and inspect sends[]. k=3 sprays to a SUBSET (forwarded_k=k/2=1);
//       k=15 floods to BOTH C and D. The K-tier mechanism, isolated from sockets/timing.
//   (2) DELIVER-GATE UNDER FLOOD (async e2e): a source floods a k=15 CRITICAL frame → the bridge relays
//       it to C AND D over real UDP sockets. C(correct) RECEIVES + DELIVERS; D(wrong) RECEIVES (dRecv>0)
//       but its r2_trust deliver-gate REJECTS (dDeliver=0). THIS is the genuine isolation proof —
//       received>0 with delivered=0 disambiguates "gate rejected" from "never reached" (the mesh caveat).
//
// Contrast with udp-test-mesh.js (a DIRECT wrong-key unicast rejected at the gate): there the reach is
// trivial (point-to-point); here reach is earned through a RELAY's flood, so it also proves the signed
// GroupHmac span survives the relay-append (else C would fail the gate too, confounding the result).
'use strict';
const { HiveUdp } = require('./hive-udp');
const { HiveBridge, UdpBearer } = require('./hive-bridge');

const TG = 0x1234abcd >>> 0;
const HK = Array.from({ length: 32 }, (_, i) => (i * 7 + 1) & 0xff);
const WRONG = Array.from({ length: 32 }, () => 0xaa);
const HASH = 0x11223344 >>> 0;

const BR = 0x000000b0, C = 0x000000c3, D = 0x000000d4, X = 0x0000001a;
const P_BR = 21180, P_C = 21181, P_D = 21182, P_X = 21183;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Test body - see the file head for the scenario + pass/fail criteria.
async function main() {
  let cRecv = 0, cDeliver = 0, dRecv = 0, dDeliver = 0;
  const floodTargets = []; // bridge's kind-6 sends[] during the async k=15 relay

  // BRIDGE — correct key; UDP bearer knows C's + D's addrs. onRoute records the real dispatched targets.
  const bridge = new HiveBridge(BR, {
    hk: HK, tgHash: TG,
    onRoute: (_id, out) => {
      for (const s of out.sends || []) if ((s.kind >>> 0) === 6) floodTargets.push(s.target >>> 0);
    },
  });
  bridge.addBearer(new UdpBearer({ peers: { [C]: `127.0.0.1:${P_C}`, [D]: `127.0.0.1:${P_D}` }, bindPort: P_BR }, 6));

  // C(correct) / D(WRONG) — UDP-only receivers. onRoute fires on EVERY non-echo frame that ARRIVES
  // (before/independent of the deliver-gate); onDeliver fires only when verifyFrame passes. The pair
  // disambiguates "gate rejected" (recv>0, deliver=0) from "never reached" (recv=0).
  const c = new HiveUdp(C, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: HK,    tgHash: TG, bindPort: P_C, onDeliver: () => cDeliver++, onRoute: () => cRecv++ });
  const d = new HiveUdp(D, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: WRONG, tgHash: TG, bindPort: P_D, onDeliver: () => dDeliver++, onRoute: () => dRecv++ });
  // X — the frame source (correct key). Only seeds the bridge; floods critical frames INTO it.
  const x = new HiveUdp(X, { peers: { [BR]: `127.0.0.1:${P_BR}` }, hk: HK, tgHash: TG, bindPort: P_X });

  await Promise.all([bridge.connectAll(), c.connect(), d.connect(), x.connect()]);
  await sleep(80);

  // C + D announce over UDP → the bridge's route core learns BOTH as kind-6 neighbours (formation is
  // TG-agnostic per AB-004, so the wrong-key D forms a link too — the whole point).
  c.originate(c.buildHeartbeat(1));
  d.originate(d.buildHeartbeat(1));
  await sleep(120);

  // ── PROOF (1): FLOOD-PLAN DISCRIMINATOR (synchronous, deterministic) ─────────────────────────────
  // Route the SAME broadcast at k=3 (ordinary) vs k=15 (critical) through the bridge's route core and
  // inspect the flood plan. source_hive=X excludes X from its own flood (F2), so targets are just C/D.
  const frameK3 = x.buildFrame(0, HASH, [1, 2, 3], 10);          // k=3 ordinary broadcast
  const frameK15 = x.buildCriticalFrame(0, HASH, [1, 2, 3], 11); // k=15 CRITICAL broadcast
  const planTargets = (frame) => {
    const routed = JSON.parse(bridge.hive.route_frame(X >>> 0, 6, frame, 5, 0.5));
    return (routed.sends || []).filter((s) => (s.kind >>> 0) === 6).map((s) => s.target >>> 0);
  };
  const k3Targets = planTargets(frameK3);
  const k15Targets = planTargets(frameK15);
  const k15HasC = k15Targets.includes(C), k15HasD = k15Targets.includes(D);

  // ── PROOF (2): DELIVER-GATE UNDER FLOOD (async e2e over real sockets) ────────────────────────────
  // X floods a k=15 CRITICAL frame INTO the bridge → the bridge relays it to C AND D over UDP.
  // (distinct msg_id from the plan-check frames above, so the bridge's dedup doesn't swallow it.)
  x.originate(x.buildCriticalFrame(0, HASH, [9, 9, 9], 12));
  await sleep(200);

  bridge.close(); c.close(); d.close(); x.close();

  const fmt = (a) => `[${a.map((t) => '0x' + t.toString(16)).join(',')}]`;
  console.log(`PLAN k=3  (ordinary) kind-6 targets=${fmt(k3Targets)}  count=${k3Targets.length}  → SPRAY (forwarded_k=1)`);
  console.log(`PLAN k=15 (CRITICAL) kind-6 targets=${fmt(k15Targets)}  count=${k15Targets.length}  C=${k15HasC} D=${k15HasD}  → FLOOD (all viable)`);
  console.log(`E2E  bridge relayed k=15 to kind-6 targets=${fmt(floodTargets)}`);
  console.log(`E2E  C(correct-key) received=${cRecv} delivered=${cDeliver}  (want recv>=1 deliver>=1)`);
  console.log(`E2E  D(WRONG-key)   received=${dRecv} delivered=${dDeliver}  (want recv>=1 deliver=0 = deliver-gate REJECT)`);

  // The K-tier discriminator: k=3 sprays to strictly fewer next-hops than k=15 floods, and only k=15
  // reaches BOTH C and D. THEN, having reached D, the deliver-gate is what stops delivery — not reach.
  const planOk = k3Targets.length < k15Targets.length && k15HasC && k15HasD;
  const gateOk = cRecv >= 1 && cDeliver >= 1 && dRecv >= 1 && dDeliver === 0;
  const pass = planOk && gateOk;

  console.log('');
  console.log(planOk
    ? '  ✓ PLAN: k=3 sprays to a subset; k=15 floods BOTH C and D (K by-criticality, §8.4).'
    : '  ✗ PLAN: expected k=3 count < k=15 count with k=15 reaching both C and D.');
  console.log(gateOk
    ? '  ✓ GATE: D RECEIVED the flooded frame yet delivered 0 → r2_trust deliver-gate REJECTED the wrong key.'
    : '  ✗ GATE: expected D recv>=1 & deliver=0 (reached-then-rejected) and C recv>=1 & deliver>=1.');
  console.log(pass
    ? 'PASS bridge-deliver-gate: genuine TG isolation — full-mesh reach (k=15) delivers to the correct key '
      + 'and the deliver-gate rejects the wrong key AT the reached node (§8.4 flood + r2_trust §7.5.4).'
    : 'FAIL bridge-deliver-gate');
  process.exit(pass ? 0 : 1);
}
main().catch((e) => { console.error(e); process.exit(1); });
