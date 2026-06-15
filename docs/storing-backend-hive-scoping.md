# Storing Backend Hive — scoping (design + cost)

**Status:** scoping for Roy's go/no-go. **Not a build.** Authored by hive (Roy commission via supervisor).
**Inputs:** bos's concrete requirements (fleet); reuse-surface survey of r2-core/r2-workshop/r2-specifications;
hive's own Linux platform layer + storage seam. core (r2-engine/r2-wasm) + composer (ensemble/sync) peer-asks
in flight — numbers below may refine, the shape will not.

---

## 0. The gap in one sentence

R2 today is **peer-to-peer sync through a dumb relay forwarder with NO durable server-side store** — Notekeeper
is "no server", workshop is a dumb forwarder, the relay has no state. A business app (BOS: records, versions,
meetings, proposals, audit + provenance) needs a **record-of-truth that survives every client being offline**.
The storing backend hive is the always-on Linux member that provides it.

## 1. Architecture

In the one-codebase model the storing backend is **not a new codebase** — it is an **always-on Linux r2-hive**
(my platform layer) running a **persistence ensemble** that drains the event bus into a **durable store behind a
new `RecordStore` seam**, and serves reads/writes back to clients over the existing transports.

```
            clients (browser/wasm hives, PAI/MCP)         ← R2-WIRE over WS/UDP/relay
                         │  events (create/update/propose/…)
                         ▼
   ┌─────────────────────────────────────────── Linux storing-backend hive ──┐
   │  HiveState (my platform layer: clock/RNG/sockets/storage seams)          │
   │      router::route_frame → EnsembleRegistry.dispatch (DispatchTarget)    │  ← EXISTS today
   │                         │                                                │
   │         ┌───────────────┴────────────┐                                  │
   │         ▼                            ▼                                   │
   │   business sentants           Persistence sentant  ── NEW               │
   │   (durable-state)             (subscribes to record/audit/proposal      │
   │                                events; applies write-authority;          │
   │                                appends to the durable store)             │
   │                                       │                                  │
   │                                       ▼  RecordStore seam  ── NEW        │
   │                          ┌────────────────────────────┐                 │
   │                          │ durable store impl:         │                 │
   │                          │  Postgres / SQLite (now)    │  ← record-of-   │
   │                          │  R2-native event-log/CRDT   │     truth       │
   │                          │  (later, same seam)         │                 │
   │                          └────────────────────────────┘                 │
   └──────────────────────────────────────────────────────────────────────────┘
```

### REUSES (already built / exists today)
- **Linux platform layer** (`r2-hive-bin/src/platform.rs` `LinuxPlatform` + `r2-hive-core::platform::Platform`) — clock, RNG, sockets. The backend is just a long-running instance of it.
- **The storage seam I already factored** (`r2-hive-core::identity::{IdentityStore, StoreError}` + bin `FileStore`: atomic write, 0600, idempotent load_or_create). This is the **exact template** for the new `RecordStore` seam — trait in core, platform impl in bin, error abstracted. *We have done this move once already; the second seam is cheap.*
- **EnsembleRegistry hosting** — `HiveState` already owns `Arc<EnsembleRegistry>` and routes inbound events to it via `DispatchTarget::dispatch` (`router::route_frame` → `state.ensembles.dispatch`). **A Linux hive hosts sentants TODAY, full std.** (See §4 — this is the headline advantage.)
- **r2-engine EventBus** — `drain_outbound()` is the natural tap point for an archive sink; hash-based multicast means a persistence sentant just subscribes to the event hashes it must record.
- **Sentant `durable-state` flag** (R2-SENTANT §2.2) — business sentants can already declare they persist; the backend supplies the snapshot store the flag implies.
- **Transports + relay driver + multi-transport send** (this branch: WS relay driver, UDP-LAN, tested) — clients reach the backend over the same fabric; no new transport work.
- **r2-trust** identity / TG membership / **revocation CRDT** (the one existing distributed-consensus primitive) — reused for who-is-in + key revocation.
- **r2-cbor / r2-fnv** — payload codec + event-name hashing → durable keys.

### NEW (must build)
- **`RecordStore` seam** — durable, append-friendly store trait (records + versions + audit, point-in-time read). Modeled on `IdentityStore`. Platform impls swap behind it (see §6 recommendation).
- **Persistence sentant** — subscribes to record/proposal/audit events, applies write-authority, writes the store, serves reads. The "archive sink" the registry lacks.
- **Durable event log + snapshot store** — append-only `(seq, ts, actor, action, before/after, event)`; the record-of-truth. Absent everywhere today (all R2 state is ephemeral in-memory; only sentant `durable-state` snapshots and the revocation G-Set persist).
- **Write-authority / proposal serialization** — bos's PROPOSALS model is the answer (see §2): AI/agent mutations go through typed-op diffs → human accept/reject → atomic apply. The "apply" event becomes the *only* mutation path = a clean serialization point. New, but small and elegant.
- **Inbound-write path** — the relay/transport "write to the server" direction workshop flagged "(future)". Today clients drain_outbound → relay → peers; the backend needs to be an addressable *write target*, not just a peer. Mostly wiring on top of the existing inbound route path.
- **Read-authority filter** (`canSee(person, entity)`) — enforced on all reads (bos requirement #3); maps to TG capability + confidential scopes.

## 2. Multi-user write-authority + audit

bos's model maps onto R2 cleanly and is **better than the LWW Notekeeper shows** (LWW silently drops the loser; a business record-of-truth cannot):

- **Actor = person(human | agent).** Every mutation attributed. TG membership (r2-trust) + per-message Ed25519 (the relay v0.2 handshake already proves device identity; per-actor identity is the extension) provide the cryptographic attribution. bos's interim `x-bos-actor`/M365 seam → TG membership later.
- **Proposals as the serialization point.** AI/agent record-changes are **typed-op diffs** that a human accept/rejects; on accept they apply **atomically** with `via_proposal + approved_by + provenance`. In R2 terms: a `proposal.*` event stream + an `apply` event that the persistence sentant treats as the *sole* authorized mutation. This sidesteps general multi-writer conflict resolution (which R2 lacks) by **funnelling all contested writes through one ordered, attributable gate.** Human direct edits can be a degenerate auto-approved proposal, keeping one write path.
- **Append-only `audit_event` per mutation** (actor + action + before/after, human-vs-agent) — this IS the durable event log from §1, projected. One log serves both audit and record-of-truth.
- **Versioning over LWW** — `knowledge_version` history is the event log replayed per entity; no information loss.

**Gap:** R2 has no write-authority spec and no audit/provenance canon (only revocation CRDT). The *mechanism* is buildable on the event/sentant model now; the *normative rules* are specs' to author (§5).

## 3. Persistence while all clients offline

This is the crux — P2P per-device storage cannot provide it. The always-on backend is a TG member that:
- **Holds the durable store** so the record-of-truth exists with zero clients connected.
- **Buffers + serves catch-up** — clients reconcile on reconnect (the relay's catch-up buffer pattern already exists in the WS handshake path; the backend makes it durable instead of in-memory).
- **Runs server-side PAI/MCP** (bos: these run server-side) — agents act against the record-of-truth directly, not a device replica.

**Store options (behind the `RecordStore` seam):**
1. **Conventional embedded — SQLite** (single-process, zero-ops, WAL crash-safety, good to ~10s GB). Fastest path to a working BOS.
2. **Conventional server — Postgres** (multi-process, rich query, mature). If BOS wants SQL reporting now.
3. **R2-native event-log/CRDT** (append-only log + per-entity projection; CRDT or proposal-gated). The north-star, but the canon doesn't exist yet (§5) — months of design+spec.
4. **sled / redb** (embedded KV, Rust-native) — middle ground if we want pure-Rust no external dep.

## 4. Phase-5e relationship + the full-std advantage

This is **adjacent to** workshop's "Phase-5e" (the inbound-write/server tier) but a **cleaner cut**: 5e as workshop framed it is "make the relay/firmware store inbound writes." The storing-backend-hive reframes it as **"a normal Linux hive that happens to persist"** — which is more north-star-correct (one codebase, platform layer + a persistence ensemble) than bolting storage onto the dumb relay.

**Headline advantage (call out for Roy):** the backend is **full-std Linux, so it can host ensembles/sentants TODAY** — `HiveState` already owns `EnsembleRegistry` and dispatches to it. **None of the MCU no_std re-tiering** (r2-def/ensemble/dispatch → no_std, the firmware-tier blocker) is on the critical path here. The storing backend is buildable on the stack as it exists **now**, in parallel with (and unblocked by) the hardware-gated firmware tier.

## 5. Spec gaps (FLAG — specs' job to author, Roy-gated)

| Gap | Canon today | Needed |
|---|---|---|
| Durable event-log / audit-trail primitive | none (R2-RUNTIME §8 = abstract KV only) | append-only log abstraction + ordering/sequence semantics |
| Write-authority / multi-writer model | none ("no central authority", no tie-break) | proposal-gated write-authority; actor attribution rules |
| Conflict resolution standard | app-level LWW only (Notekeeper); revocation G-Set | per-entity policy (proposal-gated / CRDT / immutable) |
| Read-authority (`canSee`) / confidential scopes | TG membership exists; no per-entity ACL | scope model → TG capability mapping |
| Provenance / audit canon | none | actor + human-vs-agent + before/after normative shape |
| Crash-safety / WAL for sentant state | R2-SENTANT §4.5 sketch, unimplemented | snapshot+log recovery contract |

The **mechanisms** are buildable now against the event/sentant model; the **normative wording** is specs', and several (write-authority, audit, scopes) are wire-/cross-component-visible so Roy-gated. Recommend hive builds the seam + reference impl; specs ratifies the contract in parallel (the same spec-first pattern that worked for the USB type-byte + pairing vocab).

## 6. Cost (decomposed; estimates in hive session-units = focused work sessions)

| # | Step | Reuse vs New | Est. | Gating |
|---|---|---|---|---|
| 1 | `RecordStore` seam (trait in core + `StoreError`-style abstraction) | **Reuse pattern** (IdentityStore) | 1 | none |
| 2 | SQLite-backed `RecordStore` impl (records, versions, append-only audit log, point-in-time read) | New (conventional) | 2–3 | 1 |
| 3 | Persistence sentant (subscribe → apply → write; serve reads) | New on reuse (EnsembleRegistry/DispatchTarget exist) | 2–3 | 1,2 |
| 4 | Proposal/write-authority gate (typed-op diff, accept/reject, atomic apply, attribution) | New (small, elegant) | 2 | 3 |
| 5 | Inbound-write path (backend as addressable write target over existing transports) | Mostly wiring on route path | 1–2 | transports (done) |
| 6 | Read-authority filter (`canSee` + confidential scopes → TG capability) | New | 2 | r2-trust TG caps |
| 7 | Catch-up / reconcile-on-reconnect made durable | Reuse (WS catch-up buffer) → durable | 1–2 | 2 |
| 8 | Crash-recovery (snapshot + log replay on boot) | New (R2-SENTANT §4.5 sketch) | 1–2 | 2 |
| 9 | BOS schema mapping + integration spike | New (with bos) | 2–3 | bos |
| | **Total** | | **~14–20 units** | |

**Gating deps:** r2-trust TG capabilities (step 6); specs ratifying write-authority/audit/scope canon (steps 4,6 — can proceed against a draft, spec-first); bos schema detail (step 9). **Not gated on** MCU no_std re-tier or core D3b (those are firmware-tier).

**Recommendation — HYBRID, seam-first (build-storing-layer-first, conventional store behind it):**
Build the `RecordStore` seam + persistence sentant + proposal gate **now**, backed by a **conventional embedded store (SQLite)** as the first impl. This gives BOS a working, durable, attributable record-of-truth on the **shortest path** (steps 1–5,9 ≈ 8–11 units to a usable BOS), while the **R2-native event-log/CRDT store is deferred behind the same seam** — swapped in later with zero change to the sentant logic. This is the altitude-correct cut: the seam is the durable decision; the store impl is replaceable. It avoids both (a) blocking BOS on unwritten distributed-consensus canon and (b) a throwaway conventional app we'd later rip out — the conventional store lives *behind the R2 seam* from day one, so BOS is an R2 app immediately, not a migration target.

**Rejected alternative:** "conventional store now, migrate to R2 later" as a *separate non-R2 app* — it duplicates the data model, has no R2 attribution/TG story, and the migration is the expensive part. Putting the conventional store behind the R2 seam from the start is strictly better.

## 7. Risks / unknowns + spikes

- **R2-native vs conventional long-term** — if Roy wants the record-of-truth itself to be R2-native (CRDT/event-log, mesh-replicated, no SQL) that's the months-long path; the hybrid keeps the option open but defers it. *Decision for Roy.*
- **Proposal model coverage** — does every BOS mutation fit typed-op diffs, or are there bulk/streaming writes (ai_scan, notifications) that need a fast non-proposal path? **Spike with bos** (step 9).
- **Write-authority canon is Roy-gated + cross-component** — composer (fleet orchestration) and clients all see it; needs specs + a fleet-wide agreement, not a unilateral hive impl. Spec-first mitigates.
- **Multi-backend / HA** — one always-on Linux hive is a single point of truth; replication/failover is out of scope for v1 (eventual-consistency replicas later, leaning on the same event log). Flag, don't solve now.
- **Read-authority performance** — `canSee` on every read needs an index, not a per-entity scan. Design into the SQLite schema (step 2/6).
- **Spike needed:** (a) bos schema → RecordStore mapping + proposal coverage (1 unit, with bos); (b) confirm EventBus `drain_outbound`/dispatch gives a clean, ordered tap for the persistence sentant without a core change (0.5 unit, confirm with core — peer-ask in flight).

---

**Bottom line for Roy:** the storing backend hive is a **normal always-on Linux R2 hive + one persistence ensemble + a `RecordStore` seam** — buildable **now** on the existing full-std stack (no firmware-tier blockers), reusing the platform layer, ensemble hosting, transports, and the exact storage-seam pattern hive already factored. Shortest path to a durable, attributable BOS record-of-truth is **~8–11 session-units** (seam + SQLite impl + persistence sentant + proposal gate + inbound-write + bos mapping), with the R2-native event-log/CRDT store deferrable behind the same seam. The hard parts are **canon, not code**: write-authority/audit/scope rules are specs' to ratify (Roy-gated, spec-first). Recommend **go** on the hybrid seam-first path.
