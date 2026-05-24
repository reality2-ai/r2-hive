# R2 crates — what runs where

A map of every crate in the R2 stack with its target audience, no_std
status, and where it lives in the source tree. Use this when picking
dependencies for a new R2 project — especially MCU work, where
pulling in a `std` crate breaks the whole build.

## Tier model

R2 deployments span four tiers, smallest to largest:

| Tier | Example hosts | Memory | Stack capability |
|---|---|---|---|
| **1 (MCU)** | ESP32-S3, RP2040, Arduino UNO-Q sub-MCU | < 1 MB RAM | Wire framing, FNV, CBOR, lightweight engine |
| **2 (Tier-1 host)** | Pi Zero 2 W, BeagleBone | 256 MB – 1 GB | Tier 1 + transports + L4 routing |
| **3 (Tier-2 host)** | Pi 5, laptop, server | ≥ 1 GB | Full hive: ensembles, web, identity, mesh |
| **4 (cloud)** | Container, VM | unlimited | Tier 3 + internet-relay scaling |

The protocol crates are designed so a Tier-1 device can speak to
Tier-3 peers using the **same wire format**. The crates that work in
Tier 1 are the ones marked **MCU-ready** below.

## Crate inventory

Crates marked **`no_std`** can be built without the standard library
and are appropriate for embedded targets. Crates marked
**`alloc`** require a heap (`extern crate alloc`) but no `std`.
Crates marked **`std`** use threads, files, network sockets, etc.

### Protocol crates (r2-core repo)

These ship from the `r2-core` workspace and are consumed by every
peer that speaks R2 — host or MCU.

| Crate | Path | Target | no_std | Audience |
|---|---|---|---|---|
| `r2-fnv` | `r2-core/crates/r2-fnv/` | FNV-1a 32-bit hashing of event class strings | **always** | MCU, host |
| `r2-cbor` | `r2-core/crates/r2-cbor/` | CBOR Compact mode encoder/decoder (R2-CBOR) | **always** | MCU, host |
| `r2-wire` | `r2-core/crates/r2-wire/` | Extended-frame and compact-frame R2-WIRE codec | **always** | MCU, host |
| `r2-route` | `r2-core/crates/r2-route/` | L3/L4 route engine — neighbours, paths, transport quality | **always** | MCU, host |
| `r2-trust` | `r2-core/crates/r2-trust/` | L5 trust groups — Ed25519, X25519 join, HKDF, wire HMAC | feature-gated (`std` opt-in) | MCU (with `default-features = false`), host |
| `r2-dispatch` | `r2-core/crates/r2-dispatch/` | Local event-dispatch contract (R2-DISPATCH §1) | feature-gated (`std` opt-in) | MCU, host |
| `r2-engine` | `r2-core/crates/r2-engine/` | Sentant runtime — FSM, ActionBuf | feature-gated (`alloc` default, `std` opt-in) | MCU (alloc), host |
| `r2-transport` | `r2-core/crates/r2-transport/` | Transport abstractions (BLE/UDP/WS/LoRa) | feature-gated | MCU, host |
| `r2-uart` | `r2-core/crates/r2-uart/` | UART framing for R2-USB | feature-gated | MCU, host |
| `r2-discovery` | `r2-core/crates/r2-discovery/` | mDNS, UDP beacon, BLE scan — concrete transport bindings | **std** (axum/bluer/mdns-sd) | host only |
| `r2-def` | `r2-core/crates/r2-def/` | Score parser (YAML/JSON/TOML) for sentants/ensembles/swarms | **std** (serde_*/toml) | host (linters, validators), MCU-side codegen |
| `r2-ensemble` | `r2-core/crates/r2-ensemble/` | Ensemble registry — OTP-style supervision, restart ledgers | **std** (parking_lot/tokio) | host only |
| `r2-harness` | `r2-core/crates/r2-harness/` | Test harness for protocol conformance | **std** | dev tooling |

### Platform crates (r2-core/platforms)

| Crate | Path | Target | Audience |
|---|---|---|---|
| `r2-esp` | `r2-core/crates/r2-esp/` | ESP-IDF modules — OTA, WiFi, BLE L2CAP, provisioning, identity | ESP32 firmware (xtensa/riscv32) |
| `r2-nif` | `r2-core/crates/r2-nif/` | Erlang NIF bindings to the protocol crates | BEAM hosts |
| `r2-wasm` | `r2-core/crates/r2-wasm/` | WASM bridge for browser-resident peers | wasm32 |

### Hive (this repo)

| Crate | Path | Target | Audience |
|---|---|---|---|
| `r2-hive` | `r2-hive/crates/r2-hive-bin/` | The hive daemon — tokio + axum + ensemble registry + web auth + autoconfig | host only |
| `r2hive-cli` | `r2-hive/crates/r2hive-cli/` | Operator CLI for the management socket | host only |

## Targeting an MCU — picking the right deps

For a Tier-1 device speaking R2 framing **without** the full hive,
this is the typical dep set:

```toml
[dependencies]
r2-fnv = { path = "../r2-core/crates/r2-fnv", default-features = false }
r2-cbor = { path = "../r2-core/crates/r2-cbor", default-features = false }
r2-wire = { path = "../r2-core/crates/r2-wire", default-features = false, features = ["alloc"] }
r2-route = { path = "../r2-core/crates/r2-route", default-features = false }

# Optional — only if you need TG crypto on the MCU. Big binary impact;
# consider whether your device is the key-holder or a member.
r2-trust = { path = "../r2-core/crates/r2-trust", default-features = false }

# If you need to run sentants on the MCU (rare for Tier 1):
r2-engine = { path = "../r2-core/crates/r2-engine", default-features = false, features = ["alloc"] }
```

### What NOT to pull in on MCU

- **`r2-discovery`** — needs axum/bluer/mdns-sd. Use platform-native
  BLE/WiFi APIs and feed frames into `r2-wire` directly.
- **`r2-def`** — uses serde_yaml. Score parsing belongs on the
  Tier-2+ peer that loads ensembles; the MCU receives serialised
  events, not YAML.
- **`r2-ensemble`** — uses parking_lot, tokio, std::panic::catch_unwind.
  Ensembles are a Tier-2+ concept.
- **`r2-hive`** itself — the daemon. The MCU's job is to be a peer the
  daemon talks to.

### Lilygo ESP32-S3 e-paper specifics

The S3 has 512 KB SRAM + 8 MB PSRAM (depending on variant) and a dual
RISC-V Xtensa core. The protocol crates fit comfortably; the full
trust-group crypto (`r2-trust` with X25519 + ChaCha20Poly1305 + HKDF)
is the heaviest single dep. Recommended approach:

1. Start with `r2-fnv` + `r2-cbor` + `r2-wire` only — proves the
   wire format is being produced/consumed correctly. Use a
   Tier-3 hive's `r2hive event subscribe --any` to watch your
   device's frames arrive.
2. Add `r2-route` once you need to join a multi-hop mesh.
3. Add `r2-trust` last, only when you need cryptographic TG
   membership (i.e. your device is doing privileged work, not just
   reporting telemetry).

The `r2-esp` platform crate has BLE L2CAP and provisioning helpers
already; lean on those rather than re-implementing.

For e-paper specifically, the relevant pattern is: the device
subscribes to a single event class (`com.example.epaper.update` or
similar), the hive's ensemble emits frames carrying the new image
data, the device renders. The device doesn't need ensembles, plugins,
or web — it's a *consumer* peer.

## Two ESP32 firmwares: dongle vs standalone

There are two genuinely different ESP32-S3 firmware projects in the
R2 ecosystem, and they share a hardware family but **not** an
application stack. Don't conflate them — that confusion has cost time
twice already.

| | **DFR1195 dongle** | **LilyGo standalone (future)** |
|---|---|---|
| **Role** | R2-USB v2 peripheral — thin radio appliance | Tier-1 full hive — peer on the mesh |
| **Spec** | R2-USB §3, R2-HIVE §6.4 (peripheral half) | R2-WIRE + R2-TRUST + R2-ROUTE + R2-HIVE §3 |
| **Has `hive_id`?** | No — it's "owned by" a host | Yes (derived from master secret) |
| **Joins trust groups?** | No | Yes |
| **Runs sentants?** | No | Yes (e-paper UI driven by ensemble) |
| **Runs R2-WIRE stack?** | No — peripheral never parses R2-WIRE | Yes (full L4) |
| **Runs R2-ROUTE?** | No | Yes |
| **USB role** | Active — CDC-ACM data path | Inactive — USB only for flashing |
| **§6.4 pairing** | Peripheral half (factory-bonded link key per §6.4.5.1) | N/A — TG membership is the trust mechanism |
| **Master secret?** | No (link key only) | Yes (per R2-HIVE §3) |
| **Partition table** | `factory` + `ota_*` + `r2_pair` | `factory` + `ota_*` + `r2_master` |
| **Memory footprint** | Smaller — no R2-WIRE state, no sentants, no TG state | Larger — neighbour table, paths, sentant heap, ensemble registry |
| **UI surface** | LCD shows SAS during pairing, status text otherwise | Full e-paper UI driven by ensemble sentants |

### Crate reuse picture

What's **shared** between the two firmwares — the value of keeping
the protocol crates `no_std`-friendly:

| Crate | DFR1195 uses? | LilyGo uses? |
|---|---|---|
| `r2-fnv` | Yes — FNV-1a of event class strings (CAPS) | Yes — same purpose |
| `r2-cbor` | Yes — CAPS encoding, pairing-frame bodies | Yes — R2-WIRE payloads |
| `r2-wire` | **No** — peripheral never parses R2-WIRE | Yes — bread and butter |
| Pairing crypto (X25519 / commit-reveal SAS / HKDF / HMAC) | Yes — peripheral half of §6.4 | No — TG crypto is in `r2-trust` |
| `r2-trust` | Maybe (HKDF helper for §6.4.5 link-key derivation) | Yes — full TG join, beacons, HMAC envelope |
| `r2-route` | No | Yes |
| `r2-engine` | No (no sentants) | Yes (runs e-paper sentants) |
| SX1262 driver + ESP-IDF infrastructure | Yes | Yes |

### What this means for firmware authoring

- **DFR1195 dongle firmware** is a *small, specialised* codebase —
  USB framer + CAPS advertisement + §6.4 pairing peripheral half +
  SX1262 relay logic. A few thousand lines once the SX1262 driver is
  factored out. Its host-side counterpart is `r2-hive/crates/r2-hive-bin/src/usb.rs`
  and `usb_pair.rs`.

- **LilyGo standalone firmware** is a *larger, more ambitious*
  codebase — port as much of the host-side R2 stack as `no_std` will
  allow onto the ESP32-S3, plus the e-paper sentants. Its host-side
  counterpart is the entire r2-hive daemon, but reproduced in
  embedded form.

The two firmwares can coexist as separate `r2-core/platforms/esp32-s3/`
binary crates, each with its own `main.rs` and feature set selecting
which protocol crates to pull in. They build under the same ESP-IDF
toolchain.

### Anti-patterns to avoid

- ❌ "Use the LilyGo as the dongle, then reflash for standalone." The
  two firmwares share *crates*, not *applications*. There is no
  meaningful "configuration switch" that takes you from one to the
  other.
- ❌ "Embed `r2-hive` (the daemon) on the ESP32." `r2-hive` is the
  Tier-3 host daemon — tokio, axum, ensemble registry, web auth. It
  does not fit and was not designed to fit. The standalone firmware
  reuses *crates*; it doesn't run the daemon.
- ❌ "The DFR1195 firmware is a stripped-down hive." It isn't a hive
  at all. It's a USB-attached radio appliance. The R2-USB v2
  peripheral mode (R2-USB §1.4) is normative on this point: the
  peripheral has no `hive_id`, no R2-WIRE state, no L5+ semantics.

## Versioning and MSRV

All crates target Rust **1.78+** (current stable - 6 months) and
build on `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`,
`xtensa-esp32s3-elf`, `riscv32imac-unknown-none-elf`, and
`wasm32-unknown-unknown`. The CI matrix in
`.github/workflows/ci.yml` runs the host targets; MCU targets are
exercised via `r2-build` and the field rig.

## See also

- Top-level [README](../../README.md) — what r2-hive does.
- [`crates/r2-hive-bin/docs/architecture.md`](../../crates/r2-hive-bin/docs/architecture.md)
  — how the daemon's modules fit together.
- [`crates/r2-hive-bin/docs/mgmt-api.md`](../../crates/r2-hive-bin/docs/mgmt-api.md)
  — every event class on the management socket.
- `r2-specifications` — the normative specs each crate implements.
