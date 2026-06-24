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
