#!/bin/bash
# Install the r2-hive binary and auto-start it as a wayfinder service.
# Supports Linux (systemd) and macOS (launchd).
#
# Usage:
#   cd r2-hive
#   ./install.sh            # build, install, start on boot
#   ./install.sh --remove   # stop and remove service + binary
#
# Build note: the R2 protocol crates are path-dependencies on a sibling
# r2-core checkout, so this repo must sit next to r2-core/ to build.

set -e

USER_NAME="$(whoami)"
USER_HOME="$HOME"
PORT="${R2_HIVE_PORT:-21042}"
BIND="${R2_HIVE_BIND:-127.0.0.1}"
ALLOW_PUBLIC_BIND="${R2_HIVE_ALLOW_PUBLIC_BIND:-0}"
OS="$(uname -s)"
INSTALL_DIR="/usr/local/bin"
BINARY="target/release/r2-hive"

echo "r2-hive installer"
echo "User: $USER_NAME"
echo "Platform: $OS"
echo "Port: $PORT"
echo "Bind: $BIND"
echo ""

# ── Remove ──

if [ "${1:-}" = "--remove" ]; then
    echo "Removing r2-hive..."
    if [ "$OS" = "Darwin" ]; then
        PLIST_FILE="$USER_HOME/Library/LaunchAgents/ai.reality2.hive.plist"
        launchctl bootout "gui/$(id -u)/ai.reality2.hive" 2>/dev/null || true
        rm -f "$PLIST_FILE"
        echo "  launchd agent removed"
    else
        if command -v systemctl &>/dev/null; then
            sudo systemctl stop r2-hive 2>/dev/null || true
            sudo systemctl disable r2-hive 2>/dev/null || true
            sudo rm -f /etc/systemd/system/r2-hive.service
            sudo systemctl daemon-reload
            echo "  systemd service removed"
        fi
    fi
    sudo rm -f "$INSTALL_DIR/r2-hive"
    echo "  Binary removed"
    echo ""
    echo "Done."
    exit 0
fi

# ── Check Rust ──

if ! command -v cargo &>/dev/null; then
    echo "Rust not found — installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
fi
echo "Rust: $(rustc --version)"

# ── Check sibling r2-core (path dependencies) ──

if [ ! -d "../r2-core" ]; then
    echo ""
    echo "WARNING: ../r2-core not found. r2-hive's protocol crates are"
    echo "path-dependencies on a sibling r2-core checkout. Clone it next"
    echo "to this repo and re-run:"
    echo "    git clone https://github.com/reality2-ai/r2-core.git ../r2-core"
    echo ""
fi

# ── Build ──

echo "Building (release)..."
cargo build --release --bin r2-hive
echo "  Built: $(du -h "$BINARY" | cut -f1)"

# ── Install binary ──

echo "Installing to $INSTALL_DIR/r2-hive..."
sudo install -m755 "$BINARY" "$INSTALL_DIR/r2-hive"

# ── Service ──

if [ "$OS" = "Darwin" ]; then
    PLIST_FILE="$USER_HOME/Library/LaunchAgents/ai.reality2.hive.plist"
    mkdir -p "$USER_HOME/Library/LaunchAgents"
    cat > "$PLIST_FILE" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>ai.reality2.hive</string>
  <key>ProgramArguments</key>
  <array>
    <string>$INSTALL_DIR/r2-hive</string>
    <string>--bind</string><string>$BIND</string>
    <string>--port</string><string>$PORT</string>
$(if [ "$ALLOW_PUBLIC_BIND" = "1" ]; then echo '    <string>--allow-public-bind</string>'; fi)
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict></plist>
EOF
    launchctl bootout "gui/$(id -u)/ai.reality2.hive" 2>/dev/null || true
    launchctl bootstrap "gui/$(id -u)" "$PLIST_FILE"
    echo "  launchd agent installed and started"
else
    sudo tee /etc/systemd/system/r2-hive.service > /dev/null <<EOF
[Unit]
Description=R2 Hive - Reality2 mesh runtime (wayfinder)
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/r2-hive --bind $BIND --port $PORT --no-usb$(if [ "$ALLOW_PUBLIC_BIND" = "1" ]; then echo ' --allow-public-bind'; fi)
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
    sudo systemctl daemon-reload
    sudo systemctl enable --now r2-hive
    echo "  systemd service installed and started"
fi

echo ""
echo "Done. r2-hive is running on port $PORT."
echo "  Dashboard: http://localhost:$PORT/"
echo "  For internet access with TLS, put Caddy/nginx in front (see Caddyfile),"
echo "  or use ./deploy.sh to provision a VPS with automatic HTTPS."
