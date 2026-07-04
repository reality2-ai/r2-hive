#!/usr/bin/env python3
r"""r2-carrier-bridge — host (Alfred) bridge to a DFR1195 CARRIER over USB-serial.

Closes the all-radio-via-MCU loop: the carrier mirrors every over-the-air R2-WIRE
frame to the host as `R2RX <hex>`; this bridge feeds those frames to the REAL
current-TN routing core (the r2-hive wasm-hive, via the node `router.js` child) and
writes the hive's relay decisions back as `INJECT <hex>` — so the host PARTICIPATES
in the R2 mesh as a real node, not a passive scanner.

It also surfaces heartbeat-VISIBILITY directly (the carrier's `ESP-NOW peer MAPPED`
lines = real over-the-air heartbeat reception), so visibility works even with
routing disabled.

╔══════════════════════════════════════════════════════════════════════════════╗
║ DTR/RTS SAFETY — the bench is REMOTE. A board knocked into ROM download mode    ║
║ goes SILENT and Roy CANNOT power-cycle it. The ESP32-S3 native USB-Serial-JTAG  ║
║ reacts to the USB DTR/RTS lines. This tool opens the port with DTR *and* RTS    ║
║ DE-ASSERTED before open and NEVER toggles them, and ABORTS if it cannot set     ║
║ that state. Intended for the CARRIER board (new firmware w/ clear-at-boot, so   ║
║ even an open-time reset recovers to the app — never sticky download).           ║
║ Do NOT point this at an old-firmware board unless you accept that risk.         ║
╚══════════════════════════════════════════════════════════════════════════════╝

Usage (on Alfred, with vendored pyserial on PYTHONPATH — see run-bridge.sh):
  python3 r2-carrier-bridge.py --port /dev/serial/by-id/usb-Espressif_..._-if00 \
      --hive a1f5ed00                 # VISIBILITY + route (logs would-be INJECTs)
  ... add --participate               # actually write INJECT to the mesh
  ... --no-route                      # visibility only (no node child)
  ... --control                       # read this process's STDIN for host-injected frames
  python3 r2-carrier-bridge.py --selftest   # exercise the routing wiring, no serial

HOST CONTROL CHANNEL (--control): the host (e.g. composer's adapter weaving the
browser/IP wasm hives into the mesh) writes lines to this bridge's STDIN:
    RX <hex>   feed the frame to the carrier hive (RELAY path — routed/deduped,
               re-flooded as the hive decides). Use for normal TG traffic so the
               carrier acts as a repeater. (No-op under --no-route.)
    TX <hex>   write `INJECT <hex>` to serial VERBATIM (transparent egress, bypasses
               routing). Use when the frame is already fully addressed. Honors
               --participate (no serial write in read-only mode).
"""
import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))

# Set by main(): when True, every event is emitted as one JSON line (dashboard
# ingest) instead of the human log line. The shape composer asked for:
#   {"t":<epoch>, "kind":"peer_mapped"|"frame"|"route"|"inject", hive?, mac?,
#    "bytes"?, "outcome"?, "sends"?, "hex"?}
JSON_MODE = False
_PEER_RE = re.compile(r"hive=([0-9a-fA-F]+).*mac=([0-9a-fA-F:]+)")


def log(msg):
    # In JSON mode keep stdout PURE JSON: human diagnostics go to stderr.
    stream = sys.stderr if JSON_MODE else sys.stdout
    print(f"{time.strftime('%H:%M:%S')} {msg}", file=stream, flush=True)


def jline(**fields):
    """Emit one machine-readable JSON line (stdout). t = epoch seconds."""
    fields.setdefault("t", round(time.time(), 3))
    print(json.dumps(fields, separators=(",", ":")), flush=True)


def open_safe(port, baud=115200):
    """Open the port with DTR+RTS de-asserted BEFORE open and never toggled.

    Aborts loudly if pyserial is missing or the safe state can't be requested —
    a careless open can brick an unreachable board.
    """
    try:
        import serial  # vendored alongside this script (see run-bridge.sh PYTHONPATH)
    except ImportError:
        sys.exit("FATAL: pyserial not importable. Run via run-bridge.sh (sets "
                 "PYTHONPATH to the vendored ./pyserial-vendor). Refusing to "
                 "hand-roll serial I/O near a remote board.")
    s = serial.Serial()
    s.port = port
    s.baudrate = baud
    s.timeout = 1
    # LINE DISCIPLINE (corrected 2026-07-04, solved the zero-bytes mystery): the
    # ESP32-S3 USB-Serial-JTAG console gates its TX on TERMINAL-READY — an open
    # holding DTR=0 reads ZERO console bytes (the firmware sees no host and
    # suppresses println output). The SAFE + WORKING pattern is a STEADY
    # DTR=1 / RTS=0 set *before* open() and never toggled after: the reset/ROM-
    # download hazard is the DTR/RTS TOGGLE DANCE (esptool-style reset
    # sequences), NOT a steady terminal-ready attach. Worst case on first attach
    # is one benign app reboot; attach-once-stay-attached (+ stty -hupcl on the
    # port) prevents any further resets. Field-proven: FR-4 / TN-L2-XT-BL-001
    # raw-serial captures ran this way mid-run on these exact boards.
    s.dtr = True
    s.rts = False
    s.open()
    # Belt-and-braces: hold the state steady after open and verify — RTS must
    # stay LOW (RTS toggling is half of the reset dance).
    s.dtr = True
    s.rts = False
    if not s.dtr or s.rts:
        s.close()
        sys.exit("FATAL: could not hold DTR=1/RTS=0 — aborting rather than "
                 "risk a toggle sequence on an unreachable board.")
    log(f"# opened {port} @ {baud} (DTR=1 RTS=0 steady, terminal-ready, no reset dance)")
    return s


def start_router(hive_hex, pkg_dir):
    """Spawn the node wasm-hive router. It has NO serial access (can't brick)."""
    args = ["node", os.path.join(HERE, "router.js"), hive_hex]
    if pkg_dir:
        args.append(pkg_dir)
    p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, text=True, bufsize=1)
    # surface router stderr (its ready banner / errors)
    threading.Thread(target=_pump_stderr, args=(p,), daemon=True).start()
    return p


def _pump_stderr(p):
    for line in p.stderr:
        log(f"[router] {line.rstrip()}")


def render_rx(line):
    """Surface the carrier's telemetry — heartbeat-VISIBILITY + raw frames."""
    if "peer MAPPED" in line:
        if JSON_MODE:
            m = _PEER_RE.search(line)
            jline(kind="peer_mapped",
                  hive=(m.group(1) if m else None),
                  mac=(m.group(2) if m else None))
        else:
            log(f"OTA-RX  {line}   <- heard a peer's heartbeat over the air")
    elif line.startswith("R2RX "):
        hexstr = line[5:].strip()
        if JSON_MODE:
            jline(kind="frame", bytes=len(hexstr) // 2, hex=hexstr)
        else:
            log(f"FRAME   {len(hexstr) // 2}B over-the-air R2-WIRE frame")
    elif line.lstrip().startswith('{"t":"rt.'):
        # RouteEngine telemetry (R2-DIAGNOSTICS v0.2 §5 Streaming Envelope) from a board's `viz` feature —
        # already a JSON-Lines record (rt.snap / rt.nbr / rt.path, self-keyed by `dev`). Pass it THROUGH
        # verbatim so carrier-r2-adapter.js can forward it to the viz-events WS (:21060) per device. (This
        # bridge forwards the rt.* of whatever serial it reads — run one bridge/adapter per board serial for
        # all-boards coverage; the `dev` field disambiguates.)
        if JSON_MODE:
            print(line.strip(), flush=True)
        else:
            log(f"RT {line.strip()}")
    elif not JSON_MODE:
        log(line)


def router_reader(router, ser, participate):
    """Read the router's INJECT decisions; write them to serial (if participating)."""
    for line in router.stdout:
        line = line.rstrip()
        if not line:
            continue
        if line.startswith("INJECT "):
            hexstr = line[7:].strip()
            sent = bool(participate and ser)
            if sent:
                ser.write((line + "\n").encode())
            if JSON_MODE:
                jline(kind="inject", hex=hexstr, sent=sent)
            elif sent:
                log(f"INJECT> {hexstr[:24]}… (sent to mesh)")
            else:
                log(f"would-INJECT (read-only; --participate to send): {hexstr[:24]}…")
        elif line.startswith("# route "):
            if JSON_MODE:
                parts = line.split()  # ['#','route','<Outcome>','sends=N']
                outcome = parts[2] if len(parts) > 2 else None
                sends = int(parts[3].split("=")[1]) if len(parts) > 3 and "=" in parts[3] else 0
                jline(kind="route", outcome=outcome, sends=sends)
            else:
                log(f"[router] {line}")
        elif not JSON_MODE:  # other diagnostics
            log(f"[router] {line}")


def control_reader(router, ser, participate):
    """Host control channel (this bridge's STDIN) — inject frames LIVE.

    `RX <hex>` → carrier hive router (relay/dedup/re-flood as the hive decides).
    `TX <hex>` → `INJECT <hex>` straight to serial, verbatim (transparent egress).
    This is how the browser/IP wasm hives cross frames onto the radio via the carrier.
    EOF (host closes stdin) just ends the thread; the bridge keeps running.
    """
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        parts = line.split(None, 1)
        verb = parts[0].upper()
        hexstr = parts[1].strip() if len(parts) > 1 else ""
        if verb == "RX":
            if router:
                router.stdin.write(hexstr + "\n")
                router.stdin.flush()
                if JSON_MODE:
                    jline(kind="control", verb="RX", hex=hexstr, routed=True)
                else:
                    log(f"CTRL RX  {hexstr[:24]}… -> carrier hive (relay)")
            elif not JSON_MODE:
                log("# control RX ignored (--no-route: no hive to relay through)")
        elif verb == "TX":
            sent = bool(participate and ser)
            if sent:
                ser.write(("INJECT " + hexstr + "\n").encode())
            if JSON_MODE:
                jline(kind="control", verb="TX", hex=hexstr, sent=sent)
            elif sent:
                log(f"CTRL TX> {hexstr[:24]}… (verbatim -> mesh)")
            else:
                log(f"CTRL TX  {hexstr[:24]}… (read-only; --participate to send)")
        elif verb in ("VMASK", "VRSSI", "VDIST", "VCLR", "VBLK"):
            # Carrier-firmware BENCH CONTROL verbs (feature `benchdist`): §2.3A egress mask (VMASK <hex>) +
            # §2.3C/§2.3B virtual-distance overrides (VRSSI/VDIST/VCLR/VBLK). Forward the raw line VERBATIM to
            # the carrier serial — its uart_rx_task parses them. Enables restoring the Mesh bit (VMASK ff) and
            # driving the drag-to-inject virtual-distance bench through the bridge.
            sent = bool(participate and ser)
            if sent:
                ser.write((line + "\n").encode())
            if JSON_MODE:
                jline(kind="control", verb=verb, arg=hexstr, sent=sent)
            elif sent:
                log(f"CTRL {verb}> {line} (-> carrier bench control)")
            else:
                log(f"CTRL {verb}  {line} (read-only; --participate to send)")
        elif not JSON_MODE:
            log(f"# control: unknown verb {verb!r} (use 'RX <hex>' | 'TX <hex>' | VMASK/VRSSI/VDIST/VCLR/VBLK)")


def run_live(args):
    ser = open_safe(args.port, args.baud)
    router = None
    if not args.no_route:
        router = start_router(args.hive, args.pkg)
        threading.Thread(target=router_reader,
                         args=(router, ser, args.participate), daemon=True).start()
        log(f"# routing ON (participate={args.participate})")
    else:
        log("# routing OFF (visibility only)")
    if args.control:
        threading.Thread(target=control_reader,
                         args=(router, ser, args.participate), daemon=True).start()
        log("# host control channel ON (stdin: 'RX <hex>' relay | 'TX <hex>' verbatim)")
    log("# bridge live. Ctrl-C to stop (board left RUNNING).")
    try:
        while True:
            raw = ser.readline()
            if not raw:
                continue
            line = raw.decode("utf-8", "replace").rstrip("\r\n")
            if not line:
                continue
            render_rx(line)
            if router and line.startswith("R2RX "):
                router.stdin.write(line[5:].strip() + "\n")
                router.stdin.flush()
    except KeyboardInterrupt:
        log("\n# stopped (board left running).")
    finally:
        # Leave DTR asserted through close — dropping it here would be a
        # toggle (the exact hazard class); with -hupcl set on the port the
        # close does not pulse the lines and the board keeps running.
        ser.close()
        if router:
            router.terminate()


def run_selftest(args):
    """No serial: feed canned frames through the router to prove the wiring."""
    log("# SELFTEST: routing wiring only (no serial port touched)")
    router = start_router(args.hive, args.pkg)
    threading.Thread(target=router_reader,
                     args=(router, None, False), daemon=True).start()
    time.sleep(0.4)
    for hexframe in ["deadbeef", "00" * 12, "nothex"]:
        log(f"# inject canned R2RX {hexframe}")
        router.stdin.write(hexframe + "\n")
        router.stdin.flush()
        time.sleep(0.2)
    time.sleep(0.3)
    router.terminate()
    log("# selftest done (no INJECT expected for garbage frames).")


def main():
    ap = argparse.ArgumentParser(description="DFR1195 carrier <-> wasm-hive bridge")
    ap.add_argument("--port", help="serial device (by-id path preferred)")
    ap.add_argument("--baud", type=int, default=115200)
    ap.add_argument("--hive", default="a1f5ed00", help="this host's hive_id (hex)")
    ap.add_argument("--pkg", default=None, help="wasmhive-node pkg dir (default: ./wasmhive-node)")
    ap.add_argument("--participate", action="store_true",
                    help="actually write INJECT frames to the mesh (default: read-only)")
    ap.add_argument("--no-route", action="store_true", help="visibility only, no router")
    ap.add_argument("--control", action="store_true",
                    help="read STDIN for host-injected frames ('RX <hex>' relay | 'TX <hex>' verbatim)")
    ap.add_argument("--selftest", action="store_true", help="exercise routing, no serial")
    ap.add_argument("--json", action="store_true",
                    help="emit JSON-lines (dashboard ingest) instead of human log lines")
    args = ap.parse_args()
    global JSON_MODE
    JSON_MODE = args.json
    if args.selftest:
        run_selftest(args)
    elif args.port:
        run_live(args)
    else:
        ap.error("need --port (or --selftest)")


if __name__ == "__main__":
    main()
