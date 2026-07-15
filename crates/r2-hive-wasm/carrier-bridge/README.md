# r2-carrier-bridge — host ↔ DFR1195 carrier ↔ R2 mesh

Makes a host (Alfred) a **real R2 mesh node** through ONE DFR1195 flashed as a
transparent serial↔ESP-NOW *carrier*. The carrier mirrors every over-the-air
R2-WIRE frame to the host (`R2RX <hex>`) and broadcasts frames the host hands it
(`INJECT <hex>`). This bridge runs the **real current-TN routing core** (the
r2-hive wasm-hive) over that link.

```
  4 mesh nodes ──air/ESP-NOW──▶ CARRIER DFR1195 ──USB R2RX──▶ this bridge
                                       ▲                          │
                                       └────── USB INJECT ◀─ wasm-hive route
```

## Setup (vendored deps are gitignored — recreate once)
```sh
# 1. wasm-hive nodejs pkg (from crates/r2-hive-wasm):
wasm-pack build --release --target nodejs --out-dir carrier-bridge/wasmhive-node
# 2. vendored pyserial (pure-python; no install/sudo needed):
curl -sSL -o /tmp/ps.whl https://files.pythonhosted.org/packages/07/bc/587a445451b253b285629263eb51c2d8e9bcea4fc97826266d186f96f558/pyserial-3.5-py2.py3-none-any.whl
( cd carrier-bridge && unzip -oq /tmp/ps.whl 'serial/*' )
```
(Already done + scp'd to Alfred at `~/carrier-bridge/` for the EOD bench.)

## Pieces
- `r2-carrier-bridge.py` — owns the serial port **DTR/RTS-safe** (the only thing
  that touches the port; a careless open bricks an unreachable board).
- `router.js` — the wasm-hive routing brain (node). No serial access → cannot brick.
- `serial/` — vendored pyserial 3.5 (pure-python; no install needed).
- `wasmhive-node/` — the current-TN wasm-hive (nodejs target).
- `run-bridge.sh` — sets PYTHONPATH to the vendored pyserial and runs the bridge.

## ⚠ DTR/RTS SAFETY (remote bench — no power-cycle)
The ESP32-S3 native USB-Serial-JTAG reacts to DTR/RTS; a bad open can drop a
board into ROM download = silent until power-cycle, which Roy can't do remotely.
The bridge opens with **DTR=0, RTS=0 set BEFORE open and never toggled**, and
aborts if it can't. Use it on the **carrier** board (new firmware has clear-at-boot
so an open-time reset recovers to the app, never sticky download).

## Run (on Alfred)
```sh
# heartbeat-VISIBILITY + routing (read-only: logs would-be INJECTs, sends nothing):
./run-bridge.sh --port /dev/serial/by-id/usb-Espressif_USB_JTAG_serial_debug_unit_<MAC>-if00 --hive a1f5ed00

# PARTICIPATE (actually inject the hive's relay frames onto the mesh):
./run-bridge.sh --port <dev> --hive a1f5ed00 --participate

# visibility only (no node router):
./run-bridge.sh --port <dev> --no-route

# prove the wiring with no serial port touched:
./run-bridge.sh --selftest
```

`--participate` is OFF by default — the bridge watches + logs what it *would*
inject, so an unattended run never spams the mesh. Flip it on once you want the
host to actively relay.

### `--control` (host injects frames LIVE — weave in the browser/IP hives)

```sh
./run-bridge.sh --port <dev> --hive a1f5ed00 --participate --control
```

With `--control` the bridge reads its **stdin** for host-injected frames — the path
that crosses a browser/IP wasm hive's outbound frame onto the radio via the carrier:

| line        | effect                                                                       |
|-------------|------------------------------------------------------------------------------|
| `RX <hex>`  | feed to the carrier hive (**relay** path — routed/deduped/re-flooded as the hive decides; the carrier acts as a repeater). Use for normal TG traffic. No-op under `--no-route`. |
| `TX <hex>`  | write `INJECT <hex>` to serial **verbatim** (transparent egress, bypasses routing). Use when the frame is already fully addressed. Honors `--participate`. |

For a *TG-member* browser hive, sign its frame with `WasmHive.withGroupHmac(hk,tgHash)`
first so it `verifyFrame()`s on the real boards — then `RX` it for repeater relay, or
`TX` it for a dumb pipe. EOF on stdin just ends the channel; the bridge keeps running.

### `--json` (dashboard ingest)
With `--json`, **stdout is pure JSON-lines** (human diagnostics → stderr), one
object per event:
```
{"t":<epoch>,"kind":"peer_mapped","hive":"502698aa","mac":"f4:12:fa:50:26:98"}
{"t":<epoch>,"kind":"frame","bytes":10,"hex":"045300001000aabbccdd"}
{"t":<epoch>,"kind":"route","outcome":"Flooded","sends":1}
{"t":<epoch>,"kind":"inject","hex":"0441…","sent":false}
```
So an adapter can `for line in proc.stdout: ev = json.loads(line)` without
regex-parsing the human view. Combine with `--participate` to set `inject.sent`.

## Test vector (proves the full R2RX→route→INJECT loop)
Feed these two `R2RX` frames in order; the second relays to the learned neighbour:
```
echo -e '045300001000aabbccdd00000000000000000000000101000000aa\n045300001234aabbccdd0000000000000000000000aa01000000bb' | node router.js a1f5ed00
# => # route Flooded sends=1
#    INJECT 044100001234aabbccdd0000000000000000000000aa02000000bba1f5ed00
```
(The host hive `a1f5ed00` is appended to the route stack = it relayed.)

## Heartbeat-visibility (minimal path — ONE Roy command, no node)
The simplest real-HW signal needs no bridge at all: after flashing the carrier,
`espflash flash --monitor --chip esp32s3 ~/r2-dfr1195-carrier.elf` streams the
carrier's `R2RX`/`ESP-NOW peer MAPPED` lines = live proof it hears the mesh.
The bridge adds participation (the host injects too).
