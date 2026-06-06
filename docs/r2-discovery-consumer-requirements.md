# R2-DISCOVERY — consumer requirements from r2-hive

Status: **input for the normative R2-DISCOVERY spec** (Roy ruled spec-first, 2026-06-06).
This documents exactly how r2-hive (the one known consumer today) uses `r2_discovery`,
extracted from its call sites. It is requirements input, not a spec — the normative
API is specs' to design, and may differ. Feature-tier it for the future multi-target
r2-hive (no_std ESP32 / alloc / std Linux / wasm).

All call sites: `crates/r2-hive-bin/src/{hive.rs, main.rs, plugins/word_codes.rs}`.

## 1. `AsyncTransport` (trait) — the common send abstraction
Imported wherever a transport is used; r2-hive calls:
```rust
// hive.rs:427,440,459 ; word_codes.rs:88 ; (broadcast uses hive_id = 0)
transport.send(hive_id: u32, frame: &[u8]).await   // -> Result<_, _>, used via .is_ok()
```
Implemented by: `UdpLanTransport`, `BleTransport`, `LoraTransport` (and conceptually the
WS peer path, though WS exposes `send` via its PeerMap — see §2).

## 2. `WebSocketTransport` (host/relay transport, owned not Option)
`hive.rs`: `pub ws_transport: WebSocketTransport`, built `WebSocketTransport::new(4096)` (capacity: usize).
Exposes a peer map via `.peers()` with:
```rust
peers().add_peer(hive_id: u32, quality: LinkQuality).await -> Receiver<Vec<u8>>  // outbound_rx (handshake.rs)
peers().push_inbound(hive_id: u32, data: Vec<u8>, quality: LinkQuality).await
peers().peer_count().await -> usize
peers().remove_peer(hive_id: u32).await
peers().hive_ids().await -> Vec<u32>                  // main.rs:572,689
peers().send(hive_id: u32, frame: &[u8]).await -> Result // main.rs:574 ; hive.rs:414,630
```
`LinkQuality` is `r2_transport::transport::LinkQuality` (already vendored in r2-core).

## 3. Concrete radio/IP transports (all `Arc<_>`, `.clone()`-able, behind feature tiers)
Stored as `RwLock<Option<Arc<T>>>` in HiveState; setters take `Arc<T>`.

### `bindings::udp_lan::UdpLanTransport`  (feature: udp-lan, std)
```rust
UdpLanTransport::bind(addr: &str).await -> Result<Arc<UdpLanTransport>, String>
udp.recv().await -> Option<Frame>            // Frame { data: Vec<u8>, source_hive: u32 }
udp.add_peer(hive_id: u32, addr: std::net::SocketAddr).await
udp.send(hive_id, frame).await               // via AsyncTransport (hive_id 0 = broadcast)
```

### `bindings::ble::BleTransport`  (feature: ble)
```rust
BleTransport::new(name: String).await -> Result<(Arc<BleTransport>, disco_rx), Error>  // NOTE: returns a TUPLE
ble.sched().clone()                           // scheduler handle, passed to BleBeaconAdvertiser::new
ble.recv().await -> Option<Frame>
ble.register_peer(hive_id: u32, addr: bluer::Address).await
ble.send(hive_id, frame).await                // via AsyncTransport
// disco_rx is consumed by BleBeaconScanner::new(disco_rx) — see §4
```

### `bindings::lora::LoraTransport`  (feature: lora)
```rust
LoraTransport::with_socket(socket: &str).await -> Result<Arc<LoraTransport>, String>
lora.recv().await -> Option<Frame>            // carries COMPACT R2-WIRE frames
lora.send(hive_id, frame).await               // via AsyncTransport
```

## 4. Beacon discovery
Traits brought into scope: `BeaconScanner`, `BeaconAdvertiser`.

### `discovery::udp_beacon::UdpBeacon`
```rust
UdpBeacon::new(class_hash: u32, rbid: &[u8], port: u16, bloom: &[u8], bloom_k: <int>) -> UdpBeacon
beacon.start(tx: mpsc::Sender<Discovered>).await -> Result<(), String>
// Discovered { address: Vec<u8> (UTF-8 "ip:port"), class_hash: u32, rbid: [u8; 8] }
```

### `discovery::ble_beacon::BleBeaconAdvertiser`
```rust
BleBeaconAdvertiser::new(sched: <Sched>, rbid: [u8; 8]) -> BleBeaconAdvertiser
advertiser.start(class_hash: u32, bloom: &[u8], bloom_k: <int>).await
```

### `discovery::ble_beacon::BleBeaconScanner`
```rust
BleBeaconScanner::new(disco_rx) -> BleBeaconScanner   // disco_rx from BleTransport::new
scanner.next_beacon().await -> Option<Beacon>          // Beacon { address: Vec<u8> (>=6 = BLE MAC), rbid: [u8;8] }
```

## 5. Identity mapping — ⚠️ SUPERSEDED (r2-hive's current shape is WRONG)

> **CORRECTION (specs, 2026-06-06):** r2-hive *currently* uses pure functions that
> encode hive_id in the high 4 bytes of an 8-byte rbid:
> ```rust
> rbid_for_hive_id(hive_id: u32, rotating: [u8; 4]) -> [u8; 8]   // hive_id in high 4
> hive_id_from_rbid(rbid: &[u8]) -> u32                          // extracts hive_id
> ```
> This **CONTRADICTS canon and must NOT be built**. Per **R2-BEACON §6.1** the RBID is
> `HMAC-SHA256(session_key, epoch)[0:8]` — a privacy-preserving *rotating* ID whose
> whole purpose is to *prevent* device tracking. Encoding hive_id in it would leak
> identity and defeat rotation. **You cannot extract hive_id from an rbid.**
>
> **Canonical model (R2-TRANSPORT §2.1.3) — a RESOLVE, not a pure fn:**
> - Beacon discovery matches an observed rbid to a *known* peer (via that peer's
>   `session_key`) and registers `add_peer(hive_id, transport_address)`.
> - Receive does reverse-lookup `transport_address -> hive_id`.
> - So rbid→hive_id is a **registry/resolver LOOKUP**; minting an rbid needs the
>   peer's `session_key` + `epoch`, not `[u8; 4]`.
>
> **r2-hive impact:** the discovery wiring in `main.rs` (`start_lan_discovery`,
> `start_ble`, both beacon handlers) is built on the pure-fn shape and the
> "same hive_id across UDP/BLE via rbid encoding" assumption — both must be
> reworked to the resolver model when R2-DISCOVERY lands. Tracked separately.

## 6. Shared types implied
- `Frame { data: Vec<u8>, source_hive: u32 }` — returned by every transport's `recv()`.
- `Discovered` / `Beacon` — beacon results (fields above).
- Depends on `r2_transport::transport::LinkQuality`.

## 7. Feature tiers r2-hive selects
Cargo: `r2-discovery = { features = ["websocket","mdns","udp-lan"] }` always; `ble`, `lora` gated by r2-hive's own `ble`/`lora` features. `websocket/mdns/udp-lan` are std; `ble`/`lora` are radio (embedded-capable). Please tier so the no_std core (ESP32) and wasm builds drop the std/host transports cleanly.

---

## 8. Migration delta — r2-hive current code → ratified R2-DISCOVERY v0.1 API

The spec (R2-DISCOVERY.md §4, ratified 2026-06-06) is now the canonical contract;
r2-hive's existing consumer code predates it and must be migrated. Do this
**compiler-assisted once core lands the r2-discovery crate** (the surface below has
type details — `TransportAddress`, `LinkQuality` fields, `WebSocketTransport`'s
`AsyncTransport` impl, the UDP scan model — that are easiest to finalize against the
real crate). Concrete deltas:

1. **`recv()` Option → Result.** `while let Some(frame) = t.recv().await` →
   `loop { match t.recv().await { Ok(frame) => …, Err(Closed) => break, Err(e) => … } }`.
   Sites: `main.rs` udp/ble/lora recv loops.
2. **`PeerMap` methods are sync** (§4.4 `pub fn`, no `async`). Drop `.await` on
   `peers().add_peer / push_inbound / peer_count / remove_peer / hive_ids`.
   Sites: `handshake.rs`, `hive.rs`, `main.rs`.
3. **No `PeerMap::send`.** r2-hive calls `peers().send(hive_id, frame)` in
   `main.rs:574`, `hive.rs:414,630`. Ratified outbound to a WS peer goes via the
   `OutboundRx` from `add_peer`, or `WebSocketTransport: AsyncTransport::send`.
   Rewire those 3 sites (likely `ws_transport.send(hive_id, frame).await`).
4. **`LinkQuality` source.** `add_peer(hive_id, link: r2_discovery::LinkQuality)`
   (§4.1), but `handshake.rs:41` builds `r2_transport::transport::LinkQuality`.
   Switch the type (fields are transport-defined — confirm against core's impl).
   **NOTE: this touches the committed v0.2 handshake (branch `v0.2-relay-handshake`).**
5. **rbid resolver.** Replace `hive_id_from_rbid(&rbid)` →
   `PeerRegistry::resolve_rbid(&Rbid) -> Option<HiveId>` (+ provisional FNV-1a of the
   transport address for unknown peers, §3.3); replace `rbid_for_hive_id(hive_id,[u8;4])`
   → `PeerRegistry::own_rbid(epoch) -> Rbid`. r2-hive must obtain/implement a
   `PeerRegistry` (knows trusted peers' `session_key`s). The "same hive_id across
   UDP/BLE via rbid" assumption and the `rbid == self` beacon self-skip both go away.
6. **Newtypes / shapes.** `Rbid([u8;8])` (wrap `random_rbid()`); `UdpBeacon::new`
   and `BleBeacon{Advertiser}::new` take `Rbid`; beacon results are
   `BeaconObservation { rbid: Rbid, class_hash, transport_address: TransportAddress, link }`
   replacing the ad-hoc `Discovered`/`Beacon` (`.address: Vec<u8>` → `.transport_address`).
7. **Errors.** `DiscoveryError` (§4.2) replaces the `String` errors that
   `bind`/`with_socket`/`start` returned; adapt `start_lan_discovery`/`start_ble`/
   `start_lora` return types + `?` sites.
8. **UDP scan model.** r2-hive uses `UdpBeacon::new(...) + beacon.start(tx)` with a
   channel of `Discovered`; ratified discovery is `BeaconScanner::next_beacon() ->
   BeaconObservation`. Confirm the UDP scanner shape against core's impl.

§6 above ("`Discovered`/`Beacon`", "depends on `r2_transport::…::LinkQuality`") is
superseded by this section.
