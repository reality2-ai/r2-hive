# R2-HW per-board hardware-root matrix (#71 — survey-first, for Roy to ratify)

**Question (supervisor 2026-07-14):** for each target MCU, does the SILICON *natively* provide each of the 3
persistent-claim-state roots?
1. **OTP virgin sentinel** — an irreversible one-time-programmable bit/fuse (a "this device has left the
   factory / lifecycle" mark that cannot be un-set).
2. **Monotonic HW counter** — a *dedicated* hardware monotonic counter for the anti-rollback `hw_epoch`,
   DISTINCT from the firmware `security_version` counter (and how many bits / increments).
3. **Encrypt-at-rest for the persona blob** — ⚠ REFRAMED per core's ruling (2026-07-14): the contract
   (`r2-trust/src/persist.rs:10-14`) is NOT "does silicon expose a HUK" — the serialized persona/member blob
   carries RAW secrets (member: DEV_SK+DEK+HK+cert) and *"Callers MUST encrypt or protect the bytes at rest
   using platform-appropriate mechanisms."* So root-3 = "does the platform provide encrypt-at-rest" (any of:
   transparent encrypted-flash, keychain, HUK-derived AEAD); mechanism is unspecified. "K_slot/K_persona" was
   THIS matrix's terminology, not core's — a per-slot HUK derivation is only needed IF core later defines one.
   Load-bearing for a MEMBER: DEK/HK/DEV_SK can't be derived-on-demand (no TG_SK) → they MUST be persisted →
   platform encrypt-at-rest is mandatory.

**Tiers:** T1 native-persistent · T2 needs-external-SE · T3 non-persistent-only.
**Status:** root-3 RULED by core 2026-07-14 (folded below). Grounded in silicon + in-repo config; confidence
+ FLAGs explicit. **Dev-trial context (Roy ratified 2026-07-14):** the RAK dev persona is PLAINTEXT-CBOR under
the accepted wipe-only soft-seal model — so the root-3 gap below is a deferred PROD/specs decision, not a
dev-flash blocker.

## The boards → the real silicon (only 3 distinct dies)
| Board | MCU | Core ISA | In-repo evidence |
|---|---|---|---|
| RAK4630 | **nRF52840** | ARM Cortex-M4F | `rak4630-fw` (embassy-nrf, CC310) |
| XIAO | **ESP32-S3** | Xtensa LX7 | supervisor; same die as ↓ |
| DFR1195 (FireBeetle 2) | **ESP32-S3** | Xtensa LX7 | `platforms/dfr1195/Cargo.toml` → esp-hal esp32s3, `xtensa-esp32s3-none-elf` |
| FireBeetle-2 | **ESP32-C6FH4** | RISC-V (rv32imac) | `platforms/esp32` → `MCU=esp32c6`, `riscv32imac-esp-espidf` |

→ XIAO and DFR1195 are the **same die (ESP32-S3)**; the matrix has THREE distinct silicon rows.

## ★ CORE ROOT-3 RULING (folded 2026-07-14 — the K_persona/encrypt-at-rest contract; core, read-only)
Ground truth = `persist.rs:10-14` (encrypt-at-rest, mechanism unspecified — NOT a core-defined HUK derivation).
- **ESP32-S3 / C6 → CONFORMANT (T1), with ONE config condition.** Flash-Encryption (XTS-AES, eFuse key HW-only,
  transparent) satisfies encrypt-at-rest DIRECTLY, zero core key-derivation. (HMAC-peripheral + eFuse `HMAC_UP`
  key = an UPGRADE path only if core later mandates domain-separated per-slot keys — that's where a real
  K_persona would live; DS peripheral also present.) **Condition:** conformant ONLY in the HARDENED config =
  Flash-Encryption **Release** mode (NOT Development) **+ Secure Boot v2**. Dev-mode flash-enc is
  re-flashable/bypassable; without SB-v2 a malicious image reads decrypted flash. So "ESP32 satisfies it" is
  TRUE IFF Release-flash-enc + SB-v2 are provisioned — a **BOM/provisioning gate, not a silicon gate**. HIGH.
- **nRF52840 → NONCONFORMANT natively (literal contract gap).** APPROTECT is ACCESS-CONTROL (debug-lock), NOT
  encryption → ZERO at-rest confidentiality; the member persona's DEV_SK/DEK/HK sit as PLAINTEXT in flash. Two
  extraction paths defeat it: (1) PUBLISHED fault-injection bypass — LimitedResults "nRF52 Debug Resurrection"
  (2020) voltage-glitches the APPROTECT load → full SWD → dumps plaintext flash (newer revs harden APPROTECT
  but it's STILL only a debug-lock, not encryption); (2) decap/microprobe reads flash regardless. So vs a
  PHYSICAL-extraction threat model it does NOT meet "MUST encrypt at rest". **Roy decision (the one real call):**
  (a) physical-extraction resistance ⇒ **external SE MANDATORY** on the RAK BOM (ATECC608A-class: wrap the
  sealing key in the SE, never in nRF flash); OR (b) Roy's ratified **wipe-only** model (attacker can ERASEALL-
  wipe but not EXTRACT) ⇒ soft-seal acceptable, NO SE — **but** then `persist.rs`'s at-rest clause must be
  EXPLICITLY amended ("T2/T3 platforms MAY substitute an APPROTECT-gated soft-seal under the wipe-only model"),
  a **specs decision (route to specs)**, else contract+silicon silently disagree. HIGH on the mismatch + bypass.
  ⟹ **Dev-trial takes (b) implicitly** (plaintext dev persona, reflashable); the specs amendment + SE choice is
  a PROD-flash decision, deferred. FLAG: exact RAK4630 rev APPROTECT-hardness = bench-verify (doesn't change
  "not encryption").
- **Q3 unary-eFuse counter:** BLOCK_USR_DATA = 256 one-time bits (S3+C6); a unary `hw_epoch` ≤ 256 IF the whole
  block is free, but it's SHARED with root-1 + other user data ⇒ real budget < 256. SB-v2 `SECURE_VERSION` is a
  SEPARATE dedicated eFuse field (= the fw security_version), distinct from a USR_DATA-unary `hw_epoch` — as the
  matrix says. DEFERRED (no fabricated number): exact `SECURE_VERSION` width + free USR_DATA remainder = read
  off-silicon (`espefuse.py summary` + IDF `esp_efuse_table` per chip-rev). C6 HMAC+DS = HIGH-conf present.

## THE MATRIX

### nRF52840 (RAK4630) — **Tier T2/T3** (the outlier; weakest HW root)
| Root | Native? | Detail | Conf. |
|---|---|---|---|
| 1. OTP virgin sentinel | **NO (true OTP)** | No user-burnable eFuse. UICR is *flash* — erasable by `ERASEALL`/CTRL-AP, so any UICR "sentinel" is REVERSIBLE by a wipe. `APPROTECT` (UICR) locks debug but is CLEARED by `ERASEALL` (that IS the recovery path). FICR is factory read-only, not user-burnable. CC310 exposes NO lifecycle-state OTP (that is CC312 on nRF5340/9160). | HIGH |
| 2. Monotonic HW counter | **NO** | No dedicated monotonic-counter peripheral. Anti-rollback must be flash-based (the `seq` floor — exactly what the RAK bootloader/JOURNAL does). | HIGH |
| 3. HUK / SE | **NO native HUK** | No eFuse-HUK, no KMU (KMU is nRF5340/9160), no transparent flash-encryption-at-rest. `FICR.DEVICEID` is a *readable identifier*, NOT a secret key. CC310 can AES/HMAC but has no HW-protected unique key that isn't SW-readable. So `K_persona` at rest = a SW key in flash, protected only by `APPROTECT` (debug-lock) = plaintext-at-rest to a decap/physical read. | MED-HIGH · FLAG |

**Verdict:** nRF52840's "irreversibility" is `APPROTECT` + `ERASEALL` *semantics* (debug-lock + destructive wipe),
NOT silicon persistence. **T3** for roots 1&2; **T2** for root 3 (a true HUK-sealed persona needs an external
SE, e.g. ATECC608 — otherwise it's an `APPROTECT`-gated SOFT seal). ⚑ This aligns with Roy's ratified
wipe-only/repurpose theft model, but Roy should rule whether an `APPROTECT`-soft persona seal is acceptable or
an external SE is required on the RAK. Core asked (root-3).

### ESP32-S3 (XIAO + DFR1195) — **Tier T1** (native-persistent)
| Root | Native? | Detail | Conf. |
|---|---|---|---|
| 1. OTP virgin sentinel | **YES** | eFuse `BLOCK_USR_DATA` (256 user bits), one-time (0→1 irreversible). Burn a virgin/lifecycle sentinel bit. | HIGH |
| 2. Monotonic HW counter | **YES (unary eFuse)** | Two options: the Secure-Boot-v2 anti-rollback `SECURE_VERSION` eFuse field IS the firmware `security_version` counter; for a DISTINCT `hw_epoch`, burn `BLOCK_USR_DATA` bits *unary* (N bits → N monotonic increments). No auto-increment peripheral; increments = bits you allocate (≤256 in USR_DATA, shared w/ root-1). | HIGH · FLAG (exact bit budget) |
| 3. HUK / SE | **YES** | eFuse-stored keys + HMAC peripheral (key burned `HMAC_UP`, HW-only-readable) → derive `K_slot`/`K_persona`; + Digital-Signature (DS) peripheral; + Flash-Encryption (XTS-AES-256, eFuse key, transparent at-rest). Genuine HUK. | HIGH |

### ESP32-C6FH4 (FireBeetle-2) — **Tier T1** (native-persistent)
| Root | Native? | Detail | Conf. |
|---|---|---|---|
| 1. OTP virgin sentinel | **YES** | eFuse user block (one-time). | HIGH |
| 2. Monotonic HW counter | **YES (unary eFuse)** | Same as S3: `SECURE_VERSION` eFuse (= security_version) + user-eFuse unary for a distinct `hw_epoch`. In-repo `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y` confirms the IDF anti-rollback path is wired. | HIGH · FLAG (bit budget) |
| 3. HUK / SE | **YES** | Same security family as S3: eFuse + HMAC peripheral + DS peripheral + Flash-Encryption (XTS-AES). | MED-HIGH · FLAG (confirm C6 HMAC+DS block sizes — C6 is newer RISC-V; the sdkconfig handles `EFUSE_BLOCK_REV`, chip rev v0.1/blk rev v0.2) |

## Key finding for Roy
The two ESP32 dies (S3, C6) are **T1** — full native persistent roots (OTP + eFuse-counter + eFuse-HUK +
flash-enc). The **nRF52840 is the outlier**: NO true OTP, NO monotonic counter, NO eFuse-HUK, NO
flash-encryption-at-rest. Its persistent-state story rests entirely on `APPROTECT`+`ERASEALL` semantics.
**Design implication:** any R2 persistent-claim-state root that needs true silicon persistence must EITHER (a)
accept a **T2 external SE** on the RAK, OR (b) be designed to degrade gracefully to the `APPROTECT`-soft model
on nRF52840 (which matches the wipe-only threat model but is not hardware-irreversible).

## Open items (flagged, not guessed)
- **Core ruling (root-3, all boards):** is `K_persona` a HW-HUK seal or a SW/HKDF key today? Is nRF52840
  `APPROTECT`-soft acceptable, or is an external SE mandatory on the RAK? ESP32: HMAC-peripheral-HUK vs
  flash-encryption-at-rest? (fleet ask core sent; fold answer in.)
- **eFuse bit budgets** (ESP32-S3/C6): exact USR_DATA bits free for a unary `hw_epoch` after Secure-Boot /
  Flash-Enc key uses — confirm against the IDF eFuse table per chip rev before Roy sizes `hw_epoch` width.
- **ESP32-C6 HMAC+DS presence/sizes:** high confidence present, but verify against the C6 TRM (newer part).
- **nRF52840 CC310 vs CC312:** confirmed CC310 (no KMU/lifecycle-OTP); if any RAK variant ships a different
  crypto block, this row changes — verify the exact part on the bench RAK.
