# Field-firmware triplet — on-metal validation (2026-06-27, Alfred)

**Result: keystone + radio PROVEN on hardware; OTA networked round-trip = documented follow-up (bench-topology-blocked, not a firmware gap).**

Roy FLASH-GO, fleet-flashes-with-identity-check. Worktree `dfr1195-fw` @ base r2-core `c46383e`, firmware
worktree HEAD `0f87bd3`. Image = `xiao,field,loraroute,loratcxo,multitg` (1.32 MB). composer mint out-dir
`/home/roycdavies/r2-bench/mariko-triplet/`, bench field TG `1494e803-6993-45d3-9a46-49feab7533bb`
(tg_pk `d9fb84d4..`). One signed image; the RPF1 record @0x17000 config-activates the role.

## Boards (flashed via STABLE /dev/serial/by-id MAC paths; boot-banner identity-verified)

| Role | by-id MAC | banner hive | composer mint | role-activation | persona |
|------|-----------|-------------|---------------|-----------------|---------|
| sensor   | 14:C1:9F:C4:FC:8C | `c01cee4d` | `c01cee4d` ✓ | `sensor` duty=2 §3.2.2-provisioned | true |
| repeater | E8:3D:C1:FB:E5:20 | `296f308b` | `296f308b` ✓ | `repeater` duty=1 | true |
| bridge   | D8:3B:DA:75:C3:3C | `bd72902e` | `bd72902e` ✓ | `bridge` duty=1 | true |

(4th XIAO E8:3D:C1:FB:DB:44 = spare. The 5 DFR1195 `F4:12:FA:*` also on Alfred = untouched.)

## Validations

- **PASS — Role-activation (R2-RUNTIME §3.2 keystone).** All 3 boot the SAME image and config-activate
  their role from the RPF1 NVS record; all 3 hive_ids match composer's mint; §3.5 persona re-attach
  ("RE-ATTACH -> persona valid; resuming role", no join). One image, ensemble-differentiated — PROVEN on metal.
- **PASS — R2-BEACON §8.1 LoRa beacon.** Bridge decoded a peer's §8.1 beacon
  `LORA-BEACON rbid=6acdd5.. class=991db9af` — class 991db9af = fnv1a32("r2.sensor") (composer-confirmed),
  i.e. the SENSOR's beacon decoded end-to-end (encode on sensor + decode on bridge) on real RF. RBID, no
  hive_id, no seq — §8.1.2 privacy conformance.
- **PASS — LoRa data-plane + XIAO+Wio-SX1262 HAL.** Triplet mutually receive (c01cee4d/296f308b/bd72902e,
  masked=false) + hear the co-located DFR1195 mesh (480e900e/2cab5f69). XIAO+Wio first-light, SPI pin-map
  (SCK7/MISO8/MOSI9/NSS41/RST42/BUSY40/DIO1=39), and DIO2-as-RF-switch all WORKING (core's r2-sx1262
  configure() keys DIO2 unconditionally).
- **PASS — §1A.2 config-validation (incidental).** Sensor boot fired the SCF-TTL warning
  (`scf_ttl_s=120 NOT >> wake_interval_s=300`) — the config check works on metal (composer emitted both as
  firmware-defaults; benign on bench, duty advertised-only).
- **FOLLOW-UP — OTA confirmed-boot networked round-trip.** Firmware path IMPLEMENTED (signed
  verify-before-write receiver + confirmed-boot + anti-rollback floor) + otadata slot-switch metal-validated
  (`OTA slot=ota_0 ... test-b PASS`). Trust model mutually confirmed: receiver accepts composer's §2.4
  TG_SK-direct (issuer_pk==tg_pk=d9fb84d4, empty authority certs, floor 0, seq=1>0); mint-ota would NOT
  (no role-0x05 cert). composer signer ready (`tg ota-sign`, f7cd3fe). BLOCKED ONLY by bench network
  topology: triplet on the DFR-D1-served isolated soft-AP (192.168.4.x), Alfred on the LAN (192.168.1.33),
  no route + no push host on the soft-AP. PATH B (sensor on a LAN-reachable AP: change FIELDLAB_SSID +
  reflash) ready on Roy's go + LAN WiFi creds. Wire contract handed to composer (OST/ODT/OCM UDP :21043, NOT
  the 0x03 blob).

## Metal-caught bug (fixed `0f87bd3`)
`read_persona` buffer was 256B but composer's persona is 336B → truncation → spurious §3.5 UNPROVISIONED
(persona=false, hive=mac_low3). Bumped to 512B; re-flashed; all 3 then persona=true. No compile check finds this.

## Safety note (the near-miss discipline paid off)
After USB re-enum the `ttyACMn` numbers REMAPPED (board-info on /dev/ttyACM1 read a different eFuse MAC than
its old by-id) AND 5 DFR1195 appeared on Alfred (ttyACM6-10). Flashing by ttyACMn would have hit a wrong
board. Used stable `/dev/serial/by-id/usb-Espressif..._<MAC>-if00` paths + board-info-verified each eFuse MAC.

## composer follow-up flagged
composer's Deploy-sentant still emits UNSIGNED CMD_START; the signed primitive (build_preamble/tg ota-sign)
isn't wired into Deploy + no one-shot field push CLI — to close before the field OTA path goes live.
