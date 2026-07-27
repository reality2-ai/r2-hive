# RESUME — r2-hive

Updated 2026-07-27. `main` clean + pushed (ahead=0). Compacted to one current snapshot; full v4→v8.7 cycle history in
`RESUME-archive.md`. Firmware lives in r2-core branch `dfr1195-fw-blerole-coex` (hive designs/builds/attests; never edits core).

---

## g23 VALUE-SCRUB (2026-07-27, Roy: keep gates public, scrub values, FORWARD-ONLY) — LANDED

Publish-HALT LIFTED (bookkeeping public again). Scrubbed VALUES, kept REASONING. Replacements = self-announcing tokens (`<scrubbed-ip>`/`<scrubbed-ssid>`/`<scrubbed-psk>`/`<scrubbed-subnet>`/`<scrubbed-tg-uuid>`/`<scrubbed-tg>`), NOT plausible reserved-range numbers. SCRUBBED (counts): SSID ×10, PSK ×1, RFC1918/CGNAT IPs ×77 (18 distinct), subnet-`.x` ×6, mesh-VPN name (Tailscale→mesh-VPN) ×5, real TG UUIDs ×22 + 8-hex UUID prefixes ×9. KEPT+LIST (ambiguous; over-scrub is silent loss): fnv on-air hashes (tg_hash 04bc57e7 / board hive_ids / decimal tg_id) — transmitted CLEARTEXT in frame `target_group`/route-stack, so scrubbing docs is theatre; `<pi-host>` hostname (dev-box-name class); truncated TG pubkey fingerprint. Corrected 1 over-scrub: WS_GUID (gateway.js protocol constant, reverted). KEPT (Roy's explicit): reasoning/decisions/commit-ids/repo-names/username/<build-host>-<rig-host>. **⚠ SCRUB ≠ UN-PUBLISH: the credential lines were on a public ref (forward-only, history kept) → treat SSID/PSK as DISCLOSED; ROTATION is the real fix and is Roy's action, not closed by this scrub landing green.** [[preserve-before-scrub]].

**ROUND 2 — host de-name + 3 supervisor rulings (2026-07-27):** (1) **On-air fnv hashes KEEP** (tg_hash/hive_ids broadcast cleartext in frame headers — scrubbing docs is theatre). ★DELTA to re-examine ON DEPLOYMENT (not re-argue): on-air = public to anyone IN RADIO RANGE; a document is public with NO range requirement — immaterial for a bench fleet, MATERIAL for deployed units someone could correlate without visiting. (2) **Host names SCRUBBED** (I mis-read the hygiene-gate allowlist as "g23-keep" — different concerns): the SBC build host → `<build-host>` (~229), the rig host → `<rig-host>` (~81), **the Pi host → `<pi-host>` ×2 (HIGHER class — its name carried the principal's = a personal identifier)**; the config.rs test fixture host-name → `testhost` (value+assert together, stays green); carrier-bridge py docstrings de-named. The gate `ci/public-hygiene.sh` FLIPPED advisory→**hardfail-forbid** (forbid-list = the two bench hosts + the Pi host, self-excluded from its own scan; neg-control confirmed it now FAILS on a host name). ⚠ NB describing this in any tracked file must NOT quote the literal host names — the forbid-gate greps this file too ([[hygiene-gate-greps-all-tracked-files]]); use role tokens only. (3) **Truncated pubkey fingerprint KEEP** (public by construction). Over-scrub catch (WS_GUID) vindicated keep-and-list: a matcher right about the pattern can be wrong about the subject.
**g23 CLOSED (Roy 2026-07-27): bench wifi is rotatable + the network short-lived ⇒ the disclosure has a natural end. STOP tracking rotation as open** (carrying a closed item forward = the stale-record defect). Residual truth kept: a forward-only scrub does not un-publish — bounded HERE by the network being temporary, NOT by the scrub working (the next disclosure may not have a natural end).

**PATTERN-SET DIFF (hive gate vs composer's 3 gates, 2026-07-27):** ★my one-off g23 enumeration ≠ my GATE — `public-hygiene.sh` scans NO IPs (no RFC1918, no CGNAT), no UUIDs, no API-keys (composer's device-id/key gates do). Composer's documented KNOWN-LIMIT (partial-MAC/mac_low3 tails) is what MY gate covers, along with terms/macron/gateway-product/host-names. **DONE (a0589e7): added IP(RFC1918+CGNAT)/dashed-UUID/API-key SHAPE classes to public-hygiene.sh** — KATs 56/56, production gg-path positive-controlled (planted IP/UUID/key all FAIL; WS-mesh GUID + nil UUID token-allowlisted; public/loopback/RFC5737/4-part-semver + commit-SHA + 8-hex-hive-id all pass). Shape patterns are safe inline (name no value); pre-push now enforces them. **VALUE-CLASS MIGRATION DONE (2026-07-27): terms/gateway/host lists MOVED OUT of public-hygiene.sh into an OUT-OF-REPO gitignored denylist** (`$R2_HYGIENE_DENYLIST` or `~/.config/r2-fleet/hygiene-denylist`; TSV class/value/replacement; schema-version 1). Fail-closed `load_denylist()` (missing/unreadable/no-schema-version/zero-records/git-tracked → refuse, with a WHERE+WHO error message); loader KATs prove it (selftest 61/61). Gate file now carries ZERO value literals (the enforcer is no longer the last public copy — [[enforcer-can-become-the-last-copy]]). Owner=hive (single-source canonical); composer verifies via content-sha `2ecf18d9…`. Per-host one-time placement documented in setup-hooks.sh. **★ SHAPE-SHARING SHAPE CHANGED (supervisor 2026-07-27): NOT a byte-identical module/block — SHARED SYNTHETIC VECTORS, independent impls, both must pass** (a shared .pl forces perl onto composer's shell/mjs gates; a blob-sha pins TEXT when we care WHETHER THE SAME VALUE IS CAUGHT; a PORT survives by construction if it passes). CONTRACT = `ci/shape-scan-vectors.tsv` (49 synthetic vectors, 26 flag/23 pass incl semver/40-hex-SHA/loopback/8-hex-hive-id FP-PASSES + 4-group=2-window count), **co-pinned by CONTENT-sha `79fdc80e…`** (pinned in --selftest as the drift alarm). **hive RUNS all 49 from the FILE via `run_vectors` — conformance is TRUE, not "equivalent inline set" (supervisor: run-from-file is what makes the claim true; a file-sha pin proves the file unchanged, not that inline copies equal it); selftest 111/111.** Each gate runs the full set, any language; a vector added anywhere bumps the sha + fails the others' pin until adopted. ★KEY-shape prefix TOKENIZED `<KEYPFX>` (runner expands, FAIL-SAFE by construction — a missed expansion makes the KEY positive non-key-shaped ⇒ fails loud): the fleet secret-scan bans a literal `sk-` fleet-wide (synthetic key-shape ≡ real key), so no shared secret-shape vector may store it — reported, not bypassed. Vector file EXCLUDED from each gate's own sweep. d6b68136 shape-block-blob-co-pin DEAD. **NEXT HARDENING (flagged): SALTED-HMAC denylist digest pin in the loader** — a bare content-sha of a private list is a guess-confirmation oracle; HMAC over content with a salt STORED IN THE FILE removes the oracle (salt rides the file, no new distribution). Loader today proves valid-not-canonical; this closes stale-but-valid. + composer mirrors vectors @79fdc80e + ports. Rule: SHAPE = shared VECTORS (content-sha, port-agnostic); VALUE = out-of-repo denylist (content-sha, not forked literals). **★ ERROR-vs-CLEAN AUDIT PASSED (supervisor-accepted 2026-07-27, the fleet false-green sweep): does this gate read ERROR as CLEAN? NO.** `set -euo pipefail` + `gg()` returns rc>1 on any engine error (rc0/1 = match/no-match); the macron check `exit`s on rc>1; macron pattern is a 10-codepoint git grep -P CHARACTER CLASS (not an alternation) ⇒ structurally immune to the ugrep complexity blow-up AND fail-closed anyway. Idiom scan CLEAN (no `2>/dev/null`-hides-error, no end-of-pipe `|| true`, no `if git grep;then finding;fi` in a production-fatal spot). Three controls, ALL committed to --selftest so they re-prove every run (113/113): a NEGATIVE control (confirmed against the REAL production path — planted synthetic MAC+IP+macron in a tracked file → gate FAILED exit 1 → restored via `git checkout`, NEVER cp; not a stdin-only pattern test), an ERROR control (gg broken-PCRE → rc>1 fail-closed), and the DENOMINATOR in the OK line (214 files scanned). Forced-failure, not read-the-script. [[status-recorded-as-a-constraint]] [[never-conclude-from-a-null]] **✅ HOSTED CI HYGIENE RESOLVED (fac9edf, hosted GREEN — verified via `gh run list`): was RED since `11aa3595` (runner has no out-of-repo denylist ⇒ `load_denylist` fail-closed; same script, different INPUT availability). Supervisor ruled OPTION (b): CI runs an explicit `--ci-shapes-only` mode (SHAPE/signal classes only, no denylist needed) as a green watchable backstop; VALUE-classes stay enforced PRE-PUSH via the fail-closed denylist (a host without it CANNOT push ⇒ value coverage enforced-or-refused, never dropped). Rejected (a) denylist-as-CI-secret (a new copy in a third store). The mode is explicit + declared, prints what it skips + where enforced + the denominator, and is unreachable-locally-by-accident (default still fails closed). bash-e safety: `gg()` captures rc via `|| rc=$?` (rc1 clean-no-match is success for us / failure for set -e — the CLEAN branch is invisible to match/error controls); added a clean-path-under-bash-e KAT. selftest 114/114. Owned: I'd reported "hygiene green" on LOCAL runs while hosted was red — SAY WHICH GREEN. [[local-check-vs-hosted-ci]]**

---

## ‼ READ FIRST — device + authorization state (ledger wins any conflict below)

- **★ NAMING (Roy 2026-07-27) — TWO AXES, don't collapse ([[carrier-role-vs-device-identity]]):** DEVICE IDENTITY = per-device, BUS-BOUND (`X1`/`D4`/`D5` = efuse MAC) — the ONLY thing to key a rig-map/persona/device-record on. ROLE = per-ensemble, NOT unique (`radar-sensor-xiao`=X1 real-hw; `complex-hive-xiao`=Android TG peer, OFF bus; `sensor-simulator-dfr1195`=D4/D5, **SIMULATED sine — fabricates readings**) — a carrier+ensemble descriptor, NEVER a key (collides when 2 share a role). "THE XIAO" RETIRED. Don't rename records by inference — mark AMBIGUOUS; resolve only where a MAC/reading/session determines it. No asserted-MAC row for an off-bus board. A SIM reading read as REAL > worse than an ambiguous name.

- **DEVICE (D5 / DFR1195 ESP32-S3):** runs **v8.7.3 at `513c949db0f9ec0eebbf7d6df3febec39561a13a`** — flashed and verified.
  The OTA-coex hang campaign is **CLOSED** (double-fault family verdict RULED AND RATIFIED).
- **★ PER-BOARD CUSTODY SNAPSHOT (2026-07-27, from RECORDS — NO live device read; digest not filename):** **D5 = `adc6cc18…cbd68`** (`d5-otarx-v873.bin`, 878864 B, v8.7.3 coex `otal2cap`; B/otafail pair `8a1ee68e`). **D4 = UNKNOWN** — no flash record in my custody; `cbd6bf67` (benchsf7) + g18 `30ffcdfd` were BUILD+ATTEST ONLY, never granted/flashed; a record-vs-metal SF mismatch (D4 ran SF12/`lora_dr=0`) is unexplained by any image I recorded flashing ⇒ NO digest asserted. **X1 = composer-custody** (not firmly mine): records show otal2cap v3 A `ae5fadb3` flashed to ota_0 during the OTA run + g18 x1-xiaobridge `cee2f004` build-only — defer to composer, no fabricated digest.
- **★ v8.7.3 COEX WiFi-STA ATTESTATION (PINNED, source-only, for a held Roy ruling 2026-07-27):** feature set (authoritative, `~/build-v873.sh`) = `bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive,otal2cap` (+`otafail` on B) — **`staota` ABSENT**. At **r2-core `513c949d`** (`git cat-file`-verified; `git show 513c949d:platforms/dfr1195/src/main.rs`): the infrastructure-join is `#[cfg(feature="staota")]` @934 (env creds→lab WiFi) + `#[cfg(feature="staota")]` DHCP @~987 ⇒ **NOT COMPILED**. A STA client IS compiled under `ble` (`wifi::new`, `wifi_task`@997, Station cfg) but targets the synthetic self-mesh SSID `r2-tn-form`@937 and its `connect_async()`@9024 fires only on `DATA_PLANE_JOIN`, which the in-source fw-doc @7313 states never fires when staota is absent. ⇒ **v8.7.3 coex compiles a STA client but NO infrastructure-join, and NEVER associates** — canon sec3.1.3 (bars joining infra WiFi) does not reach it. **NO sensor build was ever flashed to D4/D5** (recipe scaffold, no i2c sensor plugin binds the S3 DFR) ⇒ that WiFi question has no subject. [[marker-grep-cannot-see-comments]] (attested from the recipe + cfg-gates, not a `strings` count) [[positive-control-the-tree-not-just-the-tool]] (pinned ref, not the working tree).
- **★ BOARD-CLASS WiFi QUESTION CLOSED (supervisor 2026-07-27, ACK on the first PINNED attestation in the chain): NOT going to Roy — no bench board joins infrastructure WiFi, nothing to except.** OPEN ITEMS: (1) **D4 IDENTIFIED then SOURCE-ATTESTED (2026-07-27):** composer's passive banner read = `build_id coex.iter9.0723` — coex-FAMILY but EARLIER than the v8.7.3 (`coex.v873.0725`) I first attested; supervisor had over-generalised the v873 attestation to D4 (the "bench does X without naming the image" error) and corrected it. I attested the ACTUAL image, **PINNED: r2-core `70960dbc`** (iter-9 #d013; recipes `~/build-iter9pair.sh`/`~/build-d5-iter9.sh` carry `BID=coex.iter9.0723`→`70960dbc`). Feature set `bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive` — **no `staota`, no `otal2cap`**. Infra-join `#[cfg(staota)]`@621/675 NOT compiled; compiled STA targets synthetic self-mesh `r2-tn-form`@624; **STA NEVER associates — `DATA_PLANE_JOIN.signal()` has ZERO call sites @70960dbc** (only `.wait()`@8055; comments @761/5395/8052), `connect_async` unreachable; M8c-join-suppress `56d39498` in ancestry. **The `.40` (composer-read STA IP = the ESP-SoftAP-subnet `.40` host) = a STATIC self-assign** (`ipv4_static`@667, `<softap-subnet>.<mac_low3&0xFF>`@664, gw `.1`) — assigned unconditionally, NOT a DHCP lease, association-independent (subnet = the espressif SoftAP RFC1918 default; literal tokenized to satisfy the IP-shape gate). ⇒ HOSTILE reading (iter9 joins infra) **refuted at source**; canon sec3.1.3 does not reach D4 either. Still **NO flashed-image digest** (banner build_id ≠ a digest; no flash record) — do NOT infer one. ★ RESIDUAL RE-CHARACTERISED (source attest @70960dbc): the "D4=SF12" reading is likely a **METRIC MISREAD** (same class as the `.40`). **`lora_dr` is NOT the spreading factor** — it prints `LORA_TX_DROPPED` (main.rs:1074/215) = `lora.tx_overflow()` (:5947), a rate-based SHED counter; the in-source comment (:1065-66) says `lora_dr>0` is the expected steady state AT SF12, so `lora_dr=0` is consistent with **SF7** (no shed) or idle — the opposite of the "SF12" reading. **`benchsf7` DOES gate SF: `cfg.spreading_factor=7`@5816** (else as923 SF12 default @5798); iter9 has benchsf7 ⇒ SF7. No runtime SF override in source (boot-static). Banner "LORA-ROUTE up (SF{sf})"@5824 prints the SAME `cfg.spreading_factor` variable ⇒ banner≠code EXCLUDED for the SF value. **Real SF ground truth = D4's LORA-ROUTE boot-banner SF{n}** (composer's console read, same instrument as the build_id) — asked. If SF7 ⇒ anomaly dissolves; if SF12 ⇒ look at the 5606 `as923_nz` VERBATIM second cfg path (deliberately SF12 to match the RAK, NOT benchsf7-gated), a separate path not a benchsf7 failure. [[marker-grep-cannot-see-comments]] (2) **X1 full sha died with the overwritten grant file** — composer holds an ABBREVIATED digest only; grants now archived before overwrite (won't recur).
- **g18 (NEW, 2026-07-26):** D4 + X1 fault-forensics **rebuild at `8530327309b82fdc0707063b72a8c00c0166a9c6`** — BUILT +
  attested, **both variants ELIGIBLE=YES**. See the g18 build record below.
- **AUTHORIZATION (authoritative, supervisor 2026-07-27): the ONLY live grant is READ-ONLY** — DFR partition-table read
  @`0x8000` len `0x1000`, target **D5**, blocked on Roy putting D5 in download mode. **ONE GRANT AT A TIME.** **There is NO
  `cbd6bf67` flash grant** (no D4 flash — brick-history board, needs Roy's explicit go regardless); any draft naming
  `cbd6bf67` with a HOST target is VOID (a grant target MUST be an opaque DEVICE handle, never a host). The RAK relay image
  `858bc638` is STAGED for Roy STEP3 (never flashed). No flash taken. See the bench-mesh regression section below. Ignore
  any stale "on metal / grant live / flash-auth being written" language — a grant exists only as a supervisor
  flash-authorization file, and the only one is the D5 read-only read above.
- **HIVE POSTURE:** g18 build+attest legs **COMPLETE**. Record-and-report only. Re-engage ONLY on an explicit
  supervisor order; do not poll, do not assign peer legs ([[fleet-posture-authority]]).
- **#d003 RAK FREEZE STANDS:** no RAK stage or release without BOTH an explicit sha order AND Roy lifting the freeze.
- **#d005 BUILD GATE BINDS:** no flashable-artifact build without (1) inbox drained (check supersedes/retractions),
  (2) an explicit CURRENT supervisor order naming the pinned sha, (3) a clean detached checkout of that sha with tree
  state verified, (4) the build-target guard (below). A DONE status is not proof of a build order.
- **★ NEW — GRANT ELIGIBILITY = TWO LEGS, tested IN ORDER** (since hive stood down; bears on any future build ordered):
  1. **HANG_CAP capture instrument PRESENT** in the artifact, THEN
  2. **`__user_exception` call-free within its TRUE extent** measured from `nm -S` (the symbol's real size), NOT a fixed
     objdump window.
  **The ~40 staged XIAO + sensor artifacts FAIL leg 1 entirely** — no capture instrument at all, plus a flash-resident
  windowed call from the handler = total fault blindness + the v8.6 re-fault loop. **They are DO-NOT-FLASH.**

---

## RAK COMPACT-RELAY + BENCH-MESH (2026-07-27) — #d001 CONFIRMED (measured); SameCarrier override orphaned by a tree re-export, re-applied @70f442b9; bench mesh SF-split open

**★ SETTLED (supervisor 2026-07-27, rename ratified): #d001 is a RATIFIED PASS, now CONFIRMED BY MEASUREMENT for the specific image `e5c7073e` (see the rebuild-compare below) — citable for THAT image only, nothing else.** The cause is no longer open: the `set_relay_egress(SameCarrier)` override was **orphaned by a tree re-export** (the current rak4630-fw layout dropped the old-monorepo tree; the override was re-applied as a NEW commit `70f442b9`), so a build from the pre-fix `7011934e` did not relay. The current bench mesh is separately **not forming — cause UNDIAGNOSED (2026-07-27)**: the earlier "SF split (D4 SF12 vs RAK SF7)" diagnosis rested entirely on reading `lora_dr=0` as SF12, and `lora_dr` = `LORA_TX_DROPPED` (a shed counter), NOT the spreading factor — so there was never evidence D4 runs SF12, and the mesh failure needs a fresh explanation. The #d001 relay cause IS measured + named; the mesh-forming failure is re-opened.

**★ WHY THIS SECTION EXISTS + A DENOMINATOR WARNING: this campaign ran in THIS session, was LOST TO COMPACTION, and I nearly reconstructed a false-empty from `git log` — which returned 100% hygiene commits, ZERO RAK/D4 trace. The artifacts are scp-only + gitignored, so THE REPO IS A FALSE DENOMINATOR for artifact-producing work: `git log` gives a confident, complete-looking, WRONG answer. Do NOT reconstruct this campaign's history from git; the record is HERE + on the build/flash host + the transcript.** Supervisor confirmed no second hive writer (hive-codex idle since 2026-07-21); the driving context was my own, pre-compaction. Flagging-not-inventing was correct.

- **★ A RAK IMAGE WAS BUILT (not "no RAK image") — supervisor ACCEPTED the #d003 reading (2026-07-27): reproducing the #d001-passing REFERENCE image is producing the reference, not building out beyond it; staged-never-flashed, serial-DFU-only, no nRF feature work. NO FURTHER RAK BUILDS without a fresh order naming a pinned sha.** hive rebuilt the RAK4630 compact-relay hex `858bc638…bb2e` (ELF `d1aeefdc…3958`) from core `rak4630-fw` HEAD `70f442b9`. Fix baked = `set_relay_egress(RelayEgress::SameCarrier)` main.rs:844 (the relay leg was masked by the `CrossCarrier` default → `relay_on==0` → `route_len` stuck at 1). Superseded the STALE `8215b52a` (decode-fix only; a filename-reuse collision on the flash host was caught + resolved — [[attest-baked-bytes-and-sha-pin-handoff]]). Features `dev,blespike,uf2,baked_persona,benchsf7` + baked persona `8d5d099f`. Composer packaged the nRF UF2 .zip (no partition table, @0x26000): `image_digest e5c7073e` (118624 B) reproduced 3 ways = packager roundtrip PROVEN; `flash_package_digest d51b5b86`. **STAGED, never flashed.**
- **★ #d001 SETTLED BY REBUILD-AND-COMPARE (2026-07-27) — #d001 STANDS, CONFIRMED BY MEASUREMENT.** Composer held the 07-22 flash record (image_digest `e5c7073e…49d56`, raw image @0x26000, 118624 B, flashed 2026-07-21 serial-DFU, relay-proven 07-22, RAK not reflashed between). Supervisor ordered a determinism-first rebuild-and-compare (explicit #d005 order, two pinned shas, FLASH NOTHING). Built on `<build-host>` (same env that derived `e5c7073e`):
  - **DETERMINISM (checked first): `70f442b9` built TWICE, clean detached, `rm -rf target` each → BYTE-IDENTICAL** (118624 B, sha256 `e5c7073ec5e551f691a677bc6825c583c83f6d4c8e5dfbb0e96e990d74e49d56`) ⇒ build deterministic, comparison valid.
  - **`70f442b9` (WITH `set_relay_egress(SameCarrier)`) = `e5c7073e…49d56`, 118624 B = EXACT match to the FLASHED image.** ⇒ the 07-22 proving image CARRIED the override → it relayed → **#d001's warrant is checkable after all and CONFIRMED.**
  - **`7011934e` (pre-fix, grep-confirmed 0 `set_relay_egress`) = `83ff6f62…`, 118528 B — does NOT match**; it reproduces composer's STALE `8215b52a` decode-only image (composer's own staging digest `83ff6f62`/118528 B) → the pre-fix lineage was NEVER flashed.
  - Composer's grant-artifact TAG `70f442b9` treated as a LABEL, not evidence — the DIGEST match is the evidence ([[safety-claims-name-what-is-enforced]]). Built BOTH shas + the determinism pair (4 builds); did not stop at agreement.
  - **⇒ The current-branch masking (`7011934e` dropped the override) is a re-vendor LINEAGE REGRESSION — my earlier lean, now MEASURED.** git corroboration: `CrossCarrier` mechanism = `2f7d9866` (07-17); the override-call exists in only 2 commits across all refs (`2f7d9866` NOT an ancestor of current; `70f442b9` parent=`7011934e`). [[positive-control-the-tree-not-just-the-tool]] (lineage beats date; DIGEST beats both), [[env-baked-const-needs-full-clean]] (rm -rf target so the env-baked persona re-bakes), [[tn-base-is-mixed-sha-assembly]].
  - FLASH-NOTHING honored: `rust-objcopy -O binary` digests only; secret-bearing raw bins (baked persona `8d5d099f`) DELETED; no `.zip`/hex produced; rak4630-fw-wt restored to `rak4630-fw`@`70f442b9` clean, ahead=0; no grant sought. **Re-proof of the CURRENT-lineage mesh (unify SF → route_len=1 → route_len=2) is still the fix for the regression**, but #d001's historical warrant no longer needs it — measurement closed that.
- **★ RE-VENDOR DIVERGENCE AUDIT (supervisor: "one lost commit is rarely alone"; read+diff only, no build):** MECHANISM = **tree RE-EXPORT onto a new layout** (not a cherry-pick set / squash) — `2f7d9866` carries the old-monorepo tree (`tools/r2-relay`/`r2-provision`/`r2-sensor` present), current `70f442b9` does not; merge-base `5100933d` 07-05; `70f442b9` re-adds the override as a NEW commit. **The mechanism predicts the loss class: a re-export carries vendored-CRATE behavior but ORPHANS hand-edits to the platform `main.rs`** — exactly where `set_relay_egress` lived. The 342-file/43k-line raw `mb..2f7d9866` diff is a FALSE DENOMINATOR (parallel divergence on two layouts, not a dropped list); scoped to the firmware surface the behavioral delta is small: **(1) `set_relay_egress(SameCarrier)`** — the orphaned fix, RE-ADDED @`70f442b9`; **(2) `set_egress_enabled_mask(PHY_LORA)`** — present on `2f7d9866`, absent on current, but NOT a break: current defaults `egress_enabled=PHY_ALL` (r2-dataplane lib.rs:299, includes LoRa) + uses `SameCarrier` ("all carriers except arrival"), so the flashed image relayed; the `2f7d9866` call was a spike-specific LoRa-only NARROWING that current's fuller model supersedes — **flagged to core to confirm PHY_ALL egress is intended for the repeater** (design Q, not a dropped fix); **(3)** LED set_high/low + import list = cosmetic. No other behavioral drop; current is the FULLER line (~840 vs ~200-line main.rs, adds ota.rs), not a subset. [[tn-base-is-mixed-sha-assembly]] [[shared-checkout-path-dep-coupling]]
- **D4 (DFR1195 ESP32-S3) benchsf7 image `cbd6bf67…653c`.** ARTIFACT fact: `cbd6bf67 != a23c21ea` (non-benchsf7) proves `benchsf7` is not a no-op **IN THE IMAGE** — says NOTHING about what the board executes. **★ WITHDRAWN: "the flashed D4 runs SF12 (`lora_dr=0`), benchsf7 didn't take" — this was a MISREAD.** `lora_dr` = `LORA_TX_DROPPED` (main.rs:1074, a shed counter), NOT the spreading factor; `lora_dr=0` is consistent with SF7 (no shed). There is NO evidence D4 runs SF12, and `benchsf7` DOES set SF7 at source (5816, no runtime override). D4's actual SF is disclosed ONLY in the boot banner (@5824), not in any periodic telemetry — so positive SF7 confirmation waits for the next legitimate reflash (reset barred). Persona = the #d001 shared bench TG (`tg_hash 0x6E31DEC6` / `wire_id 0xCC788B17` / `tg_id 730c29e7`); `baked_persona` avoids the old 0x12000 brick path.
- **RULINGS (hive, supervisor-accepted):** (1) **ALL-SF7** for the bench mesh (one SF) — grounded in AIRTIME (29B@SF12 ~1647ms ToA ≈16× too slow for the 1/s apiary; SF7 ~67ms meets it; R2-LORA §5 v0.4.19); all-SF12 regresses the req, PHY-only so §5.1 vector untouched. **★ This POLICY stands on the airtime physics ALONE — it does NOT depend on D4's current SF** (the "D4=SF12" claim was the lora_dr misread, withdrawn above; ALL-SF7 remains the right target regardless). (2) **`0x6E31DEC6` (tg_id 730c29e7) CANONICAL, do NOT re-mint** — confirmed by authoritative `parse_persona(8d5d099f)`, NOT a rodata scan (tg_hash is derived-not-stored ⇒ scan structurally blind); stale composer criteria `0x3eb54833/0xd256dc00` named a different provisioning, superseded. (3) **`route_len=2` proves RELAY (the #d001 claim), NOT persona** — same-TG members relay regardless; persona-correctness rests on d001-ratification + the parser. [[shared-radio-config-is-a-base-not-a-guarantee]] [[sf12-airtime-cant-carry-sensor-stream]]
- **PROVEN:** packager roundtrip (3-way digest); benchsf7 differential. **NOT PROVEN / OPEN — the mesh is NOT forming, so NO on-air relay proof exists:** composer 100s dual capture — D4 emits 4 APIARY `64cedb11` (seq 567–570, 29B compact, ENQUEUED→LoRa) but XIAO forwards ZERO (count=0, own beacon only) AND hears NOTHING direct from D4; DFRs `leaderless-0.4` role=STA, nbrs~0, synced=false. **NO `route_len` anywhere — not even the direct D4→XIAO `route_len=1`.** **★ ROOT CAUSE UNDIAGNOSED (was "SF split D4 SF12 vs RAK SF7" — now UNSUPPORTED, rested on the `lora_dr`-as-SF misread).** Needs a fresh explanation on the next live look. Direct D4→XIAO `route_len=1` still the first milestone; RAK relay `route_len=2` untestable until the mesh forms. [[never-conclude-from-a-null]] [[sequential-flashing-phase-aligns-boards]]
- **★ FLASH STATE — AUTHORITATIVE (supervisor 2026-07-27): there is NO `cbd6bf67` grant.** The only live grant is READ-ONLY (DFR partition-table read @`0x8000` len `0x1000`, target **D5**), blocked on Roy putting D5 in download mode. **ONE GRANT AT A TIME.** No D4 flash (brick-history board; needs Roy's explicit go regardless). Any draft flash-auth naming `cbd6bf67` with a HOST target is **VOID** — a grant target MUST be an opaque DEVICE handle resolved locally, never a host name (my earlier RESUME wrongly quoted a host-targeted draft; corrected). **OWED:** D4 reflash on Roy's go; RAK `-9dBm` power AFTER the SF is unified, not before.
- **DENOMINATOR WARNING (kept): this campaign's artifacts are scp-only + gitignored → `git log` is a FALSE denominator (returns a confident, complete-looking, WRONG empty). Do NOT reconstruct from git; record is HERE + build/flash host + transcript.** Compaction lost my own driving context (no second hive writer — hive-codex idle since 07-21); flagging-not-inventing was correct. Summary sent to supervisor for DECISIONS.md (claude-fleet had zero record — its context compacted too). [[never-conclude-from-a-null]] [[status-recorded-as-a-constraint]]

---

## ENSEMBLE PHASE — trace-first (2026-07-27, source-only, NO build; Roy directive via supervisor)

Supervisor's three mechanism questions, traced at `dfr1195-fw-wt` **b25a21eb** (comment-delta over live 4b4a71e5), caller-not-mention. Sent to supervisor. **Verdict: all-as-ensembles is a PLATFORM feature FIRST, not five scores on an existing seam.**

- **Q1 score-loading = MUST-BE-BUILT on MCU.** Real declarative-score loader EXISTS in source — `crates/r2-ensemble` (`loaded.rs` owns parsed `EnsembleScore`, `factory.rs` "walks each sentant entry in the score", `registry.rs`, `serde_yaml 0.9`) — but **std-only** (tokio+parking_lot+r2-engine[std]+r2-dispatch[std]), **not a dep of any platform, unlinkable no_std/xtensa**. In the binary: NO score concept; on-device r2-engine = **compile-time EventBus** (main.rs:8377); parts chosen at COMPILE TIME by cargo features. Boot-read set = persona@0x12000/RPF1@0x17000/board@0x13000 (main.rs:3475), no score partition. Neg-control: zero yaml/score deserialize in any MCU-linkable crate.
- **Q2 registration = split.** PRIMITIVE exists+reachable: `r2-engine EventBus::register_plugin` (bus.rs:106)/`register_sentant` (:94), CALLED under `fakesensor` (main.rs:7900–7906) and empty under `otaengine` (:8032) — but register-INTO-ONE-BUS. R2-ENSEMBLE 2.1.2 hive-SHARED singleton (N ensembles→1 plugin, R2-WEB) = **MUST-BE-BUILT** (only the std-only r2-ensemble registry/supervision does it). BOTH **absent from the flashed OTA image** (otal2cap,lora,xiao,benchsf7 reach neither otaengine nor fakesensor — feature closure verified).
- **Q3 NVS role-profile read = EXISTS + reachable EVERY boot.** `read_role_profile()` decodes flash @0x17000 (ROLE_PROFILE_OFFSET, magic RPF1 0x52504631) → Role + duty/ble_role/keepalive_period_ms/scf_cap/scf_ttl_s/reach_conf/silence_s; CALLED by `resolve_role_profile(my_hive)` (main.rs:771) at boot (NVS wins else compile-derive). Cadence knobs ARE role-profile fields — the R2-RUNTIME 210 boot-activation seam + cadence home (field 900s/bench 5–10s). The ONE seam already present.

Memory: [[ensemble-mechanisms-trace]].

### PERSONA-READ + PARTITION + BUDGET (2026-07-27, source/artifact-only, assembled for Roy — NOT acting)

**★ CORRECTED 2026-07-27 (supervisor): I traced the PLATFORM csv, not the FLASHED one. X1 is a XIAO — composer flashed the board-catalogue template, device enumeration is ground truth.**
**PERSONA READ = RAW ABSOLUTE 0x12000, brick-safe TODAY but UNPROTECTED.** Flashed set has no `baked_persona` ⇒ `read_persona()` (main.rs:3349) = `esp_storage::FlashStorage.read(PERSONA_OFFSET=0x12000)`, raw-absolute, NEVER the partition table; called UNCONDITIONALLY at main.rs:661. Mechanism corroborates console `@0x12000`. Composer's "NVS 0x9000" wrong on mechanism.
**FLASHED TABLE (X1) = r2-composer/catalogue/boards/esp32-s3-xiao-wio-sx1262/templates/partitions.csv — matches the device boot enumeration EXACTLY:** nvs 0x9000/0x6000 · otadata 0xF000/0x2000 · **phy_init phy 0x11000/0x1000** · ota_0 0x20000/0x30_0000 · ota_1 0x32_0000/0x30_0000 · storage fat 0x62_0000/0x1E_0000. (The DFR table I first traced — r2cfg@0x11000, ota_1@0x20_0000, 0x1E_0000 slots — was NEVER flashed to X1.)
**BRICK ANSWER:** 0x12000 sits in an **UNCLAIMED GAP** (phy_init ends 0x12000, ota_0@0x20000; nothing claims 0x12000..0x20000). Brick-safe TODAY holds (app@0x20000, 0x12000 not app code) — but it is **UNPROTECTED**: no partition declares it, so a future table edit / erase-region / compacting tool can clobber it silently. "Brick-safe today AND unprotected" ≠ "brick-safe". composer's `phy_init@0x11000` was RIGHT.
**BUDGET (★ELF ≠ flashed image [[elf-is-not-the-flashed-image]] — method stands, DENOMINATOR moved via the FILE):** slot = 0x32_0000−0x20000 = 0x30_0000 = **3.000 MiB** (composer's 0x32_0000 RIGHT; my 0x20_0000 was the DFR table = WRONG). flashed `.bin` = 857,600 B (0.818 MiB, stable). **HEADROOM = 2.182 MiB, image = 27% of slot.** fakesensor (engine+apiary) = 852 KiB text+data, ~15 KiB over bare OTA. no_std parser: "not worth it" HOLDS (contract_only scores = zero runtime benefit), but "could NOT fit" is now an **OPEN measurement** at 2.18 MiB spare, not settled-no.
**⚠ LATENT HAZARD (core+composer to reconcile, not hive):** platforms/dfr1195/.cargo/config.toml runner passes `--partition-table partitions.csv` = the DFR r2cfg table. A XIAO flashed via THAT runner installs DFR geometry (1.875 MiB slots, ota_1@0x20_0000) — DIFFERENT from X1's current xiao-wio (3 MiB). One platform table, two boards; a re-flash via the platform runner silently changes X1's geometry.

**PERSONA SYNC PULL-VERIFY (2026-07-27, Roy order — report DIVERGENCE; read-only):** flash-board.sh (r2-composer/orchestrator/bench:70-71,97) flashes the COMPOSER CATALOGUE template BY CARRIER (`DFR`→esp32-s3-dfr1195 = **r2cfg@0x11000**; `XIAO`→xiao-wio = phy_init@0x11000 + **gap**), NOT `r2-hive/docs/dfr1195-partitions.csv` (stale doc, deprecated). **★ NEITHER LAYOUT IS CANON-CONFORMANT (R2-KEYSTORE §9.12.4a @ eb72f04 / v0.55, verified at sha):** r2cfg is CONFIG-lifecycle and persona(0x12000)+role(0x17000)+**anti-rollback floor(0x18000)**+OTA-pending(0x1A000) ALL sit inside r2cfg(0x11000..0x1FFFF), so a legitimate config-erase clobbers the sealed identity + downgrade defence = §9.12.4a violation, MORE dangerous than the gap (a DELIBERATE, legitimate erase, not an accidental write). **§9.12.4a is now a MUST generalised off partitions: "naming a thing for what it is stored WITH rather than what it IS grants every operation defined on the neighbours — the name is the AUTHORISATION SURFACE" (any shared namespace/key-prefix/config-blob/scope, not just tables).** **STATE IS TIME-SPLIT, not carrier-split (v0.55 cut the board mapping from canon — canon keeps the RULE, the fleet ledger/this RESUME keeps the STATE; my per-carrier detail is CORRECT and belongs here, do NOT delete to match canon):** the flasher hard-coded a no-persona-region table, composer fixed it mid-session, so a DFR flashed BEFORE carries the gap, AFTER carries r2cfg — which a deployed board has is unknowable from any repo, only on-device (no device on bus; only X1 confirmed). Nameable in canon (table-independent): `read_persona` raw-absolute 0x12000 — a CONSTANT can't resolve through a descriptor. Only pt7's DEDICATED persona region is conformant. So: (Q1) the hive-doc file-claim CONFIRMED (no r2cfg/no persona region) BUT its premise-as-flashed-table REFUTED. (Fleet claim) "every DFR persona in an unclaimed gap" = **REFUTED for DFR** (flash-board + platform-runner both give DFR r2cfg-declared), **CONFIRMED for XIAO** (X1 device-enum). Device-confirmed only X1; DFR = 3 sources agree r2cfg but no DFR device-read (composer's). NONE of the 3 tables matches canon pt7 (phy_init@0x11000 + DEDICATED persona region @0x12000 not-r2cfg — new to all). (Q2) **NO hive build bakes a persona address** beyond core's `const PERSONA_OFFSET=0x12000` (main.rs:3294): build.rs bakes only the BLOB (rodata, no address), no linker/memory.x/header/script. Canon pt2/pt8 divergence (core-owned): read_persona does a raw const-offset read, not descriptor-resolved — const must go. Fixes: CORE (const + descriptor resolution), COMPOSER (tables + dedicated persona region + DFR device-read), HIVE (DEPRECATED the stale doc — done). **★ POST-FIX CAVEAT (supervisor, now standard):** the three agreeing sources are HOURS OLD — composer landed the DFR catalogue + repointed flash-board.sh THIS session, so they say nothing about a DFR flashed last week; **only X1 is device-confirmed**, a DFR on-device read is composer's to close. **DEPRECATION DONE (supervisor: "do it, don't just annotate"):** `docs/dfr1195-partitions.csv` + `-8mb.csv` → `docs/archive/` (rows stripped, redirect stub to the composer catalogue); fixed the AGENTS.md MUST (was `--partition-table docs/dfr1195-partitions.csv` — the wrong instruction that drove the misread) + docs/r2-per-carrier-builds.md to name the composer catalogue as source of truth.
**0x18000 NEAR-MISS on record (Roy/supervisor ask, confirmed from git):** const `HUMAN_LABEL_OFFSET` was **0x18000** (sha 712fc34), moved to 0x1B000 at **sha a501bff0** (2026-06-30, r2-core/dfr1195/main.rs). 0x18000 = OTA anti-rollback floor (security_version, survives reflash); a label WRITE there would reset the floor on re-provision = downgrade-bypass. **FOUND BY REVIEW, NOT a fired failure** (commit: "found by cross-reading ota_recv_signed"); never clobbered a real board. But two UNDECLARED raw offsets collided in the gap on the most security-critical sector, caught only by a human cross-read — the concrete precedent that the unclaimed-gap hazard is real, and the strongest single argument for declaring the region (a declared region / overlap-rejecting flasher makes it impossible-by-construction). Same root as the fleet-wide persona-in-gap finding.

**CORE ownership SETTLED (core@ee500ae2):** NVS region-scoping TYPE = CORE-owned crate `crates/r2-region` (shared behaviour → core; hive never owns firmware source); offset MAP stays in platforms/dfr1195. Core builds Region + region-scoped typed key (cross-region unrepresentable, E_REG_CONFLICT + neg-KAT); platform path-deps + wraps read/write in the typed API. ★Canon home = **R2-DEF §7.4a:865** (`memories` singleton: "region a caller cannot name it cannot address, enforced by NOT BEING EXPRESSIBLE"); pattern = R2-RUNTIME:1140 SealedLockfile. My earlier "R2-KEYSTORE 184" cite was a RELAYED-from-core cite I repeated unverified — R2-KEYSTORE §184 is the key-custody boundary this ENFORCES, a different concern ([[cite-canon-before-claiming-a-finding]]). §7.4a is also the canonical home of the ensemble Q2 shared-singleton registration contracts (indicator @:867). **CONVERGED core@2c8839ad** (both ground-truthed independently): requirement=R2-DEF §7.4a:865, enforced boundary=R2-KEYSTORE §184. Two forward facts: (a) R2-DEF:842's example region IS `outbound-queue` ⇒ item-4 FRAM own-origin outq is a §7.4a `memories` region — core builds it as ONE region of the item-3 type (items 3+4 share the singleton contract); (b) **indicator is hive's capability** for the registry work (R2-INDICATOR §5 reserved states) — scope noted, build still gated on supervisor's ensemble plan call.

**INDICATOR SHAPE SETTLED (supervisor, ground-truthed vs R2-INDICATOR v0.6 — PENDING ROY, re-confirm ratification before cutting code):** the plugin = pure map board-state → (envelope, period, optional hue), **byte-identical cross-board** (§2:23-26); output stage **renders only, never re-decides** what a state looks like; build **ONE OPTICAL output stage, NO RF carrier** (§3.2:56-76 "RF is not a transducer; carrier is NOT an output stage" — envelope does not abstract: brightness/hue don't map to a packet, so RF would carry STATE not ENVELOPE = category error). Sealed opaque field unit **already conforms** (§8:162 "no indicator trivially conforms"); GPIO21 stays SERVICE indicator (visible box-open), not a defect when sealed. Machine-readable sealed-unit state = **R2-DIAGNOSTICS** (§3.2(d), read-only/TG-gated/rate-bounded), a different mechanism — NOT indicator work. main.rs LED logic non-conformant to this shape = the owed next-phase refactor. Memory [[indicator-envelope-does-not-abstract]].

**REALISED-SET MANIFEST EMITTED (R2-DEF §7.10.2; supervisor pre-cleared as analysis, NOT an artifact build — d005 ungated).** Composer builds the assert in the build path; hive emits. Mapping hive owns: a score part (plugin/sentant Type impl'ing r2_engine Plugin/Sentant) is REALISED iff `xtensa-...-nm` shows a trait-impl vtable symbol carrying INVARIANT identifier substrings (Type + trait tail + `r2_engine`) — matched not predicted (survives mangling-version shift; a predicted full-mangle would silent-ABSENT). Fail-loud: denominator = declared score-set (NOT-REALISED on a declared part = nonzero exit), per-run positive controls (nm≥100, `r2_dfr1195` present, canary = same match on a known-good ELF). LEVEL-2 shown (the case recipe-checks fail): fakesensor ELF 4/4 REALISED (exit0); otav3-A same-4-declared 0/4 (exit1) — apiary.rs IS in otav3's tree but cfg-disabled. **★ FALSE-PRESENT fix (supervisor review): my first controls all defended false-ABSENT/dead-instrument, NONE the fail-open false-PRESENT. Bare substring false-presented a shorter id in a linked longer one (RAN it: `TickSource`/`Button`→REALISED on fakesensor). FIXED = LENGTH-ANCHORED match `<lenType><Type>`+`<lenTrait><Trait>` (mangling invariant, boundary without predicting the mangle) — collision closed, level-2 regression clean; each control now LABELS its defended direction.** Emitter `~/realised-manifest.sh` (<build-host>, anchored+labels), handed to composer. Memory [[realised-set-manifest-from-elf]].
**MAPPING-SOURCE GAP (composer, OPEN):** score keys parts by name + reverse-DNS class, NOT Rust Type — so realised==declared needs a declared→Type map the score lacks. My vote + composer's = **(b) register-macro marker**: emit `#[export_name="r2_part_<unique-score-key>"]`, composer greps the exact score name (no Rust-type map, no mangle dependence, realised-name==declared-name by construction). CONSTRAINT (keeps it fail-closed): marker MUST be DCE-correct — non-`#[used]`, reachable ONLY via the registration path (a `#[used]` marker survives DCE on an unlinked part = FALSE-PRESENT). Evidence the property exists: otav3-A has 0 apiary vtable syms (present-iff-registered, not #[used]). Rejected (a) in-score impl_type (drift-prone 2nd copy of type-identity, leaks impl into a declarative score, keeps mangle dependence). Vtable matcher stays as interim + independent cross-check. **FINALIZED (core+composer converged):** key = **NAME** (composer measured sentant names unique 10/10 across catalogue scores; core adds sentant-name-uniqueness to r2-def validate ⇒ NAME injective by construction). Core's correction: **there is NO register macro** — registration is the runtime method `bus.register_sentant/register_plugin` (bus.rs:119/132); the marker is a **NEW r2-engine `register_part!(bus,Ctor,key)` macro** (construction-tied, non-#[used], registration-reachable-only), a **SEPARATE unit from the r2-def schema re-vendor** (can't fold in). Core builds it + brings me the exact `r2_part_<name>` string; I wire the emitter to match exactly. My length-anchored vtable matcher stays the interim emitter + independent 2nd mechanism (not blocked). **CONVERGED (core+composer):** own unit AFTER the current 5-item re-vendor (deny_unknown/runtime_executable/local_dev/part-version/PluginRef-capability); NAME injective-by-construction once core adds `E_ENS_SENTANT_DUP` to r2-def validate (only plugin-uniqueness enforced today). Composer wires+TESTS the assert mechanism now against my demo ELFs (`~/realised-manifest.sh`, fakesensor 4/4 / otav3 0/4) — NOT shipped live on the catalogue until the marker resolves the denominator. Golden-ELF canary = composer's. Nothing owed by hive until the symbol string lands.

NEXT: await supervisor's plan call (platform-first vs otherwise) + specs' ensemble-canon finish (Q1/Q4/Q5). Build nothing. (The compiled-vs-runtime ENCODING is already ruled — Roy #69 = COMPILED, see below; do NOT carry it as an open ruling.)

---

## ★ BUILD A-baked FOR X1 — BUILT + ATTESTED (supervisor #d005 GO 2026-07-27), NO BOARD WRITE, awaiting grant

**Order (satisfied):** build image A for **X1** at pinned **`4b4a71e5`** with `baked_persona`, `DFR_PERSONA_PATH` = composer's dev persona.bin (dev TG `2e98fddb`); GO landed with blob path+sha256+intended tg_pk. **DONE + ATTESTED.**
- **Pin:** `4b4a71e5bc523ea44f85fd8efeb8cc9d2c7e9087` clean detached, tree empty; markers verified in tree (build.rs CARGO_FEATURE_BAKED_PERSONA×2; main.rs:3346 `persona_from_bytes(BAKED_PERSONA)`).
- **Blob (verified before bake):** `~/.r2-dev-trial/radar-devtg-20260727/persona-X1-2e98fddb.bin`, 336 B, sha256 `243ab04…426e` (== composer's reported); tg_id `2e98fddb…` (full UUID in the sent attestation, not recorded here per UUID-custody policy); intended tg_pk `4e2a9a30…7706`.
- **Build cmd (verbatim, from `~/dfr1195-fw-hive-build/platforms/dfr1195`):** `rm -rf target` then `env R2_BUILD_ID=otav3.A.baked.0727 DFR_PERSONA_PATH=<blob> cargo build --release --target xtensa-esp32s3-none-elf --features otal2cap,lora,xiao,benchsf7,baked_persona`. **`DFR_ROLE_PATH` UNSET by design.** Finished 1m52s, 0 errors.
- **Artifact:** ELF `~/x1-otav3-A-baked.elf` 1363108 B, **sha256 `4e595a458c1cb260c256ff34975975209c1c25d1ebfa116f8f0d6e5404b49ac8`**. NO flashable .bin produced — `rust-objcopy -O binary` = 101 MB gap-padded (high-VMA), not a valid image; the real image = `espflash save-image` = flasher/grant step, deferred ([[elf-is-not-the-flashed-image]]).
- **tg_pk IN THE BINARY (artifact evidence, not source):** full 336-byte blob baked VERBATIM, found once in ELF at file offset `0xb904`; extracted-336B sha256 == blob sha256 (exact). Section = `.rodata` (PROGBITS, vaddr `0x3c000120`) which is in **PT_LOAD segment 00** (`.flash.appdesc .rodata .rodata.wifi`) ⇒ LOADABLE, reaches the flashed image (not DWARF). tg_pk `4e2a9a30…7706` ×2 at `0xb9ba`/`0xba2e`, **both inside** the persona region `[0xb904,0xba54)` — carried by the persona, no stray copy.
- **Role omission = STATUS QUO not regression** (supervisor's reasoning, recorded so a later reader doesn't misread a derived role as new damage): unset `DFR_ROLE_PATH` ⇒ `BAKED_ROLE_PROFILE` empty ⇒ `read_role_profile` magic-fails at main.rs:3823 ⇒ derived fallback. Today's live A has NO baked_persona at all, so `role_profile_bytes` reads NVS `0x17000` = app code on the default table, magic-fails, SAME derived fallback. Omitting the role changes nothing vs today.
- **Scope:** addresses OTA signer-trust (`ctx.tg_pk == B.issuer_pk` ⇒ gate-4 reason=4 UnauthorizedSigner passes) only. The r2-tn-form concern is orthogonal + core-owned; supervisor ACCEPTED that flag so this attestation isn't overread as 3.0b-clean.
- **NO WRITE TO ANY BOARD.** Grant comes from supervisor AFTER attestation.
- **★ FLASHABLE .bin — HELD, STOPPED ON THE INSTRUMENT (2026-07-27, even AFTER Roy authorised):** hive CANNOT produce the .bin and the gate deny is CORRECT-BY-DESIGN, not a bug. `espflash save-image` (pure file-write) is hard-denied by the PreToolUse gate `claude-fleet/hooks/auto-approve.sh` (`hs_bash` keyword-matches `espflash`); no ungated fallback (esptool absent). The ONLY sanctioned pass = `_hs_authorized()`: a live `.fleet/flash-authorization` whose `artifact`+`target` the command must name. **The current grant (rewritten by supervisor 19:00) is board-write-shaped: `artifact=PENDING-HIVE-DIGEST-DO-NOT-MATCH`, `sha256=0…0`, `target=X1`, `OPERATOR: composer` — DELIBERATELY unmatchable, "fails closed by content … the supervisor fills those two fields when hive attests … THE OPERATOR NEVER FILLS THEM … MUST NOT RENAME AN ARTIFACT TO FIT THIS FIELD."** ⇒ **CIRCULARITY:** supervisor's message asks hive to PRODUCE the .bin, but the field that would let the gate pass is only filled AFTER hive attests the .bin — hive-as-producer is closed by the instrument. **Held on the file, refused the message** (the doctrine the grant file itself ratifies). Refusals, named: no field-fill (forge), no rename-to-fit, no `FLEET_FIRMWARE_GATE=off`, no wrapper/ssh-launder, no hand-rolled image (false pin) — [[espflash-trips-firmware-gate]], [[espflash-gate-bypassed-by-file-and-remote-exec]], [[attest-baked-bytes-and-sha-pin-handoff]]. **Escalated a/b/c:** (a) composer produces the .bin (named operator, has an espflash path) → hive attests; (b) supervisor writes a FILE-shaped save-image authorization (artifact=ELF basename, target=output path, not X1) → hive produces+attests; (c) Roy runs the one save-image. Verbatim cmd ready: `espflash save-image --chip esp32s3 --partition-table platforms/dfr1195/partitions.csv ~/x1-otav3-A-baked.elf ~/x1-otav3-A-baked-app.bin` (app-partition-only, explicit PT, no merge = grant act-1 shape; app offset 0x20000). On any produced .bin: report sha256+size, RE-RUN blob byte-search ON THE .bin (checked not inherited), STOP+report if padded/absent/offset-wrong.
- **★ GRANT FILE = the ACT-1/ACT-2 sequence (supervisor, digest-only facts hive must honor):** ACT 1 = cable app-image write to X1 (composer), app-only, explicit PT, NO merge/erase/PT-write, NVS preserved, one serial opener, `pgrep espflash` kill-check first, efuse+banner MATCH/NO-MATCH (never the value), `--after hard-reset` OK but standalone `espflash reset` FORBIDDEN. **Positive control between acts:** board hive_id MUST stop being MAC-fallback `00fbe520` and become TG-derived, tg_pk==`4e2a9a30…`; absence-of-error is NOT proof; if unchanged, ACT 2 VOID. ACT 2 = OTA delivery AIR-ONLY, unbaked payload FIRST (single-variable gate-4 flip), payload sha256 `70619b6d…4012` (857600 B) signer `4e2a9a30…`, pinned on payload+signer NOT the sealed stream (per-run timestamp). **Falsifier (written before push):** if the unbaked payload does NOT de-provision the board (hive_id back to `00fbe520`), something wrote flash = the no-write claim is REFUTED = report it as the finding. Same blob `243ab04…426e`/TG `2e98fddb` in BOTH images (a payload with a different persona is NOT authorised — specs refuted "identity can't change OTA": a different-persona payload sealed by the current TG passes the signer gate then installs a new identity, and the owner-change guards live in the keystore path a rodata persona never reaches).
- **★ B-LINEAGE BAKED IMAGE — BUILT + ATTESTED (supervisor queued order 2026-07-27):** second baked image from the B payload lineage, SAME blob, distinct build_id. Source **r2-core `b25a21eb`** (clean detached, tree empty, markers present); features `otal2cap,lora,xiao,benchsf7,baked_persona`; `DFR_ROLE_PATH` unset; `R2_BUILD_ID=otav3.B.baked.0727`. ELF `~/x1-otav3-B-baked.elf` 1363108 B, **sha256 `22e497cd86822b98339beca1d70d3ac1f961ccaedcbb9c089d1dc153ab593bf3`**. Persona baked verbatim at ELF off `0xb950`, extracted-336B sha == blob sha; tg_pk `4e2a9a30…7706` ×2 at `0xba06`/`0xba7a` both inside `[0xb950,0xbaa0)`; `.rodata` PROGBITS vaddr `0x3c000120` LOADABLE. Its .bin faces the same gate.
- **★ TREE RELATIONSHIP 4b4a71e5 (A) ↔ b25a21eb (B) — supervisor asked mirror/vendor/divergent = NONE:** BOTH are commits in **r2-core** (the DFR firmware is r2-core/platforms/dfr1195; b25a21eb is NOT in r2-hive — supervisor's naming was off). **b25a21eb = 4b4a71e5 + EXACTLY ONE commit** `docs(update): cite canon clauses for the v3 DeviceContext device-posture values` (merge-base=4b4a71e5, b25 ahead 1, 4b4 ahead 0 = direct parent→child). **COMMENT-ONLY:** only main.rs changed; comment-stripped diff at the two shas is empty of code (8 blank lines where full-line comments were). **Code byte-for-byte identical.** ⇒ A and the UNBAKED B have identical code; a post-swap behaviour delta has NO hidden second cause from the tree pair — it isolates to the persona present(A)/absent(unbaked-B) variable, plus benign panic-loc string shifts in .rodata ([[source-diff-is-not-binary-identity]], why the persona sits at 0xb904 in A vs 0xb950 in B).
- **UNBAKED B kept AS-IS (supervisor):** delivered FIRST as the single-variable gate-4 flip AND the falsifier for the compiled-in-only claim (board MUST come up unprovisioned after an unbaked payload replaces the rodata persona; if it stays provisioned the claim is refuted). No change from hive to that image.
- Artifacts non-secret (public builds); the persona blob is secret-bearing ⇒ gitignored, digest-only, never committed/published.

## CURRENT: otal2cap v3 OTA — run PASSED (reason=4 signer gate, the pre-committed advance); chunk stream still unreached

**★ RESULT (2026-07-27): reason=4 UnauthorizedSigner = the PRE-COMMITTED PASS.** Last night = reason=1 at the VERSION
gate (stale v2); tonight the same board reached the **SIGNER gate** → **the v3 HEADER PARSES**. My `movi 137` artifact
proof is now confirmed BEHAVIOURALLY ON METAL (the .rodata constant is the one the board acted on). **Clean protocol
reject** (the 4th tree-leg I flagged missing): NO reset, zero panic, zero watchdog; A uninterrupted, beats→165, still on
`otav3.A.0727`. NOT a chunk death → **fire-branch has no trigger; b25a21eb pair stays HELD** for the provisioned run.
Advance = the failure moved ONE gate forward (version→signer). **The chunk stream — where every historical failure lives — has still NEVER been reached.** Connect + header-delivery + version-parse now proven; signer + chunk-stream + apply/flip/run still owed.

**★ reason=4 DECODED (source-attested @4b4a71e5, for the Roy-OTA refocus 2026-07-27): `VerifyError::UnauthorizedSigner`** (r2-update `reject_reason()` lib.rs:812; emitted by `check_acceptable_signer` :521). **Order matters: the header sig is verified FIRST — a bad sig is reason=3 — so reaching 4 means the signature VERIFIED (image B is validly signed); reason=4 is purely an AUTHORISATION verdict.** Gate-4 accept needs ANY of: (1) `header.issuer_pk == receiver ctx.tg_pk` (TG_SK-direct, :477); or (2) a valid non-revoked in-floor in-window role-0x05 `update_authority` cert binding issuer_pk (:490-521). `ctx.tg_pk`+certs come from the RECEIVER'S PROVISIONED PERSONA; certs are EMPTY here ⇒ the ONLY path is (1): **B's signer must equal X1's provisioned tg_pk.** reason=4 = they differ.
- **★ THE PERSONA-REGION MIGRATION IS NOT ON THIS CRITICAL PATH.** Gate 4 cares that the receiver's tg_pk MATCHES B's signer, not WHERE the persona is stored (§9.12.4a safe-storage is a real but SEPARATE concern). Cheapest green, possibly NO metal / NO write: **(A) composer RE-SIGNS image B with X1's EXISTING tg's TG_SK** (if X1 has a valid persona) ⇒ issuer_pk==tg_pk ⇒ gate 4 passes. **(B) only if X1 is wholly unprovisioned:** a raw persona WRITE giving it a matching tg_pk (identity write, Roy-gated) — still NOT the region migration. So the identity write is required ONLY in case (B).
- **PIVOT FACT neither lane has stated as a value: X1's provisioned tg_pk vs image B's issuer_pk** — both composer/device custody (X1 persona UNREAD in my records; B's signer = composer's ota-sign key). Everything downstream is a TG-alignment resolvable at the SIGNER, not on metal, unless X1 is unprovisioned.
- **SHORTEST PATH (steps/blocker):** 1. read X1 tg_pk+validity [composer/device]; 2. read B issuer_pk+its TG [composer]; 3. align — re-sign B to X1's TG (A, no metal) else provision X1 (B, Roy-gated write); 4. re-deliver B over BLE-CoC ⇒ past gate 4 ⇒ CHUNK STREAM (never reached) ⇒ apply/flip/run [grant + composer]. Nothing owed by hive (answer delivered; the pivot facts are composer's).
- **★ baked_persona = THE CONFORMANT UNBLOCK (source-attested @4b4a71e5, supervisor OTA refocus 2026-07-27): the bench boards were provisioned BY THE IMAGE, a proven mechanism.** (Q1) **arbitrary TG at build time — YES**: build.rs (:76) bakes whatever `persona.bin` `DFR_PERSONA_PATH` points at, verbatim as the `BAKED_PERSONA` const (:89); TG lives in the blob, nothing hardwired. (Q2) **input = env var `DFR_PERSONA_PATH`** (required; +optional `DFR_ROLE_PATH`; RAK = `RAK_PERSONA_PATH`). (Q3) **lives COMPILED IN (rodata), read directly** — `read_persona` under `#[cfg(baked_persona)]` (main.rs:3344) = `persona_from_bytes(BAKED_PERSONA)`; the flash-0x12000 read is the `not(baked_persona)` ELSE branch (:3349), not taken. **AIRTIGHT: `BAKED_PERSONA` is only ever parsed, NEVER an argument to a flash write — NO 0x12000 write, no first-boot write, no region touch, ever.** ⇒ **sidesteps the persona-region question ENTIRELY; NOT the raw-write non-conformance in a build-time costume.** **Fixes reason=4 directly:** gate-4 `ctx.tg_pk` comes from `read_persona` = the baked const, so building image A with `baked_persona` for the SAME TG that B is signed with ⇒ `ctx.tg_pk == B.issuer_pk` ⇒ TG_SK-direct accept. The missing 0x00D3 CoC provisioner is irrelevant (nothing is provisioned over a bearer). **"A over USB carrying the persona" = the conformant FIRST HALF of the objective.** NEEDS (not this turn): composer mints a dev persona.bin for the chosen OTA TG (SECRET, gitignored) + signs B with that TG's TG_SK; hive REBUILDS A with baked_persona (#d005 order + pinned sha + clean checkout; hive builds/attests, never flashes). baked-TG and B-signer-TG must be the SAME keypair — composer controls both, guaranteeable off metal. [[dfr1195-nvs-config-needs-partition-table]] [[all-firmware-dev-mode-standing]]

### run detail (LIVE = 4b4a71e5 pair; b25a21eb = next/provisioned run, held)

**LIVE (THIS run) = the 4b4a71e5 pair. My b25a21eb rebuild CROSSED a flash already in progress — b25a21eb is the NEXT
(provisioned) run, NOT a supersession.** Both v3 builds are byte-EQUIVALENT in behaviour (r2-update crate byte-identical;
only main.rs panic-location bytes shift from the canon-cite comments). **DO NOT MIX pairs across a run** — A and B must
differ ONLY by baked BUILD_ID; pushing b25a21eb B onto a 4b4a71e5 A board would add the panic-loc shifts and break the
clean BUILD_ID observation. Both v2 pairs DEAD (`7aa01f81`/`dd355bc7`).
- **★ v3 REACHED THE BINARY (artifact proof):** `verify_header` = `movi a12,137` (HEADER_LEN=137) vs v2 `movi a12,123`.
  Holds on BOTH v3 builds (crate byte-identical). PACKAGE_VERSION=3 coupled (v3 header IS 137B). **⚠ SOURCE-DIFF IS NOT
  BINARY-IDENTITY** (I measured both binaries; comments shifted panic-location DATA → different shas even with identical
  codegen — 3rd instance of the report≠artifact / source≠binary family tonight). [[presence-and-absence-at-symbol-level]]
- **LIVE (4b4a71e5 build): PRIMARY** `otal2cap,lora,xiao,benchsf7` (LoRa core0), size 1363936 — **A `ae5fadb3…` FLASHED to
  ota_0, RUNNING (MAC matched, NVS preserved); grant bound to B `4cd1e333…`, composer MID-RUN.** SECONDARY (fire-branch,
  LoRa core1) A `05874e40` / B `90f5e95c`.
- **NEXT run (b25a21eb build, HELD — the provisioned run rebuilds anyway):** PRIMARY A `59f609e1` / B `8ec5f876`;
  SECONDARY A `72bacdee` / B `d2dac4a3`. Both attestations held; all 4 (each build) ELIGIBLE=YES + controls + BUILD_ID
  differential clean.
- **CORE-MAP (both builds, symbol-proven):** PRIMARY LoRa core0 (`lora_task` present + `lora_route_task` absent),
  SECONDARY LoRa core1 (route present + lora_task absent). Residual core0 = BLE(+CoC) + ESP-NOW (irreducible) + wifi idle.
- **★ PANIC-FORENSICS MAP (supervisor ruled, consequence of source-diff≠binary-identity):** an unexpected panic on the
  LIVE board decodes against **4b4a71e5 line numbers** (the FLASHED build), NOT b25a21eb — the canon-cite comments shifted
  main.rs lines. If anything panics mid-run, use the 4b4a71e5 build's ELF/map, not the pinned tip's.
- **Two-leg: all 4 ELIGIBLE=YES** (HANG_CAP@0x600fe000; `__user_exception`@0x40378c44 size 0x52, 0 windowed). Pos D5=YES;
  neg xiao-acc8=LEG1 FAIL. BUILD_ID differential: all 4 carry only their own (0 cross).
- **CORE-MAP HEADLINE (presence-AND-absence):** PRIMARY `lora_task` PRESENT(2)+`lora_route_task` ABSENT(0) = LoRa core0
  (max load); SECONDARY `lora_route_task` PRESENT(3)+`lora_task` ABSENT(0) = LoRa core1. Residual core0 (both) = BLE(+CoC)
  + ESP-NOW (irreducible) + wifi idle; PRIMARY adds SYNC lora_task. [[presence-and-absence-at-symbol-level]]
- **reset_reason discriminator (4 outcomes):** B-boot=complete · CoreSw 0x03=fault (recovers via reset tail) · RWDT 0x10=
  os110 executor stall (HANG_CAP EMPTY) · NO-reset=clean protocol reject. Attribution on a SECONDARY pass = core0-load-
  relief-as-a-class, NOT coex.
- **PERSONA PRECONDITION (binding):** flash A app-only → A reports persona AFFIRMATIVELY → provision ONLY on
  affirmative-absent/invalid → THEN B. Silence=STOP; different-TG=STOP+ESCALATE, never overwrite.
- **Grant is LIVE, bound to the 4b4a71e5 pair** (PRIMARY A `ae5fadb3` flashed+running, B `4cd1e333` mid-run). Fire-branch
  for THIS run = 4b4a71e5 SECONDARY A `05874e40`. On the NEXT (provisioned) run, rebuild anyway and the b25a21eb pair
  applies. **No action from hive: do NOT rebuild, re-bind, or flash — the run in flight is consistent.**
- **ImageSink `staged_rollback_value()=ExplicitlyNotApplicable`** sanity-checked CORRECT (no ESP anti-rollback; R2 seq
  floor @0x18000 sole floor; caveat if secure-version ever enabled → `Value(security_counter)`).

## superseded: staota transport + v2 otal2cap pairs (dead — staota STA never connects; v2 rejects v3)

## CURRENT-was: otal2cap OTA — first-ever round-trip attempt RUNNING on X1 (composer, grant bound to A `7aa01f81`)

Two attested pairs built; supervisor RULED the **as-built (heavier-core0) pair RUNS FIRST** — core's asymmetry argument: a
PASS with MORE core0 load is STRONGER, and the only ambiguous outcome (chunk-0/1 fire) is exactly where the loraroute pair
is the right next move. My loraroute pair is the pre-built FIRE-BRANCH (removes a build cycle from the failure path).

- **RUNNING — as-built pair** `otal2cap,lora,xiao,benchsf7` @85303273 (grant bound to A, composer executing NOW): A
  `otal2cap.A.0726` **`7aa01f81d7fa7f1d16690613fd738dbe567847e2bc521b6cbca46adb311af7fa`** / B `otal2cap.B.0726`
  `03368c8ffc4beeb1ebde2b94cd296b856cf464a4a0f82ca979e84fb533a2a1d4`. Both ELIGIBLE=YES. **ALL radios core0** (ble+CoC,
  lora_task SYNC, espnow, wifi idle) — os110 + my tri-radio-core0 hazard are ONE hazard here (core R-20260727-01), so a
  chunk-0/1 fire is CONFOUNDED (intrinsic vs contention inseparable); a PASS is stronger for the same reason.
- **FIRE-BRANCH — pre-built loraroute pair** `otal2cap,loraroute,xiao,benchsf7` @85303273: A `otal2cap-lr.A.0727`
  **`dd355bc72c673fce7426142f91b8984843b82b81c66024bb65bbd8a801b5fc87`** / B `otal2cap-lr.B.0727`
  `f3351d536627333c7d7aa9cfbfe50381d405b068d4394e43998387456d1a2505`. Both ELIGIBLE=YES. **LoRa CONFIRMED on CORE1
  (symbol-level: `lora_route_task` PRESENT + `lora_task` ABSENT)** — residual core0 = BLE(+CoC) + ESP-NOW (irreducible) +
  wifi idle. On a chunk-0/1 death → supervisor re-binds grant to `dd355bc7`, composer runs immediately (no rebuild).
  [[presence-and-absence-at-symbol-level]]
- **★ ATTRIBUTION CONSTRAINT (pre-committed, if the loraroute pair passes):** the claim is **CORE0 LOAD RELIEF AS A CLASS —
  NOT coex-relief.** loraroute swaps SYNC lora_task → continuous-RX lora_route_task AND pulls alloc + RouteEngine, so two
  mechanisms move together; no green separates them (core D-20260727-02). **Do NOT write "coex" in a verdict.**
- **PRE-COMMITTED DECISION TREE (grant file):** B observed running = FIRST-EVER round-trip on this hardware; ODT chunk 0/1
  = old stall → loraroute rebuild(=fire-branch); later index = NEW mode (report as new, not os110); os110 surviving with
  LoRa on core1 = contention refuted strongest → USB-CDC receiver (rung-1, core builds) earns its cost.
- **reset_reason DISCRIMINATOR (relayed to composer as the capture-time interpretation key):** B prints its BUILD_ID = done;
  CPU fault → HANG_CAP captured + CoreSw 0x03 (recovers via reset tail @490); **executor/CoC stall (os110) → HANG_CAP EMPTY
  + RWDT 0x10 — an empty capture is NOT absence of a fault, it is a fault the instrument cannot record.**
- **PERSONA PRECONDITION (binding):** flash A app-only → A reports persona AFFIRMATIVELY → provision ONLY on
  affirmative-absent/invalid → THEN B. Silence=STOP; valid persona in a DIFFERENT TG=STOP+ESCALATE, never overwrite.
- **HIVE POSTURE:** build+attest COMPLETE for both pairs; composer runs. **Nothing owed unless the run fires a chunk-0/1
  death** — then the fire-branch is already built (no action) and I report the core-map headline on re-bind. Do NOT
  pre-build anything else.

### superseded: staota transport (core-confirmed broken; kept as the "why we're on otal2cap")

**⚠ STOP CONFIRMED BY CORE (D-20260726-13): staota's WiFi STA NEVER associates → OTA-over-WiFi BROKEN, nobt-independent.**
wifi_task's `connect_async()`@9027 is gated behind `DATA_PLANE_JOIN.wait()`@9021; **Q1: esp-radio 0.18.0 does NOT
auto-associate — explicit connect_async REQUIRED** (wifi/mod.rs:2515; new()@2205 sets mode only). **Q2: DATA_PLANE_JOIN =
0 signals** (core's working positive control: generic `.signal(` = 5 real sites, DATA_PLANE_JOIN = 0; my bit5 finding
holds). So the STA never connects → no DHCP → :21043 unreachable. **Q3: otal2cap is NOT proven either** (os110 aborts +
post-abort hang = the v8.6-v8.7.3 campaign trigger; the round-trip was F1 target, never demonstrated). **NEITHER
transport proven.** My tri-radio-core0 prediction + bit5 both stand. **No staota A/B build until supervisor rules
transport:** (a) core adds a small `cfg(staota)` boot-connect, or (b) switch to otal2cap-at-2-radios (retires the
tri-radio hazard, may unblock os110 if coex-driven). Awaiting supervisor.
- **★ PERSONA-GATE inside the OTA health check:** `ota_health_check` items 3+4 (@4084+) require a present+non-degenerate
  persona (hive_id!=0, tg_pk!=0) AND a GroupHmac sign/verify round-trip ⇒ **image B ROLLS BACK if X1's persona doesn't
  validate**, independent of OTA transport (R2-LORA 6.5.1, landing IN the health gate). X1 enrolment UNREAD (composer).
- **nobt RULING (supervisor, variable isolation — an OTA failure then means OTA, not coex-stall ambiguity):** build A+B
  with `nobt`. STOP-check result: `staota,lora,xiao,nobt` COMPILES; radio-liveness gate passes on LORA_UP (nobt OK there);
  nobt gates ONLY the ble_task spawn @1090. BUT blocked by the staota-connect STOP above (nobt pulls ble → wifi_task takes
  the DATA_PLANE_JOIN-gated branch).
- **CORE-MAPPING (load-bearing, was invisible in the feature list):** under staota,lora,xiao ALL THREE radios on CORE0
  (wifi_task + ble_task + lora_task@1236, main spawner). Only loraroute isolates LoRa on core1. Tri-radio-on-core0.
- **SEQUENCE (once unblocked):** A+B with nobt over a SYNTHETIC AP → prove round-trip; THEN the tri-radio (ble-spawned)
  image as a SECOND over-air push, rollback as the net (a boot-hang at advertise → health gate never marks valid →
  bootloader auto-reverts) → answers the coex question the prediction is about. Re-attest each build + state each radio's core.

### superseded: earlier Option-B framing (empty-creds attest stands)

Bare R2 relay-floor OTA-capable X1 image, build+attest only. Supervisor RULED **Option B** (`staota,lora,xiao` at pinned
`85303273`) — the true bare `staota` floor does NOT compile (3× E0425, `current_beacon_epoch` lora consts — THIRD
instance of the radarprobe defect class; core's held cfg fix owed, DEFAULT must be in its regression set). OTA is
WiFi-independent of LoRa (ota_task off the RouteEngine), so LoRa is present-but-incidental; xiao = honest XIAO Wio-SX1262
pin-map.
- **Feature justification (from code):** `staota` = WiFi OTA (ota_task@1056, signed :21043 on WiFi netif); `lora` =
  **REQUIRED** (Roy: dual-bearer BLE+LoRa beacons — incidental framing WITHDRAWN); `xiao` for the SX1262 bringup @1158
  pins (XIAO physically carries the Wio-SX1262). staota,lora,xiao = the compile-forced set == the capability set Roy wants.
- **Beacon reachability (image A, verified static, NEITHER gated):** BLE §7 = `ble_task`@1099 (ble via staota, not nobt)
  → `peripheral.advertise(`@4590 (encode_advert @4518), BLE_UP@1101. LoRa §8.1 = `lora_task`@1236 (cfg not(loraroute)
  TRUE) → `radio.transmit(pl)`@6454 (build_lora_beacon@6451), print "LORA-TX §8.1 beacon"@6462, LORA_UP@1237. STATIC
  reachability only — on-air TX is metal (⚠ coex advertise-hang note @1208; tri-radio combo metal-unverified).
- **STOP check (mark-image-valid) PASSES — health-gated, not blind:** `set_current_ota_state(Valid)`@main.rs:4162 runs
  only inside `if ota_health_check()`@4161, deferred 8s (`ota_confirm_task`@4136), PendingVerify-only; FAIL → Invalid +
  rollback record + revert + reboot (`ota_health_check`@4074 = §5.2 min-2 radios-up). Rollback protection intact.
- **Images A + B** (clean detached, `rm -rf target` each, differ ONLY by baked `R2_BUILD_ID`, same size 1346876 B):
  A `relayfloor.A.0726` sha256 `0722485d8d49160a3d036b6325b5e13fdeff5a38e5ed800c1c1f030834da83f9` /
  B `relayfloor.B.0726` sha256 `a571710ad42df8355b4abb147831ee2c0069f7a74a8a6e6b0fe726c2da518387`. Differential:
  A-has-only-A / B-has-only-B (0 cross); BUILD_ID printed at boot → B running is observable = OTA round-trip proof.
- **Two-leg eligibility on THESE artifacts (not inherited):** A + B both **ELIGIBLE=YES** (HANG_CAP@0x600fe000;
  `__user_exception`@0x40378c44 size 0x52, 0 windowed calls). Pos control D5 v8.7.3=YES; neg control xiao-acc8=LEG1 FAIL.
- **⚠ CREDS — reason CORRECTED (supervisor):** built with EMPTY WiFi creds. The reason creds must stay out of the
  artifact record is **R2-SECRETS 3.1 — no real value as a LITERAL in ANY tracked file/commit/recipe/attestation/message**
  — NOT the g23 hold (USE ≠ publication; build.rs reading env is compliant; Roy has issued NO ruling on the creds, g23
  open). The FLASH build bakes `R2_WIFI_SSID`/`R2_WIFI_PASS` via build.rs env → different sha256 → **re-attest at
  flash-build** (both images, both legs, controls). The sha above attests structure+instrument+BUILD_ID+eligibility
  (creds-independent).
- **★ CREDS = THE LIVE FINDING (supervisor 2026-07-27, CORRECTING a stale ALL-CLEAR I wrote). NOT resolved. g24 is OPEN.**
  ROY-GATES.md:122 — g24 is **RULED synthetic OVERNIGHT, PENDING ROY REVIEW**, in the OPEN set NOT the closed table; the
  supervisor's synthetic-AP ruling is a RECOMMENDATION awaiting a one-line overturn, **not a closure**. My earlier
  "CREDS = SYNTHETIC AP, RESOLVED" was a stale all-clear — the DANGEROUS direction (a stale unblock costs an action nobody
  re-examines; re-verify unblocks with the same rigour as blocks). [[status-recorded-as-a-constraint]]
- **★ R2-WIFI 3.0b (specs, landed 2026-07-27) = a canon MUST that binds hive builds DIRECTLY:** a build NOT given
  credentials **MUST NOT substitute a compiled literal**. Either FAIL the build, or produce an image that **cannot
  associate AND SAYS SO**. **ABSENCE OF A CREDENTIAL MUST NOT RESOLVE TO A CREDENTIAL.** **LIVE CODE FINDING (core's
  firmware source — no build.rs in hive):** core CONFIRMED the 3 cfg branches (dfr1195 main.rs:936-941): (1) `staota`
  → env creds but `unwrap_or_default()` bakes an EMPTY string on absence = "cannot associate but does NOT SAY SO" (the
  silent half-miss, not a literal); (2) `ble,not(staota)` → COMPILED LITERAL pair = 3.0b VIOLATION; (3) `not(ble),not(staota)`
  → `FIELDLAB_SSID`/`FIELDLAB_PASS` consts :386-387 = 3.0b VIOLATION. Comment :934 "NEVER hardcoded" is true of the
  staota branch ONLY. **Class-not-instance (supervisor): a default credential is the same CLASS as a permissive default —
  the value may be byte-identical to a chosen one, only its PROVENANCE differs; 3.0b is a provenance rule.** **Fix is
  CORE's** (queued behind the persona write-pair + 5-item re-vendor): emit-inert `Option::None` uniform (no assoc + logs
  "no credentials at build — WiFi inert, cannot associate") + build.rs FAIL-CLOSED scoped to `staota` (WiFi definitionally
  required there). **hive RE-ATTEST criteria post-fix (owner=hive):** a non-staota no-creds build → `strings`-on-image
  finds NO SSID/passphrase + the inert log; a `staota` no-creds build → FAILS TO COMPILE. **hive MUST NOT emit a violating
  image**; any future wifi-bearing build order must not precede core's fix. Falsifier stands: inspect the image for an
  embedded SSID/passphrase = finding one is a MUST-miss.
- **Synthetic-AP context (kept, NOT a closure):** composer stood up an AP on <build-host>'s spare phy2 (sustained, 0
  drops/7 polls/~80s, sole-uplink wlp3s0 default-route untouched). If used, creds are CHOSEN/synthetic. **⚠ synthetic ≠
  safe-to-publish when the value GRANTS ACCESS — a chosen PSK is a WORKING PSK while the AP is up.** SSID/PSK fine in fleet
  mail + composer's mode-700 dev-trial, **MUST NOT enter any tracked/public file** (this RESUME included). Build reads env
  (R2-SECRETS 3.1) — but per R2-WIFI 3.0b a MISSING env MUST NOT fall through to a literal. **Empty-creds attestation
  STANDS** (attests structure/instrument/BUILD_ID, creds-independent). [[use-is-not-publication-secrets-boundary]]
- **★ PROVISIONING IS A PRECONDITION, not a follow-up (supervisor ruling — persona load-bearing TWICE: R2-DEVICE-LIFECYCLE
  publish + the OTA health-gate items 3+4).** Revised sequence: flash A **app-only** (NVS survives) → A boots + REPORTS
  whether its persona validates → if NOT, composer mints the dev-TG persona (its delegated class) + provisions → ONLY
  THEN push B. Else B is GUARANTEED to auto-revert and a correct rollback reads as broken OTA.
- **★ otal2cap REROUTE ON THE TABLE (supervisor→core):** Roy asked for OTA working, no transport specified. otal2cap
  (OTA-over-BLE-CoC) drops 3 radios→2 (retires the tri-radio-core0 hazard) and its receive path is more static-verified
  than staota's. BUT I confirmed **otal2cap is NOT proven end-to-end** (no slot-flip / running-image on record; the
  campaign fought the hang that pre-empted every OTA window; closed on the hang fix, not an OTA completion). So it's the
  better-UNDERSTOOD unproven path, not a proven one — either transport still needs a first real round-trip.
  **★ NO OTA HAS EVER COMPLETED ON THIS HARDWARE IN ANY TRANSPORT — tonight is a FIRST, not a re-run.** My
  better-understood read is DEVICE-SIDE ONLY; the uncosted half = **does a HOST-SIDE pusher exist on <build-host> tonight**
  (otal2cap needs a BLE central opening an L2CAP CoC + streaming; staota needs TCP over the now-proven AP) — composer's
  lane, asked. A device path is worthless if nothing can talk to it.
- **★ USB-CDC decomposition (supervisor idea): the 4-stage round-trip = RECEIVE (transport-specific) + WRITE/FLIP/RUN
  (bearer-agnostic apply).** Stages 2-4 ARE wired + bearer-agnostic: `FlashSink`(ImageSink)@8232 → `apply_signed`
  @8335 → `SignedOtaApply::start`@8353 — the SAME orchestrator both wired receivers drive. **But USB-CDC RECEIVE is
  UNWIRED** (Q-A): `apply_signed` has exactly TWO callers — `ota_receive_over_coc`(BLE-CoC)@4733 + `ota_receiver`(net
  :21043)@8043; NO USB-CDC caller. The CDC readers that exist (`xiao_bridge_task`@6934, `uart_rx_task`@7519) don't feed
  apply. USB-CDC is LISTED in r2-core's r2-update but unwired here (Q-B moot — nothing to compile). **Salvage: proving
  3-of-4 via USB-CDC needs ONE small core change** — a CDC-read receiver task feeding `apply_signed` (mirroring the two
  wired receivers), core0, reusing the proven apply path. Smallest honest path to a slot-flip+running-image observation
  with zero radio unknowns — supervisor's call + core's build.
- **Open before any grant (NOT hive):** composer read of X1 persona + OTA-TG `<scrubbed-tg>` membership — X1 must VERIFY the
  update signer or OTA is rejected on arrival. Board currently unplugged from both hosts.
- **NO FLASH** taken; no grant.

### Next-phase context — ENSEMBLE CANON (specs Q1/Q4/Q5 + Roy #69; DO NOT build until specs finishes — ★ CREDS ARE THE LIVE FINDING, g24 OPEN pending Roy, R2-WIFI 3.0b MUST binds any build; see the CREDS block)
- **OTA delivers the rest:** image A over USB (ONE USB write all night), B over air proves round-trip, then each
  capability arrives OVER THE AIR. Sequence: (1) core hive+OTA [done, pending re-attest], (2) memories, (3) sensor, (4)
  battery.
- **Ensemble = a SCORE, not a binary/feature** (R2-ENSEMBLE v0.3, schema R2-DEF 7; NOT installed, no binary; two part
  types = Sentants + plugins). Shape = **ONE hive binary, FIVE scores** (each ≥1 Sentant). Active ensembles AND the
  900s-vs-5s cadence = **NVS ROLE-PROFILE boot config** (R2-RUNTIME 210 — role+knobs selected AT BOOT from NVS, NOT
  compile-time; proven no_std), NOT cargo features. Encoding COMPILED (Roy #69); boundary = the score. **Cite #69 for
  encoding, R2-ENSEMBLE for boundary; NEVER §2.2B (bearer-presence only).**
- **★ C1 — memories/battery/LED are HIVE-SHARED SINGLETON PLUGINS WITH REGISTRATION, not ensemble-owned** (R2-ENSEMBLE
  2.1.2: a plugin wrapping an OS-exposed-once singleton — port/device/hw-cap — MUST be hive-shared + have a registration
  mechanism). NVS/FRAM/ATECC608 each exposed once; the ADC (battery) once; the LED once. The ensemble is the thing that
  USES + REGISTERS against the shared plugin (R2-WEB pattern: one HTTP server, N ensembles register routes). **NON-
  CONFORMANT: N ensembles each owning an NVS driver — do NOT build that.**
- **★ HARD MUST-NOT (structural, R2-KEYSTORE 184):** secret key material MUST NOT leave the protected boundary in
  plaintext ⇒ a generic read-NVS capability is non-conformant the moment it CAN address the key region. The NVS cap MUST
  be **REGION-SCOPED BY CONSTRUCTION — incapable of EXPRESSING the key range** (not well-behaved, not documented-forbidden:
  structurally incapable). Make-bad-state-unrepresentable-by-type. [[identity-verified-is-not-function-verified]]
- **Q4 capability names:** no canonical storage classes (R2-CAP 3.2 — no central registry; social agreement + docs).
  Ruling: **MINT `ai.reality2.cap.storage.*` + DOCUMENT in the SAME commit** (docs ARE the registry; undocumented =
  unregistered). `ai.reality2.cap.env.scalar` is NOT canon — a fleet string; keep using, stop calling it canonical.
- **★ LED — R2-INDICATOR v0.5 is NORMATIVE (do NOT invent a pattern); indicator is a PLUGIN byte-identical across boards,
  output stage RENDERS only, never re-decides a state ⇒ LED logic in platform main.rs is NON-CONFORMANT.** Fixed
  envelopes: Healthy = dub-dub double-pulse (lub@0.00, dub@0.18, 20 BPM, DIM — NOT 25BPM/full-bright, the rejected edge)
  · Updating/OTA = 0.18s strobe white · Boot = ~100ms flash then dark · Error = 0.25s pulse red · Identify = solid white
  · Low-batt = 1.5s pulse orange. Overlay priority (highest first): Identify > Updating > Low-batt > underlying (Identify
  outranks OTA deliberately). Healthy AND Updating MUST signal in BOTH dev+prod. **Dev-only delta = the R2-INDICATOR 6
  event-arrival BLIP** (quick tick on event arrival, may collide with HB, collision OK/no arbitration; PROD MUST NOT show
  it; gated on **R2-BUILDMODE**, not a hand-set flag). sensor-read/battery-read are NOT signature states; app extensions
  allowed but MUST NOT redefine a reserved envelope.
- **Sensor = enable-settle-read MUST be REAL code** (only the VALUE simulated while USB attached — USB cuts the 5V rail):
  enable 5V gate → WAIT settle → attempt read → substitute value when rail known-cut. NO stub skipping gate+settle
  (never-run-path class). Await circuits for gate pin/polarity/settle (do NOT invent settle). **SIM MARKER MANDATORY**:
  a simulated reading MUST be distinguishable AT THE EVENT (same path ⇒ event carries the signal); SIM MUST NEVER LEAK
  INTO LIVE. Cadence = score param. **duty_class RETRACTED (was: "likely SCF duty-class"): duty_class is a RADIO-SLEEP
  property, NOT peripheral-power** (R2-DIAGNOSTICS 58 {Unknown,AlwaysOn,Intermittent}, wire R2-WIRE 12.6 `dc`; its ONLY
  job = telling peers whether to BUFFER-on-fade while your RADIO sleeps — R2-RUNTIME 236). Toggling the 5V sensor rail
  does NOT make the radio fade (board stays reachable) ⇒ the 5V cycle is INVISIBLE to duty_class, MUST NOT drive it.
  **USB-powered X1 = AlwaysOn.** Power source OVERRIDES role; class set STATICALLY from provisioning (a runtime flip would
  leave a battery node AlwaysOn+flooding) — if X1 ever runs on battery it MUST be provisioned DutyCycled BEFORE, never
  flipped at runtime. Settle: canon SILENT (a driver detail), so 1500ms is a config knob not a spec obligation.
- **X1 enrolment UNKNOWN:** send-over-TN needs X1 = a TG member with a working persona; persona + OTA-TG `<scrubbed-tg>`
  membership UNREAD (composer's lane). Do NOT design around X1 enrolled. Button-free entry PROVEN on D5, UNPROVEN on X1
  — gates the first USB write (composer, amended grant).

### Electronics (circuits, AUTHORITATIVE for firmware — for the sensor/memory/battery/LED ensembles, not yet built)
- **5V gate = D0/GPIO1, ACTIVE-HIGH** (TPS61023 EN; low = true 0.1µA disconnect). **Settle = 1500ms proven-good** (radar
  replied on battery at that value); min UNCHARACTERISED — do NOT go below a few hundred ms without a test. Use 1500 as
  a CONFIG KNOB, not a literal.
- **Radar UART = TX GPIO43 / RX GPIO44, 115200 8-N-1**, Modbus RTU slave 0x01; water level = holding reg 0x0003 in mm;
  request `01 03 00 03 00 01 74 0A`, reply `01 03 02 mm_hi mm_lo crc_lo crc_hi`. **XC4486 = PASSIVE bidirectional shifter
  — NO DE/RE, NO enable, auto-direction; FIRMWARE DOES NOTHING FOR IT — strip any direction handling** (the withdrawn
  radarprobe's RADAR_DE_RE=GPIO6 was WRONG for this shifter).
- **FRAM 0x50 = MB85RC, 16-bit addressing, BYTE-WRITABLE**, no page boundaries, no write delay, endurance ~unlimited at
  5-10s; WP+A0-A2 strapped GND (write-enabled, no WP handling). **SIZE UNKNOWN (32KB or 512KB) — PROBE it, do not assume.**
  **⚠ PAGING HAZARD (circuits): a 512KB MB85RC4M addresses BEYOND 16 bits — the high bits page into the I2C DEVICE-ADDRESS
  byte, NOT the 2 address bytes. The wrap test still distinguishes 32KB from larger, but full addressing of the large part
  REQUIRES those page bits — without them WRITES ALIAS (silent corruption past 64KB). So DETECT size AND implement paging;
  aliasing-writes-that-appear-to-succeed is the worst failure shape for a store-and-forward buffer.**
- **Storage-queue canon (for the FRAM plugin):** R2-ROUTE 3B.3 **OUTQ-1..4** @769a255 (read the clauses, not a summary) —
  a SEPARATE section from peer-custody: OWN-ORIGIN custody ends when YOUR OWN transmission succeeds; PEER-custody ends when
  the PEER reappears — conflating them mis-sizes both buffers.
- **claim_state persistence (R2-DEVICE-LIFECYCLE 3 invariant 2):** requires 3 hw roots — irreversible VIRGIN sentinel +
  dedicated monotonic hw_epoch counter + per-device HUK. Lacking ANY ⇒ FAIL CLOSED at build/provision; canon permits BENCH
  BUILDS AS EXPLICITLY NON-PERSISTENT ONLY. If the S3 lacks any of the three, **DECLARE the bench build NON-PERSISTENT
  deliberately IN THE BUILD** (don't let it emerge as a surprise).
- **ATECC608 0x60 = LOCK STATE UNKNOWN — QUERY the lock before assuming anything; do NOT provision blind.**
- **Battery = D1/GPIO2, 2×470k divider (read ×2), 100nF at pin.** Dry joint NOT confirmed reflowed; fed from LiPo+ which
  USB CUTS → reads ONLY on battery. Code it, do NOT expect a green tonight, say so in the attestation.
- **LED = GPIO21, ACTIVE-LOW**, free (not shared with LoRa B2B pins). **0x7C = CONFIRMED PHANTOM** (only 0x50 + 0x60 are
  real) — bind nothing to it, don't let an enumerator claim it.
- **Testable on USB tonight:** FRAM, ATECC608, SX1262 SPI, LED, I2C scan, gate GPIO logic. **NOT testable:** real radar
  comms, battery, actual 5V rail rise.

## radarprobe build order (2026-07-26) — WITHDRAWN by supervisor (wrong artifact); defect recorded, no hive build

**ORDER WITHDRAWN.** Roy's actual goal = a per-component bring-up smoke test, whose instrument is a **throwaway Arduino
sketch circuits already has** (I2C scan / FRAM + secure-element probe / battery ADC / gated Modbus), NOT an R2 no_std
feature — so this is NOT hive's artifact. Supervisor owned the mis-inference chain (radar test → radarprobe feature →
DFR1195 crate-path=board), all unverified. Hive builds nothing here. The finding below STANDS (recorded by supervisor;
core HOLDS a ready-but-UNAPPLIED fix — supervisor told it to HOLD, must NOT move the tip since g18 artifacts are pinned
to 85303273). My build attempt surfaced it. Nothing in flight on radarprobe. (To know if core landed anything, read the
TIP — the emitter — not a summary or intent.)

Explicit #d005 build order (supervisor, Roy GO). Feature `radarprobe = ["dev"]` (Cargo.toml:290). Pinned sha = MY
ls-remote authority `85303273` (== branch tip; supervisor's `4e82c36a` is on origin/main + r2-core-consolidation, NOT
this branch, NOT an ancestor — I pinned my own ls-remote as instructed). **Reachability verified in code:** radarprobe
block @main.rs:901 spawns `radar_probe_task`@913 then a diverging `loop{}` → radio/mesh spawns (net/wifi/io/ota/ble/lora,
all >line 919) unreachable (`allow(unreachable_code)`@603). Pre-probe spawns (rwdt_feed@623, hang_reprint@840) still run.
- **BUILD FAILS — 5× E0425 cannot-find-in-scope:** `role_class_hash` (def `#[cfg(any(lora,ble))]`) used in ungated
  `io_task`@1729; `DATA_TX` (`#[cfg(ble)]`) @1760; `COARSE_TIME_ANCHOR_S` + `LORA_BEACON_T_ROTATE_S` (`#[cfg(lora)]`) in
  ungated `current_beacon_epoch`@6264. radarprobe=[dev] pulls neither ble nor lora, so the unguarded refs don't resolve;
  every shipping set (fakesensor/xiaobridge/bridge) pulls lora and/or ble, so radarprobe has never compiled standalone.
- **No ELF → attest moot** (two-leg instrument un-checkable). Not fixing (core owns source; build-only order). No flash,
  no target prepared. Owed: core gates those refs under cfg (or radarprobe stubs them); then re-issue on the fixed sha.
- **TARGET = X1 (XIAO ESP32-S3 + Wio-SX1262)** — Roy corrected supervisor's DFR1195-only inference; the radarprobe
  RS-485 pins are XIAO-authored (RADAR_UART_TX=43=XIAO D6). **radarprobe needs NO `xiao` feature** (verified code +
  build): `xiao=[]` pulls nothing; its pin-map cfg sites (main.rs:1178-1241) are radio-region, unreached under the
  radarprobe diverging loop; the radar block uses hardcoded GPIO43/44/6. `radarprobe,xiao` FAILS the identical 5 E0425
  (xiao adds no ble/lora). SX1262/DIO2-RF-switch warning does NOT bind (radio init lora-gated + unreached). **On the
  fixed sha, build `--features radarprobe` ALONE.** ⚠ an X1 radarprobe flash REPLACES the bridge role + the g18
  x1-xiaobridge image (`cee2f004`) X1 currently carries — stated for Roy.

## g18 build record — D4 + X1 fault forensics rebuild `8530327309b8…a9c6` (BUILT + attested; NO FLASH)

Explicit #d005 order (supervisor, Roy-ruled, 2026-07-26). A REBUILD, not a port: xiaobridge and fakesensor are FEATURE
VARIANTS of the one platforms/dfr1195, not separate platforms. The ~40 staged XIAO/sensor siblings were STALE (built
before the instrument landed); a rebuild carries the full HANG_CAP/`__user_exception`/reprint instrument (unconditional
in main.rs, `esp-hal default-features=false` platform-wide so the strong handler symbol wins). ledger: g18 (Roy).

- **Provenance:** ls-remote authority == pinned tip; `verify-build-target 85303273 main.rs HANG_CAP __user_exception
  hang_reprint_task` → touches main.rs + all present → OK. (85303273 is comment-only touching main.rs — the RTS-EN
  premise fix — but NOT a docs-sha trap: the instrument IS in the tree, unlike 514c31a4 where the ordered markers were
  absent.) Preflight: HEAD==pinned, tree clean, worktree-instrument present, `rm -rf target` per build.
- **Features** (Cargo.toml:273 canonical pairing, both carry benchsf7 — mixed-SF can't demodulate; baked_persona OFF):
  **D4 = `fakesensor,benchsf7`** · **X1 = `xiaobridge,benchsf7`**. BUILD_ID `g18.0726`.
- **ELFs:** d4-fakesensor-g18 `30ffcdfdcd7a6ac11bc00b4ced0564af28ef6381f7ad85a9516062d50224870f` (1383864 B) /
  x1-xiaobridge-g18 `cee2f004fd6f51f9ef839461ecd3b456b8ecee0b96640930a54b218e7158d983` (1153904 B).
- **★ TWO-LEG ELIGIBILITY — both ELIGIBLE=YES** (`<build-host>:~/eligibility-g18.sh`, in order, on the EXACT artifact via
  toolchain nm/objdump): **LEG1** HANG_CAP static present (both @0x600fe000); **LEG2** `__user_exception` call-free
  within its true `nm -S` extent (D4 @0x40378c48 size 0x52, X1 @0x40378bd0 size 0x52 — 0 windowed calls, 0
  software_reset each). Positive control: D5 v8.7.3 → ELIGIBLE=YES. Neg-control: stale siblings xiao-acc8 + d4-init8 →
  LEG1 FAIL (no instrument) — the check discriminates real presence from real absence.
- **★ INSTRUMENT-BUG SELF-CATCH:** my first eligibility run false-FAILED LEG1 (`<none>`) — the Rust static is MANGLED
  (`_R…8HANG_CAP`) so `grep -w` missed it + host-nm fallback. I did NOT report absence; positive-controlled against the
  known-good D5 ELF (it ALSO showed `<none>` under the broken check ⇒ instrument bug, not artifact), fixed to toolchain
  nm + case-sensitive `HANG_CAP`. A false-absent is the worst misreport (presence-before-quality).
  [[positive-control-the-tree-not-just-the-tool]] [[never-conclude-from-a-null]]
- **Does NOT carry g15:** the dataplane join-carriage (core `259ea8e5`) is on branch r2-core-consolidation; 85303273
  does not contain it. Intentional (g18 = fault forensics, not the join relay). Per core **D-20260726-08** the firmware's
  vendored `r2-dataplane` is a **pre-g15 PIN = structurally cannot-carry = SAFE**; g15 arrives at the next re-vendor. Do
  not reason "rebuilt after g15 ⇒ siblings have it" — they will not.
- **NO FLASH taken/authorized** — build+attest only; the flash grant is a separate supervisor decision.

## Campaign result — v8.7.3 `513c949db0f9…a13a` (CLOSED, on metal)

D5 = DFR1195 ESP32-S3; OTA-over-BLE-CoC coex firmware series. The ~4-min PRIMARY double-fault silent-wedge was the true
blocker (pre-empted every OTA window). v8.7.3 = base `2249bcf0` (SW_SYS_RST IRAM-safe reset tail) + the WITNESS pre-zero
read-back (`hang_reprint_task` reads HANG_CAP back AFTER the pre-zero and its zero-OUTCOME branch emits its own line, so
the baseline is WITNESSED not inferred). Ledger: r2-core/DECISIONS.md **D-20260725-01 @ d970693c**.

- **Provenance (before build):** `verify-build-target 513c949d main.rs "WITNESSED all-zero" "PRE-ZERO INEFFECTIVE"` →
  touches main.rs + both markers present → **OK**. Worktree markers also verified in the CHECKED-OUT tree, not only via
  `git show` (catches a checkout that never materialised). [[docs-sha-is-not-a-build-target]]
- **Conformance:** **check(24) PASS** (four legs, zero-OUTCOME branch keyed — not a println count; the silent-zero-arm
  control FAILS, so it catches the defect it targets). Neg-lock `2249bcf0` FAILS clean; carries untouched.
- **ELFs** (BUILD_ID `coex.v873.0725`, clean detached build, `rm -rf target`, worktree-markers verified):
  d5-otarx-v873 `52af3bfe09f2ff6e3b69dfd4cecea7cc34239c000dd845d01eb9093636bbb7ab` (1388708 B) /
  d5-otafail-v873 `7d5d67387d3a6a9a036c8e30ec45feb1c4774e7080318b71eb95528eb69c1d84` (1387560 B). Core ELF-attest GREEN
  (independent build d4e6cd66 reproduces markers; zero-branch-emits confirmed from the artifact's `.rodata`).
- **BINS (clean-custody, USED-logged):** grant `d5-ota-v873` READ first; `env`-prefixed espflash LITERAL in the gated
  command text so the gate SEES it; pre-hash==pins; partition e0e49127/39rows; TWO USED lines written.
  d5-otarx-v873.bin `adc6cc186eb52d5e423dd9af430e836fb9bdad6b3bceeb1690e46d2a9a6cbd68` (878864 B, 0xE9) /
  d5-otafail-v873.bin `8a1ee68ee6cf31928717f6aa02b0f9a72a295f630300c309457bdb27d84d1584` (877440 B, 0xE9). DISTINCT.
- **★ First genuinely BLIND 3-way of the campaign:** hive == composer BYTE-FOR-BYTE on both bins (two hosts / two
  toolchains / one pinned input; supervisor withheld both directions), anchored by core's ELF-attest.
- **★ Gate-bypass caught + fixed:** a BARE `VAR=value` first token made auto-approve.sh:554 parse base off the
  assignment — never saw espflash, never gated, wrote NO USED. The ABSENT USED line was the tell (held custody, refused
  to assert the cause, escalated). Fix = `env` prefix. [[espflash-gate-bypassed-by-file-and-remote-exec]] (third variant).
- **Post-flash boot method (record, D-20260725-11 @ claude-fleet 705c0e5):** espflash-driven (`--after hard-reset`,
  observed CoreUsbUart) within a grant, OR CTRL+R, OR the Roy button. **RAW TTY RTS/EN is NOT a reset on S3 native
  USB-JTAG** (peripheral does not map DTR/RTS to CHIP_PU/GPIO0) — removed as a boot step, not merely qualified.

## Prior cycles (all CLOSED; detail in RESUME-archive.md)

- **v8.7.2 `2249bcf0`:** handler tail spin → IRAM-safe RTC_CNTL SW_SYS_RST (bit31 @0x60008000, CoreSw=0x03, RTC
  preserved). Proven in fault context (5 distinct faults, all rst:0x3 CoreSw, capture preserved+reprinted, zero wedges)
  — the v8.7.1 silent-wedge class eliminated. Base of v8.7.3.
- **v8.7.1 `7e774742`:** deliberate wedge reproducer. **★ my check(22) self-catch (supervisor-ratified):** the v8.7
  call-free spin tail (`j <self>`) I attested "the fix" WEDGES silently (cpu1 parks, cpu0 feeds RWDT, no reset, RX dead);
  byte-accurate attest, wrong mechanism. check(22) "no flash software_reset" was correct-but-INSUFFICIENT (absence of the
  wrong thing ≠ presence of recovery). [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]]
  [[identity-verified-is-not-function-verified]]
- **v8.7 `33219370`:** call-free fault handler; 3-way closed. **v8.6 `30cb3d6d`:** hang instrumentation; root-caused the
  secondary-fault-overwrite v8.7 fixes; the v86 authorization episode (my gate-bypass + fabricated grant) fully owned +
  cured. **v8.5 `9ebad32b`:** set_wake_window RX-quiesce + BLE-lease; identified the ~4min double-fault as the blocker.
  **v8.4 `afaab9ab`** and earlier: archived.

## Standing operational constraints

- **#d005 build gate + build-target guard** (`~/verify-build-target.sh <sha> <src> [marker…]`, fails closed): the named
  sha must `git show --name-only`-include the source file AND the change's target markers must be present in
  `git show <sha>:<src>` — else it's a docs-only / wrong-change sha that checks out clean+green but builds OLD firmware
  (the 514c31a4 case). A sha match alone is not build provenance. [[docs-sha-is-not-a-build-target]]
- **espflash/openocd LITERAL in the gated command text** — no ssh-wrapped scripts, no file indirection (the gate scans
  command text only; a file-hidden keyword bypasses silently — v86). ssh OK when the invocation is inside the quoted
  string. Carry auto-approve tokens as `env VAR=… VAR=… espflash …` — a BARE `VAR=…` prefix bypasses the gate (no USED).
- **Grants supervisor-only; verify authorization by READING the grant file + its USED log entry**
  (`$ws/.fleet/flash-authorization`, from repo CWD = `Development/R2/.fleet`), NEVER inferred from a command succeeding.
  USED = one entry per approved COMMAND (not per espflash run); the grant `sha256=` field is RECORDED not enforced —
  pre-hash==pin is the real identity mechanism. A missing USED line = the gate never evaluated, not a silent approval.
- **Workers never assign each other legs** — work allocation is posture; ask supervisor to add/reassign.
  [[fleet-posture-authority]]
- **RULING-A transform-3-way:** all lanes derive bins from ONE set of pinned ELFs (pre-hash==pin) — byte 3-way despite
  ELF-sha path-noise (panic-location abspaths in .rodata; `--remap-path-prefix` backlogged).
- **Hive never flashes** (composer = sole serial opener / Roy). Never bypass `ci/public-hygiene.sh` (MACs/device-tails
  off tracked files; HH:MM:SS colon-triple false-matches the mac-tail scanner). The **pre-push hook DOES run it now**
  (wired e7c4bcde) — but with the LOCAL out-of-repo denylist present, so pre-push green = LOCAL green only.
  **★ SAY WHICH GREEN (supervisor 2026-07-27): local pre-commit/pre-push and HOSTED CI are DIFFERENT CLAIMS — report
  "GATES PASS LOCALLY, CI NOT CHECKED" unless you've read the hosted run (`gh run list`). A gate nobody watches is
  switched off by neglect; check hosted after every push that touches CI'd paths.** [[local-check-vs-hosted-ci]] Every
  commit: `Decision-Log:` trailer (`none` for routine); verify ahead=0 via `git ls-remote origin`. Fleet msgs: file + `"$(cat f)"`, never inline. NVS 0x17000 raw role-write = brick (bake via
  `DFR_ROLE_PATH`). Env-baked const verify = full `rm -rf target` + the DIFFERENTIAL. Build on <build-host> (`export-esp.sh`);
  nohup detach kills export-esp.sh — attached ssh only.

## The rig (static conformance suite, on <build-host>; never compiles/runs tests — a PASS ≠ a green suite)

`~/preflight-v8.sh <sha>` (checks 1-11) + `~/check{12..24}.sh` + `~/check22prime.sh` + `~/perbearer.sh`. `~/strip_src.py`
= strings-first comment/string stripper (per-check/per-LEG: string-literal discriminators — cfg names, printed markers,
`extern "C"` anchors — on RAW; code on stripped). Every check neg-locked (a known-bad sha must FAIL) and positive-bound
to core's REAL identifiers (requirement-not-shape). `~/hangcap-analyze.py` = capture-consistency/family analyzer
(UNOBSERVED != FAILED; a family needs ≥2 corroborated captures). All proven + neg-locked; parked with the campaign.

## Backlog (Roy-gated, not started)

D5 reflash/provision (stays 11f2d2ef; needs Roy word) · **SEN0676 radar plugin (PARKED):** Modbus RTU over plain
TTL-UART (circuits-confirmed; not ADC/I2C, no RS-485 transceiver); radar target = XIAO per g4, so a DFR1195 UART bus in
board.toml is needed only if radar also runs on DFR; which DFR pins carry it = a board-map decision hive+composer own;
ref r2-core#91 + docs/radar-xiao-node.md · RAK relay-LED (bench, under #d003 freeze) · DFR1195 display mislabel
(cosmetic) · RAK tx_power −9dBm (core, lora_leaf_config:1219) · AGENTS.md partitions-csv doc-drift.

## ROUTE-ORIGIN-1 GROUP_MGMT carve-out — router.rs universal drop = a CANON-vs-CANON DIVERGENCE (core owns the fix; 2026-07-27)

FINDING (my code+docs gate sweep, CONFIRMED by core @ `r2-dataplane/src/lib.rs` rx-disposition ~:915, test `originless_group_mgmt_signals_join_carriage_not_a_silent_drop` :2101, + supervisor): `crates/r2-hive-bin/src/router.rs:210-212` drops ALL route-less (R=0) frames (`_ => Dropped`); comment 199-207 asserts ROUTE-ORIGIN-1 as universal-RATIFIED. But g14 (R2-WIRE v0.65 §9.5.1) EXEMPTS GROUP_MGMT (g21 = dedup on `msg_id` ALONE, fabricating an origin forbidden; g15 = join relay ≤1 hop). `route_frame` is the SINGLE routing decision point (all transports — main.rs:753/852/1002/1332); `compat::handshake` = browser-WS only ⇒ a route-less GROUP_MGMT over UDP/BLE/LoRa is wrongly dropped. `router_integration.rs:131-142` bakes the same universal reading (suite CONFIRMS the implementation belief about the spec, stays green).
- **★ ROOT CAUSE (supervisor, specs found it independently): NOT an unabsorbed exemption — a CANON-vs-CANON DIVERGENCE.** R2-ROUTE §3.8 pseudocode :661-662 drops BEFORE type-dispatch (`route_stack` empty ⇒ return) = R2-ROUTE INSTRUCTING the drop that R2-WIRE §8.2 normatively FORBIDS. router.rs faithfully implements ONE of two disagreeing specs; my "universal" comment was TRUE OF ONE SPEC. **Conformance to one document is not conformance; checking code against ITS cited spec cannot find this — only CROSS-SPEC comparison can.** specs verifies whether R2-ROUTE §3.8 is marked NORMATIVE (illustrative ⇒ editorial repair; NORMATIVE ⇒ genuine two-spec conflict ⇒ Roy). Direction settled + NOT gated on g21: dispatch by type BEFORE the route check, accept route-less GROUP_MGMT (GROUP_MGMT carries NO origin ⇒ a composite key is UNSATISFIABLE ⇒ msg_id alone is the ONLY key, not a preference).
- **★ FIX IS CORE'S, NOT HIVE'S — refusing to edit was RIGHT** (a hive-side fork = two divergent readings of one spec, worse than the bug). Core owns the router fix + the test; the fix must derive the expectation FROM CANON, not from the router.
- **⚠ WHEN HIVE MIRRORS (after core lands the reference): the carve-out is SIGNAL-ONLY, NOT a forward.** Reference asserts `deliver=false`, **`relay_on=0`** — the router NEVER forwards/re-originates the origin-less frame. Do NOT turn `router.rs:210-212 Dropped` into a *forward* (forwarding a route-less frame skips the §8.5 append-origin MUST = violation). Turn it into a **SIGNAL to the join handler**, which TERMINATES-AND-RE-ORIGINATES (a NEW frame with its own `route_stack[0]`, byte-identical inner §10.2, retains the pending-join association). Guard: the exemption is **R=0 ONLY** (R=0 structurally proves DIRECT-FROM-JOINER — any carrier appends its origin; an origin-BEARING GROUP_MGMT takes the normal path + MUST NOT be re-carried, g15 ≤1). Layer note: core's dataplane is below-L5 (TG-agnostic, signals only, never re-originates); hive's carve-out must hand the frame to whatever in hive-bin owns terminate-and-re-originate + the pending-join association (if that step doesn't exist at `route_frame` yet, the carve-out is "don't drop → hand to the join handler," and the join handler re-originates). Test: add the carve-out case to `router_integration.rs:131-142` mirroring core's :2101 — positive (route-less GROUP_MGMT → signalled/not-dropped) + negative control (route-less EVENT still drops).
- **SEVERITY = latent-vs-live (the discriminator decides severity, NOT the fix):** today hive drops GROUP_MGMT over UDP/BLE/LoRa (WS-only join) = the bug's symptom; **the first non-WS transport that carries a join turns it live with NO warning.** Latent still gets fixed. Nothing owed by hive until core lands the reference; then mirror (router + test) per the signal-only shape above. [[status-recorded-as-a-constraint]] [[marker-grep-cannot-see-comments]]

## Open threads (post-campaign, not blockers)

sensor-provider_capable canon CLOSED (bit2=0 all boards; pending 3-board metal re-score only) · conn-liveness watchdog
parked (keepalive covers it) · InvalidRouteLen CLOSED benign (the 2 beacon classes are OURS, 5511 FNV; canon-correct drops).

## Standing artifacts (LIVE on <build-host>, secret-bearing, off-tree) + safety

iter-8 pair `~/d4-init8.elf`/`~/xiao-acc8.elf` · D5 cosine `~/d5-cos5.elf` (11f2d2ef) · personas `~/.r2-dev-trial/`
(MACs off-tree) · v8.7.x ELFs+bins `~/d5-*-v87*.*` + `~/v87*-staging/`. Plain non-force pushes only; never
`--all`/`--mirror`. Three local keep refs preserve removed security material (do not repack/prune). Branches:
`storing-backend` (real WIP, needs rebase) · `hygiene-scanner-v2`/`platform-trait`/`v0.2-relay-handshake`
(stale/contained, do not merge). Key rulings in `DECISIONS.md`. Ops hazard: [[reference-xiao-boot-flush-wedge]].
- **★ DAEMON HYGIENE (supervisor order 2026-07-27, measured not remembered): HIVE owns ZERO persistent detached processes.** Verified: `lsof /dev/ttyACM0..3` = no tty holders; `pgrep -af` for espflash/openocd/probe-rs/nrfutil/cat-tty/tail-f/logger/while-loop/r2-hive = none mine; no user systemd path/timer units, no crontab, no udev r2 rules. Only transient Bash-tool tasks (finds/greps) that self-complete, touch no hardware/port/artifact, no trigger. **A long-running background process is an UN-REVOKED AUTHORISATION — a stale process ACTS (needs no reader).** FOUND (NOT mine, flagged to supervisor+composer): PID `2666788` `cat >> /tmp/d5-score.log` ALIVE since 07-25, socket-fed (no tty held), parent = the shared `fleet-r2` tmux root = the surviving D5 logger; composer's log-preservation completed but the PROCESS was never killed, and D5's return to the bus re-armed its appends — composer's to kill (D5 ttys are composer's), I did not touch it.
