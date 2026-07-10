# R2-RUNTIME §3.2.1 — pure-edge-bridge standby carve-out — RATIFIED

> **Status:** ✅ **RATIFIED** — landed as **R2-RUNTIME v0.25 §3.2.6 @4072063** (specs, merged main,
> gates green). Drafted by hive against specs' pre-loaded anchors; landed **verbatim** with ONE
> specs correctness-refinement to the sizing invariant (see §3). **No new wire** (reuse R2-WIRE §12.6
> `dc` class-only + §3B.1 SCF, both UNCHANGED); a **normative role-model carve-out** only. This doc is
> hive's mirror of the ratified text; **R2-RUNTIME §3.2.6 is the normative source**.
>
> **Reconciles the contradiction specs flagged:** R2-RUNTIME §3.2.1 says `bridge = AlwaysOn`
> (§3.2.1 table; "a duty-cycled bridge is invalid" ~line 308; Bridge=AlwaysOn ~line 222) — which a
> power-standby edge bridge would violate. The carve-out makes standby legal for exactly the case
> where nothing is stranded.

## The carve-out (normative)
The **bridge** role is **AlwaysOn by default**. **Exception — the PURE EDGE BRIDGE:** a bridge whose
**sole downstream is a single presence-driven sink** (e.g. a phone over USB/BLE), such that **no
mesh-dependent downstream node expects it awake to reach a destination**, MAY enter **standby** — it
advertises `dc = Intermittent` (R2-WIRE §12.6, key `dc=1`, value `2`) and duty-cycles its bearer
while the sink is **absent**, and returns to **AlwaysOn** (or an active wake window) while the sink
is **present**.

## The invariant (the discriminator — the load-bearing rule)
> **Standby is legal IFF no dependent downstream node expects this bridge awake to reach its
> destination.**

Equivalently, matching §3B.1's *will-the-destination-be-awake-to-hear-a-flood* semantics: the
bridge's only downstream is a **sink that drives its own presence** — there is no third party whose
delivery depends on the bridge being awake independently of that sink. When the sink is gone there
is, by construction, nothing to deliver, so sleeping strands nothing.

**DISTINCT from a TRANSIT bridge** (explicitly excluded): a bridge carrying a LoRa **island's**
traffic out to cloud / another mesh — where downstream nodes depend on it for reach — **MUST remain
AlwaysOn**; sleeping would strand the island. **A transit bridge advertising `Intermittent` is a
conformance violation.** The distinction is the *presence of a mesh-dependent downstream*, not the
hardware.

## SCF — no new mechanism (reuse §3B.1)
A standby edge bridge advertising `dc = Intermittent` is **SCF-buffered by upstream neighbours per
§3B.1 automatically** (self-asserted, no-auth, `SCF_TTL_S = 120`, F2-proven) — the upstream sensor
(D4) holds destined-through frames exactly as it already holds for any Intermittent neighbour.
**Sizing invariant (normative — as specs REFINED it):** the standby **`wake_cadence` MUST be shorter
than the UPSTREAM buffering node's `scf_ttl_s`** (§3.2.2 policy, F2-default **120 s**) — **NOT the
literal 120 s**. My draft said "`< SCF_TTL_S` (120 s)"; specs corrected it to the *relationship*
because `scf_ttl_s` is a deployment-tunable knob on the *upstream* node (a DIFFERENT knob than the
bridge's own cadence): if an operator validly sets upstream `scf_ttl_s`=60 s, a bridge sleeping 90 s
satisfies "<120 s" yet its buffered frames drop at 60 s → silent loss. The relationship (cadence <
upstream TTL) is always correct; 120 s is only the field-proven default. (This is the
independent-knobs discipline: check same-quantity vs independent knobs before pinning an ordering
MUST.)

## Phone-presence transition (R2-RUNTIME §3.2.x — runtime state, self-asserted)
The edge bridge transitions **AlwaysOn ↔ Intermittent** on **sink-presence**, self-asserted like
`duty_class` (no auth), re-advertising the `dc` byte on transition so upstream SCF sizing tracks it:
- **sink PRESENT** — USB-resume / BLE-connect / app-active → **AlwaysOn** (or the active wake window).
- **sink ABSENT** — USB-suspend / BLE-disconnect / app-closed → **Intermittent** standby.

The transition is a runtime power-state, orthogonal to the deploy-time role (the node is *still a
bridge*); only its advertised `duty_class` changes. Presence is a local hardware/host signal
(USB suspend-resume, BLE link state, app heartbeat), not a wire-authenticated claim.

## Landing map — AS LANDED
- **R2-RUNTIME §3.2.6** (NEW, v0.25 @4072063) — the carve-out + discriminator invariant + transit
  exclusion + phone-presence transition + sizing invariant. Annotated **§3.2.1** Bridge row + **§3.3**
  power admissibility. This is the normative source; this doc is hive's mirror.
- **R2-WIRE §12.6 / R2-ROUTE §3B.1** — UNCHANGED (reuse `dc=Intermittent` class-only + existing
  hop-by-hop SCF custody; no wire surface, no new §3B.x rule).
- Impl (separate, non-spec, **HELD on Roy scope-eyeball**): `r2-sx1262` `SetRxDutyCycle` (core-owned)
  + `dfr1195-fw` off-by-default `standby` feature.

---
*✅ RATIFIED as R2-RUNTIME §3.2.6 @4072063. hive mirror (host/impl view); specs owns the normative text.*
