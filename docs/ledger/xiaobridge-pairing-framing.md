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

## Superseding conjecture (v2)
"Converge the xiaobridge to a FULLY §3.5-conformant R2-USB v2 link: prepend a `local_id` type byte to LoRa-bridge
frames, emit a §3.6 CAPS frame (`0xFE`) after SYNC advertising `hive_id_bytes = usb_link_id` + the transports, and
carry pairing on `0xFF` control frames. Coordinate the egress change with android (whose parser reworks anyway)."
- Resolves stronger: spec-clean (survives the §3.5 MUST), and matches the north-star (conformant R2-USB, not a
  bespoke bridge stream). Cost: android parser rework + egress format change → MUST be sequenced (don't break the
  live capture). Escalated to supervisor for the conformance call + sequencing.

## Open attacks (generated, not yet run)
- Does the COMPLEX-HIVE reframe (USB = INTERNAL bus, not an external R2-USB provisioning link) EXEMPT the bridge
  from full §3.5 conformance? i.e. is the internal faculty↔faculty bus spec-governed at all, or impl-free? | est.
  severity 0.7 | — the pivotal question; only specs/supervisor can rule. If exempt, v1 (hybrid) may be acceptable.
- Byte-verify the framing against android's ACTUAL built host-TX (currently only design-intent; their SM is HELD).
  | est. severity 0.5
- Does a `local_id` type byte on LoRa frames interact with the 0xA1 sighting envelope (which is NOT an R2-WIRE
  frame)? Sightings would need their own type-byte treatment under a conformant link. | est. severity 0.4

## Value flags (separate channel — never moves confidence)
- Conformant-now vs bench-expedient is partly a values/priority call (spec-purity vs not disrupting the live
  capture) — routes to supervisor, does not move the epistemic confidence above.
