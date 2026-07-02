# Field-Firmware Role-Profile Prep — current-state inventory + gap analysis

**Status:** prep / gap-analysis (NOT implementation). Authored 2026-06-26 by hive while LoRa Phase 0
metal waits for a board. **Spec-first:** this documents what the firmware does TODAY and the gap to
clean role-profiles, then poses the open questions to **specs** (owner of the field-firmware canon).
Nothing here is built ahead of canon — it is input FOR the canon.

Refs: `[[custom-sensor-3stage-architecture]]`, `[[lora-message-passing-metal]]` (FR-4 role-sim),
`platforms/dfr1195/src/main.rs` (worktree `dfr1195-fw`).

---

## 1. Current role model = HARDCODED BY hive_id (the gap)

The firmware assigns role behaviour by matching the board's provisioned `my_hive` against pinned
constants — there is **no role-profile abstraction**. Observed in `main.rs`:

| Role (de-facto) | Pinned hive_id | Board | Behaviour in firmware today |
|---|---|---|---|
| **SENSOR / originator** | `0x480e900e` (`FR4_SENSOR_HIVE`) | D1 | the ONLY board that originates Events; `my_duty = Intermittent` (duty-cycled); dest forced to RECEIVER in FR-4 (main.rs:191-197, :666) |
| **RECEIVER / sink** | `0x06ae082b` | D4 | tracks per-SENSOR last-heard + infers ABSENCE → `msg.silence` (E4 silence-is-signal, main.rs:696-706, :897); seeded to expect the SENSOR |
| **BRIDGE** | `0xf91c8911` | D3 | dual-radio (LoRa + ESP-NOW), auto-bridges via plan_forward's transport-aware best_transport (the `bridge` feature) |
| **REPEATER / router** | (any other in-mesh) | D2 etc | plain relay via r2-route plan_forward; no special role code |
| default duty | — | mains boards | `my_duty = AlwaysOn` |

**The gap:** role identity is welded to specific provisioned hive_ids (the tuxedo D1–D4 rig). Deploying
a real pilot-site field requires a board to KNOW its role independent of which physical unit it is. That is
the role-profile abstraction this prep targets.

## 2. Role behaviour today is FEATURE-GATED, not profile-selected

Roles are also partly encoded as Cargo features (compile-time), per `Cargo.toml` + the build matrix:

| Feature | Role it produces | Carriers spawned |
|---|---|---|
| `loraroute` (= lora + routetest + r2-transport/alloc) | LoRa leaf (sensor/repeater) | LoRa only |
| `bridge` (= loraroute + ungated espnow) | bridge | LoRa + ESP-NOW |
| `routetest` | ESP-NOW receiver | ESP-NOW only |
| `fr4` | role-sim (duty-cycle SENSOR + absence-tracking RECEIVER) | per the sim topology |
| `multitg` / `loratcxo` / `nobt` / `pco` / `benchkeepalive` | orthogonal knobs (TG keying / TCXO / no-BT / phase-lock / bench HB rate) | — |

**The gap:** "what role am I" is split across (a) compile-time features AND (b) runtime hive_id pins.
A field deployment wants ONE coherent runtime role-profile (provisioned, like the persona) that selects
behaviour — not a per-role firmware build × an hive_id match.

## 3. LoRa-beacon / HB emit + parse — current wire reality

Two distinct things ride the LoRa carrier today:
- **Beacon (loraroute task, main.rs:2757-2769):** an 8-byte frame = `my_hive` (4B BE) ++ `seq` (4B BE),
  TX'd each ~3s listen/TX cycle. RX logs `LORA-RX from=<hive8> len= rssi= snr=` (the 4B sender-hive
  prepend = the can_hear_hive mask input). This is the neighbour-presence/mutual-RX beacon.
- **Heartbeat (R2-WIRE §12.6):** the HB payload is the `{0:seq, 1:dc}` Compact-CBOR (via
  `encode_dc_seq_cbor`), originator in `route_stack[0]` (ROUTE-ORIGIN-1A), GroupHmac-verified before any
  liveness/dc ingest (H9-secure HB-rx). `dc` = self-asserted DutyClass (the §3B.1 / §12.6 duty_class).

**The gap / question:** is the 8-byte hive+seq beacon a SEPARATE canonical thing from the §12.6 HB, or
should the field canon unify them (the beacon carrying the §12.6 CBOR)? Today they coexist on loraroute.

## 4. GAP ANALYSIS — supervisor's (a)–(e), from current firmware ground truth (2026-06-26)

### (a) Role set today = FOUR; define 'receiver'
SENSOR · REPEATER · BRIDGE · RECEIVER (the plan doc lists 3 — it omits RECEIVER).
**RECEIVER is genuinely distinct, NOT the bridge's ingest side:** it is a *terminal sink/display leaf* —
io_task's deliver-gate fires on a delivered routed Event → `DELIVERED++` + an LED flash on RECEIPT
(main.rs:99 "LED on message arrival, NOT heartbeat" — the proof at the C node), PLUS per-SENSOR
absence-tracking (`sensor_seen`, emits `msg.silence` when an expected sensor goes silent). It does NOT
originate and does NOT relay-onward. The BRIDGE, by contrast, is a *transit* node: 2 carriers +
auto-relay (plan_forward best_transport), no terminal deliver/display. So receiver = terminal, bridge =
transit — keep both.

### (b) Role/profile CONFIG STRUCT — does NOT exist today (the core gap)
There is **no config struct**: role is selected by `my_hive ==` constant matches (FR4_SENSOR_HIVE
0x480e900e / FR4_RECV_HIVE 0x06ae082b, main.rs:1618/1624) × compile-time features
(loraroute/bridge/routetest/fr4), and every parameter is a hardcoded `const`. The KNOBS that exist today
(= the fields a canonical role-profile struct would absorb), with current values:
- role-selector: hive_id match + features → **would become `role: enum{Sensor,Repeater,Bridge,Receiver}`**
- `duty: DutyClass` — Intermittent (Sensor) / AlwaysOn (else); **hardcoded by role, self-asserted only**
- dest/subscription: `ROUTETEST_DEST` (forced for the originator; NVS `SENDTO` overrides)
- HB cadence: `HB_PERIOD_MS=2000` (oscillator), `HB_TICK_MS=50`, `KEEPALIVE_PERIOD_MS` 8000 bench/30000 ship
- beacon cadence: the loraroute task's ~3s listen-then-TX loop (30×100ms listen + 1 TX) — **not a named const**
- SCF buffer: `scf_buf` cap **8**, `SCF_TTL_S=120`, `REACH_CONF=0.3`
- absence: `SILENCE_S=30`; mesh peer liveness: `MESH_PEER_TTL_S=7`
- radio-set: derived from features (loraroute→LoRa; bridge→LoRa+ESP-NOW; routetest→ESP-NOW)
**Recommended struct (for you to canonize/reconcile, NOT yet built):** `role`; `radio_set`; `duty` +
`{wake_cadence, wake_window, sleep_policy}` (NEW — see ⚠); `beacon_cadence`; `hb/keepalive_cadence`;
`scf_policy{cap, ttl_s, reach_conf}`; `dest/subscription`; `ota_window_policy`. Selected from
persona/NVS (raw blob like persona@0x12000), collapsing §1 hive-pins + §2 feature-gates into ONE
provisioned profile = the PILOT-SITE-6 unified-hive (one no_std build, role-by-config).

### (c) 8-byte LoRa beacon layout (exact, main.rs:2757-2759)
`payload[0..4] = my_hive` (u32 **big-endian**) ++ `payload[4..8] = seq` (u32 **big-endian**) = 8 bytes,
no more. RX (main.rs:2740) parses `payload[0..4]` as the sender hive (`LORA-RX from=`). This is the
loraroute presence/discovery beacon and is SEPARATE from the routed §12.6 HB CBOR. **Firmware-path
artifact, not canon** — per your ruling, evolve it into R2-BEACON §8 (class-fingerprint 4B FNV +
rotating RBID + airtime-bounded); I won't over-invest in the 8-byte form.

### (d) Per-role behaviour deltas in firmware TODAY
- **SENSOR:** the ONLY originator; forces dest; `duty=Intermittent` (advertised); also the collector.
- **REPEATER:** relay only (plan_forward) — no originate, no terminal deliver.
- **BRIDGE:** BOTH carriers (LoRa+ESP-NOW), auto-bridge via transport-aware best_transport; espnow_task un-gated.
- **RECEIVER:** terminal deliver-gate (`DELIVERED++` + LED-on-receipt) + absence-tracking (`msg.silence`);
  no originate, no onward relay.

### (e) Join / re-attach state — ⚠ NO self-enrol handshake today
Persona is read from NVS @0x12000 at boot (`read_persona`): present → real hk/tg/hive; absent →
UNPROVISIONED → mac_low3 fallback + demo TG (loud boot warning). `multitg`: a runtime-provisioned TG key
@0x14000 OVERRIDES the persona/demo TG (swaps the trust group, NOT the board id); live re-PROVISION via
`PENDING_PROVISION` (uart_rx_task validates+persists → io_task picks up live). MASK (topology allow-list)
restored from NVS. **FIRST-POWER = no autonomous enrolment protocol** — identity is provisioned
out-of-band (composer's serial provision_bridge); "join" = power on + start beaconing/HB. **RE-ATTACH =
persona persists in NVS across reboot → board silently resumes its role + re-announces (resumes HB +
beacon emit); no re-enrol.** ⚠ For a real field this is a gap: there is no join-request / enrolment
handshake — a fresh board can't self-enrol, it must be externally provisioned first.

## 5. Two NEW-BEHAVIOUR flags for the canon (not in firmware yet)
- ⚠ **Sensor duty-cycle is advertised, not enforced:** `duty=Intermittent` only sets the `dc` byte on the
  HB — there is NO actual wake/read/send/sleep cycling. Real `{wake_cadence, wake_window, sleep_policy}`
  behaviour is net-new firmware (ties to [[custom-sensor-3stage-architecture]] SENTINEL→MCU→SBC sleep/wake).
- ⚠ **No self-enrol** (see (e)) — if the field canon wants first-power autonomous join, that's a new protocol.

**Sent to supervisor 2026-06-26 as the canon-authoring input. No firmware behaviour change in this doc.**
