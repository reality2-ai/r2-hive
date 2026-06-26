# FORKS.md — live spec-vs-impl divergences in r2-hive

Every entry is a known, INTENTIONAL-but-temporary divergence between the firmware impl and ratified canon. Each MUST
carry an owner, a spec-link, and a **removal-condition** so it cannot silently outlive the spec it forks. A fork with
no removal-condition is a bug. Review this file at every spec landing + every firmware reconcile. Empty is the goal.

| Fork | Owner | Spec / canon | Removal-condition | Tracking gate | Status |
|---|---|---|---|---|---|
| **OTA transport framing** — DFR receiver uses **OST/ODT/OCM packetized UDP** (start+hdr+sig / offset-chunks+OAK / commit) on UDP :21043; canon (+ composer + r2-core HEAD) is the **CMD_START_SIGNED TCP stream** ([0x03][hdr 123][sig 64]+payload) on TCP :21043 | hive | **R2-UPDATE §3.1.2.3** (CMD_START_SIGNED TCP stream) | specs rules: hive adopts the canon TCP stream **OR** specs ratifies a no_std UDP profile of §3.1.2.3 — impl converges to the ruling | `ota_wire_canon_tcp_stream` (to add) | **RESOLVING** — specs (owns the canon) requested the exact OST/ODT/OCM byte layouts (2026-06-27) to **ratify the DATAGRAM binding ALONGSIDE the §3.1.2.3 TCP binding** (toward option b). Framing handed over. Signing+reject SHARED+correct (verify_header passes both) — transport-only. Not blocking (bench network-parked). On ratify → removal-condition met (impl IS the binding); flip to Resolved. |
| **OTA anti-rollback floor ORDERING** — DFR OCM bumps the security_version floor at TRANSFER-COMMIT (main.rs:3804, `write_anti_rollback` right after `activate_next_partition`) | hive | **R2-UPDATE v0.21 §5.1 (boot_confirm_late) + §9.2** AFTER-CONFIRM | OCM stages candidate (set_boot) + a PENDING(seq,floor) record ONLY; the LIVE floor bump moves to a confirmed-boot step (candidate boots + passes §5 health), mirroring core esp32 `stage_pending_seq`/`confirm_or_rollback_on_boot` (fcded3d) | `ota_after_confirm_floor_bump` (to add) | **OPEN** — self-flagged + specs-confirmed 2026-06-27. BRICK RISK: a bad image that fails to boot can't roll back below an already-bumped floor. Fix needs the no_std esp-bootloader-esp-idf confirmed-boot/rollback API (verify availability) + metal validation WITH the OTA round-trip (bench network-parked). Not blocking now (no OTA in flight). |

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
