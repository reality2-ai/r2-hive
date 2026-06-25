# FORKS.md — live spec-vs-impl divergences in r2-hive

Every entry is a known, INTENTIONAL-but-temporary divergence between the firmware impl and ratified canon. Each MUST
carry an owner, a spec-link, and a **removal-condition** so it cannot silently outlive the spec it forks. A fork with
no removal-condition is a bug. Review this file at every spec landing + every firmware reconcile. Empty is the goal.

| Fork | Owner | Spec / canon | Removal-condition | Tracking gate | Status |
|---|---|---|---|---|---|
| _(none — empty is the goal)_ | | | | | |

## Resolved (log)
- **HB byte-8 `power_state`** vs **R2-WIRE §12.6** — RESOLVED 2026-06-25. The firmware dc re-emit landed: byte-8 →
  the §12.6 `{0:seq,1:dc}` Compact CBOR (single-sourced `r2_dataplane::encode_dc_seq_cbor`), originator moved to
  `route_stack[0]` (ROUTE-ORIGIN-1A), and the H9-secure HB-rx (`route.origin()` + verify-first `couple_ok` +
  `accept_keepalive` + `parse_dc`). Builds green (ble + loraroute). The tracking gate
  (`heartbeat_v12_6_dc_seq_canonical`) is now **un-ignored and passing**.

## How an entry leaves this file
1. The removal-condition is met (the impl converges to canon).
2. The tracking gate flips (the xfail/ignored test passes; un-ignore it).
3. Delete the row + note it in RESUME.md's state log.

## Non-forks (intentional, NOT tracked here)
- Off-by-default bench/rig features (`benchkeepalive`, `labrig`) — not forks; they ride the one codebase OFF by default.
- The firmware OTA receiver being **unsigned today** (`#17 ota_task`) — tracked as the **OTA-secure-receiver** work
  item (migrate to `CMD_START_SIGNED`, coordinate with core's A7), not a wire-format fork. See
  `docs/proposals/ota-secure-receiver-device-requirements.md`.
