# r2-hive management API reference

Concrete byte-level documentation for every event class the daemon
recognises on its management surface (Unix-domain socket
`${XDG_RUNTIME_DIR}/r2-hive.sock` and loopback WebSocket `/r2/mgmt`).

For the normative spec see
`r2-specifications/specs/r2-core/R2-HOST-API.md`. This file is a
companion: it describes the implementation as it ships in this version
of r2-hive, with the exact CBOR shapes the daemon parses and emits.

---

## Transport

Every request and response is a single
[R2-WIRE extended frame](https://github.com/reality2-ai/r2-specifications)
preceded on the wire by a 4-byte big-endian length prefix (UDS only;
the WebSocket carries each frame as a single binary message).

- **Header**: 22 bytes (R2-WIRE §6).
- **`event_hash`**: FNV-1a 32-bit of the event class string (R2-FNV).
- **Payload**: CBOR map with integer keys (R2-CBOR Compact mode).
- **`msg_type`**: always `MsgType::Event` for management traffic.
- **HMAC**: not present — local management traffic does not cross the
  trust boundary.

Maximum frame size on UDS: 64 KiB.

### Common payload key

| Key | Type | Meaning |
|---|---|---|
| `0` | uint | `correlation_id` — opaque caller-supplied id, echoed in every response, error, and (for subscribe) every later `r2.api.event.delivery` notification |

### Error envelope

Every event class can answer with `r2.mgmt.event.error` instead of its
nominal response. The error envelope is:

```text
{
  0: <correlation_id : uint>,
  1: <code           : text>
}
```

### Error codes

| Code | Meaning |
|---|---|
| `bad_frame` | frame failed R2-WIRE decode or its CBOR payload is malformed |
| `unknown_event` | the daemon does not recognise this event class |
| `unsupported_dialect` | `r2.mgmt.ensemble.load` got a dialect other than `yaml`/`json`/`toml` |
| `bad_score` | score parse / validation failed |
| `already_loaded` | tried to load an ensemble whose name is already taken |
| `no_factory` | no registered `SentantFactory` could build a sentant |
| `external_unsupported` | score uses `SentantEntry::External`; v0.1 supports inline only |
| `bad_event_class` | event class string is not valid R2-FNV input |
| `not_loaded` | `info`/`stop`/`reset` referenced an unknown ensemble id |
| `not_in_tg` | event-send was issued without an attached trust group |
| `tg_not_found` | tg.* request referenced an unknown TG hash |
| `peer_not_found` | peer.query / event.send target is unknown |
| `backpressure` | subscriber's mpsc channel is full; delivery was dropped |
| `no_handler` | no sentant subscribes to the event |

---

## `r2.mgmt.daemon.status`

### Request payload
```text
{ 0: cid }
```

### Response payload
```text
{
  0: cid,
  1: <version       : text>,
  2: <build_hash    : text>,
  3: <uptime_seconds: uint>
}
```

---

## `r2.mgmt.identity.status`

### Request payload
```text
{ 0: cid }
```

### Response payload
```text
{
  0: cid,
  1: <present              : bool>,
  2: <fingerprint          : text>,    // 16-hex-char SHA-256 prefix
  3: <backend              : text>,    // "file" | "libsecret" | "keychain" | "wincred" | "none"
  4: <path                 : text>,    // filesystem path of the file store, "" otherwise
  5: <created_this_start   : bool>     // true iff this start generated a fresh master secret
}
```

---

## `r2.api.peer.list`

### Request payload
```text
{ 0: cid }
```

### Response payload
```text
{
  0: cid,
  1: [
    {
      1: <hive_id : uint>,
      2: <self    : bool>,
      3: <transports : [text, …]>     // any of "ws", "udp", "ble", "lora"
    },
    …
  ]
}
```

---

## `r2.api.peer.query`

### Request payload
```text
{ 0: cid, 1: <hive_id : uint> }
```

### Response payload
```text
{
  0: cid,
  1: <hive_id    : uint>,
  2: <status     : uint>,             // 0=Self, 1=Online, 2=Stale, 3=Unknown
  3: <transports : [text, …]>,
  4: <quality    : float32>           // 0..1; absent if status != Online
}
```

---

## `r2.api.tg.current`

### Request payload
```text
{ 0: cid }
```

### Response payload
```text
{
  0: cid,
  1: <tg_id     : bytes>,             // 32 bytes; absent when detached
  2: <role      : uint>,              // 1=Member, 2=KeyHolder; absent when detached
  3: <hive_id   : uint>                // absent when detached
}
```

When detached only `{ 0: cid }` is returned.

---

## `r2.api.cap.query`

### Request payload
```text
{ 0: cid, 1?: <target_hive : uint> }
```

If `target_hive` is omitted the response describes the local daemon.

### Response payload
```text
{
  0: cid,
  1: <target_hive : uint>,
  2: [<capability : text>, …]         // bloom-derived class strings; empty in v0.1
}
```

---

## `r2.api.event.send`

### Request payload
```text
{
  0: cid,
  1: <event_class : text>,
  2: <payload     : bytes>,
  3?: <target_hive  : uint>,
  4?: <target_class : text>
}
```

### Response payload
```text
{ 0: cid, 1: <msg_id : uint> }
```

`msg_id` is the R2-WIRE message id assigned to the outbound frame; the
caller can use it to correlate with later route-engine events.

---

## `r2.api.event.subscribe`

### Request payload
```text
{
  0: cid,
  1?: <event_class : text>,
  2?: <event_hash  : uint>,
  3?: <from_hive   : uint>,
  4?: <from_tg     : bytes>          // first 8 bytes of SHA-256(TG_PK)
}
```

At most one of `event_class` and `event_hash` may be present.

### Response payload
```text
{ 0: cid, 1: <sub_id : uint> }
```

### Subsequent unsolicited push (`r2.api.event.delivery`)
```text
{
  0: <sub_id        : uint>,         // NB: cid is the original sub correlation
  1: <event_class   : text>,
  2: <event_hash    : uint>,
  3: <payload       : bytes>,
  4: <source_hive   : uint>,
  5?: <source_tg    : bytes>,
  6: <msg_id        : uint>
}
```

---

## `r2.api.event.unsubscribe`

### Request payload
```text
{ 0: cid, 1: <sub_id : uint> }
```

### Response payload
```text
{ 0: cid, 1: <sub_id : uint> }
```

---

## `r2.api.service.advertise`

### Request payload
```text
{
  0: cid,
  1: <service_class : text>,         // FNV-hashed; the service-sentant subscribes to this
  2?: <state        : text>          // optional initial state hint for the registry
}
```

### Response payload
```text
{ 0: cid, 1: <service_id : uint> }
```

After advertise, every event of `service_class` reaching this hive is
forwarded back to the advertising connection over the same mpsc channel
that drives `event.delivery`.

---

## `r2.api.service.retract`

### Request payload
```text
{ 0: cid, 1: <service_id : uint> }
```

### Response payload
```text
{ 0: cid, 1: <service_id : uint> }
```

---

## `r2.mgmt.web.provision`

Mint a single-use word code for browser provisioning (R2-PLUGIN §13.5).
The code expires after 1 hour. Operators run `r2hive web provision` to
get one and dictate / paste it into the browser at
`/r2/web/provision`.

### Request payload
```text
{ 0: cid }
```

### Response payload
```text
{
  0: cid,
  1: <words : text>                   // hyphen-separated three-word code
}
```

Errors:

- `auth_unavailable` — the daemon is in dev-mode (no master secret
  loaded). Web plugins serve unauthenticated; provisioning is
  meaningless until identity is configured.

---

## `r2.mgmt.ensemble.load`

### Request payload
```text
{
  0: cid,
  1: <dialect : text>,                // "yaml" | "json" | "toml"
  2?: <source : text>,                // the score, in the chosen dialect
  3?: <path   : text>                 // absolute path to a score file
}
```

Exactly one of `source` (key 2) and `path` (key 3) MUST be present.

- **`source`**: the score is shipped inline. Web plugins (R2-PLUGIN §13)
  declared in the score CANNOT be auto-mounted because the daemon has no
  bundle directory; the daemon logs a warning and the load proceeds for
  the rest of the ensemble.
- **`path`**: the daemon reads the score from the given filesystem path
  and resolves the manifest's `bundle:` field against the file's parent
  directory. Web plugins are mounted at the §13.4 lifecycle moments.
  The CLI uses this form for `r2hive ensemble load <path>`.

At least one of `source` and `path` MUST be present; if both are sent, `path` wins. If neither is present, the daemon returns `bad_frame`.

### Response payload
```text
{
  0: cid,
  1: <ensemble_id    : text>,         // == score.name
  3: <sentant_count  : uint>,
  4: <score_hash     : uint>          // 32-bit FNV-1a of canonical score
}
```

(Key `2` is reserved here for forward compatibility with status.)

---

## `r2.mgmt.ensemble.list`

### Request payload
```text
{ 0: cid }
```

### Response payload
```text
{
  0: cid,
  1: [
    {
      1: <id            : text>,
      2: <status        : uint>,     // 0=Healthy, 1=Degraded, 2=Failed
      3: <sentant_count : uint>
    },
    …
  ]
}
```

---

## `r2.mgmt.ensemble.info`

### Request payload
```text
{ 0: cid, 1: <ensemble_id : text> }
```

### Response payload
```text
{
  0: cid,
  1: <id            : text>,
  2: <status        : uint>,
  3: <sentant_count : uint>,
  4: <score_hash    : uint>
}
```

---

## `r2.mgmt.ensemble.stop`

### Request payload
```text
{ 0: cid, 1: <ensemble_id : text> }
```

### Response payload
```text
{ 0: cid, 1: <ensemble_id : text> }
```

---

## `r2.mgmt.ensemble.reset`

### Request payload
```text
{ 0: cid, 1: <ensemble_id : text> }
```

### Response payload
```text
{ 0: cid, 1: <ensemble_id : text> }
```

Clears the ensemble's restart-intensity ledger and rebuilds every
sentant via the registered factories. The ensemble returns to
`Healthy`.

---

## CLI ↔ event-class mapping

| CLI command | Event class |
|---|---|
| `r2hive daemon status` | `r2.mgmt.daemon.status` |
| `r2hive identity status` | `r2.mgmt.identity.status` |
| `r2hive tg current` | `r2.api.tg.current` |
| `r2hive peers list` | `r2.api.peer.list` |
| `r2hive peers query <id>` | `r2.api.peer.query` |
| `r2hive cap query [--target id]` | `r2.api.cap.query` |
| `r2hive event send <class> [...]` | `r2.api.event.send` |
| `r2hive event subscribe [class\|--any]` | `r2.api.event.subscribe` (and listens for `r2.api.event.delivery`) |
| `r2hive ensemble load <path> [--json\|--toml]` | `r2.mgmt.ensemble.load` |
| `r2hive ensemble list` | `r2.mgmt.ensemble.list` |
| `r2hive ensemble info <id>` | `r2.mgmt.ensemble.info` |
| `r2hive ensemble stop <id>` | `r2.mgmt.ensemble.stop` |
| `r2hive ensemble reset <id>` | `r2.mgmt.ensemble.reset` |
| `r2hive web provision` | `r2.mgmt.web.provision` |

The CLI is the reference R2-HOST-API client. Every command is a thin
wrapper around a single round-trip; nothing in the daemon treats it
specially.
