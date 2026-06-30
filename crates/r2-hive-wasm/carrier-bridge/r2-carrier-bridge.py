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
  python3 r2-carrier-bridge.py --selftest   # exercise the routing wiring, no serial
"""
import argparse
import os
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))


def log(msg):
    print(f"{time.strftime('%H:%M:%S')} {msg}", flush=True)


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
    # CRITICAL: set the initial control-line state to LOW *before* open() so the
    # open does not pulse them (pyserial applies these during open on POSIX).
    s.dtr = False
    s.rts = False
    s.open()
    # Belt-and-braces: hold them low after open, and verify the requested state.
    s.dtr = False
    s.rts = False
    if s.dtr or s.rts:
        s.close()
        sys.exit("FATAL: could not de-assert DTR/RTS — aborting to avoid a "
                 "download-mode reset on an unreachable board.")
    log(f"# opened {port} @ {baud} (DTR=0 RTS=0, no reset)")
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
    """Heartbeat-VISIBILITY: tag the carrier's own telemetry lines for a human."""
    if "peer MAPPED" in line:
        log(f"OTA-RX  {line}   <- heard a peer's heartbeat over the air")
    elif line.startswith("R2RX "):
        n = (len(line) - 5) // 2
        log(f"FRAME   {n}B over-the-air R2-WIRE frame")
    else:
        log(line)


def router_reader(router, ser, participate):
    """Read the router's INJECT decisions; write them to serial (if participating)."""
    for line in router.stdout:
        line = line.rstrip()
        if not line:
            continue
        if line.startswith("INJECT "):
            if participate:
                ser.write((line + "\n").encode())
                log(f"INJECT> {line[7:][:24]}… (sent to mesh)")
            else:
                log(f"would-INJECT (read-only; --participate to send): {line[7:][:24]}…")
        else:  # `# route …` diagnostics
            log(f"[router] {line}")


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
        ser.dtr = False
        ser.rts = False
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
    ap.add_argument("--selftest", action="store_true", help="exercise routing, no serial")
    args = ap.parse_args()
    if args.selftest:
        run_selftest(args)
    elif args.port:
        run_live(args)
    else:
        ap.error("need --port (or --selftest)")


if __name__ == "__main__":
    main()
