# Proposal: collapse the heartbeat subsystem to a loose-jittered foundation

**Status:** decide-ready (Roy). Drafted by hive for the Occam's-Razor pass; supervisor to headline.
**Scope:** R2-HEARTBEAT canon + the dfr1195 firmware that implements it.
**Premise alignment:** fading-foundations (fewer load-bearing foundations = more robust) · calm-tech (glanceable, auditable) · security-paramount (fewer footguns). This is the charter applied to the heartbeat architecture.

## The conjecture

> The leaderless PCO heartbeat-sync and the leaderless-HB-mesh stack are excess complexity — load-bearing foundations that the metal refuted and Roy deprioritized. Removing them loses no security or functionality and makes the system more robust.

It survives refutation (below): nothing it protects/enables is still needed on the actual deployment.

## Evidence (why these foundations don't earn their keep)

1. **TN-FR-1-REL (metal, refutation-grade):** the leaderless PCO (Mirollo–Strogatz phase-coupling) does **NOT** achieve phase-sync over lossy half-duplex LoRa — `synced=FALSE` on all boards in **both** arms (tight-PCO and loose). No phase-lock → no synchronized-fire → the contention the sync machinery exists to manage **never occurs** on the deployment medium. Delivery was high (78–95%) regardless of HB mode.
2. **Roy's reframe:** the heartbeat is **loose-background path-maintenance**, not tight phase-sync. The visible data-plane is the **LED-flash-on-message-receipt**, not the heartbeat. So tight PCO sync is not just moot (won't establish) — it's **unwanted** (wrong model).
3. **Roy's mesh deprioritization:** the leaderless-HB-mesh is "an artificial synchronized-fire problem." The real goal — the **message-passing data-plane** — is proven on metal **separately** (TN-FR-1 A→B→C, TN-FR-2 cross-transport, TN-FR-4 role-sim + SCF). The mesh stack is orthogonal to that proven path.

## The proposal

### P1 — demote the cross-node PHASE-LOCK from the canon-default heartbeat
**Precise cut (engine-verified):** KEEP normative the heartbeat FRAME (MsgType::Heartbeat keep-alive), its LIVENESS role (feeds the neighbour table / coupling graph that r2-route reads, the all_well LED, power_state), AND the **per-node heartbeat beat** (the loose-jittered LED lub-DUB — the calm-tech visual is preserved per node). DEMOTE only the **cross-node phase-LOCK** (everyone-beats-together): the 3 distributed correction terms — concave-PCO `phase_response`, reachback delay-compensation, distributed-rate consensus (`RATE_BETA`) + the `K_PHI` coupling nudge — plus the R2-HEARTBEAT §8 directed-spanning-tree convergence PRECONDITION and the HBSYNC-01..10 falsifier campaign.
- The canon-default heartbeat becomes a **low-rate, loose-jittered, TUNABLE keepalive** (websocket-style; NOT the high PCO rate — that was only for phase-lock convergence, retired) at **TWO SCOPES** (specs' kept-liveness refinement): (1) **intra-TG** member-to-member (data-plane multicast to TG members); (2) **across-entanglement** (bidirectional keepalive per live entanglement — each entangled TG knows the other is alive). Fits the sentinel wake-hierarchy (wake→beat→sleep, minimal airtime). **LIFE-SAFETY:** the cross-entanglement keepalive + DG-1 silence-inference = *guardian-alerted-when-the-grid-goes-dark* (flood → heartbeat silent → alert); the rate is the silence-detection-latency knob (a flood sensor beats faster as the river rises). The `loosehb` flag retires (its behavior, generalized to the two scopes, becomes the default).
- **LED roles (Roy's split):** the retired heartbeat-flash is replaced by — (canon) the **light-now TG-directive** (intentional TG-wide identification, separate proposal); (test-only, NOT canon) a **tiny event-arrival flash** (the visible data-plane for demos/debug — exactly TN-FR-1's LED-on-receipt; **confirmed brief: `recv_flash=8` ≈ 400ms** then off). Migration #4 = drop the FIRE-driven HB beat LED; keep the brief event-flash (test); wire light-now (canon).
- **ENGINE FACTS (verified, the rationale's load-bearing claim):** NOTHING functional depends on phase-sync — the phase gates only the per-node LED beat + the HB-emit timing (+ test originate). There are NO phase-gated TX/listen windows, NO TDMA slotting, NO coordinated sleep/wake/power-duty-cycling (the §4.2 airtime-duty is neighbour-*density*-based, not phase-based; power_state is an advertised flag). The neighbour-table/coupling-graph is fed by HB RECEIPT, independent of the phase/rate terms (additive state on top) — so routing/liveness stay fully intact. So phase-sync is the synchronized-fireflies VISUAL plus the contention TN-FR-1-REL refuted — no power-coordination today.
- **Remove** the PCO machinery from the default path (firmware inline = deletable; r2-heartbeat crate PCO = optional-flag or delete per Q1; r2-harness firefly.rs/leaderless.rs/firefly_sweep = retire-with-campaign).
- **Decide-ready Q1 for Roy:** keep the PCO as an **optional, off-by-default** mechanism (documented "for a medium where genuine synchronized-fire occurs AND phase-lock establishes"), or **remove it entirely**? (No medium in the campaign needs it; "optional-but-off" preserves the ready-fix at the cost of carrying dead-ish code — fading-foundations leans toward full removal, git keeps it recoverable.)

### P2 — retire the leaderless-HB-mesh stack
- Remove `blemesh`, `loramesh`, `lorareach`, `lora_mesh_task`, the half-duplex-PCO bridge, and the **continuous-RX/CSMA redesign WIP** (this week's in-progress commits for the deprioritized problem).
- **Decide-ready Q2 for Roy:** full retire, or keep a minimal mesh capability for a future use? (The data-plane goal is met without it; recommend full retire.)

### P3 — cascade-retire the PCO test-instrumentation
- `driftinject`, `rateoff`, `blackout` test the PCO sync (the HBSYNC experiments). With the PCO retired they are dead → remove (git-recoverable). (Supervisor already approved this class under Occam #4.)

## What it saves
- The entire PCO machinery + the leaderless-HB-mesh stack + the CSMA redesign WIP.
- **~7 firmware feature flags** retire: `loosehb` (→ default), `blemesh`, `loramesh`, `lorareach`, `driftinject`, `rateoff`, `blackout`.
- The heartbeat subsystem collapses to **one small loose-jittered-background foundation**.

## Guardrail checks (never cut security or functionality)
- **Security:** nothing cut. The heartbeat is not a security mechanism; the mesh is not a security mechanism; the §7.5.4 deliver-gate + SYNC-1 (the actual security surface) are untouched and unrelated.
- **Functionality:** the heartbeat's real job — loose-background path-maintenance (neighbour liveness / route observation) — is **preserved** (loose-jittered period). The visible data-plane (LED-on-receipt + the message-passing routing) is **preserved** (separate mechanism, proven on metal). Only the moot-and-unwanted tight-sync and the deprioritized mesh are removed.

## Migration (low-risk, when greenlit)
1. Make the loose-jittered period the default HB; delete the PCO nudge/phase-response/rate-consensus paths.
2. Remove the `loosehb`/`blemesh`/`loramesh`/`lorareach`/`driftinject`/`rateoff`/`blackout` features + `lora_mesh_task` + the CSMA WIP.
3. R2-HEARTBEAT canon: demote PCO from the normative default to (optional appendix | removed, per Q1); state the loose-jittered default. (specs lands the canon; spec-first.)
4. specs already added the R2-HEARTBEAT §8 note that PCO sync is medium-dependent — this proposal completes that thread.

## Net
A refuted foundation (PCO) and a deprioritized subsystem (HB-mesh) retired together — the session's own metal findings and Roy's calls paying a second dividend as the architectural simplification. Fewer load-bearing foundations, more robust, more auditable.
