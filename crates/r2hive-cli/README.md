# r2hive-cli

Reference CLI for the [r2-hive](../r2-hive/) management surface. Every
subcommand is a thin wrapper around a single R2-WIRE round-trip on the
daemon's Unix-domain socket — nothing in the daemon treats this CLI as
privileged. Use it as a working tool, a debugging aid, or as a model
for writing your own R2-HOST-API client.

The binary name is `r2hive`. The package name is `r2hive-cli`.

---

## Install / build

From the workspace root:

```bash
cargo build -p r2hive-cli --release
./target/release/r2hive --help
```

Or run via cargo for development:

```bash
cargo run -p r2hive-cli -- daemon status
```

> **Note.** When testing the `event subscribe` long-running stream,
> invoke the binary directly (`./target/debug/r2hive event subscribe …`)
> rather than `cargo run`. Cargo's child-process supervisor swallows
> Ctrl-C signals and can intercept stderr in ways that confuse the
> stream loop.

---

## Connection

By default `r2hive` connects to:

```text
${XDG_RUNTIME_DIR}/r2-hive.sock     # Linux
${TMPDIR}/r2-hive.sock              # macOS
/tmp/r2-hive-<uid>.sock             # fallback
```

Override with `--socket <path>` (global flag, before any subcommand):

```bash
r2hive --socket /tmp/test.sock daemon status
```

---

## Subcommand surface

```text
r2hive
├── daemon
│   └── status              show version, build, uptime
├── identity
│   └── status              master-secret presence + fingerprint + backend
├── tg
│   └── current             current TG attachment, if any
├── peers
│   ├── list                hive_ids visible in the active TG
│   └── query <hive_id>     status + transports for one peer
├── cap
│   └── query [--target id] capability set for self / a peer
├── event
│   ├── send <class> [...]  send an event into the mesh
│   └── subscribe [class | --any]
│                           stream matching events to stdout
└── ensemble
    ├── load <path> [--json | --yaml | --toml]
    ├── list
    ├── info  <id>
    ├── stop  <id>
    └── reset <id>
```

---

## Examples

### Inspect a running daemon

```bash
r2hive daemon status
# version: 0.1.0  build: unversioned  uptime: 47s

r2hive identity status
# present:     true
# fingerprint: 2f9a08c4d1aa7b3e
# backend:     file
# path:        /home/you/.local/share/r2/identity
# created_this_start: false

r2hive tg current
# (detached — no trust group attached)

r2hive peers list
# 0xCAFEBABE  self  ws,udp
```

### Send a one-shot event

```bash
r2hive event send com.example.ping
# msg_id: 1

r2hive event send com.example.ping --target 0xC0FFEE --payload-hex 0102deadbeef
# msg_id: 2

r2hive event send com.example.ping --target 0xDEADDEAD
# r2hive: daemon error: peer_not_found
```

### Subscribe to events

```bash
# All events:
./target/debug/r2hive event subscribe --any

# Just one class:
./target/debug/r2hive event subscribe com.example.ping
```

Each delivery prints a single line:

```text
[delivery] sub_id=1 class=com.example.ping hash=0x4571F2EC src=0xCAFEBABE msg_id=2 payload=deadbeef
```

Ctrl-C exits cleanly.

### Load and manage an ensemble

```bash
r2hive ensemble load examples/notekeeper.yaml
# loaded ensemble 'notekeeper': 1 sentants, score_hash 0x8F3C1AA2

r2hive ensemble list
# notekeeper      Healthy   1 sentants

r2hive ensemble info notekeeper
# id:          notekeeper
# status:      Healthy
# sentants:    1
# score_hash:  0x8F3C1AA2

r2hive ensemble stop notekeeper
# stopped ensemble 'notekeeper'

r2hive ensemble reset notekeeper       # clears Failed → Healthy
```

`--json` / `--toml` switch the score dialect; default is YAML. `-` reads
from stdin:

```bash
cat my-score.yaml | r2hive ensemble load -
```

---

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Request round-tripped and the daemon's response was a success |
| `1` | Any error: socket unavailable, daemon error envelope, decode failure |

The error message goes to stderr in the form `r2hive: <message>`.

---

## Wire format reference

For the byte-level CBOR shapes of every request and response, see:

- [`tools/r2-hive/docs/mgmt-api.md`](../r2-hive/docs/mgmt-api.md)
- The normative spec: `r2-specifications/specs/r2-core/R2-HOST-API.md`
- Conformance vectors:
  `r2-specifications/testing/test-vectors/r2-host-api-vectors.json`

The CLI's `build_*_request` helpers live in
[`r2-hive::mgmt::api`](../r2-hive/src/mgmt/api.rs) and
[`r2-hive::mgmt::ensemble`](../r2-hive/src/mgmt/ensemble.rs); they are
public so you can use them from your own client.

---

## R2 crates this CLI uses

| Crate | Role |
|---|---|
| [`r2-hive`](../r2-hive/) | Imported as a library for `mgmt::api` and `mgmt::ensemble` `build_*_request` / `parse_*_response` helpers, the framing module, and the `default_socket_path` resolver |
| [`r2-wire`](../../crates/r2-wire/) | Decodes daemon responses into `ExtendedMessage` |
| [`r2-cbor`](../../crates/r2-cbor/) | Decodes CBOR payloads to extract response fields |
| [`r2-fnv`](../../crates/r2-fnv/) | Computes event-class hashes for sniffing error envelopes |

External dependencies: `clap` (4.x derive), `tokio` (rt-multi-thread,
net, io-util), `rand` (correlation-id generation).

---

## Writing your own client

Anything that can speak length-prefixed R2-WIRE extended frames can be
a client. The CLI is one viable model; the same job can be done from:

- **Rust** — pull in `r2-hive` as a library and use the `build_*_request`
  helpers (preferred for tooling that ships with the daemon).
- **Python** — `socket` + a small R2-WIRE encoder. The conformance
  vectors include sample frames hex-dumped for direct replay.
- **Browser** — connect to `/r2/mgmt`, send binary `WebSocket` messages
  containing R2-WIRE frames. A higher-level browser client (`r2-client`)
  is planned for Phase 7.
- **Elixir** — Phase 6 ships `R2.HiveClient` over the same UDS surface.

The CLI's source is intentionally small (~600 lines) so reading it is a
realistic on-ramp.

---

## Known issues

- The `subscribe` command holds one daemon connection open and reads
  forever. Killing the daemon while subscribed leaves the CLI hanging
  for one read timeout before exiting.
- `cargo run` swallows Ctrl-C in child binaries on some shells. Use the
  built binary directly when iterating on subscribe.
- The `--json` / `--yaml` / `--toml` flags on `ensemble load` are
  exclusive but the CLI does not yet auto-detect by file extension —
  YAML is assumed unless you say otherwise.

---

## License

Reality2 follows an **open-core** model
(`r2-specifications/specs/thurisaz/TH-ESG.md §8`):

- The R2 protocol suite — including this CLI — is open source.
- The Mariko marketplace and vertical-market services (TH-MARKET) are
  licensed commercially and live elsewhere.

This crate is dual-licensed under either of:

- **Apache License, Version 2.0** ([`LICENSE-APACHE`](../../LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- **MIT License** ([`LICENSE-MIT`](../../LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option — the standard permissive Rust ecosystem dual license.
No copyleft obligation.

Contributions are accepted under the same dual license unless you say
otherwise, per the Apache-2.0 contribution clause.
