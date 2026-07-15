# r2-hive Multi-Node Test Rig

A working physical setup for end-to-end testing of `r2-hive` across multiple
transports (BLE + UDP/WiFi + WebSocket) and multiple devices. This rig is what
brought up the first 4-node BLE+UDP mesh on real hardware. Reproducible and
incrementally extendable.

## Hardware inventory

| Role | Device | OS / arch | BLE adapter | Network access |
|------|--------|-----------|-------------|----------------|
| **laptop** | x86_64 dev box | Linux (Manjaro) | built-in `hci0` (`xx:xx:xx:xx:xx:xx`) | WiFi `192.168.1.x`, Tailscale |
| **alfred** | x86_64 SBC reachable via `ssh alfred` | Linux | `hci0` (`xx:xx:xx:xx:xx:xx`) — BLE asymmetric (weak receive) | WiFi `192.168.1.54`, Tailscale, USB to UNO Q's |
| **reality2-3** | Arduino UNO Q | Debian aarch64 (Linux MPU + MCU) | QCA WCN3990 `hci0` (`xx:xx:xx:xx:xx:xx`) — fragile, see below | WiFi `192.168.1.20` |
| **reality2-1** | Arduino UNO Q | Debian aarch64 | QCA WCN3990 `hci0` (`xx:xx:xx:xx:xx:xx`) | WiFi `192.168.1.22` |
| **royspi5** | Raspberry Pi 5 | Ubuntu 24.04 aarch64 | — | Tailscale `100.96.237.112` (build host, no test role) |

The UNO Q's reach the network only via WiFi (`The_Metaverse` SSID, WPA2-PSK).
Their LoRa radio is wired up but unused in this rig. BLE adapters use the QCA
`hci_uart_qca` driver and are fragile under sustained operation (see below).

ADB serials, when needed via `ssh alfred 'adb devices'`:

| Device | adb serial |
|--------|------------|
| reality2-3 | `4206527534` |
| reality2-1 | `586443026` |

## Network topology

```
                                The_Metaverse WiFi (192.168.1.0/24)
                                                │
            ┌─────────────────────┬─────────────┼────────────────────┐
            │                     │             │                    │
        laptop                 alfred       reality2-3           reality2-1
       192.168.1.x           192.168.1.54   192.168.1.20         192.168.1.22
       hci0 BLE              hci0 BLE       hci0 BLE             hci0 BLE
            │                     │             │                    │
            └─── BLE bubble A ────┘             └─── BLE bubble B ───┘
                 (laptop ↔ alfred                  (reality2-1 ↔ reality2-3
                  bidir, with                       bidir, very strong:
                  asymmetry)                        physically adjacent)
```

BLE bridges between bubbles are weak — that's expected and is exactly why the
mesh needs UDP+WiFi as well. Each pair of BLE-bubble nodes also reaches every
other node via UDP on the same `192.168.1.0/24` subnet.

## Build paths

There are two ways to produce the aarch64 r2-hive binary that the UNO Q's run.
Both are valid; pick whichever is faster for your iteration.

### Path A — Native build on Pi 5 (preferred)

The Pi 5 is itself aarch64, so `cargo build --release` (no `--target`) produces
a binary that runs unchanged on the UNO Q's.

```bash
# 1. Sync working tree from laptop to Pi 5 (excludes target/, .git/)
cd /path/to/r2-core
rsync -az --exclude=target --exclude=.git ./ \
    roycdavies@100.96.237.112:~/Development/R2/r2-core/

# 2. Build natively on Pi 5 (~3 min cold, ~30 sec incremental)
ssh roycdavies@100.96.237.112 \
    'export PATH=$HOME/.cargo/bin:$PATH; \
     cd ~/Development/R2/r2-core && \
     cargo build --release -p r2-hive --features ble'

# 3. Pull binary back to laptop
scp roycdavies@100.96.237.112:~/Development/R2/r2-core/target/release/r2-hive \
    /tmp/r2-hive-arm64
```

> **Note:** Pi 5 has both a stale system rustc 1.75 in `/usr/bin` and a current
> rustup toolchain in `~/.cargo/bin`. Always export `PATH=$HOME/.cargo/bin:$PATH`
> before invoking `cargo`, otherwise the system rustc shadows it.

### Path B — QEMU cross-build on laptop

Slower than Pi 5 native because of QEMU emulation, but no remote host required.
See `build/scripts/build-aarch64.sh`.

```bash
podman build --no-cache --platform linux/arm64 \
    -t r2-build-aarch64:latest \
    -f build/containers/aarch64.Containerfile .   # one-time

cd /path/to/r2-core
podman run --rm --platform linux/arm64 \
    --memory 8g -e CARGO_BUILD_JOBS=8 \
    -v "$PWD":/src -w /src \
    r2-build-aarch64:latest \
    cargo build --release -p r2-hive --features ble \
        --target aarch64-unknown-linux-gnu

# Output: target/aarch64-unknown-linux-gnu/release/r2-hive
```

### Native x86_64 for laptop and alfred

Standard:

```bash
cd /path/to/r2-core
cargo build --release -p r2-hive --features ble
# Output: target/release/r2-hive
```

## Deployment

### Deploy to alfred

```bash
ssh alfred 'pkill -f r2-hive 2>/dev/null; sleep 1; rm -f /tmp/r2-hive'
scp target/release/r2-hive alfred:/tmp/r2-hive
```

### Deploy to UNO Q's via alfred + adb

The UNO Q's are reachable only via `adb` from alfred (USB tether). They are
**not** independently SSH-able. Their Linux MPU runs as user `arduino`
(uid 1000, in group `bluetooth`). Sudo password is `Matang1#` if needed.

```bash
# Copy binary onto alfred first
scp /tmp/r2-hive-arm64 alfred:/tmp/r2-hive-arm64

# Then push via adb to both UNO Q's
ssh alfred 'adb start-server; sleep 2; \
    for d in 4206527534 586443026; do \
        adb -s $d shell "pkill -f r2-hive 2>/dev/null"; \
        adb -s $d push /tmp/r2-hive-arm64 /home/arduino/r2-hive; \
    done'
```

## Bringing the rig up

The order matters slightly — UNO Q's first, then alfred, then laptop. UNO Q's
need their hci0 freshly initialised (the QCA chip degrades over time, see
"Known issues" below).

### 1. UNO Q's (BLE + WiFi UDP)

```bash
ssh alfred 'adb start-server; sleep 2; \
    adb -s 4206527534 shell "RUST_LOG=info nohup /home/arduino/r2-hive \
        --ble --lan --port 21099 --name reality2-3 \
        > /tmp/r2-hive.log 2>&1 &"; \
    adb -s 586443026 shell "RUST_LOG=info nohup /home/arduino/r2-hive \
        --ble --lan --port 21099 --name reality2-1 \
        > /tmp/r2-hive.log 2>&1 &"'
```

### 2. alfred

```bash
ssh alfred 'rm -f /tmp/alfred.log; \
    setsid sh -c "RUST_LOG=info /tmp/r2-hive --ble --lan \
        --port 21099 --name alfred > /tmp/alfred.log 2>&1" \
    < /dev/null &'
```

### 3. laptop

```bash
cd /path/to/r2-core
RUST_LOG=info target/release/r2-hive --ble --lan \
    --port 21099 --name laptop > /tmp/laptop.log 2>&1 &
```

## Verification

### Check that all four nodes see each other

After ~30 seconds of running:

```bash
# Laptop
grep -E "self hive_id|peer registered" /tmp/laptop.log

# Alfred
ssh alfred 'grep -E "self hive_id|peer registered" /tmp/alfred.log'

# UNO Q's
ssh alfred 'adb -s 4206527534 shell "grep -E \"self hive_id|peer registered\" /tmp/r2-hive.log"'
ssh alfred 'adb -s 586443026 shell "grep -E \"self hive_id|peer registered\" /tmp/r2-hive.log"'
```

A healthy 4-node mesh shows each node logging its own `self hive_id` plus three
distinct peer hive_ids, with the same hive_id appearing for the same physical
device regardless of whether it was discovered via BLE or UDP. Stable hive_ids
are derived from `--name` via FNV-1a 32-bit and embedded in the high 4 bytes of
the beacon `rbid`; the low 4 bytes rotate. Helpers live at
`crates/r2-discovery/src/beacon.rs`.

### Per-node dashboards

Each running r2-hive serves a dashboard on its `--port`:

| Node | URL |
|------|-----|
| laptop | <http://localhost:21099/> |
| alfred | <http://192.168.1.54:21099/> |
| reality2-3 | <http://192.168.1.20:21099/> |
| reality2-1 | <http://192.168.1.22:21099/> |

The dashboard shows current peers per transport, frames routed, uptime, and
WebSocket connections.

## Cleanup

```bash
# Stop r2-hive on every node
pkill -f "target/release/r2-hive" 2>/dev/null
ssh alfred 'pkill -f r2-hive 2>/dev/null; \
    adb start-server; sleep 2; \
    adb -s 4206527534 shell "pkill -f r2-hive 2>/dev/null"; \
    adb -s 586443026 shell "pkill -f r2-hive 2>/dev/null"'
```

## Known issues

### QCA WCN3990 BLE chip fragility (UNO Q's)

The Qualcomm WCN3990 BT chip on the UNO Q's UART (driver `hci_uart_qca`) is
fragile under sustained operation:

- Works fine fresh from boot.
- Crashes during normal use after hours/days. Symptom in `dmesg`:
  `command 0x... tx timeout` → `crash the soc to collect controller dump`
  → `mem_dump_status: 3` → infinite retry loop with
  `Reading QCA version information failed (-110)`.
- **Once crashed, the chip cannot be recovered without a hard power cycle.**
  Neither rfkill toggle, `bluetoothctl power on`, nor mgmt-API reset works.
- The arduino user via adb cannot run `systemctl reboot` (silently fails).
  Use a physical USB unplug/replug to recover.
- This happens at the firmware/driver level, below any HCI socket. A
  hypothetical bluer-bypass stack would NOT fix it.

For long-running tests, watchdog the chip and physically power-cycle when it
locks up. Or fall back to UDP-only on the UNO Q's after BLE drops.

### BLE stale-cache hive_id race — fixed

Previously, when two devices restarted and BLE-discovered each other within
seconds, BlueZ's `device.manufacturer_data()` could return a cached value from
before the peer's new advertiser fired, so the first `DeviceAdded` event
reported the old `rbid` (usually `[0;8]`, yielding `hive=0x00000000`).

Fixed 2026-04-07 in `crates/r2-discovery/src/bindings/ble_sched.rs` by spawning
a per-device `PropertyChanged` watcher (via bluer's `device.events()`) on first
`DeviceAdded`. The watcher re-emits discoveries whenever `manufacturer_data`
updates, dedup'd against the last rbid emitted for that address. Watchers are
tracked in a `HashMap<Address, JoinHandle<()>>` in the scheduler loop and
aborted on `DeviceRemoved`.

**Side benefit:** BLE address rotation (LeRandom) now works correctly. The same
physical hive with two rotating MAC addresses is recognised as the same
canonical hive_id because identification is via the beacon rbid, not the MAC.

### Asymmetric BLE between laptop and alfred

Alfred's BLE chip receives advertisements poorly while transmitting its own.
Laptop sees alfred reliably; alfred often does not see laptop. This is hardware
RF coexistence and not a code issue. Bringing WiFi up on the same chip
(observed on the QCA-equipped UNO Q's) tends to *improve* BLE reception —
likely shared antenna/clock subsystems being properly initialised.

### `r2-demo` was a previous test binary on UNO Q

If a UNO Q's r2-hive `--ble` start fails with `[L2CAP] Failed to bind listener:
Address already in use`, an old `r2-demo` binary is probably running and
holding PSM 0xD2. Kill it and rename the binary so systemd's auto-respawn
fails:

```bash
ssh alfred 'adb -s 4206527534 shell "kill -9 \$(pgrep -f r2-demo) 2>/dev/null; \
    mv /home/arduino/r2-demo /home/arduino/r2-demo.bak"'
```

Reverse on cleanup if you want LoRa testing back.

## Next pieces (planned)

- **BLE stale-cache fix** — subscribe to per-device `PropertyChanged` in
  `ble_sched.rs` and re-emit on `manufacturer_data` updates.
- **Frame-flow demo** — send an event from one node and trace it across BLE
  and UDP transports through the route engine.
- **Capability ensemble test** — load a notekeeper-style ensemble (Sentants +
  plugins + WebApp UI, see `R2-ENSEMBLE`) and exercise it across the rig.
- **WiFi multi-island bridging** — when no shared AP exists, devices form
  ad-hoc SoftAP islands; bridge nodes with dual WiFi capability link them.
- **LoRa transport** — bring the UNO Q LoRa radios into the rig as a fifth
  transport binding.
