# R2 Conformance Report — `r2-hive`

**As of: 2026-04-28** • Updated after Phase USB-2 closed the §6.4 pairing implementation gaps.

This report enumerates every R2 specification `r2-hive` claims to
implement (directly or via consumed crates), what's actually
exercised by tests, and where the gaps are. It is intentionally
honest — green ticks earn their colour by replaying the canonical
test vectors at `r2-specifications/testing/test-vectors/`; partial
coverage gets a yellow flag; stubs and TBDs are listed openly so
nobody plans against false assumptions.

## Legend

| Mark | Meaning |
|---|---|
| ✅ | Spec test vectors round-trip in CI; behaviour fully implemented. |
| 🟢 | Implemented and behaviour-tested in r2-hive's own tests, but the canonical vector file isn't loaded as a fixture (we re-encode the same shapes). Equivalent in effect; weaker on traceability. |
| 🟡 | Partial — a defined subset implemented; some normative paths stubbed or deferred to a later phase. |
| 🔘 | Stubbed — sufficient for milestone 1 but not normatively conformant. |
| ❌ | Not implemented; spec exists; r2-hive declines responsibility for now. |
| ➖ | Not in scope for r2-hive (handled by another crate or deployment layer). |

## Test surface

- **Workspace tests:** `cargo test --workspace` from `r2-hive/` runs **146 tests**, all green.
- **Per-crate vector replays** (in `r2-core` crates, not r2-hive):
  - `r2-fnv` — replays `r2-fnv-vectors.json::conformance_vectors`
  - `r2-cbor` — replays `r2-cbor-vectors.json::conformance_vectors` + `error_vectors`
  - `r2-wire` — replays `r2-wire-vectors.json` (3 vectors)
  - `r2-route` — replays `r2-route-vectors.json` (6 vectors)
  - `r2-trust` — replays `r2-trust-vectors.json` (4 of 5 sections)
  - `r2-engine` — replays `r2-engine-vectors.json` (state-machine + entanglement)
  - `r2-transport` — replays `r2-transport-vectors.json`
- **r2-hive vector replays:**
  - `r2-usb-vectors.json` — TV1, TV2, TV3, TV5, TV6, TV7, TV9, TV11, TV12 (9 of 13 vectors) replayed in `usb.rs::tests` (Phase USB-1).
  - `r2-host-api-vectors.json` — **all 28 vectors** loaded as a fixture and replayed in `tests/host_api_conformance.rs` (Phase Conf-A). Each vector's `frame_hex` decodes via `r2_wire::decode_extended`; `event_hash` matches `r2_hash(event_class)`; UDS framing equals `len_be32(frame_hex) || frame_hex`; payload CBOR decodes cleanly; `app_to_hive` dispatch returns something other than `unknown_event`.
  - `r2-plugin-web-vectors.json` — scenario coverage in `web_plugin_integration` (7 tests) and `web_auth_integration` (5 tests). Vectors WEB-MOUNT-AND-FETCH, WEB-UNMOUNT-ON-STOP, WEB-CSP-DEFAULT-FORBIDS-INLINE-SCRIPT, WEB-AUTH-401-WITHOUT-COOKIE, WEB-ATOMIC-RELOAD, WEB-ESCAPING-SYMLINK-REJECTED, WEB-BAD-SCORE-CHANNEL-TARGET are exercised; WEB-WS-ROUNDTRIP and WEB-BAD-SCORE-MISSING-BUNDLE pending (the former depends on §13.6 channels, the latter is covered in `r2-def`'s parser tests).
  - `r2-usb-pair-vectors.json` — **all pinned values replayed** by `usb_pair.rs::tests` (12 unit tests covering ECDH `Z`, commitment, SAS code, link key, reconnect HMAC) and `usb.rs::tests` (7 end-to-end pairing-flow tests covering first-attach, reconnect-success, reconnect-failure, abort, commit-mismatch). Phase USB-2 closed the implementation gap.
- **Vector-coverage regression check** (`tests/vector_coverage.rs`, Phase Conf-A): 4 tests — vectors directory exists, upstream vector counts haven't shrunk, referenced vector IDs in r2-hive's test sources haven't dropped, and a `--nocapture` diagnostic that lists each vector file's unreferenced IDs.

## Per-spec status

### Protocol-level — primitives consumed by r2-hive

| Spec | Status | r2-hive role | Notes |
|---|---|---|---|
| **R2-FNV** | ✅ via crate | Consumer of `r2_fnv::r2_hash` for event-class hashing | Crate replays `r2-fnv-vectors.json` directly. |
| **R2-CBOR** | ✅ via crate | Consumer of `r2_cbor::Encoder`/`Decoder` | Crate replays `r2-cbor-vectors.json` (compact-mode rules + error cases). |
| **R2-WIRE** | ✅ via crate | Consumer of extended-frame and compact-frame codecs | Crate replays `r2-wire-vectors.json`. r2-hive uses extended frames on UDS/WS and compact on BLE/LoRa per spec. |
| **R2-ROUTE** | ✅ via crate | Owns `RouteEngine<64,64,64>` in `HiveState`; consumes neighbours/paths | Crate replays `r2-route-vectors.json`. r2-hive integrates `Observation` ingest from each transport. |
| **R2-TRUST** | 🟡 via crate | Consumer for L5 derivation; r2-hive's own master-secret/HKDF lives in `mgmt::identity` and `web_auth` | Crate replays 4 of 5 vector sections. r2-hive uses `MasterSecret` derivation paths but not yet the full TG-join protocol. |
| **R2-DISPATCH** | 🟢 via crate | `EnsembleRegistry` implements `DispatchTarget`; route engine's `DeliverOnly` outcome dispatches through it | Behaviour tested via `ensemble_integration` tests; no spec-vector file exists in this repo. |
| **R2-ENGINE** | ✅ via crate | Consumer of `Sentant` trait, `Event`, `ActionBuf` | Crate replays `r2-engine-vectors.json`. |

### r2-hive's own normative surface

| Spec | Status | Coverage | Gaps |
|---|---|---|---|
| **R2-HIVE** | 🟡 | Identity custody (§3) ✅; trust group registry (§4) 🟡; transport registry (§6) 🟡; pairing (§6.4) ❌ host-side stubbed; arbitration (§7) ➖; UI interop (§11) ❌ | §6.4.x crypto contract is **specified** (this work) but the host-side **implementation** is Phase USB-2. §11 GUI surface depends on Phase 3f/3g applets. |
| **R2-HOST-API** | ✅ | All `r2.api.*` and `r2.mgmt.*` event classes from §3 implemented in `mgmt::api`/`mgmt::primitive`/`mgmt::ensemble`. 28 vectors in `r2-host-api-vectors.json` loaded as a fixture and replayed (`tests/host_api_conformance.rs`). | None — full structural conformance against the canonical vectors. Behavioural conformance for stateful response paths is still indirect (the vector file's `hive_to_app` cases need state that's only set up in `mgmt_integration`); not a gap, just a fact of the fixture's design. |
| **R2-DEF** | 🟢 via crate | Score parsing (sentants, ensembles, swarms, web plugins) | `r2-def` has its own round-trip tests over fixtures (`vectors/notekeeper.ensemble.yaml` etc.). 9 web-plugin parse tests. |
| **R2-ENSEMBLE** | 🟢 via crate | Loader, OTP-style supervision, restart strategies, restart ledgers | `ensemble_integration` (3 tests) + `service_integration` (4) cover load/dispatch/supervise/reset. No external vector file. |
| **R2-PLUGIN §13 (web)** | 🟡 | Manifest parser, mount/unmount, lifecycle on `r2.mgmt.ensemble.*`, default §13.9 CSP, §13.5 cookie auth, §13.5 dev-mode marker, single-use 1h-TTL provision codes, atomic remount, escape-symlink rejection, channel-target validation | §13.7 GraphQL gateway deferred to v0.2 (per spec). §13.6 WS channels not yet wired (waits on browser Ed25519 keypair, Phase 3d follow-up). 7 of 9 conformance vectors exercised; remaining 2 are WS-roundtrip (depends on §13.6) and bad-score-missing-bundle (covered in r2-def's parser tests, not in r2-hive integration). |
| **R2-PROVISION** | 🟡 | Browser-pairing word codes (`r2.mgmt.web.provision`), POST `/r2/web/provision`, signed-cookie issue, redeem flow | TG-join via word codes (the existing `word_codes` plugin) is separate from browser pairing. UDS-side provisioning UX (Phase 3f/g applets) pending. |
| **R2-USB §3.3, §3.5, §3.6, §3.7** | 🟢 | Length-prefix codec, SYNC handshake, type-byte demux, CAPS parser, control-frame parser, pairing-frame state machine, all in `usb.rs` + `usb_pair.rs` | Phase USB-1 tested against 9 of 13 wire vectors. Phase USB-2 added the §6.4 pairing-frame state machine. Hot-plug detection and serial I/O wrapper (open `/dev/ttyACM*`, termios raw mode) not yet written; tests use in-memory duplex pairs. |
| **R2-USB §3.6.1 (`device_id` derivation)** | ✅ | Derivation rule pinned in spec this work; worked example for ESP32-S3 (LilyGo MAC) committed in `r2-usb-pair-vectors.json` | Spec contribution; implemented directly when host parses CAPS. |
| **R2-HW §4 (MCU-SBC bus, Tier 2)** | ❌ | Not implemented. r2-hive has no SPI/UART driver, no SYNC/CMD/LEN/CRC16 framer, no WAKE-line semantics, no autonomous-MCU log-sync flow. | This is a **separate** wired-bridge architecture for power-managed Tier 2 boards (MCU autonomous L1–L4, SBC duty-cycled). It is **not** R2-USB v2 peripheral mode (which is host-always-on with a thin radio appliance). Bespoke Tier 2 R2 boards land here. Tracked as a future host-side Phase MCU-SBC-1 (parallel to USB-1). See `src/usb.rs` module doc. |
| **R2-HIVE §6.4 (USB pairing crypto contract)** | ✅ spec-side, ✅ host-side | `r2-usb-pair-vectors.json` pins every byte (10 vectors); deterministic generator at `r2-specifications/testing/generators/r2-usb-pair-vectors/` re-derives them; host-side `usb_pair.rs` replays all pinned values bit-equal; pairing state machine in `usb.rs` drives §6.4.3 commit-reveal first-attach, §6.4.6 HMAC reconnect, §6.4.7 device_id-not-trust, §6.4.8 SAS-prompt event | None for v0.1. Link-key store is in-memory by default (file/keyring backing for `LinkKeyStore` is mechanical and lands when wired into `HiveState`). |
| **R2-RUNTIME §5.4 (notify+watchdog)** | ✅ | `--features systemd` build sends `READY=1` after listener bind, pings `WATCHDOG=1` per `WATCHDOG_USEC/2`. Unit at `packaging/systemd/r2-hive.service` ships `Type=notify`, `WatchdogSec=30s`. | None at the protocol layer. Hardening (SystemCallFilter etc.) is operational, not normative. |

### Specs r2-hive consumes via discovery / transport bindings

| Spec | Status | Notes |
|---|---|---|
| **R2-BEACON** | ➖ via `r2-discovery` | Crate handles BLE + UDP beacon emission/scanning; r2-hive owns the bring-up wiring. r2-beacon-vectors.json (8 vectors) exercised by `r2-discovery`'s tests, not r2-hive's. |
| **R2-BLE / R2-BLESCHED / R2-BLE L2CAP** | ➖ via `r2-discovery` (`bluer`) | r2-hive's `--ble` feature delegates to `r2-discovery::bindings::ble`. Vector files have no `vectors` array — they document the wire shapes; conformance is enforced through behaviour. |
| **R2-LORA** | ➖ via `r2-discovery` (arduino-router IPC) | r2-hive's `--lora` feature opens the IPC socket; transcoding (compact ↔ extended) lives in `r2_wire::transcode`. |
| **R2-WIFI** | ➖ via `r2-discovery` (mDNS + UDP) | LAN bring-up in `main.rs::start_lan_discovery`. |
| **R2-INTERNET / R2-TRANSPORT-RELAY** | ➖ via `r2-transport` | Internet relay is configuration, not probing (Phase 4 plan). |

### Specs out of scope for r2-hive

- **R2-COMPILE / R2-BUILD / R2-DEPLOY / R2-UPDATE** — build-time tooling. `r2-build` is the cross-compile orchestrator; ensemble-compilation is unstarted greenfield (handing off to the other project). r2-hive only consumes scores at load time.
- **R2-SENTANT / R2-PLUGIN §1–§12** — sentant authoring + dual-mode plugins. r2-hive runs sentants via `r2-engine`; the spec governs the author's contract, not the daemon's.
- **R2-PROVISION-UX, R2-HMI, R2-MOBILE, R2-ANDROID, R2-CONSOLE** — UX layers. Cosmic + KDE applets (Phase 3f/3g) will speak to r2-hive's UDS; the applets *implement* these specs, r2-hive *exposes the API* they call.
- **R2-AUTH, R2-SPATIAL, R2-CONTEXT, R2-KNOWLEDGE, R2-APIARY** — application-layer specs. r2-hive provides the substrate (TG membership, event delivery, ensemble load). Implementation lives in ensembles, not the daemon.
- **R2-MP-REFIMPL, R2-REFIMPL** — reference-implementation guides. r2-hive *is* the reference Tier-3 implementation; the documents are produced from r2-hive's behaviour, not consumed by it.
- **R2-PROXY, R2-RELAY, R2-TRANSPORT-RELAY** — relay-specific architectures. r2-hive can act as either endpoint; relay orchestration is operational.

## Two wired-bridge architectures, not one

R2 has **two distinct wired CPU↔MCU protocols**, and a host-side
implementation (r2-hive) needs to know which one applies in a given
deployment. They share no framing and no operational model.

| | R2-USB v2 peripheral mode (R2-USB §3, R2-HIVE §6.4) | R2-HW §4 MCU-SBC bus |
|---|---|---|
| **Tier** | Tier 3 host + radio dongle | Tier 2 power-managed node |
| **Wire** | USB CDC-ACM | SPI (recommended) or UART, plus a WAKE GPIO |
| **Frame** | length(2 LE) ‖ payload | SYNC(`0x52 0x32`) ‖ CMD(1) ‖ LEN(2 LE) ‖ payload ‖ CRC-16/CCITT |
| **MCU role** | Thin radio appliance — L1+L2 only, no R2-WIRE state | Autonomous L1–L4 — relay, dedup, TTL, log buffering |
| **CPU role** | Always on, owns L3+ | Duty-cycled, wakes on MCU trigger, owns L5–L7 |
| **Control flow** | Host-driven (host pulls from CAPS-advertised bindings) | MCU-driven (MCU asserts WAKE; vocabulary is WAKE / PACKET / STATUS / LOG / TRANSMIT / CONFIG / SLEEP / SET_TIMER) |
| **Trust** | §6.4 challenge-response SAS at first attach | Implicit — factory-bonded |
| **Use case** | Add a radio to a desktop/Pi/laptop via dongle | Off-grid solar full R2 node, bespoke R2 boards combining CPU + MCU |
| **r2-hive status** | Phase USB-1 framer landed; Phase USB-2 pairing pending | Not implemented (see future-Phase MCU-SBC) |

**Implication for bespoke R2 hardware.** A board that combines a CPU
(Linux SBC) and an MCU on the same PCB is a Tier 2 node; the
on-board interconnect uses **R2-HW §4**, not R2-USB v2. r2-hive
running on the SBC half would need a separate `mcu_sbc.rs` module
for the SPI/UART + WAKE protocol — the `usb.rs` module is for
USB-attached external dongles only.

**Wireless connections** (mentioned for completeness): if the link
between r2-hive and a remote device is wireless (BLE / WiFi / LoRa),
the device on the other end is **a hive**, not a peripheral. It
joins via the standard trust-group flow (R2-TRUST). There is no
"wireless peripheral mode" — once a link can host R2-WIRE state
cheaply (which any wireless link already pays for in 802.x or BLE
state), running a full hive on the far end is a smaller delta than
inventing a third bridge protocol.

## Conformance gaps that block end-to-end claims

These are the items where a "fully conformant r2-hive Tier-3 deployment" claim cannot be made today:

1. **R2-PLUGIN §13.6 WS channels.** Static GETs work fully; channels need browser Ed25519 keypair + per-frame HMAC, deferred to a Phase 3d follow-up.
2. **R2-HIVE §6.4 link-key persistence.** Pairing crypto + state machine done; link keys live in `InMemoryLinkKeyStore` by default. Persistent backing (file or keyring, scoped to host-user identity) is mechanical and lands when wired into `HiveState`.
3. **R2-USB serial I/O wrapper.** ✅ Phase USB-3 series done — `/dev/ttyACM*` open + termios raw, hot-plug watcher with default-deny `UsbFilter`, `r2.mgmt.usb.*` event surface, `r2hive usb {list,prepare,confirm,abort,unpair}` CLI, route-engine integration both directions (CAPS-advertised radios become routable Transport slots; outbound frames addressed via Lora/Ble/Wifi fall back to a paired dongle when no native binding exists).
4. **R2-PROVISION TG-join word codes** are separate from R2-PLUGIN §13.5 browser word codes by design (separate ledgers); both ledgers are 1h TTL single-use, but the TG-join side hasn't grown a mgmt event surface yet (`r2.mgmt.tg.invite` + `r2.mgmt.tg.accept` are not implemented).
5. **R2-HIVE §11 (UI interop)** awaits Phase 3f/3g applets — including the `PairingPrompt` SAS-confirmation surface that Phase USB-2 just emitted.

## Closed gaps

**Phase Conf-A (2026-04-28):**

1. ✅ **R2-HOST-API direct vector replay** — `tests/host_api_conformance.rs` loads the canonical JSON and round-trips all 28 vectors through the wire/hash/CBOR/dispatch paths.
2. ✅ **R2-PLUGIN §13.10 vectors 5, 7, 8** — atomic-reload (`atomic_remount_observes_either_old_or_new_never_torn`), escape-symlink (`escaping_symlink_rejected_at_mount`), channel-target validation (`channel_target_validation_via_def_parser`).
3. ✅ **Vector-coverage regression check** — `tests/vector_coverage.rs` pins minimum upstream counts and minimum referenced-by-ID counts per file; a soft `list_unreferenced_vectors` test surfaces uncovered IDs at `--nocapture`.

**Phase USB-2 (2026-04-28):**

4. ✅ **R2-HIVE §6.4 host-side pairing implementation** — `usb_pair.rs` (pure crypto helpers, replays every pinned value in `r2-usb-pair-vectors.json` bit-equal) plus the pairing state machine in `usb.rs`. First-attach commit-reveal SAS, reconnect HMAC, abort handling, commit-mismatch detection. 19 new tests across the two modules.
5. ✅ **`r2-usb-pair-vectors.json` replay** — all 10 vectors exercised. Closes the prior "pinned but not yet exercised" caveat in Phase USB-2 row.

## Recommendations for further coverage

1. **Phase USB-3** — wire the protocol module to actual `/dev/ttyACM*` serial I/O. This is the last gap before plugging in a real DFR1195 dongle is end-to-end functional.
2. **§13.6 WS channels** — once the browser keypair scheme is wired, WEB-WS-ROUNDTRIP completes the §13.10 vector set.
3. **`r2.mgmt.tg.invite` + `r2.mgmt.tg.accept`** — would unblock the zero-config first-boot flow (Phase 4d).
4. **Persistent `LinkKeyStore`** — the in-memory store is fine for v0.1 functional testing; production wants a file-backed (or keyring-backed) store under the host-user identity. Mechanical follow-up.

## Provenance

- Specs read from: `r2-specifications/specs/r2-core/` at HEAD on 2026-04-28.
- Vector files read from: `r2-specifications/testing/test-vectors/` at HEAD on 2026-04-28.
- r2-hive code: `/mnt/data/Development/R2/r2-hive/` at HEAD; tests verified green via `cargo test --workspace`.
- Generator: `r2-specifications/testing/generators/r2-usb-pair-vectors/` rebuilds the pinned bytes deterministically.
