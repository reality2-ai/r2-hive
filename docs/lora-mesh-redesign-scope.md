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

## DECISION MADE — core specified the fix (2026-06-23, reference pattern delivered)
The carrier-architecture question is **answered by core**: **jitter the TRANSMISSION + continuous-RX +
event-driven** (= option A). The PCO keeps the FIRES synchronized (the sync goal); a random jitter only
staggers the TRANSMISSIONS so while board A TXs, B+C are in RX and HEAR it. Use `r2_route::jitter::relay_jitter_ms`.

**Implementation for hive's structure** (io_task = PCO→DATA_TX; lora_mesh_task = carrier): when an HB is
drained from DATA_TX, do NOT transmit immediately — set `tx_at = now + relay_jitter_ms(rng, Lora, congested)`
and hold the HB; keep the radio in RX (poll RxDone → read → §4.2 couple → `radio.listen()` immediately);
when `now >= tx_at`, `transmit()` → poll TxDone → `radio.listen()`. Net: radio is in RX the entire time
except the brief jittered transmit. (Events still dropped = the shaping.)
- **§4.2 + jitter caveat (core):** TXing jitter-after-fire means the sender's phase at TX-start is
  beat+rate·jitter, not 0. For small jitter (tens of ms vs 2000ms) the ToA-only d_hop has a small bias —
  fine to start. CLEAN fix = stamp `tx_time`/phase_at_tx into the HB at TX-start → §4.2 uses
  delay_ms = rx_clock − tx_time (specs' PREFERRED MAC-timestamp path; needs a 4-8B wire add). Start ToA-only.
- **Use ACTUAL frame_len for the §4.2 ToA** (HB = 30B unsigned on nobt, not 62B).
- **LBT/CAD:** proper listen-before-talk needs a channel-sense primitive core's driver doesn't expose yet
  (GetRssiInst 0x15 / SetCad 0xC5). Jitter+continuous-RX should unblock 3-board first; if collisions persist
  at scale, core adds `radio.channel_rssi()` / a CAD mode (small driver add).

So the MORNING build = implement core's reference (a focused lora_mesh_task restructure + r2_route::jitter),
re-run arm2 → HBSYNC-03 green, then arm3 mixed-SF. Not an open decision anymore — a specified build.

## Refs
core: `HeartbeatPco::on_verified_pulse_delayed` (bdda130), `lora_airtime::time_on_air_ms` (70f831a,
branch r2-core-consolidation), the "continuous-RX/event-driven/ToA-aware" pattern (asked for a reference
shape). specs: HBSYNC-03 2×2 attribution pinned. Firmware: `lorameshb`/`lorareach` in `platforms/dfr1195`.
