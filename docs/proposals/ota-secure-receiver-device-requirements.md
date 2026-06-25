# Device-side requirements for secure OTA (hive input to #20 / TG-manager / key-minter)

**Author:** hive (firmware). **For:** supervisor + core + specs, to seed the #20 (real TG
provisioning + keystore/wallet) + TG-manager + production key-minter design.
**Why:** the DFR/nRF54 OTA receiver has the proven transfer + slot-switch (`#17`, `ota_task`/
`R2_OTA_PORT`), and core landed the **verification** primitive (`r2-update`, R2-UPDATE v0.6,
verify-before-write). The remaining gap to *secure* OTA is the **OTA-authority chain ON THE
DEVICE** — the trust inputs `r2-update`'s `DeviceContext` needs. `r2-update` is below-L5 /
raw-bytes-in (no `r2-trust` dep), so **the firmware must source + persist these bytes**. That
sourcing is the #20/keystore/minter arc, not a hive-solo design — this is the device-side spec.

## 1. The `DeviceContext` surface the firmware must supply (per field)

| Field | Type | Source needed | Persistence |
|---|---|---|---|
| `tg_pk` | `[u8;32]` Ed25519 TG pubkey | **GAP**: the persona carries `hk` (GroupHmac symmetric) + `tg_hash`, NOT `tg_pk`. Needs provisioning (GenPersona/keystore) to write the TG **public** key on-device. | NVS (provisioned) |
| `update_authority_certs` | `&[[u8;151]]` role-0x05 certs | The TG-manager/minter mints these (delegated OTA signers); device provisioned with the set. **Only core's TEST mint exists** — needs a PRODUCTION minter. | NVS (provisioned, updatable) |
| `revocation_gset` | `&[[u8;32]]` revoked mesh_pks | Grow-only union; seeded at provision + grown via verified RevocationEntries (scope-2 co-propagation). | NVS (grow-only) |
| `authority_epoch_floor` | `u32` | Anti-rollback backstop; provisioned floor, **bumped on every accepted verify**. | NVS (monotonic) |
| `current_seq` | `u32` | Replay floor; **bumped on every accepted update**. | NVS (monotonic) |
| `device_id_prefix` | `[u8;8]` | Already on-device (the durable device_id / persona). | existing |
| `class_hash`, `carrier_hash` | `u32` each | The board class/carrier (board-profile derivable). | derivable |
| `battery_pct` | `u8` | The battery gauge (sensor tier). | runtime |

## 2. Anti-rollback NVM layout (the firmware needs a canon slot/format)
`current_seq` + `authority_epoch_floor` must be **persisted + monotonically bumped on accept**
(a replay/rollback MUST be rejected across reboots). Need a canon NVS slot/format (like persona
@0x12000 / provisioned-TG @0x14000) — **or hive defines a new OTA-state sector** (flag which).
Must survive app reflash (distinct sector, like the board-profile @0x13000).

## 3. Production minter (the unlock)
Device-side secure OTA is gated on a **production minter** for the role-0x05 `update_authority`
certs (today only core's TEST mint exists). The minter + the TG-manager (Roy wants on
tuxedo/alfred) are the keystore/wallet arc (#20). Until a device has a real `tg_pk` + (optionally)
real `update_authority_certs`, it can only do **basic TG_SK-direct verify** (empty certs/G-Set).

## 4. Phasing (matches the supervisor's scope split)
- **Scope-1 (basic, ready to wire once §1 `tg_pk` + §2 NVS land):** verify a TG_SK-direct-signed
  `UpdateHeader` via `verify_header` + `check_header_gates`, EMPTY `update_authority_certs` +
  `revocation_gset`, `authority_epoch_floor`/`current_seq` from NVS. Verify-before-commit; reject
  → no slot-switch. **This needs only `tg_pk` provisioned + the anti-rollback NVS slot** — the
  smallest #20 increment that unlocks device secure OTA.
- **Scope-2 (later, gated on Roy's 0x0B ratify + core in-payload packaging):** delegated authority
  via `update_authority_certs` + the 0x0B recovery-section co-propagation (cert + RevocationEntry,
  receiver-atomic merge-before-activate) + G-Set growth.

## 5. What hive needs from #20 to wire scope-1 (the minimal unlock)
1. `tg_pk` written to device NVS at provision (GenPersona extension).
2. A canon anti-rollback NVS slot/format for `current_seq` + `authority_epoch_floor`.
3. `r2-update` available in the `dfr1195-fw-wt` worktree (core's worktree merge).
Then hive wires `verify_header`→`PayloadVerifier`→`finish`-before-activate into `ota_receiver`
(the design is done — see [r2-hive memory: r2-hive-multi-target-goal]).

## 6. CONFIRMED wiring (core A7/F8 alignment, 2026-06-25)
core confirmed the contract so the firmware (F8) + linux/esp32 (core A7/A8) receivers share ONE r2-update verify
path, two call-sites, same order (verify-before-ANY-flash/disk byte):
- **Opcode:** `CMD_START_SIGNED = 0x03` is canon (`r2-update` §3.1.2.3, `pub const ... = 0x03`). Wire `ota_task` to it.
- **Refuse unsigned in release:** feature-gate the legacy unsigned `CMD_START = 0x01` behind `dev-unsigned-ota`
  (OFF by default → `RESP_ERR`), so a release firmware refuses unsigned OTA.
- **Dep:** `r2-update = { path = "../../crates/r2-update", default-features = false }` (no_std) — the crate is in the
  consolidated worktree + builds green; the DFR firmware has 0 refs today, so this ADDS it.
- **Sequencing:** firmware dc re-emit FIRST, then the OTA wire. Ping core at wire-start → core confirms the exact
  `DeviceContext` field plumbing. core's A7/A8 sequence after its Wave-1 (A1/F2/F3 dedup-poisoning).

## 7. `DeviceContext` field plumbing — CONFIRMED by core (2026-06-25, the r2-update public API, STABLE)
The 10 fields `verify_header(header_bytes, sig, &DeviceContext)` takes, with the DFR scope-1 sourcing. Build it once
per OTA-START. (#20 GO; core landing role-0x05 cert Phase 1 + the linux/esp32 receiver rewrite — byte offsets may
ping, this field SET will not change.)
1. `class_hash: u32` = FNV1a-32(device CLASS string) — gate 4 (target_class 0-or-match).
2. `carrier_hash: u32` = FNV1a-32(CARRIER-board string) — gate 3 (target_carrier 0-or-match).
3. `tg_prefix: [u8;8]` = the device trust-group prefix (the 8-byte TG id already addressed with).
4. `device_id_prefix: [u8;8]` = device_id[0:8] (R2-WIRE §6.2.2; durable FIRST-firmware key prefix; provisioned/
   persisted, NOT tg-scoped).
5. `current_seq: u32` = the replay floor from the NEW NVS anti-rollback slot. BUMP on every accepted update.
6. `battery_pct: u8` = the firmware battery gate.
7. `tg_pk: [u8;32]` = `persona.tg_pk` (key 5, raw) — §2.4 acceptable-signer-1 + the verifier of the certs/revocations.
8. `update_authority_certs: &[[u8;151]]` = **EMPTY** for scope-1 (TG_SK-direct only; role-0x05 certs are #20 Phase 1).
9. `revocation_gset: &[[u8;32]]` = **EMPTY** for scope-1 (grow-only union; verify each incoming RevocationEntry via
   `verify_revocation_entry` then union, never remove — scope-2).
10. `authority_epoch_floor: u32` = from the NVS slot (anti-rollback BACKSTOP). BUMP to `VerifiedHeader.authority_epoch`
    on a successful verify, persist in NVM.

**Scope-1 = the 8 direct fields + 2 empties, NO §9.4a.** With EMPTY (8)+(9) there are no certs to revoke and no floor
to bump on the verify path, so `verify_header` + `PayloadVerifier` finish-before-activate IS the whole of scope-1. The
§9.4a recovery (`parse_recovery_section` → merge RevocationEntry + bump the NVM floor as an activation precondition)
is exercised only once update_authority delegation + the 0x0B recovery packaging land (scope-2 / #20).

**The NEW anti-rollback NVS slot** (hive defines): persists `current_seq` + `authority_epoch_floor`, monotonic, MUST
survive an app-reflash (a distinct raw-offset region in the partition gap, like persona@0x12000 / board-profile@0x13000
— NOT inside the app). Both bumped only on an accepted verify.
