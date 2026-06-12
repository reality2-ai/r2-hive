# R2-USB wire extract — for specs (type-byte §3.2.1 + pairing vocab §5.3.4)

Extracted from the r2-hive impl (`crates/r2-hive-bin/src/usb.rs`) at specs' request, to author
**R2-USB §3.2.1 (Payload type byte)** and **R2-PROVISION §5.3.4 (message vocabulary)** — both
Roy-gated. Byte-exact source of truth: `usb.rs` + test vectors `r2-usb-vectors.json` /
`r2-usb-pair-vectors.json`. Constants cited by name from `usb.rs`.

## 1. Framing (existing v0.1 §3.2 — context)

- **Length prefix:** 2-byte LE `payload_length`, then `payload` (`encode_length_prefixed`, usb.rs:1029). `payload_length` 0 = keepalive. `MAX_PAYLOAD = 4096`.
- **SYNC handshake (§3.3):** payload = `magic:u16 LE (0x5232) ‖ version:u8 ‖ flags:u8` (4 bytes; `encode_sync_payload` usb.rs:1009). Host `PREFERRED_VERSION = 2`; negotiated = `min(host, peer)`.

## 2. Payload type byte (proposed §3.2.1) — the divergence specs is ratifying

**Only present when the negotiated SYNC version ≥ 2.** After the length prefix, `payload[0]` is the type byte; the remainder is that type's body.

| `payload[0]` | Name | Body |
|---|---|---|
| `0x00`–`0xFB` | R2-WIRE tagged frame (`TYPE_LOCAL_ID_MAX = 0xFB`) | `payload[0]` **is** the `local_id`; `payload[1..]` is the R2-WIRE frame body verbatim. |
| `0xFC` | reserved | rejected (`UsbError::ReservedType`) |
| `0xFD` | reserved | rejected (`UsbError::ReservedType`) |
| `0xFE` | `TYPE_CAPS` | CBOR CAPS frame (§3.6) |
| `0xFF` | `TYPE_CONTROL` | CBOR control frame (§3.7), see §4 |

**Collision-freedom** (specs already verified): R2-WIRE compact `byte0 = (version<<6)|flags` with version=0 ⇒ `byte0 ≤ 0x3F`; extended any version<3 ⇒ `byte0 ≤ 0xBF`. So `0xFC/0xFD/0xFE/0xFF` can never be a valid R2-WIRE first byte. `local_id` space `0x00–0xFB` sits entirely within the valid-R2-WIRE-byte0 range — a `local_id` IS the R2-WIRE byte0, no separate tag.

**Legacy (v1) mode — negotiation/detection:** detected purely by the **SYNC version byte**. `version < 2` ⇒ no type byte; the whole payload is one R2-WIRE frame (surfaced as `WireFrame { local_id: 0, .. }`). This maps exactly to spec v0.1 §3.2. `version ≥ 2` ⇒ type-byte demux above. No per-frame negotiation; one handshake decides for the link. (usb.rs `dispatch_sync` :519, `dispatch_typed` :551.)

### CAPS frame (0xFE) contents
CBOR map → `CapsFrame` (usb.rs:159, `parse_caps` :1046):
- `hive_id_bytes: [u8;16]`
- `firmware_id: String`, `firmware_version: u64`
- `transports: [ TransportDescriptor ]`, each: `local_id:u8`, `kind` (int enum per Appendix A `1..8` = lora/ble/wifi/eth/zigbee/802154/nrf24/thread, `9..99` reserved, `100+` experimental; OR text name), `region: Option<String>`, `properties` (raw per-kind CBOR).

## 3. Control frame (0xFF) envelope (existing §3.7)

Body after the `0xFF` type byte = CBOR map `{0: msg_type:uint, 1: {body map}}` (`build_pair_msg` usb.rs:934). `msg_type` vocabulary: `1`=error report, `2`=log line, `3`=transport state change, `4..=11`=pairing (see §4).

## 4. Pairing message vocabulary (proposed R2-PROVISION §5.3.4 message subsection)

All ride the §3.7 control frame (`0xFF`), CBOR `{0: msg_type, 1: {fields}}`. Fields are integer-keyed. Crypto constructions for the values are the already-ratified §5.3.4 (X25519 / SAS / link key / reconnect HMAC).

| `msg_type` | Name | Dir | Body `{1: {...}}` fields |
|---|---|---|---|
| `4` | `PAIR_HELLO_HOST` | host→periph | `{1: eph_pk_host: bytes[32], 2: nonce_host: bytes[32]}` |
| `5` | `PAIR_COMMIT` | periph→host | `{1: commit_p: bytes[32]}` (SHA-256 commitment) |
| `6` | `PAIR_REVEAL` | periph→host | `{1: eph_pk_periph: bytes[32], 2: nonce_periph: bytes[32]}` |
| `7` | `PAIR_CONFIRM` | host→periph | `{}` (empty — sent after operator SAS confirm) |
| `8` | `PAIR_DONE` | periph→host | terminal ack; host then stores link_key |
| `9` | `RECONNECT_CHALLENGE` | host→periph | `{1: nonce_rc: bytes[32]}` |
| `10` | `RECONNECT_RESPONSE` | periph→host | `{1: tag: bytes[16]}` (reconnect HMAC, truncated 16) |
| `11` | `PAIR_ABORT` | either | `{1: reason: text}` |

**Flow (first attach):** SYNC → CAPS → host `PAIR_HELLO_HOST(4)` → periph `PAIR_COMMIT(5)` → periph `PAIR_REVEAL(6)` → host computes Z + SAS, operator confirms → host `PAIR_CONFIRM(7)` → periph `PAIR_DONE(8)` → Active.
**Flow (reconnect):** SYNC → CAPS → host `RECONNECT_CHALLENGE(9)` → periph `RECONNECT_RESPONSE(10)` → Active.
**Abort:** `PAIR_ABORT(11) {reason}` from either side at any point; `reason` is the verbatim operator-surfaced vocabulary (e.g. `user_aborted`, `protocol_error`).

Source handlers: senders usb.rs:660–897 (`build_pair_msg`), field extractors `extract_bstr_field`/`extract_tstr_field` :962/:988, states `SessionState` :196.
