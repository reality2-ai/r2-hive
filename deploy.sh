#!/bin/bash
# Deploy an R2 wayfinder (r2-hive) to a VPS with automatic HTTPS via Caddy.
#
# Prerequisites:
#   - A VPS with a public IP, and a domain pointing to it
#   - SSH access to the VPS
#   - The R2 protocol crates are git-pinned to r2-core in Cargo.toml, so the
#     local build needs EITHER read access to the r2-core repo (git) OR a
#     sibling ../r2-core checkout with the Cargo [patch] block uncommented
#     (local only, never committed). The binary is built locally before shipping.
#
# Usage:
#   ./deploy.sh user@your-server wayfinder.yourdomain.com
#
# Example:
#   ./deploy.sh admin@203.0.113.45 wayfinder.reality2.ai
#
# This script:
#   1. Builds the r2-hive binary for Linux
#   2. Copies it to the server
#   3. Installs Caddy if not present
#   4. Sets up a systemd service (r2-hive) + Caddy with automatic HTTPS
#   5. Starts everything

set -e

if [ $# -lt 2 ]; then
    echo "Usage: ./deploy.sh user@server wayfinder.yourdomain.com"
    echo "Example: ./deploy.sh admin@203.0.113.45 wayfinder.reality2.ai"
    exit 1
fi

SSH_TARGET="$1"
DOMAIN="$2"
PORT=21042

echo "R2 wayfinder deployment"
echo "  Server: $SSH_TARGET"
echo "  Domain: $DOMAIN"
echo ""

# Build for Linux (cross-compile if needed)
echo "Building r2-hive binary..."
if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "x86_64" ]; then
    cargo build --release --bin r2-hive
    BINARY="target/release/r2-hive"
else
    echo "  Cross-compiling for linux/amd64..."
    rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
    cargo build --release --bin r2-hive --target x86_64-unknown-linux-gnu
    BINARY="target/x86_64-unknown-linux-gnu/release/r2-hive"
fi
echo "  Built: $(du -h "$BINARY" | cut -f1)"

# Copy binary to server
echo "Copying binary to server..."
scp "$BINARY" "$SSH_TARGET":/tmp/r2-hive

# Set up on server
echo "Setting up on server..."
ssh "$SSH_TARGET" bash -s "$DOMAIN" "$PORT" << 'REMOTE'
DOMAIN="$1"
PORT="$2"

set -e

# Install binary (back up any existing one first)
if [ -f /usr/local/bin/r2-hive ]; then
    sudo cp -a /usr/local/bin/r2-hive "/usr/local/bin/r2-hive.bak-$(date +%Y%m%d%H%M%S)"
fi
sudo install -m755 /tmp/r2-hive /usr/local/bin/r2-hive
rm -f /tmp/r2-hive

# Install Caddy if not present
if ! command -v caddy &>/dev/null; then
    echo "Installing Caddy..."
    sudo apt-get update -qq
    sudo apt-get install -y -qq debian-keyring debian-archive-keyring apt-transport-https curl
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
    sudo apt-get update -qq
    sudo apt-get install -y -qq caddy
fi

# Wayfinder systemd service. Bound to loopback; Caddy fronts it with TLS.
# --no-usb: a server has no R2 USB peripherals to watch.
sudo tee /etc/systemd/system/r2-hive.service > /dev/null <<EOF
[Unit]
Description=R2 Hive - Reality2 mesh runtime (wayfinder)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/r2-hive --bind 127.0.0.1 --port $PORT --no-usb
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Caddyfile
sudo tee /etc/caddy/Caddyfile > /dev/null <<EOF
$DOMAIN {
    reverse_proxy localhost:$PORT
}
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now r2-hive
sudo systemctl restart r2-hive
sudo systemctl restart caddy

echo ""
echo "Done. R2 wayfinder is running at:"
echo "  wss://$DOMAIN/r2       (WebSocket endpoint)"
echo "  https://$DOMAIN/        (dashboard)"
echo "Caddy handles TLS automatically via Let's Encrypt."
REMOTE

echo ""
echo "============================================"
echo "  Wayfinder deployed!"
echo ""
echo "  WebSocket: wss://$DOMAIN/r2"
echo "  Dashboard: https://$DOMAIN/"
echo ""
echo "  Use in Notekeeper / other R2 tools:"
echo "    wss://$DOMAIN/r2"
echo "============================================"
