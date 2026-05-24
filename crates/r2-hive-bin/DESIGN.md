# R2 Hive Design Notes

## What is Hive

Multi-transport mesh participant for Reality2. One Rust codebase deployable as:
- **Cloud**: WebSocket/TCP transport (replaces r2-relay)
- **Edge**: BLE + WiFi + LoRa + WebSocket on hardware like the Uno-Q

Hive operates at Layers 1-4 only. It reads R2-WIRE headers, feeds observations into the route engine, and forwards frames based on routing decisions. It never decrypts payloads (Layer 5+).

## Architecture

Specs-first, crates-second, hive-third:
1. Review/update R2 specifications and test vectors
2. Implement each layer as an r2-core crate, tested against vectors
3. Hive is a thin binary composing proven crates

### Crate Structure

- **r2-wire**: Frame encode/decode, compact/extended, transcoding
- **r2-route**: RouteEngine, neighbour tables, path confidence, spray-and-wait
- **r2-transport**: Transport trait, TransportId, framing helpers
- **r2-discovery**: AsyncTransport trait, TransportRegistry, beacon discovery, transport bindings
- **r2-hive**: Thin binary composing the above + compat layer for legacy clients

### Plugin Architecture

Hive capabilities are plugins that provide axum routes:
- `plugins/word_codes.rs`: Three-word invitation codes (POST/GET /word-code)
- `plugins/dashboard.rs`: HTML status page

## Transport Model

| Transport | Discovery | Data | Wire Format |
|-----------|-----------|------|-------------|
| Local radio | BLE beacon + L2CAP | WiFi direct (+ L2CAP for small events) | Compact / Extended |
| Local network | Zeroconf/mDNS | UDP/TCP on LAN | Extended |
| LoRa | LoRa beacon | LoRa frames | Compact |
| Internet | Configured | WebSocket / TCP | Extended |

BLE and WiFi direct are one pipeline. A hive can be on all four simultaneously.

## Key Design Decisions

### Routing Above Transport (2026-04-04)

Mesh intelligence lives above the transport, not in it. WiFi doesn't route. LoRa doesn't route. The R2-ROUTE engine routes, treating all transports as interchangeable paths with different quality/cost characteristics. Bridging between transports emerges naturally from the route engine making forwarding decisions across available transports.

### Anonymous Beacon Discovery (2026-04-04)

Discovery always starts with an anonymous beacon: class hash + capability bloom filter + rotating beacon ID. No trust group identity in beacons. Trust is established after connection via certificate exchange. This applies to BLE, Zeroconf, and LoRa equally. Internet transport is the exception (configured address, no beacon).

### Extended R2-WIRE Frames for WebSocket (2026-04-04)

WebSocket uses Internet transport (R2-TRANSPORT §3.5) which requires Extended format. Notekeeper was sending Compact frames over WebSocket - not spec-conformant. Migrated to Extended frames with encrypted note content as CBOR payload (key 3 = byte string). This eliminates the separate 0xFE plugin data channel and makes all note sync frames routable by the engine.

Payload format: `{0: opCode, 1: noteId, 2: timestamp, 3?: DEK-encrypted content}`

### Plugin Data Routing (2026-04-04)

The route engine only understands R2-WIRE frames. Plugin data has two paths:
1. **Inside R2-WIRE payloads**: Small data, routed by the mesh (standard path)
2. **Independent channels**: Plugins use mesh discovery to find peers, then open their own connections for bulk transfer (like WiFi handoff for OTA)

The 0xFE plugin prefix scheme was removed. All sync data now rides inside R2-WIRE frames.

### Event-Based Sync Over Constrained Transports (2026-04-04)

Sentant state = sequence of events, not monolithic snapshots. A note isn't transferred as a blob - it's a sequence of events (note.create, note.update, ...) each carrying the change. On constrained LoRa paths (200-byte compact frames), events trickle through one at a time. The destination reconstructs state by replaying events.

One big update is equivalent to several smaller ones. No special chunking protocol needed - the auto-save can flush more frequently with smaller content on constrained paths.

### Future: Diff-Based Updates (logged 2026-04-04)

Currently every note.update sends full content. Future improvement: send diffs (character-level or CRDT operations) for efficiency on constrained transports. Low priority until BLE/LoRa transports are in use.

## Implementation Progress

### Phase 1: Verify existing crates (DONE)
- r2-wire: 32 tests, added 4 transcoding vectors (TC1-TC4)
- r2-route: 8 tests, existing vectors sufficient
- r2-transport: 34 tests, created r2-transport-vectors.json (22 vectors)

### Phase 2: r2-discovery crate scaffold (DONE)
- AsyncTransport trait, InboundFrame, TransportRegistry
- BeaconAdvertiser/BeaconScanner traits
- 5 mock transport tests

### Phase 3: Internet transport bindings (DONE)
- WebSocket and TCP AsyncTransport implementations
- Shared PeerMap for peer management
- 13 tests including 9 spec conformance tests

### Phase 4: r2-hive binary (DONE)
- Composes r2-discovery + r2-route + r2-wire
- Compat layer: HELLO/WELCOME handshake, catchup buffer, trust group prefix matching
- Plugins: word codes, dashboard
- Regression tested: Notekeeper syncs identically to r2-relay

### Phase 5: Route engine integration (DONE)
- RouteEngine in HiveState, shadow mode
- Parses R2-WIRE extended headers, ingests observations
- Periodic decay maintenance (30s interval)
- Neighbour table logging

### Phase 5b: Extended frame migration (DONE)
- Added hmac_extended_tag/verify_extended_hmac to r2-wasm
- Added cbor_encode_note_event/cbor_decode_note_event (typed CBOR, JS never touches wire format)
- Notekeeper sends single extended R2-WIRE frame per operation
- Removed 0xFE plugin data channel
- R2-CBOR §12A: CBOR encapsulation requirement added to spec

### Phase 6: Route-engine-driven forwarding (DONE)
- Route engine's plan_forward() drives frame forwarding
- Non-R2-WIRE frames (0xFF join protocol) fall back to TG broadcast
- Trust group isolation above the routing layer: engine proposes, TG map constrains
- flood_tg_peers_not_in handles freshly connected peers not yet known to engine
- Routes strengthen with use as engine learns neighbours from traffic
- Verified: Notekeeper syncs with route-engine-driven forwarding

### Phase 7: LAN discovery (IN PROGRESS)
- R2-BEACON §8.4 mDNS profile written (normative spec + 3 test vectors)
- mDNS module implemented but mdns-sd library has reliability issues with multicast
- UDP broadcast beacon as working alternative: port 21044, 5s interval
- Beacon packet: magic + version + flags + class_hash + RBID + port + bloom
- Tested: laptop (192.168.1.52) ↔ Alfred (192.168.1.54) discover in 5 seconds
- Next: auto-connect discovered peers and exchange R2-WIRE frames over UDP
