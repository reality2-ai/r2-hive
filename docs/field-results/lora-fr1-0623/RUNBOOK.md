# TN-FR-1 metal runbook — BL-200 message-passing over LoRa (A→B→C)

Staged 2026-06-23. Firmware: r2-hive commit `4042042` (`loraroute`). ELF + espflash + partitions
pre-staged on tuxedo at `~/r2-flash/`. Flash ONLY after composer pings `dfr-fr1-off` (0 tty holders).

## Boards (hive ⇄ MAC ⇄ role)
- **A** = `0dcadbf8` (xx:xx:xx:xx:xx:xx) — ORIGINATOR (auto-sends Event→C every ~6s; can_hear={B})
- **B** = `2cab5f69` (xx:xx:xx:xx:xx:xx) — RELAY (can_hear={A,C})
- **C** = `f91c8911` (xx:xx:xx:xx:xx:xx) — DESTINATION (can_hear={B}); **LED flashes on each receipt**

A↮C is masked → A→C is forced multi-hop via B. Get the live `/dev/serial/by-id/...` per-board paths
from composer at release (ports renumber on reset — never use ttyACMn).

## Flash (per board; run from alfred — my ssh to tuxedo works)
```
ssh tuxedo-os '~/r2-flash/espflash flash --chip esp32s3 \
  --partition-table ~/r2-flash/dfr1195-partitions.csv \
  --port <BY_ID_PATH> -a hard-reset --non-interactive \
  ~/r2-flash/r2-dfr1195-loraroute.elf'
```
DFR1195 LED = active-high → no board-profile (`0x13000`) rewrite needed. NVS (MASK/SENDTO) is unused
by loraroute (topology + A's dest are hardcoded), so no provisioning step.

## Capture (one shared fd per tty; reduce reset-on-open)
```
ssh tuxedo-os 'stty -F <BY_ID> -hupcl raw -echo; timeout 120 cat <BY_ID>' \
  | tee docs/field-results/lora-fr1-0623/<board>.log
```
Capture all 3 concurrently for ≥90s after all are up.

## PASS criteria (the A→B→C proof)
1. **Neighbour discovery over LoRa:** B logs ingest of both A and C (HBs); A and C each see only B.
   `status ... nbrs=` → B:2, A:1, C:1.
2. **directed_via B:** B logs `RX-EV msg_id=<n> directed_via from=2cab5f69 next_hop=f91c8911` (A's REQUEST
   routed toward C, not flooded). A's REQUEST is `RT-REQ msg_id=<n> -> f91c8911`.
3. **multi-hop forced:** C never delivers A's *direct* frame (masked); only the B-relayed copy →
   C logs `RELAY`-absent + `DELIVERED msg_id=<n> 'A...req'`. (C should show NO direct-from-A RX.)
4. **exactly_once@C:** each distinct `msg_id` `DELIVERED` exactly once at C; repeated `msg_id` → `DEDUP`.
   (Per-(origin=A, msg_id) dedup — the fr_origin fix.)
5. **LED-flash on receipt:** C's LED flashes (~400ms bright) on each `DELIVERED` (Roy's eyes = ground truth).
6. **reply retrace:** C emits reply (reply_msg_id high-bit); A logs `DELIVERED` of the reply via B.

## After
Capture serial → fill `TN-FR-1.json` (schema below) → commit `field.*` → **restore baseline**: reflash
the 3 DFR to the 2-TG demo build (`nobt,multitg`) and hand the ttys back to composer.
