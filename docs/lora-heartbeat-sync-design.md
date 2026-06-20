# Synchronized heartbeats over LoRa — design (the "fireflies" demo)

**Goal (Roy):** co-member hives of a trust group **synchronise their heartbeats to beat as one**, a living,
glanceable proof of transient networking: emergent coordination from local event exchange, self-healing
across partitions.

> **Framing (Roy, via supervisor) — this is NOT a bolted-on application sync.** R2-TRUST already maintains
> TGs/entanglements **by heartbeat** (R2-TRUST §7, *heartbeat-maintained*): co-members of a TG already exchange
> periodic maintenance heartbeats. This work does **not invent a new mechanism** — it **couples** those
> existing TG-maintenance heartbeats (pulse-coupled-oscillator entrainment) and **visualises** the result on
> the LED. Consequences that run through this whole doc:
> 1. **Same-TG membership is the SOURCE of the heartbeat, not merely a prerequisite** — no shared TG = no
>    shared maintenance heartbeat = nothing to couple = no sync. The signal is intrinsic to co-membership.
> 2. The HeartbeatSync sentant **couples the TG-maintenance heartbeat**, it is not a standalone oscillator.
> 3. The conjecture is a **trust-layer claim**: co-members of a TG synchronise their maintenance heartbeats
>    and re-synchronise after partition→heal — with the §3 TX-jitter collision-avoidance applying to the
>    announces. The demo visualises **real protocol behaviour**, not a contrivance.

## Design decision — couple the maintenance heartbeat DIRECTLY (settled)

The supervisor's open question: does sync couple the **trust-layer maintenance heartbeat directly**, or a
**TG-scoped sentant heartbeat riding on membership**? **Settled: couple the maintenance heartbeat directly.**

Grounding fact (confirmed in the code): **R2-WIRE already has a first-class `MsgType::Heartbeat`** frame
(`r2-wire`, compact 12-byte / extended 22-byte). So the TG-maintenance heartbeat (R2-TRUST §7) is a real
on-the-wire message exchanged among co-members — there is a concrete thing to couple and visualise. Reasons:

1. **It's the honest realisation of the framing.** The thing that beats IS the `Heartbeat` frame, so the LED
   shows the literal TG-maintenance protocol. A *separate* sentant oscillator would be exactly the "bolted-on
   application sync" we're steering away from (and the supervisor's implication 2 already says the sentant
   *couples* the maintenance heartbeat, *not* a standalone oscillator).
2. **The sentant's role is the COUPLING FUNCTION, not a clock.** HeartbeatSync observes inbound co-member
   `Heartbeat` frames and applies a PCO phase-nudge to the **local maintenance-heartbeat scheduler**, and
   beats the LED on `Heartbeat` TX. It owns *no* independent period.
3. **The coupling is liveness-SAFE by construction: advance-only.** The PCO nudge may only pull the next
   heartbeat *earlier* (fire sooner), never later. So it can only make a co-member's heartbeat *more* timely —
   it can never push one past an R2-TRUST §7 liveness deadline or mask a real timeout. Sync entrains without
   ever endangering the liveness semantics the heartbeat exists for. (This is the constraint that makes
   "directly" safe; a delay-capable coupling would not be.)
4. **TX-jitter (§3) applies to the `Heartbeat` announces** regardless — the LED phase entrains tightly while
   the actual `Heartbeat` frame TX is spread to dodge half-duplex collisions.

**Two timescales (settled, Roy) — the maintenance heartbeat is INFREQUENT; the visual beat is fast and
LOCAL, disciplined by it.** The R2-TRUST §7 maintenance heartbeat *must* be infrequent (tens of seconds, not
~1.5 s) — **frequent heartbeats would flood the mesh** (and blow the LoRa duty-cycle budget). So we do **not**
speed it up. Instead, a PLL-like split:

- The **visual heartbeat** is a **fast local oscillator** (~1.5 s) on each node — what the LED actually shows.
  It free-runs between reference pulses; cheap, no radio.
- The **rare maintenance `Heartbeat` frames** are the **reference / coupling pulses**: each inbound co-member
  `Heartbeat` applies the PCO phase-nudge that **disciplines the local visual oscillator** (advance-only, §3).
  Sync rides on these infrequent real frames — so **the radio stays unflooded**, yet the local beats entrain.
- Co-members converge because every node's fast oscillator is pulled by the *same* shared (mutually-coupled)
  maintenance heartbeats — like distributed phase-locked loops sharing a sparse reference.

Consequences to bake into the sim + the tuning:
- **Convergence is slower** (one correction per maintenance interval, not per visual beat) → the local
  oscillator must be **stable** enough to hold phase between references, and ε must integrate over many
  intervals. Inter-reference **clock drift** is the dominant error to budget against.
- **Collisions are naturally rarer** (few frames on air) — TX jitter (§3) still applies, but the channel is
  far from saturated. The anti-flood constraint and the collision constraint pull the same way: keep the
  radio quiet.
- Cadence (the exact maintenance interval) is a trust-layer parameter — specs' call — but the **two-timescale
  PLL model is the design** regardless of the number.

## 0. PREREQUISITE — both nodes on the SAME trust group (Roy)

Events are **TG-scoped**: a hive only relays/processes events from its own trust group (cross-TG requires a
live entanglement). So before any `r2.sync.fire` can flow between the boards, **both DFR1195s must be members
of the same TG** — otherwise each node's frames are ignored by the other and there's nothing to sync on.

What that needs (a tier of its own, alongside LoRa):
- **Per-device identity** — each board's `master_secret` in NVS → `mesh_hive_id` (TG-scoped, R2-WIRE §6.2.1),
  via the shared `r2-esp/hive_id` module (workshop). Distinct device identities, *shared* TG membership.
- **TG membership** — each board holds the `tg_id` + the TG context (the group key for R2-WIRE frame
  HMAC / R2-TRUST). On the MCU this means the *verify* path of r2-trust must be available no_std (the group
  HMAC check on inbound frames); today r2-trust is std-tier → a no_std-tiering ask for core/specs.
- **A join flow** (R2-PROVISION) to put both on ONE TG (not each its own "TG-of-one" that the firmware mints
  on first boot). Options for the demo, simplest first:
  1. **Host-provisioned:** a full hive (laptop r2-hive) creates a TG and provisions both boards into it over
     USB (the §5.3.4 pairing already gives a device↔host trusted channel to push the `tg_id` + TG context).
  2. **Join-code:** one board hosts the TG, the other joins via a word-code/proximity flow (R2-PROVISION join)
     over BLE/LoRa.
  3. **Pre-shared (demo shortcut):** flash both with the same `tg_id` + TG key at provisioning time — crudest,
     but unblocks the sync demo before the full join protocol is on the MCU. Clearly a shortcut, not the model.

**Dependency map:** workshop (identity/hive_id in NVS), core+specs (r2-trust no_std verify tier + R2-PROVISION
join on MCU), hive (wire the TG context into the firmware's transport/route so inbound TG frames are accepted).
This TG-membership tier gates the sync demo as much as the LoRa tier does — sequence both before §3's live run.
The sync *algorithm* (§1) is still host-prototypable now, independent of TG/LoRa.

## 1. The algorithm — pulse-coupled oscillators (Mirollo–Strogatz / fireflies)

Each node holds a phase `φ ∈ [0,1)` advancing at rate `1/T` (T = heartbeat period, ~1.5 s).
- When `φ` reaches 1 → **fire**: beat the LED, reset `φ = 0`, and broadcast a tiny **FIRE** event.
- On **receiving** a peer's FIRE → nudge the phase forward: `φ ← min(1, φ·(1+ε))` (coupling strength `ε`,
  small, e.g. 0.1). Firing nodes pull laggards forward; the whole population provably converges to firing in
  unison, with no leader, no clocks, no central coordinator. Robust to nodes joining/leaving mid-run.

This is decentralised + emergent — exactly the R2 transient-networking thesis. The LED is the visible phase.

## 2. R2 mapping — a HeartbeatSync sentant + the LED + LoRa

- **HeartbeatSync sentant** (composer's domain): owns `φ`, the timer, and the coupling. Emits an
  `r2.sync.fire` event on fire (→ LoRa broadcast, target=0); consumes peers' `r2.sync.fire` → phase nudge.
  Device-agnostic; the algorithm is pure logic (host-testable now, before any radio).
- **LED output** (hive device plugin): the sentant's fire → the LED "beat" (the existing GPIO21 heartbeat).
- **LoRa transport** (core D3b + hive SX1262 wiring): carries the FIRE events as small R2-WIRE frames over the
  mesh (broadcast/flood). The fire pulse is tiny (a few bytes: originator + a seq) — minimal airtime.
- One `r2.sync.fire` event type; the existing route/transport/flood machinery does the propagation.

## 3. The real-radio wrinkle (this is the interesting part — deployment-reality lens)

The textbook firefly model assumes you can always *hear* the pulses. On real half-duplex LoRa, the success
condition fights itself:

- **Synchronised firing = simultaneous TX = collisions.** As nodes converge, they all fire at the *same
  instant* → they all transmit at once → the packets collide AND a half-duplex radio can't RX while it TXes.
  So the tighter the sync, the less they can hear each other. **Mitigation:** decouple the *visible* beat
  (LED, tightly synced) from the *radio announce* — add a small per-node random **jitter** to the actual TX
  (a few×10 ms), or only a subset announce each round, or listen-before-talk. The LEDs can beat in unison
  while the radio chatter stays spread out. (A "desync" variant — nodes deliberately spread their TX in time
  while keeping a shared logical phase — is the clean fix; worth prototyping both.)
- **Propagation/airtime latency.** A FIRE arrives ~tens–hundreds ms after the peer fired (LoRa airtime + SF).
  Naive coupling syncs them with a fixed *offset* equal to the airtime. **Mitigation:** compensate the nudge
  by the known airtime (SF/BW → deterministic time-on-air), or timestamp-and-correct.
- **Duty cycle.** At T≈1.5 s each node TXes ~0.7/s; with SF7/short frames airtime is ~tens of ms → a few %
  duty. EU 1% regions may need a slower heartbeat or fewer announces. **Mitigation:** tune T + frame size to
  the regional duty budget; only the fire pulse goes on air.
- **Transient robustness (the payoff demo).** Nodes join → fold into sync. **Partition** the mesh → the two
  groups sync *independently* (two rhythms). **Heal** → the groups re-merge into one rhythm. That visible
  partition→heal→resync on the LEDs is a perfect Phase-3 TN showcase.

## 4. Sequencing + ownership

- **Now (transport-independent):** design (this doc) + **host-prototype the sync algorithm** — the
  oscillator + coupling + convergence, testable in an r2-harness-style sim (N virtual nodes, message delay +
  loss + partition injected). Proves convergence + lets us tune ε/jitter/T before touching radios.
- **Gated on the LoRa tier:** the live demo needs SX1262 LoRa up = core D3b drivers + hive SX1262 wiring +
  the embassy/WiFi-then-LoRa firmware tier (in progress). Then the sentant runs over real LoRa on the 2+ DFR1195s.
- **Ownership:** composer = the HeartbeatSync sentant; hive = LED output + SX1262 LoRa wiring + the
  deployment-reality/jitter design + hardware validation; core = LoRa transport (D3b) + `r2.sync.fire` routing;
  specs = the event vocabulary + (great fit) a **TN synchronization conjecture/demonstration** in the matrix
  (emergent sync + partition/heal — testable in sim now, hardware later).

## 5. Why it matters — calm-technology TG-cohesion status (Roy's concept)

The deeper point isn't the demo — it's a **calm-technology status signal**. **Hives in a trust group that
heartbeat *together* show, ambiently and at a glance, that the group is coherent and well.** No logs, no
numbers, no screen needed — you sense the TG's health peripherally, from the shared pulse, the way you sense
a room is calm. That is calm technology in its purest form (Weiser/Brown): information at the edge of
attention, not demanding it.

And it is **self-explaining**, because a node only entrains with **its own TG** (§0):
- **In rhythm** → "these hives are one trust group, connected, all-well." The sync *is* the visible boundary
  and health of the group.
- **A node drifting out of phase** → it's losing contact (range, interference, fading) — visible degradation
  before it's fully gone.
- **Two rhythms** → the TG has **partitioned** into two reachable clusters.
- **Re-merging into one rhythm** → the partition **healed**; the group is whole again.

So the same mechanism is both the proof of transient networking *and* the everyday glanceable status surface
for "is my trust group together and well?" — driven by the very transient-mesh event exchange it reports on.
The half-duplex wrinkle (§3) keeps it honest: it's a real distributed-systems-on-real-radios problem whose
*output is calm*. It ties straight into `r2.hw.led` (`ok` = heartbeat): a **synchronised** `ok` across the TG
is a richer, emergent "TG-coherent" status that no single node could assert alone.

## 6. Caveat — coincidental cross-TG sync (Roy)

Two **separate** trust groups can *appear* to beat in unison by chance — their independent rhythms momentarily
line up. The observer's eye can't tell "one coherent TG" from "two unrelated TGs that happen to align right
now." Worth being explicit about, because it bounds what the signal honestly claims.

What saves it: the two TGs are **uncoupled** — `r2.sync.fire` events are TG-scoped, so there is *no* coupling
across the boundary. Coincidental alignment is therefore **transient**: with independent clock drift and no
mutual correction, two uncoupled groups inevitably slide out of phase within a few cycles. *Real* intra-TG
sync is **actively locked** (continuously re-corrected by the event exchange); coincidental cross-TG sync is
**free-running** and decays. So **watching for a few beats disambiguates**: locked-and-holding = genuinely one
TG; drifting-apart = it was a coincidence. The signal is honest *over time*, ambiguous only in a single glance.

If we ever want single-glance certainty, give each TG a **deterministic identity in its rhythm** so distinct
TGs can't occupy the same phase/appearance:
- **Phase offset:** lock each TG to a target phase derived from its `tg_id` (e.g. `φ_target = hash(tg_id)`),
  so different TGs settle at visibly different phases and never coincide.
- **Signature:** encode `tg_id` in the beat's look — colour on an RGB indicator (`r2.hw.led kind:rgb`), or a
  subtle pattern on mono. (Costs some calm/simplicity — only if single-glance disambiguation is needed.)

Default stance: keep it simple (temporal disambiguation is enough for "is my group well?"); note the tg_id
phase-offset as the clean upgrade if coincidence ever proves confusing in practice. Add to the prototype as a
scenario — run two uncoupled groups and confirm they drift apart (i.e. coincidence does *not* persist).
