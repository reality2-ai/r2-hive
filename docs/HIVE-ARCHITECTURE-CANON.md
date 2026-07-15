# HIVE ARCHITECTURE CANON

> **Status:** foundational reference (Roy directive, 2026-07-10). This is the **hive
> implementation-view** of the R2 architectural canon — it MIRRORS the specifications,
> it does not fork them. Every ruling below is owned by `r2-specifications`; this doc
> records *where each invariant lives in hive code* and the code-side commitment.
> On any drift, the spec is authority — surface it as a bug, do not diverge here.
>
> **Phasing:** the four rulings are canon **now**. The one *structural refactor* they
> imply (no-group-None as a type-level invariant, §4) is **scheduled** — it lands only
> after the R2-TRUST §2.3 deliver-gate interactions are pinned (they now are, v0.40) and
> the substrate type change is coordinated with `core` (it owns `r2-dataplane`). See
> RESUME "FOUNDATIONAL — Roy no-TG-less canon". Nothing here disrupts in-flight demo work.

---

## 1. Every R2 device runs the core TN hive (role-agnostic)

**Canon (R2-ARCH v0.15, R2-RUNTIME v0.24):** all R2 devices — sensor, repeater, router,
bridge, complex-hive — run the **same** core Transient-Network hive. Role is composition
*on top of* the one substrate, never a separate firmware. The core network stack is the
**single non-plugin exception**: it is the always-present substrate, not an ensemble.

**Where in hive code:**
- **Host tier** — `r2-hive-bin` depends on the core stack directly: `r2-hive-core`,
  `r2-wire`, `r2-route`, `r2-trust`, `r2-engine`, `r2-ensemble` (see
  `crates/r2-hive-bin/Cargo.toml`). The daemon *is* the core hive; roles are ensembles/plugins.
- **MCU tier** — the firmware (dfr1195-fw, rak4630-fw) links the identical core stack via
  `r2-dataplane` (which composes `r2-route` + `r2-wire` + `r2-trust`) + `r2-engine`. The
  RAK "repeater" and the DFR "sensor" are the **same** `DataPlane` substrate with different
  ensembles — proven on metal (`rak4630/src/main.rs` doc: "Repeater = role/ensemble, NOT
  separate firmware").

**Commitment:** no role gets a bespoke net stack. A new board brings up `DataPlane`/`r2-hive-core`
first, then composes its role.

## 2. Device-composition layering

**Canon (R2-RUNTIME v0.24; R2-INDICATOR v0.5 for the dub-dub calm-LED):** the composition
order is fixed:
1. **core TN hive** — MUST (the §1 substrate).
2. **near-universal OTA ensemble + dub-dub calm-LED** — hardware-conditional (present wherever
   the board can self-update / has an indicator).
3. **dev-only report-TN ensemble** — dev builds only (telemetry/diagnostics; compiled out of prod).
4. **role plugins** — sensor / repeater / router / bridge / complex, on top.

**Where in hive code:**
- MCU: the firmware Cargo features realize the layering — the core `DataPlane`/`loraroute`
  substrate is unconditional; `otaengine` (OTA ensemble) + the calm-LED `led_signature`
  module are near-universal; `dev` gates the report-TN/diagnostic ensemble (compiled out of
  prod per R2-BUILDMODE); `fakesensor`/`xiaobridge`/repeater are the role layer.
- Host: `r2-hive-bin` mirrors this — core hive always on, OTA + management ensembles, dev-mode
  diagnostics gated by the `dev` feature (R2-BUILDMODE §5.1), role via ensembles.

**Commitment:** OTA + calm-LED default-on (HW-conditional); report-TN is dev-only and structurally
absent in prod; role is the top, replaceable layer.

## 3. All hives are dual-bearer beacons (a Heartbeat is NOT a beacon)

**Canon (R2-BEACON v0.41; R2-HEARTBEAT v0.17):** every hive **beacons on every bearer it has
hardware for** (LoRa §8.1 beacon if it has a LoRa radio, BLE R2-BEACON if it has BLE). The
**beacon is the discovery primitive** — without it a node is invisible/un-meshable. A
**Heartbeat is not a beacon**: the HB is liveness/relay (origin-only), MUST NOT be substituted
for a discovery beacon (R2-BEACON §3.3).

**Where in code (ground-truth verified).** NOTE: `build_lora_beacon` lives in the **firmware
repos** (`dfr1195-fw`, `rak4630-fw` — separate r2-core worktrees), NOT in this r2-hive tree; the
paths below are repo-qualified:
- LoRa §8.1 discovery beacon: `build_lora_beacon` (15/17B
  `[0xB2][ver][flags][rbid8][class_hash BE][tx_power][build_class]`) in
  `dfr1195-fw/platforms/dfr1195/src/main.rs` (emitted from `lora_route_task`) and — as of the
  2026-07-10 demomember bake (`rak4630-fw` e4e8334) — `rak4630-fw/platforms/rak4630/src/main.rs`
  (emitted from the beacon block, `demomember`-gated). rbid = `compute_rbid(session_key, epoch)` —
  REQUIRES TG key material (§4).
- BLE R2-BEACON: `rak4630-fw/platforms/rak4630/src/main.rs` `ExtendedBeacon`/`build_legacy_beacon`
  (the advert codec; radiate is inc-2a, task #58).
- Heartbeat (distinct): `poll_keepalive` (`r2-dataplane`) — origin-attribution liveness, **not**
  a discovery beacon. The RAK reconciliation (2026-07-10) confirmed the *bring-up* RAK emits only a
  keepalive HB; the **demomember** RAK (§4) is now a real TG member emitting a real §8.1 beacon.

**Commitment:** the beacon codec is per-bearer but the *discovery contract* is uniform; the
XIAO USB bridge forwards LoRa beacon-sightings (transport-local `0xA1` wrapper, raw beacon
verbatim) as an unauthenticated **presence signpost**, never a passport.

## 4. No TG-less device (the type-level invariant)

**Canon (R2-TRUST v0.40 §2.3; R2-PROVISION v0.30):** there is **no TG-less device and no
`group = None`**. Every device is **always** in a Trust-Group — at minimum a **singleton
TG-of-one** it self-generates at birth (real key material from birth, NOT a placeholder).

**Birth derivation** (corrected 2026-07-10 per specs-codex; each step ground-truth-verified in
`r2-trust`). The earlier draft wrongly chained HK *through* `tg_id` — HK and `tg_id` are two
**separate** derivations off the TG keypair, they do not chain:
- generate the TG keypair `TG_SK`/`TG_PK`, then `derive_group_keys(TG_SK)` → **DEK + HK**
  (R2-TRUST §3.1; `r2-trust/src/hkdf.rs:55`, `lib.rs:13`). HK is derived from the TG **secret**,
  **NOT** through `tg_id`.
- `TG_PK` → **`tg_id`** (R2-WIRE §6.2.1) — a *separate* path from the HK derivation.
- device identity: `device_master_secret + tg_id` → **`hive_id`** (`derive_hive_id`, HKDF label
  `r2-hive-id-v1`) and the TG-scoped on-air keypair **`mesh_sk`/`mesh_pk`** (`derive_mesh_key`, label
  `r2-dev-key-v1`) — both per-TG, so a different TG yields an unlinkable identity (R2-WIRE §6.2.2).
- self-issue a **key-holder certificate** whose **subject is `mesh_pk`** (`r2-trust/src/cert.rs::issue`
  signed by `TG_SK`; the subject is the per-TG mesh signing pubkey — `revocation.rs:52`
  `cert_subject_pk`; `lifecycle.rs:95` "self-issues a key-holder certificate"). **Membership ⟺ a
  valid cert**, so a singleton is a genuine member (its own key-holder over its own `mesh_pk`), not a
  keyless node.
- beacon RBID: `session_key = HKDF-Expand(PRK=HK, info="r2-beacon-rbid-v1" ‖ hive_id_be32, L=16)`
  → `rbid = HMAC(session_key, epoch_be64)[:8]` (`r2-discovery/src/beacon.rs`). The RBID keys off
  **HK** (group) + **hive_id** (per-member) — a TG peer holding HK resolves it, a stranger cannot.

A device **proximity-enrols to the area TG** (on the bench = the demo TG `0xF305FE07`); enrolment
is a **re-persona** (identity replacement: wipe the singleton material, join the area TG, adopt its
identity whole — groups do NOT merge, it is not a re-key). The **relay function is auth-free
below-L5** (a node relays any TG's frames without membership) — this is **NOT** the same as device
identity, which is always TG-anchored.

Deliver-gate (R2-TRUST §2.3, §7.5.4): a born singleton is **deliver-enabled to its OWN group**
(verifies + delivers its own GroupHmac'd traffic), and **fail-closed cross-TG** (unique HK) —
the §7.5.4 security property is fully preserved. The old `group = None` was fail-closed only
because there was *no key to verify with*; a singleton *has* its own HK.

**Where in hive code (ground-truth verified) — and the TYPE-LEVEL commitment:**
- **Firmware substrate:** `r2-dataplane::DataPlane.group: Option<GroupHmac>`
  (`crates/r2-dataplane/src/lib.rs:145` field, `:212` constructor). The RAK now has **both**
  construction sites (`rak4630-fw/platforms/rak4630/src/main.rs`, cfg-split at `DataPlane::new`):
  the default bring-up passes `None` (keyless-repeater posture) while the **`demomember`** build
  (e4e8334) passes `Some(GroupHmac::new(HK))` — a real member of demo TG `0xF305FE07`, which is the
  first concrete realization of this invariant on metal. **The invariant makes the field
  non-Optional** — `DataPlane` holds a real `GroupHmac` (born-singleton default) so that a TG-less
  device is **UN-CONSTRUCTIBLE**, a type property not a runtime check. `r2-dataplane` is
  **core-owned**; hive requests/coordinates the `Option<GroupHmac>` → non-Optional change spec-first
  with `core`, and meanwhile commits to passing a real `GroupHmac` (never `None`) at **every**
  construction site (dfr1195-fw, rak4630-fw, xiaobridge) — `demomember` proves the member path works.
- **Host daemon:** `r2-hive-bin` — membership is `hive.rs:255 group_hmacs: HashMap<u32, GroupHmac>`;
  the keyless-dev-daemon path (`router.rs:276` returns `None` "no group keys configured",
  `R2_DELIVER_UNKEYED_OPEN`) is the host analogue of `group = None`. The invariant: the daemon
  is **born with ≥1 TG** (a singleton-of-one when unprovisioned); `unkeyed_open` becomes
  *relay-only within its TG*, never *no TG*.

**Home of the type invariant:** `r2-dataplane::DataPlane` (the shared MCU substrate group field)
+ the `r2-hive-bin` membership/deliver-gate. Refactor **deferred** (scheduled) — it is a
multi-repo change (`r2-dataplane` type → firmware + `r2-hive-bin` construction sites),
coordinated with `core` and gated on R2-TRUST §2.3 (now pinned) + R2-PROVISION §8.3 re-persona.

---

## Cross-references
- Spec canon (authority): R2-ARCH v0.15, R2-RUNTIME v0.24, R2-INDICATOR v0.5, R2-BEACON v0.41,
  R2-HEARTBEAT v0.17, R2-TRUST v0.40 (§2.3 deliver-gate, §3.1 DEK/HK derivation), R2-PROVISION v0.30,
  R2-WIRE §6.2.1 (tg_id/hive_id/mesh_key), §6.2.2 (device_id unlinkability), R2-LORA §3.0/§8.1.
- Hive state/trail: `RESUME.md` (r2-hive), `dfr1195-fw/RESUME.md`, `rak4630-fw/RESUME.md`.
- Related invariant work: hive_id KS1-derivation (R2-WIRE §6.2.1, task #57); benchsf7 SF7
  bench profile (R2-LORA §3.0 v0.4.22).
