# Conjecture: xiaobridge pairing rides the existing bridge framing (v1)

Statement: "The USB pairing (CAPS + §3.7 control frames) can be added to the current xiaobridge
`[len u16 LE][payload]` stream by keying on type bytes `0xFE` (CAPS) / `0xFF` (control) while LoRa
frames continue to ride RAW (untyped), as a pragmatic bench link — no change to the egress format
android's `parse_bridge_stream` already consumes."
Kind: empirical (spec-conformance + interop)
Bundle: (a) R2-USB §3.5/§3.6 permits a partial/hybrid v2 link; (b) android's parser tolerates untyped
LoRa frames alongside typed CAPS/control; (c) the current SYNC's `v2` advert is compatible with untyped
frames; (d) my competence at reading the R2-USB conformance clauses.
Chain: fw demux keys on `payload[0]` (0xFF=control conforms to §3.5 table) [0.9]; raw LoRa frame byte0≤0x3F
is collision-free vs 0xFE/0xFF [0.95 — spec §3.5 states this]; but "v2 advertised ⇒ type byte REQUIRED on
every frame" [§3.5, 0.9].
Confidence: 0.5 → 0.25   (as of 2026-07-13)

## Attempts
- [2026-07-13] §3.5 "no dev-mode shortcut" MUST (R2-USB.md:313-315): advertising v2 while sending legacy/untyped
  frames — or skipping CAPS — is explicitly NON-CONFORMANT. The xiaobridge SYNC `04 00 32 52 02 00` advertises
  v2 (`02`) yet forwards raw compact LoRa frames with NO `local_id` type byte. | severity 0.8 | **WOUNDED (near-killed)**
  | The conjecture's auxiliary (a) "spec permits a hybrid v2 link" DIED — §3.5 requires a type byte on every
  non-empty non-SYNC payload; a conformant LoRa frame is `[len][local_id 0x00-0xFB][R2-WIRE body]`, so the current
  untagged frames are non-conformant. The *control-frame* half (0xFF) survives (it matches the §3.5 table); only the
  CAPS-absent + untagged-LoRa halves are hit.
- [2026-07-13] Interop: android's built `parse_bridge_stream` (d8696fd) consumes the CURRENT untyped egress + is
  doing a LIVE LoRa capture. Converging to a conformant type-tagged stream CHANGES the egress wire format →
  breaks android's parser + its live capture until android reworks it. | severity 0.6 | **wounded** | not a spec
  defect but a real interop/sequencing cost — the "no egress change" clause of the conjecture is false.

## Superseding conjecture (v2) — Confidence: 0.5 → 0.85 (as of 2026-07-13)
"Converge the xiaobridge to a FULLY §3.5-conformant R2-USB v2 link: prepend a `local_id` type byte to LoRa-bridge
frames, emit a §3.6 CAPS frame (`0xFE`) after SYNC advertising `hive_id_bytes = usb_link_id` + the transports, and
carry pairing on `0xFF` control frames + sightings on `0xFF` msg_type=12 observation. Coordinate with android."

### Attempts (v2)
- [2026-07-13] Interop cost: does converging break android's built parser + live capture? | severity 0.6 |
  **SURVIVED** | android: its host ALREADY has the §3.5 type-byte demux built (core-ffi/src/usb.rs — USB_TYPE_CAPS
  0xFE / USB_TYPE_CONTROL 0xFF / encode_local_id_frame / decode @389); bridge.rs converges to a thin
  SYNC+len-deframer feeding the existing §3.5 decoder = LESS code, not a rewrite; **NO live LoRa capture running**
  (XIAO quiet, no 2nd SX1262) so nothing to break. The interop-cost auxiliary of conjecture v1 is fully refuted.
- [2026-07-13] 0xA1 sighting has no conformant home under a typed link. | severity 0.4 | **killed-auxiliary:0xA1-wrapper**
  | android: the 0xA1 raw wrapper is RETIRED canon; sightings ride as `0xFF` control **msg_type=12 observation**
  (§3.7.1) — already in android's demux. Progressive (not ad hoc): removes a bespoke type, unifies on the control channel.
- [2026-07-13] usb_link_id needs an unprovisioned shared-constant decision. | severity 0.3 | **SURVIVED (refuted the objection)**
  | host binds link_key to whatever CAPS advertises → the peripheral's per-device value (MAC/master-derived),
  re-advertised identically on reconnect, is authoritative; no shared constant needed. android ACCEPTED.
- [2026-07-13] CAPS frame buildable + parseable by android's §3.6 parser? | severity 0.4 | **SURVIVED (CONFIRMED)**
  | encode_caps built + 12 host KATs green; android's Phase-2 decode_caps parses the exact 59B frame byte-exact
  (@363a39d, added as a KAT, 99 tests green): hive_id_bytes/firmware_id/version/transports[{local_id 2, kind
  LoRa}] all resolve; region as923 skipped forward-compat. §3.6 CAPS interop PROVEN cross-repo. Egress mapping
  mutually LOCKED: LoRa=[len][0x02][compact], sighting=[len][0xFF][{0:12}], CAPS=[len][0xFE], pairing=[len][0xFF].
  android host parser side READY on all 4 arms; nothing blocked on android.

- [2026-07-13] Version-drift: peripheral built @v0.50 0f61c81; android builds against merged §5.3.4 (specs main).
  Did any construction / frame shape / vector drift break byte-compat? | severity 0.6 | **SURVIVED** | diff
  0f61c81..origin/main: r2-usb-pair-vectors.json UNCHANGED (all UP1-8/13/14/18 values + frame_hex identical); TV27
  observation on main = ffa2000c01a40051b201007fce…0307 = byte-identical to my encode_observation KAT; CAPS/local_id
  framing unchanged. Main's §5.3.4 change = USB-SAS key-bearing REMOVAL (a path I never built — scope was link_key
  only) + a §3.4(b) glance-SAS fix (not USB pairing); main RESUME states "no byte drift". Peripheral CONFIRMED
  byte-conformant to android's build target.

### Open attacks (v2)
- **PIVOTAL:** does the COMPLEX-HIVE reframe (USB = INTERNAL bus) EXEMPT the bridge from full §3.5 conformance? |
  est. severity 0.6 | only specs/supervisor rule. NOTE: even if exempt, v2 is never *wrong* (strictly more
  conformant + the north-star) — so it dominates; the ruling only decides whether the simpler v1 was *also* allowed.
- Byte-verify the full choreography against android's built host-TX once its SM un-holds. | est. severity 0.5 |
  **IN PROGRESS (android building PairingHost SM vs merged v0.50, supervisor GO; will ping byte-for-byte confirm).**
  All 3 contract items (framing / usb_link_id-from-CAPS / CAPS-required-gate) RE-CONFIRMED from android's host side
  2026-07-13 + match my build; CAPS already byte-exact @363a39d. Only the built-host-TX byte-replay remains un-run.

## Value flags (separate channel — never moves confidence)
- Conformant-now vs bench-expedient is a values/priority call — routes to supervisor. (Epistemically v2 dominates
  regardless; this flag is only about *effort timing*, not correctness.)
