# LoRa multi-board mesh sustain — redesign scope (for Roy's decision)

**Status (2026-06-23):** LoRa first-light DONE (bidirectional D1↔D2, clean). Pairwise PCO-over-LoRa
GREEN (HBSYNC-02 lora: e=0.001, spread 2ms — engine + SX1262 carrier proven transport-agnostic). The
**3-board mesh does NOT sustain** (HBSYNC-03 lora pending). This is the carrier-architecture decision.

## What's already done + correct (keep)
- **§4.2 airtime-reachback** (`lorareach` feature): the PCO couples toward the airtime-advanced phase
  (rate·ToA), per core's `on_verified_pulse_delayed`. Metal-confirmed the canon's §4.2 LoRa prediction.
- **HB-prioritized shaping** (`lora_mesh_task`): TX HB-only over the slow link, drop Events. Verified
  TXing HBs correctly (b0=0x29 mt=5 txd=true).

## The real blocker (precisely diagnosed, not guessed)
Both 2×2 arms (shaping+§4.2, shaping-only) showed **no 3-board reception** (nbrs=0). HBs ARE transmitted;
the radio's `listen()` IS continuous-RX. So the failure is the **half-duplex carrier loop**, two effects:
1. **Synchronized-fire collision** — the leaderless PCO *converges toward simultaneous firing*; on a
   half-duplex shared channel that means all boards TX their HB at ~the same instant → none hears the
   others → desync → repeat. (Pairwise works because 2 boards' fires interleave; N≥3 collides.)
2. **TX blocks RX** — each HB TX (~130ms @SF7) is a window where the board is deaf; the naive poll-loop's
   timing compounds the collision.

## Redesign options (Roy's call on approach)
- **A — CSMA / listen-before-talk + random jitter** (lightest): before TXing the HB, add a small random
  backoff + carrier-sense (channel-activity-detect / RSSI check); if busy, defer. Spreads the fires so
  they don't collide. Pairs naturally with the leaderless model (no coordinator). RECOMMENDED first.
- **B — DIO1-IRQ event-driven loop**: RX on the DIO1 interrupt (not polling) + a TX scheduler with ToA
  guards. Cleaner/lower-power; more wiring (DIO1=GPIO4 IRQ handler) + restructure.
- **C — phase-offset TX** (PCO-aware): TX the HB at a deterministic phase OFFSET per node (derived from
  hive_id) so fires are staggered by construction — leaderless-compatible, no carrier-sense. Elegant but
  couples the carrier to the engine.
- A+C combinable. §4.2 + shaping stay underneath any of these.

## The decisive test once it sustains
- arm2 (shaping+§4.2 @ SF7) → HBSYNC-03 green.
- **arm3 MIXED-SF (relay SF12 + 2× leaf SF7)** = the load-bearing §4.2-NECESSITY test (per-neighbour ToA
  differs → real phase mis-attribution that ONLY §4.2 fixes) — the EG-realistic per-role-SF config. Needs
  a **per-board SF override** firmware knob (which the real relay/leaf deployment needs anyway).
- (Optional reach-only (OFF,ON) cell for clean attribution if arm1 sustains.)

## Refs
core: `HeartbeatPco::on_verified_pulse_delayed` (bdda130), `lora_airtime::time_on_air_ms` (70f831a,
branch r2-core-consolidation), the "continuous-RX/event-driven/ToA-aware" pattern (asked for a reference
shape). specs: HBSYNC-03 2×2 attribution pinned. Firmware: `lorameshb`/`lorareach` in `platforms/dfr1195`.
