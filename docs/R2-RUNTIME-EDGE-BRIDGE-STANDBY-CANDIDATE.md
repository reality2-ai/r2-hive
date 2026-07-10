# R2-RUNTIME §3.2.1 — pure-edge-bridge standby carve-out — CANDIDATE

> **Status:** CANDIDATE for specs ratification (2026-07-10, Roy GO for the XIAO heat fix). Drafted
> by hive against specs' pre-loaded anchors. **No new wire** (reuse R2-WIRE §12.6 `dc` + §3B.1 SCF);
> this is a **normative role-model carve-out** only. Blocks the path-1 flash (spec-first).
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
**Sizing invariant (normative):** the standby **`wake_cadence` MUST be `< SCF_TTL_S` (120 s)** so
upstream-buffered frames are delivered within their SCF hold before expiry. (An edge bridge that
sleeps longer than `SCF_TTL_S` would silently drop upstream-buffered traffic — a violation.)

## Phone-presence transition (R2-RUNTIME §3.2.x — runtime state, self-asserted)
The edge bridge transitions **AlwaysOn ↔ Intermittent** on **sink-presence**, self-asserted like
`duty_class` (no auth), re-advertising the `dc` byte on transition so upstream SCF sizing tracks it:
- **sink PRESENT** — USB-resume / BLE-connect / app-active → **AlwaysOn** (or the active wake window).
- **sink ABSENT** — USB-suspend / BLE-disconnect / app-closed → **Intermittent** standby.

The transition is a runtime power-state, orthogonal to the deploy-time role (the node is *still a
bridge*); only its advertised `duty_class` changes. Presence is a local hardware/host signal
(USB suspend-resume, BLE link state, app heartbeat), not a wire-authenticated claim.

## Landing map
- **R2-RUNTIME §3.2.1** — the carve-out + the invariant (this doc §1–2). Roy-GO-approved direction.
- **R2-RUNTIME §3.2.x** — the phone-presence transition (this doc §4), as a runtime power-state note.
- **R2-WIRE §12.6 / §3B.1** — UNCHANGED (reuse `dc=Intermittent` + existing SCF; no wire surface).
- Impl (separate, non-spec): `r2-sx1262` `SetRxDutyCycle` (core-owned) + `dfr1195-fw` standby feature.

---
*Open for specs ratify. hive drafts (host/impl view); specs owns the normative R2-RUNTIME text.*
