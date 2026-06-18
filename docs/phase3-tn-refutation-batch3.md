# Phase-3 TN refutation — batch 3 (SCF + XT/entanglement)

**Role:** hive = adversarial refuter, *deployment-reality* lens.
**Source:** `r2-specifications/testing/test-vectors/r2-transient-networking-conjectures.json` (v0.3, 52 conjectures).
**Grounding code:** `r2-core/crates/r2-route/` (RouteEngine), `r2-core/crates/r2-harness/src/mesh.rs` (sim entanglement), `r2-trust/SPEC.md` §7.
**Filed to specs:** 2026-06-12. Targets: the un-refuted SCF (L2) + XT/entanglement cells specs flagged as still open.

## Code facts established (read, not assumed)
- `RouteEngine` state = `{neighbours, paths, dedup, strategy}` only. **No frame buffer / queue / carry store / entanglement table.**
- `ForwardAction` = `Drop | DeliverOnly | Directed | Flood` — **no Queue/Hold/Buffer variant.**
- `plan_forward` with no directed path → `build_flood_plan`; with **no viable neighbour → `Drop(DropReason::NoViableNeighbour)`** (engine.rs:324). So "relay-without-route" = silent drop, never a held frame.
- `DedupCache` = fixed-capacity ring of `(msg_id:u16, source:u16, expires_at)`; `DEDUP_TTL_SECS = 60`; evicts on TTL **or** ring overwrite under load.
- `NEIGHBOUR_HARD_TIMEOUT = 30*60` (30 min); `FORWARDING_CONFIDENCE_FLOOR = 0.1`.
- **Entanglement is sim-only:** `r2-harness/src/mesh.rs` `Entanglement { keys, live: bool }` + `set_entanglement_live()`; crossing gate = structural `live` boolean (harness honesty #6: "structural, not cryptographic — checks a live boolean, not a verified PeeringHmac"). r2-trust SPEC §7: peering-key derivation exists but **"no negotiation protocol, tiers, keep-alive, @entangled routing."**

## Findings

### SCF family — the SCF buffer is UNREALIZED in r2-route; MUSTs mis-placed / sim-undecidable

**TN-L2-IT-BL-002** (relay-without-route MUST queue bounded-TTL; MUST NOT drop silently) — **FALSIFIED-as-stated vs impl.** Engine has no queue; no-path → `Drop(NoViableNeighbour)` = it *does* drop silently. The MUST presumes an SCF buffer layer the routing engine doesn't have. **Rec:** relocate the MUST to a *named* SCF/host layer above route, bounded by an explicit `(max_entries × max_frame_bytes)` RAM budget — on ESP32-S3 (~512 KB) the **buffer**, not TTL, is the binding constraint; "bounded TTL" alone doesn't bound memory.

**TN-L2-IT-AB-000** (late SCF delivery dispatched exactly-once) — **FALSIFIED for carry-time > dedup horizon.** SCF carry on a duty-cycled LoRa / intermittently-connected carrier is minutes-to-hours; `DEDUP_TTL_SECS = 60` + fixed-N ring (evicts under load) → the carried frame is no longer in the dedup window on delivery → re-fire / double-dispatch. **Rec:** for SCF, dedup horizon MUST be ≥ max SCF carry-time (a separate, larger, possibly persistent store), OR enforce exactly-once end-to-end at dispatch (idempotency key), not via the 60 s relay-plane ring.

**TN-L2-IT-BL-000 / TN-L2-XT-BL-000** (SCF delivers across a gap; non-member carrier SCF of cross-TG frame) — unrealized (no buffer) → cannot pass against the real stack; `tier=sim` insufficient. Membership-blind carrying is sound in principle (relay plane never reads payload) but inherits the buffer-bound + dedup-horizon problems above. **Rec:** re-tier to a tier whose model includes a bounded buffer + carry-time clock.

**TN-L2-XT-BL-001** (OOM guard: carrier MUST bound buffering; MUST NOT buffer indefinitely) — MUST correct in spirit, but **the experiment is NOT decisive at tier=sim**: a sim with host RAM buffers "indefinitely" and still passes every functional assertion; OOM is exactly what a sim cannot falsify. **Rec:** re-tier to hardware, or inject a fixed buffer cap in the harness; else an impl using a growing `Vec` passes the sim and OOMs on-device.

### XT/entanglement family — tests a sim-only structural gate, not the production crossing

**All XT-AB crossing cells** (TN-L0/L1/L2/L3-XT-AB-\*) — the experiments verify the **sim's `live`-bool gate, not an authenticated crossing.** Per honesty #6 + r2-trust §7, an impl can pass every XT-AB cell with **zero cryptographic crossing protection.** Decisive for the policy boolean, **not** for the security spirit (answers specs' Q2: impl passes while violating spirit). **Rec:** label these as testing the structural gate; the authenticated-crossing MUSTs (verified PeeringHmac) are a separate, currently-unrealized surface needing their own cells once r2-trust §7 lands.

**TN-L1-XT-BL-100 vs BL-101** (heartbeat reinforces route vs no-special-mechanism) — **CONFIRM BL-101, FALSIFY BL-100 as-stated.** No entanglement heartbeat exists (r2-trust §7 "no keep-alive"); entangled routes earn strength only via ordinary overheard-traffic confidence (BL-101). **Deployment gap:** with deliberately no heartbeat, a low-traffic entanglement on a duty-cycled link never accrues enough overheard traffic to hold its route above `FORWARDING_CONFIDENCE_FLOOR (0.1)`; neighbours hard-timeout at 30 min and confidence decays → the entangled route silently dies ("entangled but unreachable"). **Rec:** if entanglements are meant durable, add a minimal keep-alive or a decay exemption; else document that entanglement liveness ≠ route liveness.

**TN-L2-XT-AB-001** (buffered crossing bound to entanglement *instance*; retire drops, recreate doesn't inherit) — **FALSIFIED-as-stated / undecidable vs sim.** Sim entanglement has no instance/epoch id; `set_entanglement_live(false)` flips a bool, and re-entangling the same NodeId pair reuses the same map key — old and "recreated" instances are indistinguishable, so the experiment can't tell inherited-from-old vs new. **Rec:** add an instance/epoch id to entanglement for the MUST to be testable.

**TN-L2-XT-BL-100** (stale entanglement kept/deprioritised/revivable) — consistent with the no-heartbeat + decay model (stale = low confidence, revivable on new traffic), **but "kept" conflicts with neighbour hard-timeout:** a stale entanglement's *route entry* is **evicted** (not merely deprioritised) after 30 min idle. **Rec:** distinguish entanglement-record retention (policy) from route-entry retention (30 min hard timeout).

## Resolution addendum (2026-06-18) — core wired the knobs (r2-core `da89050`)

core implemented the two harness mechanisms my batch-3 findings asked for; both previously-undecidable
cells are now **decidable**, and I've re-run them against the impl (`r2-harness/src/{node,mesh}.rs`).

**TN-L2-XT-BL-001 (OOM guard) — was "not decisive at sim" → now DECIDABLE → CONFIRMED.**
`MeshNode::set_scf_buffer_cap(max_entries, max_bytes)` + `try_buffer` enforce both caps, **tail-drop** the
newest frame on overflow, and count `scf_overflow_drops`; `carry_bytes()` exposes the live total. An
unbounded-buffer impl now *fails* the assertion (`scf_overflow_drops` stays 0 while `carry_bytes` grows
past the cap) — exactly the OOM-guard property a sim couldn't falsify before. Caps on **both** entries
(fixed-table slots) and bytes (RAM) match MCU reality (whichever binds first).
- **Deployment-reality refinement (not a falsification):** the OOM guard is satisfied, but the eviction
  **policy** is **tail-drop** (shed the *newest*). For SCF, that rejects fresh/likely-still-live frames
  while stale ones (closer to TTL death) occupy the buffer — a TTL-aware or oldest-first eviction would
  deliver more under sustained inflow. Tail-drop is fine for the *guard*; flag delivery-optimality as a
  spec-note / future policy knob.

**TN-L2-XT-AB-001 (stale crossing after retire) — was "sim-undecidable" → now DECIDABLE → CONFIRMED.**
`Entanglement.epoch` bumps on re-entangle (`epoch = old+1`); `mesh.entanglement_epoch(a,b)` exposes it; so
`set_entanglement_live(false)` + re-entangle is distinguishable (epoch 0→1). The experiment can now assert
pre-retirement buffered data does NOT deliver to the new (epoch-1) instance. The epoch is the instance
binding the MUST needed (harness unit test `entanglement_epoch_distinguishes_reinstatement` proves it).
- **Deployment-reality note:** on a real MCU carrier the buffered crossing + epoch are RAM-volatile, so a
  reboot between retire and re-entangle clears both → the stale-data case is moot on a rebooting node; the
  epoch matters on a **long-running** carrier (no reboot). If a future design persists buffered crossings
  in NVS across reboot, the epoch must be persisted with them for the MUST to hold. Note, not a blocker.

**Net:** batch-3's two "needs a harness knob" cells are closed — both CONFIRMED decidable, with the two
refinements above (tail-drop eviction policy; epoch/buffer RAM-volatility vs reboot) for specs/core.
