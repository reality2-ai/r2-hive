# r2-hive architecture

A module-by-module guide to the daemon — how a frame travels in,
through, and out, where each piece of state lives, and which R2 crates
own which contract.

---

## Top-level layout

```text
tools/r2-hive/
├── Cargo.toml             — workspace member, depends on every L1–L5 R2 crate
├── README.md              — consumer-facing overview
├── DESIGN.md              — operator notes, four-profile target, deployment surfaces
├── TEST-RIG.md            — 4-node smoke rig (laptop + Pi 5 + 2× UNO-Q)
├── docs/
│   ├── mgmt-api.md        — concrete CBOR shapes for every event class
│   └── architecture.md    — this file
├── src/
│   ├── lib.rs             — library re-exports (compat, hive, mgmt, plugins, router)
│   ├── main.rs            — binary entry point; wires everything up
│   ├── hive.rs            — HiveState: transports, route engine, subscribers, TGs, ensembles
│   ├── router.rs          — route_frame: wraps r2-route and dispatches to ensembles
│   ├── compat/            — legacy compat (ring buffer, sender wrappers)
│   ├── plugins/           — built-in plugins (dashboard, word-codes)
│   ├── mgmt/
│   │   ├── mod.rs         — module aggregation, default_socket_path
│   │   ├── api.rs         — frame dispatcher, event_class → handler routing
│   │   ├── ensemble.rs    — r2.mgmt.ensemble.* handlers + HiveOutboundSink
│   │   ├── primitive.rs   — r2.api.* handlers (peer, event, tg, cap, service)
│   │   ├── socket.rs      — UDS server, per-connection writer/reader pair
│   │   ├── ws.rs          — /r2/mgmt WebSocket handler (axum upgrade)
│   │   ├── framing.rs     — length-prefixed UDS framer
│   │   ├── identity.rs    — master-secret store backends + handle
│   │   ├── state.rs       — DaemonState (version, identity, hive_state ref)
│   │   └── subscriptions.rs — per-connection SubscriptionRegistry + filters
│   └── ...
└── tests/
    ├── mgmt_integration.rs   — daemon status, identity, peer, event, tg, cap, subscribe (18 tests)
    └── ensemble_integration.rs — load/list/info/stop round-trip; bad-score; not_loaded (3 tests)
```

---

## Layered view (R2 stack)

```text
┌─────────────────────────────────────────────┐
│  L7   user application                       │   any client speaking R2-WIRE
├─────────────────────────────────────────────┤
│  L6   sentant runtime                        │   r2-engine (Sentant trait)
├─────────────────────────────────────────────┤
│  L5   trust groups                           │   r2-trust (cert, HKDF, X25519, HMAC)
├─────────────────────────────────────────────┤
│  L4   ensemble + dispatch                    │   r2-ensemble + r2-dispatch
├─────────────────────────────────────────────┤
│  L3   route engine                           │   r2-route
├─────────────────────────────────────────────┤
│  L2   wire framing                           │   r2-wire (extended) + r2-cbor + r2-fnv
├─────────────────────────────────────────────┤
│  L1   transports                             │   r2-discovery (WS, UDP, BLE, LoRa)
└─────────────────────────────────────────────┘
```

r2-hive is the integration host: it owns one instance each of the L3
RouteEngine, L4 EnsembleRegistry, L5 trust state, and the L1 transports.

---

## State ownership

`HiveState` is the single source of truth for the daemon's operational
state. It is wrapped in `Arc<HiveState>` and referenced from every
async task (router, transport read loops, mgmt handlers, the ensemble
sink).

| Field | Purpose |
|---|---|
| `self_hive_id: u32` | Canonical identifier for this hive (for the route stack) |
| `ws_transport: WebSocketTransport` | Internet-class transport |
| `udp_transport: RwLock<Option<Arc<UdpLanTransport>>>` | UDP-LAN, populated when `--lan` |
| `ble_transport` (cfg) | BLE / L2CAP CoC, populated when `--ble` |
| `lora_transport` (cfg) | LoRa via arduino-router IPC |
| `route_engine: Mutex<RouteEngine>` | r2-route engine; consults observations + table |
| `active_tg: RwLock<Option<ActiveTg>>` | Current TG attachment (R2-TRUST §13) |
| `subscribers: Mutex<Vec<Subscriber>>` | Per-connection mgmt-API subscription state + tx |
| `tg_map: RwLock<HashMap<TG, TrustGroupCompat>>` | TG → peer set + ring buffer |
| `word_codes: WordCodeStore` | Pairing flow state |
| `ensembles: Arc<EnsembleRegistry>` | Loaded ensembles + dispatch target |
| `frames_routed`, `connections_total` | Atomic counters |

`DaemonState` is a lighter wrapper around the *daemon-level* metadata
(version, build hash, identity, started_at) plus a `OnceLock<Arc<HiveState>>`
so mgmt handlers can reach the operational layer when one is attached.

---

## Inbound frame lifecycle

A frame arriving on any transport runs through `router::route_frame`:

```text
1. Transport read task receives a buffer.
2. route_frame(state, source_hive, transport, frame):
     a. r2_wire::decode_extended (with optional 32-byte HMAC trim)
     b. extract originator from route stack
     c. feed Observation to route_engine (immediate source, transport, RSSI hint)
     d. state.deliver_inbound — re-fan to mgmt-API subscribers whose filters match
     e. RouteEngine.plan_forward → ForwardAction
3. ForwardAction match:
     - Drop          → log; done
     - DeliverOnly   → build DispatchEnvelope; state.ensembles.dispatch(env)
                      (registry walks subscribers, runs Sentant::handle_event)
     - Directed(hop) → prepare_relay_extended; send_to_hive_via(hop.neighbour, hop.transport)
     - Flood         → prepare_relay_extended; broadcast via flood_tg_peers_not_in
4. send_to_hive_via tries WS → UDP → BLE → LoRa fallback chain.
```

The DeliverOnly arm is where ensemble runtime begins. Without an
ensemble registered, dispatch returns `DispatchError::NoHandler`, which
the router treats as benign — it has already done its work.

---

## Outbound from a sentant

When a sentant emits an `Action::Send` inside `handle_event`, the chain
is:

```text
1. r2_ensemble::EnsembleRegistry collects the actions after handle_event returns.
2. apply_actions converts each Send/DelayedSend to OutboundEvent and calls
   the configured OutboundSink::deliver.
3. r2-hive's HiveOutboundSink resolves Target:
     - Sender         → originator hive_id (looped back via deliver_inbound if self)
     - Sentant/Local  → fanout to mgmt-API subscribers via deliver_inbound
     - TrustGroup     → broadcast_to_tg(active TG hash, self_hive_id, frame)
     - Broadcast      → broadcast_to_tg if a TG is attached, else logged-and-dropped
4. The selected send path uses send_to_hive (WS → UDP → BLE → LoRa fallback)
   or broadcast_to_tg (iterates TG peers).
```

The sink lives entirely on the r2-hive side; r2-ensemble is unaware
of transports. This is the same boundary `r2-dispatch` defines as
normative: the registry produces `DispatchEnvelope`s and consumes
opaque `OutboundEvent`s.

---

## Mgmt-API request lifecycle

```text
UDS / /r2/mgmt WS
        │
        ▼
mgmt::framing::read_frame  (length prefix on UDS; binary message on WS)
        │
        ▼
mgmt::api::handle_frame_with_subs(frame, daemon_state, subs)
        │
   r2_wire::decode_extended
        │
   match event_hash:
        ├── r2.mgmt.daemon.*    → DaemonState read accessors
        ├── r2.mgmt.identity.*  → identity::Status fields
        ├── r2.mgmt.event.error → never inbound; only outbound
        ├── r2.api.peer.*       → primitive::handle_peer_*  (uses HiveState)
        ├── r2.api.event.*      → primitive::handle_event_* (uses subs + HiveState)
        ├── r2.api.tg.*         → primitive::handle_tg_*    (uses active_tg)
        ├── r2.api.cap.*        → primitive::handle_cap_*
        ├── r2.api.service.*    → primitive::handle_service_* (Phase 2c)
        └── r2.mgmt.ensemble.*  → ensemble::handle_*         (uses HiveState.ensembles)
        │
   build response frame (encode_extended + CBOR payload)
        │
        ▼
mgmt::framing::write_frame
```

Subscriptions use a per-connection mpsc channel with capacity 1024. The
read loop and a writer task ride the same `tokio::select`. When
`HiveState::deliver_inbound` finds a matching filter for an inbound
frame, it `try_send`s the `event.delivery` notification onto that
connection's channel; backpressure replaces the delivery with a
`r2.mgmt.event.error{code: backpressure}`.

---

## OTP-style supervision

`r2-ensemble` runs full supervisor semantics inside the registry. From
r2-hive's perspective there is one new lifecycle event: when a sentant
crashes during dispatch, the registry catches the panic, marks the
instance gated, and tokio-spawns a rebuild via the registered
`SentantFactory`. The router observes only the `DispatchError::Rejected`
return and continues — it does not need to know about supervision.

The mgmt API surface for supervision is on `r2.mgmt.ensemble.{info,reset}`:

- `info` reports `status: Healthy / Degraded / Failed`.
- `reset` is the operator escape hatch from `Failed`: clears the ledger
  and rebuilds every sentant from its def.

---

## Crate dependency graph

```text
r2-hive
 ├── r2-wire
 │    └── r2-fnv
 ├── r2-cbor
 ├── r2-fnv
 ├── r2-route
 │    └── r2-fnv
 ├── r2-transport
 │    └── r2-wire
 ├── r2-discovery
 │    ├── r2-transport
 │    └── r2-wire
 ├── r2-engine
 │    ├── r2-fnv
 │    ├── r2-cbor
 │    └── r2-wire
 ├── r2-dispatch
 ├── r2-def
 └── r2-ensemble
      ├── r2-def
      ├── r2-engine
      ├── r2-dispatch
      └── r2-fnv
```

Every leaf crate is independently usable. r2-hive is the only place the
full graph is composed.

---

## Where to look first if …

| Question | File |
|---|---|
| "Why was that frame dropped?" | `router.rs` — log statements at each `ForwardAction` arm |
| "How does an ensemble get its events?" | `mgmt/ensemble.rs` (load) + `crates/r2-ensemble/src/registry.rs` (dispatch) |
| "Where do events emitted by a sentant go?" | `mgmt/ensemble.rs::HiveOutboundSink` |
| "What does the mgmt socket recognise?" | `mgmt/api.rs` — single dispatch table by event_hash |
| "Why is my `r2hive event subscribe` not seeing events?" | `mgmt/subscriptions.rs` filters + `hive.rs::deliver_inbound` |
| "Why does my new ensemble fail to load?" | `mgmt/ensemble.rs::handle_load` → maps registry errors to one of the codes documented in `docs/mgmt-api.md` |
| "Where is the master secret derived?" | `mgmt/identity.rs` |
| "How are restarts paced?" | `crates/r2-ensemble/src/supervision.rs` |
