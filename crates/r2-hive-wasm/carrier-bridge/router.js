// r2-carrier router — the wasm-hive routing BRAIN for the carrier bridge.
//
// Pure stdin -> route -> stdout. It has NO serial access (the Python parent owns
// the port) so it CANNOT touch DTR/RTS and CANNOT brick a board. Each stdin line
// is one inbound R2-WIRE frame (hex) the carrier heard over the air; we run the
// REAL current-TN routing core (r2-hive-wasm over route_inbound_sync) and print
// `INJECT <hex>` for every frame the hive decides to (re)transmit. Diagnostics go
// to stderr / `# `-prefixed stdout lines (the parent ignores those for serial).
const path = require('path');
const readline = require('readline');

const hiveId = (parseInt(process.argv[2] || 'a1f5ed00', 16) >>> 0);
const pkgDir = process.argv[3]
  ? path.resolve(process.argv[3])
  : path.join(__dirname, 'wasmhive-node');

const wh = require(path.join(pkgDir, 'r2_hive_wasm.js'));
const hive = new wh.WasmHive(hiveId);
process.stderr.write(`# router: wasm-hive v${wh.version()} hive=${hiveId.toString(16)} ready\n`);

const t0 = Date.now();
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', (line) => {
  const hex = line.trim();
  if (!hex) return;
  let bytes;
  try {
    bytes = Uint8Array.from(Buffer.from(hex, 'hex'));
  } catch (e) {
    return;
  }
  if (bytes.length === 0 || bytes.length * 2 !== hex.length) return; // not clean hex
  const now = Math.floor((Date.now() - t0) / 1000);
  // arrival kind 5 = EspNow (the carrier's medium); source_hive 0 = derive from route-stack.
  const out = hive.route_frame(0, 5, bytes, now, 0.5);
  let res;
  try {
    res = JSON.parse(out);
  } catch (e) {
    return;
  }
  process.stdout.write(`# route ${res.outcome} sends=${res.sends.length}\n`);
  for (const s of res.sends) {
    process.stdout.write(`INJECT ${s.frame}\n`);
  }
});
rl.on('close', () => process.exit(0));
