// test-mesh — PROVE the WASM-WS binding + the real deliver-gate over a REAL WebSocket (#26).
// Spawns the gateway (separate process = real sockets), connects 3 WasmHives:
//   A + B share TG key hk1 (same tg_hash);  C has the WRONG key hk2 (same tg_hash = the carrier
//   mismatch scenario). A fires a GroupHmac-SIGNED heartbeat onto the WS bearer. The gateway
//   broadcasts it; each hive runs the REAL r2_trust deliver-gate (verify_frame):
//     B  → tg_ok && hmac_ok → DELIVER   (co-member, over real WS)
//     C  → tg_ok but hmac_ok=false → REJECT  (TG isolation: same tg_hash, wrong key)
// Exit 0 = pass. Zero npm deps (Node global WebSocket client + zero-dep gateway).
'use strict';
const { spawn } = require('child_process');
const path = require('path');
const { HiveWs } = require('./hive-ws');

const PORT = 21058;
const URL = `ws://127.0.0.1:${PORT}`;
const A = 0x0000000a, B = 0x0000000b, C = 0x0000000c;
const TG = 0x00001234;                         // shared tg_hash (target_group)
const HK1 = Array(32).fill(0x11);              // A + B key
const HK2 = Array(32).fill(0x22);              // C's WRONG key (same TG id)

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
function die(m) { console.error(`FAIL: ${m}`); process.exitCode = 1; }

// Test body - see the file head for the scenario + pass/fail criteria.
async function main() {
  const gw = spawn('node', [path.join(__dirname, 'gateway.js'), String(PORT)], { stdio: ['ignore', 'inherit', 'pipe'] });
  await new Promise((resolve, reject) => {
    let up = false;
    gw.stderr.on('data', (d) => { process.stderr.write(d); if (!up && /listening/.test(String(d))) { up = true; resolve(); } });
    gw.on('exit', (c) => { if (!up) reject(new Error(`gateway exited early (${c})`)); });
    setTimeout(() => { if (!up) reject(new Error('gateway start timeout')); }, 4000);
  });

  const del = { [B]: 0, [C]: 0 };       // deliver-gate acceptances
  const okhmac = { [B]: 0, [C]: 0 };    // of which hmac-verified
  const mk = (id, hk) => new HiveWs(id, URL, {
    hk, tgHash: TG,
    onDeliver: (who, _b, gate) => { del[who]++; if (gate.hmac_ok) okhmac[who]++; },
  });
  const hiveA = mk(A, HK1), hiveB = mk(B, HK1), hiveC = mk(C, HK2);
  await Promise.all([hiveA.connect(), hiveB.connect(), hiveC.connect()]);
  await sleep(150);

  // A fires a SIGNED heartbeat (keyed → sign_extended, target_group=TG) onto the real WS bearer.
  const hb = hiveA.buildHeartbeat(1);
  if (!hb || hb.length === 0) return die('build_heartbeat returned empty');
  console.error(`# A(${A.toString(16)}) fires ${hb.length}B SIGNED heartbeat over WS → bearer broadcasts`);
  hiveA.originate(hb);
  await sleep(400);

  // Assertions: B (co-member) delivers with hmac_ok; C (wrong key) never delivers.
  let ok = true;
  if (del[B] >= 1 && okhmac[B] >= 1) console.error(`PASS: B delivered A's HB over real WS (deliver+hmac_ok ×${okhmac[B]})`);
  else { die(`B did not hmac-deliver (deliver=${del[B]} hmac_ok=${okhmac[B]})`); ok = false; }
  if (del[C] === 0) console.error(`PASS: C (wrong key, same tg_hash) REJECTED — TG isolation holds over WS`);
  else { die(`C wrongly delivered ${del[C]} (isolation breach!)`); ok = false; }
  if (ok) console.error('PASS: WASM-WS binding + real deliver-gate + TG isolation over a real WebSocket');

  hiveA.close(); hiveB.close(); hiveC.close();
  await sleep(50);
  gw.kill();
}

main().catch((e) => { die(e.stack || String(e)); process.exit(1); });
