# FORKS.md — live spec-vs-impl divergences in r2-hive

Every entry is a known, INTENTIONAL-but-temporary divergence between the firmware impl and ratified canon. Each MUST
carry an owner, a spec-link, and a **removal-condition** so it cannot silently outlive the spec it forks. A fork with
no removal-condition is a bug. Review this file at every spec landing + every firmware reconcile. Empty is the goal.

| Fork | Owner | Spec / canon | Removal-condition | Tracking gate | Status |
|---|---|---|---|---|---|
| **HB byte-8 `power_state`** — the DFR firmware advertises a self-asserted availability class as a **fixed binary byte 8** of the Heartbeat payload (commit `d7507cd`). | hive | **R2-WIRE §12.6** — the HB payload is a **CBOR map** with uint keys `{0:seq, 1:dc, ...}`; a fixed byte-8 is NOT canon (specs caught this; `dc` = duty_class, renamed off `power_state` to avoid the R2-BEACON §7.2.1 battery collision). | The **firmware dc re-emit** lands: REVERT byte-8 + emit the §12.6 `{0:seq,1:dc}` CBOR (reusing core's `r2_dataplane::encode_dc_seq_cbor` + `DutyClass::to_wire_uint`, byte-identical, single-sourced) + the firmware HB-rx verifies-first + routes via `accept_keepalive` (H9-secure, not couples-then-ingests). | `crates/r2-hive-bin/tests` heartbeat-CBOR test — **#[ignore]** ("byte-8 fork") while diverging; UN-ignore when the dc re-emit lands (flips red→green = fork resolved). | **LIVE** (2026-06-25). §12.6 landed (R2-WIRE v0.14); r2-dataplane body landed (`71bafab`) on the CBOR; the FIRMWARE re-emit is the remaining piece. |

## How an entry leaves this file
1. The removal-condition is met (the impl converges to canon).
2. The tracking gate flips (the xfail/ignored test passes; un-ignore it).
3. Delete the row + note it in RESUME.md's state log.

## Non-forks (intentional, NOT tracked here)
- Off-by-default bench/rig features (`benchkeepalive`, `labrig`) — not forks; they ride the one codebase OFF by default.
- The firmware OTA receiver being **unsigned today** (`#17 ota_task`) — tracked as the **OTA-secure-receiver** work
  item (migrate to `CMD_START_SIGNED`, coordinate with core's A7), not a wire-format fork. See
  `docs/proposals/ota-secure-receiver-device-requirements.md`.
