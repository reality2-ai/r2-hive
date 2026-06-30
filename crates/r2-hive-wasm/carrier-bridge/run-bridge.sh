#!/usr/bin/env bash
# Run the carrier bridge with the VENDORED pyserial on PYTHONPATH (no install needed).
# Passes all args through. Examples:
#   ./run-bridge.sh --selftest
#   ./run-bridge.sh --port /dev/serial/by-id/usb-Espressif_USB_JTAG_serial_debug_unit_F4:12:FA:50:26:98-if00 --hive a1f5ed00
#   ./run-bridge.sh --port <dev> --hive a1f5ed00 --participate
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PYTHONPATH="$HERE:${PYTHONPATH:-}"   # $HERE holds the vendored ./serial package
exec python3 "$HERE/r2-carrier-bridge.py" "$@"
