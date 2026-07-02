// udp-test-mesh — e2e over REAL UDP sockets. A(sensor) + B(receiver) share a TG GroupHmac key;
// C has the WRONG key. A's SENSOR tick emits an r2.tn.routetest reading, unicast to every peer.
// B delivers it (same key → hmac_ok), C rejects it (wrong key). Proves the unicast §4.4 UDP bearer +
// the ensemble SENSOR role + the R2-TRUST §7.5.4 TG deliver-gate over an actual socket — the same
// wire a Linux r2-hive UDP peer speaks.
'use strict';
const { HiveUdp } = require('./hive-udp');

const TG = 0x1234abcd >>> 0;
const HK = Array.from({ length: 32 }, (_, i) => (i * 7 + 1) & 0xff);
const WRONG = Array.from({ length: 32 }, () => 0xaa);

const A = 0x000000a1, B = 0x000000b2, C = 0x000000c3;
const PA = 21140, PB = 21141, PC = 21142;
const peers = { [A]: `127.0.0.1:${PA}`, [B]: `127.0.0.1:${PB}`, [C]: `127.0.0.1:${PC}` };

let bDelivers = 0, cDelivers = 0;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main() {
  const a = new HiveUdp(A, { peers, hk: HK, tgHash: TG, bindPort: PA });
  const b = new HiveUdp(B, { peers, hk: HK, tgHash: TG, bindPort: PB, onDeliver: () => bDelivers++ });
  const c = new HiveUdp(C, { peers, hk: WRONG, tgHash: TG, bindPort: PC, onDeliver: () => cDelivers++ });
  a.enableSensor();
  await Promise.all([a.connect(), b.connect(), c.connect()]);
  for (let t = 1; t <= 4; t++) { a.tick(t); await sleep(40); }
  await sleep(120);
  a.close(); b.close(); c.close();

  const pass = bDelivers >= 1 && cDelivers === 0;
  console.log(`B(same-key) delivered=${bDelivers} (want >=1); C(wrong-key) delivered=${cDelivers} (want 0)`);
  console.log(pass ? 'PASS udp-mesh: unicast bearer + SENSOR role + TG deliver-gate over real UDP'
                   : 'FAIL udp-mesh');
  process.exit(pass ? 0 : 1);
}
main().catch((e) => { console.error(e); process.exit(1); });
