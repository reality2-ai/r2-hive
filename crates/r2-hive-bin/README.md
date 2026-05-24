# r2-hive

The Linux daemon at the heart of the [Reality2](https://github.com/reality2-ai)
stack. r2-hive runs the native Rust implementation of the R2 protocol stack
through Layer 5 (Trust Groups), exposes APIs to GUIs and Elixir upper levels,
auto-configures across deployment profiles (full Linux, Linux + USB MCU, plain
Linux + BlueZ, WASM, cloud), and supervises ensemble lifecycles with
OTP-style restart strategies.

It is normative for the R2-HIVE specification (see
[r2-specifications/specs/r2-core/R2-HIVE.md](https://github.com/reality2-ai/r2-specifications))
and currently the reference host for R2-HOST-API.

---

## Status

| Phase | Surface | Status |
|---|---|---|
| 0 | R2-HOST-API spec + conformance vectors | ✅ shipped |
| 1 | `r2.api.*` primitive events over UDS / `/r2/mgmt` WS | ✅ shipped |
| 2a | `r2-ensemble` registry crate | ✅ shipped |
| 2b | `r2.mgmt.ensemble.*` handlers | ✅ shipped |
| 2c | Service-sentant convention | ✅ shipped |
| 3 | R2-WEB singleton + GraphQL gateway | planned |
| 4 | Transport auto-config + keyring backends | planned |
| 5 | Extract to its own workspace + repo | planned |
| 6 | Elixir IPC contract (`HiveClient.ex`) | planned |
| 7 | Packaging, install one-liners, dev crate suite | planned |

---

## Quick start

### Build and run

```bash
cargo build -p r2-hive --release
./target/release/r2-hive --bind 127.0.0.1 --port 7878
```

The daemon prints its `self_hive_id` on startup, opens
- `/r2/wire` — peer-to-peer R2-WIRE WebSocket
- `/r2/mgmt` — management WebSocket (R2-HOST-API)
- `${XDG_RUNTIME_DIR}/r2-hive.sock` — Unix-domain management socket

### Talk to it with `r2hive`

```bash
r2hive daemon status
r2hive identity status
r2hive tg current
r2hive peers list
r2hive event send com.example.ping --payload-hex 0102
r2hive event subscribe --any
r2hive ensemble load examples/notekeeper.yaml
r2hive ensemble list
r2hive ensemble info notekeeper
r2hive ensemble stop notekeeper
```

`r2hive` is the reference R2-HOST-API client. Every command frames a CBOR
event over the management socket; nothing privileged. The same surface is
available to any client speaking R2-WIRE extended frames.

### Run the test rig

```bash
cargo test --workspace
```

A four-node smoke rig is documented in [TEST-RIG.md](TEST-RIG.md). The
in-process integration tests cover daemon status, identity custody, every
`r2.api.*` primitive, and the `r2.mgmt.ensemble.*` lifecycle.

---

## What r2-hive does

### Roles

1. **Mesh substrate.** Holds the route engine (R2-ROUTE), discovery
   (R2-DISCOVERY), and transports (WebSocket, UDP-LAN, BLE, LoRa, USB-R2).
   Routes R2-WIRE frames between local and remote peers per
   `ForwardAction::Drop / Directed / Flood / DeliverOnly`.
2. **Identity custodian.** Holds the device master secret, derives subkeys
   (HKDF-SHA-256), supplies them to peering and trust-group flows. Keys are
   zeroized on drop.
3. **Trust group host.** Tracks the active TG (R2-TRUST §13's
   single-active-hive rule) and a peer registry per TG.
4. **Application platform.** Loads and supervises ensembles via
   `r2-ensemble`. Ensemble sentants emit `Action::Send` events that
   r2-hive forwards onto the wire through `HiveOutboundSink`.
5. **Management surface.** Speaks the `r2.mgmt.*` and `r2.api.*` event
   vocabularies (R2-HIVE §5.3, R2-HOST-API §3) over a length-prefixed
   R2-WIRE Unix-domain socket and a parallel WebSocket at `/r2/mgmt`.

### Architecture

```text
              ┌─────────────────────────────────────────────┐
              │                  r2-hive                     │
              │                                              │
              │   ┌─────────────────────────────────────┐    │
              │   │      Management surface             │    │
              │   │   UDS  +  /r2/mgmt WebSocket        │    │
              │   │      r2.mgmt.* / r2.api.*           │    │
              │   └────┬─────────────────────┬──────────┘    │
              │        │                     │               │
              │   ┌────▼────┐          ┌─────▼──────────┐    │
              │   │ Daemon  │          │  Subscription  │    │
              │   │  state  │          │   registry     │    │
              │   └────┬────┘          └─────┬──────────┘    │
              │        │                     │               │
              │   ┌────▼─────────────────────▼──────┐        │
              │   │           HiveState              │       │
              │   │  ┌──────────┐  ┌──────────────┐  │       │
              │   │  │ Identity │  │ Active TG    │  │       │
              │   │  └──────────┘  └──────────────┘  │       │
              │   │  ┌─────────────────────────────┐ │       │
              │   │  │   Ensemble registry         │ │       │
              │   │  │   (DispatchTarget impl)     │ │       │
              │   │  └─────────────────────────────┘ │       │
              │   │  ┌─────────────────────────────┐ │       │
              │   │  │   Route engine + transports │ │       │
              │   │  │   (WS / UDP / BLE / LoRa)   │ │       │
              │   │  └─────────────────────────────┘ │       │
              │   └──────────────────────────────────┘       │
              └─────────────────────────────────────────────┘
```

### Event lifecycle on a packet from the mesh

```text
  L1 transport ──► L2 R2-WIRE decode ──► L3 RouteEngine.plan_forward
                                                       │
                                            ┌──────────┴──────────┐
                                            │                     │
                                       Drop / Directed     DeliverOnly
                                            │                     │
                                            ▼                     ▼
                                  send via send_to_hive    EnsembleRegistry
                                                                  │
                                                                  ▼
                                                         Sentant::handle_event
                                                                  │
                                                              ActionBuf
                                                                  │
                                                                  ▼
                                                          OutboundSink.deliver
                                                          (re-frame + send)
```

### Event lifecycle on a `r2.mgmt.*` request

```text
  UDS or /r2/mgmt WS  ──►  framing.rs (length prefix)  ──►  api.handle_frame
                                                                  │
                                                  ┌───────────────┴────────────────┐
                                                  │                                │
                                          r2.mgmt.daemon.*              r2.mgmt.ensemble.*
                                          r2.mgmt.identity.*                       │
                                          r2.api.peer.*                            ▼
                                          r2.api.event.*               EnsembleRegistry
                                          r2.api.tg.*                  load / list / info
                                          r2.api.cap.*                 stop / reset
                                                  │
                                                  ▼
                                           build response frame
```

---

## R2 crates this hive uses

r2-hive is the integration point — it pulls in most of the R2 crate suite.

| Crate | Role in r2-hive |
|---|---|
| [`r2-wire`](../../crates/r2-wire/) | R2-WIRE extended frame encode/decode for every protocol surface |
| [`r2-cbor`](../../crates/r2-cbor/) | Compact CBOR codec for management payloads |
| [`r2-fnv`](../../crates/r2-fnv/) | FNV-1a 32-bit hashing of event class strings |
| [`r2-route`](../../crates/r2-route/) | RouteEngine — Drop/Directed/Flood/DeliverOnly decisions |
| [`r2-transport`](../../crates/r2-transport/) | Transport-side framing primitives |
| [`r2-discovery`](../../crates/r2-discovery/) | mDNS/UDP-LAN/BLE/LoRa transports + neighbour discovery |
| [`r2-trust`](../../crates/r2-trust/) | L5 trust-group crypto: cert chains, HKDF, X25519 join, revocation, wire HMAC (used through identity custody and future trust flows) |
| [`r2-engine`](../../crates/r2-engine/) | Sentant trait, ActionBuf, Event types |
| [`r2-dispatch`](../../crates/r2-dispatch/) | Dispatch contract (DispatchEnvelope, DispatchTarget) used to hand DeliverOnly events to the ensemble registry |
| [`r2-def`](../../crates/r2-def/) | Ensemble/sentant score parser (YAML/JSON/TOML) |
| [`r2-ensemble`](../../crates/r2-ensemble/) | Ensemble registry + OTP supervision; r2-hive's DispatchTarget |
| [`r2-fnv`](../../crates/r2-fnv/) | Event-class hashing |

External dependencies of note: `axum` (HTTP/WS), `tokio`, `ed25519-dalek`,
`hkdf`, `sha2`, `zeroize`.

---

## R2-HOST-API surfaces

The daemon speaks two complementary vocabularies on the same wire:

### `r2.mgmt.*` — management
| Event class | Purpose |
|---|---|
| `r2.mgmt.daemon.status` | version / build / uptime |
| `r2.mgmt.identity.status` | master-secret presence, fingerprint, store backend |
| `r2.mgmt.event.error` | structured error envelope |
| `r2.mgmt.ensemble.load` | load a YAML/JSON/TOML ensemble score |
| `r2.mgmt.ensemble.list` | list loaded ensembles + status |
| `r2.mgmt.ensemble.info` | one ensemble's score hash, status, sentant count |
| `r2.mgmt.ensemble.stop` | unload |
| `r2.mgmt.ensemble.reset` | clear `Failed` state, rebuild sentants |

### `r2.api.*` — application surface (for R2-guests)
| Event class | Purpose |
|---|---|
| `r2.api.peer.list` | peers visible in the active TG |
| `r2.api.peer.query` | per-peer transport / route info |
| `r2.api.tg.current` | currently-attached trust group |
| `r2.api.cap.query` | capability bloom of self / a peer |
| `r2.api.event.send` | send an event into the active TG / mesh |
| `r2.api.event.subscribe` | filter on class/hash/from-hive/from-tg |
| `r2.api.event.unsubscribe` | drop a subscription |
| `r2.api.event.delivery` | unsolicited push for matched events |
| `r2.api.service.advertise` | register a service-sentant on this connection |
| `r2.api.service.retract` | drop a service registration |

The full byte-level description with CBOR shapes lives in
[docs/mgmt-api.md](docs/mgmt-api.md).

---

## Configuration

| Flag | Meaning |
|---|---|
| `--bind <ip>` | HTTP/WS bind address (default `127.0.0.1`) |
| `--port <u16>` | HTTP/WS port (default `7878`) |
| `--name <string>` | Self hive id seed (else derived from hostname) |
| `--socket <path>` | Override management Unix-socket path |
| `--no-mgmt` | Disable mgmt socket (for tests / cloud profiles) |
| `--identity-store <path>` | Master-secret file (default XDG_DATA_HOME/r2/identity) |
| `--lan` | Enable UDP-LAN transport |
| `--ble` | Enable BLE transport (requires `--features ble`) |
| `--lora` | Enable LoRa transport (requires `--features lora`) |
| `--buffer-size N` | TG catch-up ring buffer per TG |
| `--max-connections N` | Max concurrent UDS / WS clients |

Phase 4 will replace these explicit gates with auto-detected defaults; the
flags become overrides only.

---

## Integration paths

### Browser

Use the `/r2/mgmt` WebSocket. Send binary frames containing R2-WIRE extended
encodings of `r2.api.*` requests; receive responses and `event.delivery`
notifications on the same socket.

A higher-level browser client crate (`r2-client`, planned for Phase 7) wraps
this in a `Client::send / subscribe / advertise_service` API plus a federated
GraphQL endpoint at `/graphql` (Phase 3).

### Elixir

Phase 6 introduces `R2.HiveClient` — an Elixir IPC client that opens a
Unix-socket connection to r2-hive and exposes `ensemble_deploy`, `event_send`,
`subscribe`. The existing `r2_nif` stays for stateless primitives (FNV, CBOR,
WIRE encode/decode). State lives in r2-hive.

### Other languages

Anything that can speak length-prefixed R2-WIRE extended frames against a
Unix or WebSocket can be a client. The conformance test vectors at
`r2-specifications/testing/test-vectors/r2-host-api-vectors.json` show every
request/response pair.

---

## Specifications

- **R2-HOST-API** — `r2-specifications/specs/r2-core/R2-HOST-API.md`
- **R2-HIVE** — `r2-specifications/specs/r2-core/R2-HIVE.md`
- **R2-WIRE** — `r2-specifications/specs/r2-core/R2-WIRE.md`
- **R2-CBOR** — `r2-specifications/specs/r2-core/R2-CBOR.md`
- **R2-FNV** — `r2-specifications/specs/r2-core/R2-FNV.md`
- **R2-DEF / R2-ENSEMBLE / R2-SENTANT / R2-PLUGIN** — see the spec set

---

## License

Reality2 follows an **open-core** model
(`r2-specifications/specs/thurisaz/TH-ESG.md §8`):

- The **R2 protocol suite** — every crate in this workspace, including
  this one — is open source.
- The **Mariko marketplace and vertical-market services** (TH-MARKET,
  insights marketplace, platform-fee tier) are licensed commercially
  and are not part of this repository.

This crate is dual-licensed under either of:

- **Apache License, Version 2.0** ([`LICENSE-APACHE`](../../LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- **MIT License** ([`LICENSE-MIT`](../../LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option — the standard permissive Rust ecosystem dual license.
There is no copyleft obligation. You may use these crates to build a
peer hive, an alternative runtime, a closed-source product, or anything
else, with no requirement to publish your changes.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms
or conditions.
