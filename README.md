<p align="center"><img src="crates/r2-hive-bin/static/relay.svg" width="96" alt="r2-hive"></p>

# r2-hive

<p align="center">
Your own connectivity, on your own terms.<br>
Part of <a href="https://reality2-ai.github.io">Reality2</a>.
</p>

**r2-hive** is the R2 software stack for general-purpose hosts — a
Linux/macOS/Windows daemon. Any device running the R2 stack is a
**hive**. A hive deployed as a public connectivity point — one that
forwards encrypted frames between devices in the same trust group,
across the internet — is a **wayfinder** (the role formerly called a
*relay*).

**A wayfinder never reads your data.** It forwards sealed, encrypted
frames between devices that share a trust group — like a postal service
carrying sealed envelopes: it knows where to deliver them, but not
what's inside. r2-hive operates at the routing layers only and never
decrypts payloads.

Beyond forwarding, a hive can join trust groups over a multi-transport
mesh (WebSocket, UDP-LAN, BLE, LoRa), run sentant ensembles with
OTP-style supervision, and serve web plugins over HTTPS / WebSocket.

## Use the community wayfinder

A public wayfinder is available for anyone:

```
wss://relay.reality2.ai/r2
```

It's untrusted by design — it forwards encrypted bytes and cannot read
your data. Use it to get started without running your own; for example,
in [Notekeeper](https://github.com/reality2-ai/r2-notekeeper) enter it
as the relay URL in Settings. Switch to your own wayfinder any time.

## Run your own wayfinder

You need a wayfinder reachable on the internet if your devices should
find each other when they're on different networks (e.g. a laptop at
home and a phone on mobile data). On a single local network you don't
need one.

### Build

During development the R2 protocol crates are consumed from a sibling
**r2-core** checkout via path dependencies, so clone both side by side:

```sh
git clone https://github.com/reality2-ai/r2-core.git
git clone https://github.com/reality2-ai/r2-hive.git
cd r2-hive
cargo build --release
./target/release/r2-hive --auto
```

(Releases will pin to published crates.io versions; until then the
sibling `r2-core` checkout is required to build.)

### Deploy to a VPS (automatic HTTPS)

For an always-on wayfinder with a domain and TLS:

```sh
./deploy.sh admin@your-server wayfinder.yourdomain.com
```

This builds the binary, copies it to your server, installs
[Caddy](https://caddyserver.com) for automatic Let's Encrypt TLS, and
sets up a systemd service. Your wayfinder will be reachable at
`wss://wayfinder.yourdomain.com/r2`. Requirements: a VPS with a public
IP and a domain pointing to it. Run it from a checkout that has the
sibling `r2-core` (it builds the binary locally before shipping it).

### Run locally without a service

```sh
cargo run --release -- --auto
```

### Docker

A `Dockerfile` is included. Because of the sibling-crate path
dependencies, build it from the parent directory containing both
`r2-hive` and `r2-core` (see comments in the file).

### Checking it works

Open `http://<your-ip>:21042` — you'll see the wayfinder dashboard: a
live view of connections, trust groups, and frames being routed. The
hexagon pulses each time a frame passes through.

## Options

```
r2-hive [OPTIONS]
  --port <PORT>           Port for WebSocket + HTTP   [default: 21042]
  --bind <ADDR>           Bind address                [default: 0.0.0.0]
  --buffer-size <N>       Recent frames kept per trust group [default: 1000]
  --max-connections <N>   Max simultaneous connections [default: 10000]
  --auto                  Auto-detect transports at startup
  --lan | --ble | --lora  Enable additional transports
  --no-usb                Disable the USB-peripheral watcher (servers)
```

Run `r2-hive --help` for the full list. Settings can also come from
`$XDG_CONFIG_HOME/r2/hive.toml`; see
[`crates/r2-hive-bin/packaging/defaults/hive.toml`](crates/r2-hive-bin/packaging/defaults/hive.toml).
CLI flags override the file.

## How a wayfinder works

When a device connects, it proves its identity (Ed25519-signed HELLO)
and names its trust group. The wayfinder places it in a bucket with
every other device from the same trust group and forwards frames
between them.

- **Multiple trust groups** share one wayfinder without seeing each other.
- **Your data is encrypted** before it reaches the wayfinder — it can't read it.
- **If the wayfinder restarts**, devices reconnect within seconds; a
  per-group catchup buffer replays recent frames.
- **If it goes down**, devices still work locally — they just can't
  reach each other across the internet until it's back.

## Crates in this workspace

| Crate | Purpose |
|---|---|
| [`crates/r2-hive-bin`](crates/r2-hive-bin/) | The daemon — library + `r2-hive` binary |
| [`crates/r2hive-cli`](crates/r2hive-cli/) | `r2hive` operator CLI |

## Documentation

- [`crates/r2-hive-bin/README.md`](crates/r2-hive-bin/README.md) — daemon overview
- [`crates/r2-hive-bin/docs/architecture.md`](crates/r2-hive-bin/docs/architecture.md) — module breakdown
- [`crates/r2-hive-bin/docs/mgmt-api.md`](crates/r2-hive-bin/docs/mgmt-api.md) — management-API reference
- [`crates/r2-hive-bin/DESIGN.md`](crates/r2-hive-bin/DESIGN.md) — design rationale
- [`crates/r2-hive-bin/TEST-RIG.md`](crates/r2-hive-bin/TEST-RIG.md) — hardware test rig

The normative protocol specs live in the `r2-specifications` repo.

## Status

Field-validated across x86_64 laptop (Ubuntu / Pop!_OS), Pi 5
(aarch64), and 2× Arduino UNO-Q (aarch64). The community wayfinder at
`relay.reality2.ai` runs this stack.

## License

Dual-licensed under MIT or Apache-2.0 — your choice. See
[`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
This is the open-core layer of Reality2: the protocol stack ships free;
the Mariko marketplace ships commercial.
