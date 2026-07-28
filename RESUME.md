# RESUME — r2-hive

Updated 2026-07-28. `main` clean + pushed (ahead=0). **ONE current takeover snapshot — hard limit 65536 B; history goes to `RESUME-archive.md`, never here.** Full campaign history in
`RESUME-archive.md`. **Firmware lives in r2-core branch `dfr1195-fw-ensemble-cfg-dere`** — hive designs/builds/attests, never edits core.
**CORRECTED 2026-07-28: this line previously named `dfr1195-fw-blerole-coex`, which is STALE.** Measured: `git branch -r --contains c7a1d67a` (hive's pinned build sha) returns `origin/dfr1195-fw-ensemble-cfg-dere` and nothing else.
**★ CITE BRANCH AND SHA WITH EVERY `path:line` (fleet COMMS v22).** `platforms/dfr1195/src/main.rs` is **10129 lines** on the firmware branch and **83 lines** on `r2-core-consolidation` — a path alone is ambiguous across the two trees, and real work has been burned on it.

---

## ▶ TAKEOVER SNAPSHOT (read this alone if you are picking the lane up cold)

- **OBJECTIVE:** get the X1 OTA round-trip to complete. hive designs/builds/**attests** firmware; hive **never** writes to a board and **never** edits r2-core.
- **VERIFIED FACTS (measured, not assumed):** X1 runs **baked-A** — act 1 wrote **857088 B @ `0x20000`** app-only, exit 0; 3-limb metal control passed (MAC-fallback id gone, identity `cf1bf564` in all four emissions, build_id `otav3.A.baked.0727`); **reason 4 → 6** on a byte-identical payload ⇒ **`baked_persona` fixed the signer gate on metal.** Four artifacts attested (table in the X1 section). **Act 2 is blocked by a core-owned defect:** `read_anti_rollback` validates nothing, so X1's `0x18000` (ESP-IDF app text) decodes as `seq=1769304421` ⇒ **every OTA refused reason 6**, in front of the signer gate.
- **NEXT ACTION (not hive's):** composer runs the granted **D4 `0x18000` read** (D4 + D5 now on the bus). hive's part is already done and pre-registered — see the (c) interpretation, which is **binding**.
- **BLOCKERS:** (1) anti-rollback reason 6 — design **SETTLED** (tag-only, on authentication grounds), fix is **core's**; (2) **residual downgrade risk OPEN**, closable **only from sectors, never from logs** — writer trace narrows the population but **is not a census**, X1 is one board, D4/D5 unmeasured.
- **OWNED CHANGES THIS SESSION:** `RESUME.md`, `RESUME-archive.md`, `DECISIONS.md`, `ci/public-hygiene.sh`, `.github/workflows/public-content-hygiene.yml`. **No r2-core edits. No board writes. No detached processes** (hive daemons = ZERO, enumerated by command).
- **CHECKS:** `bash ci/public-hygiene.sh` green (214 files). Pre-push hook chain green. Every commit carries a `Decision-Log:` trailer.
- **DECISIONS:** `DECISIONS.md` **D-20260728-01** (evidence durability; supervisor-attributed; their fleet IDs D-20260727-76/-77). Ledger beats this file on any conflict. Live constraints: **#d003** RAK freeze, **#d005** build gate.
- **★ WRITE HOLD IN FORCE (2026-07-28):** supervisor holds ALL r2-hive writes except **`RESUME.md`** (queue + state only). See **⏸ WRITE HOLD + QUEUED WORK** below — five approved/owed items, each with its falsifier or proof shape. **Nothing in that queue has been applied.** Do not start item 1's warning, the rc fix, or the exit-code contract until supervisor lifts.
- **BRANCH / UPSTREAM / PUSH:** `main` → `origin/main`, **ahead 0, behind 0**, working tree clean at push time. Nothing unpushed.
- **▶ CURRENT ASSIGNED WORK (Roy, 2026-07-28, READ/MEASURE/REPORT only):** *which spec clauses have never executed on metal?* Three states — OBSERVED ON METAL / COMPILED BUT NEVER EXERCISED / NOT PRESENT. Findings so far in **🔬 METAL-EXECUTION AUDIT** below. **UNKNOWN is a legitimate answer; state denominators.**

## 🔬 METAL-EXECUTION AUDIT (2026-07-28/29) — findings, no writes outside this file

### F1 — SCF observability is NOT PRESENT in any surviving image

All four store-carry-forward / silence emit sites are gated on `feature = "fr4"`. In r2-core branch
`dfr1195-fw-ensemble-cfg-dere` @ `c7a1d67a`, `platforms/dfr1195/src/main.rs`: `#[cfg(feature = "fr4")]`
at `:2278` and `:3257`; emits at `:2304` (`msg.scffwd`), `:2311` (`SCF-DROP` println), `:2324`
(`msg.silence`), `:3279` (`msg.hold`). `emit_msg` itself is `#[cfg(any(feature = "routetest", feature = "fr4"))]`
at `:4535`.

**Measured across all 7 ELFs on disk** (`~/x1-census-c7a1d67a.elf` + 6 in `~/r2-fw-archive/`, dated
2026-07-14): markers `SCF-HOLD` and `SCF-FWD` = **0 in every one**. `msg.*` literal sets differ by config:

| ELF / config | `msg.*` literal set |
|---|---|
| census `c7a1d67a` — `otal2cap,lora,xiao,benchsf7,baked_persona` | **none** |
| `d4-fakesensor-benchsf7-b233a003`, `d4_fakesensor_DEV_79ce66c9` | `tx, rx, relay, delivered` |
| all 4 XIAO `xiaobridge,benchsf7` (±`dev`) | **none** |

**Controls, both ends** — the zeros are absence, not instrument failure: the source at the pin *has*
`msg.hold` / `msg.scffwd` / `msg.silence` as literals (1 each; tree clean at `c7a1d67a`), and `strings`
*does* find `msg.*` in the DFR ELFs. `hci_msg.o` excluded as BLE-blob filename noise, not an event.

**The builds that DID exercise SCF on metal have no surviving ELF.** `docs/field-results/fr4-lifecycle-0623/TN-FR-4.json`
names them only as *"staged ELFs fr4-sensor/router-bridge/receiver"* at `r2-hive 884b866` **plus two
metal-found fixes that were never pinned**. So the June result cannot be re-derived from any artifact.

### F2 — metal evidence for SCF, raw-log vs JSON-text discriminated

Corpus: **44 files, 10 campaigns** under `docs/field-results/`. A JSON `predicted`/`observed`/`notes`
field naming an event is **not** metal evidence; only a raw `.log` line is. Counting only `.log`:

- `msg.hold` — **27 lines / 5 logs** → OBSERVED ON METAL
- `SCF-DROP` (TTL expiry) — **4 lines / 2 logs** → OBSERVED ON METAL
- `msg.scffwd` — **1 line / 1 log** → OBSERVED, n=1 (sufficient: emission is DETERMINISTIC given the branch is reached; says nothing about rate or timing)
- `msg.silence` — **ZERO raw-log lines.** Its only two hits in the corpus are JSON `predicted` and `notes` text.

`TN-FR-4` records `E4_silence_is_signal` **pass=true** with a detailed CBOR-key observation. Its own
`notes` say that leg came from **composer's `/r2` WS stream**, which hive does not hold. So E4 is
**OUT OF MY CORPUS, not refuted** — the null carries its scope, never its explanation.

### F3 — the cap-evict path is INSTRUMENT-BLIND, not merely unobserved

`:3275-3276` evicts the oldest entry when the bounded SCF buffer (`heapless::Vec<_, 8>`, `:2022`) is full:
`if scf_buf.is_full() { scf_buf.remove(0); }` — **oldest-first, exactly as specified** (an earlier suspicion
of `swap_remove` was refuted by reading). But it has **no emitter and no println**. `:3279`'s `emit_msg("msg.hold", …)`
belongs to the PUSH, not the evict. So this path **cannot be assigned any of the three states from logs** —
absence of evidence here is guaranteed by construction, regardless of what the hardware did.

**Routed:** cap-vs-TTL to core (endorsed: emit on evict). Discard-event canon question to specs — *is a
discard event already required?* core holds until that returns. Nothing further owed from hive on it.

## ⏸ WRITE HOLD + QUEUED WORK (2026-07-28) — approved, drafted, NOT applied

**SUPERVISOR WRITE HOLD on r2-hive is IN FORCE.** No code, no gates, no patch files, no artifacts, no
scans of the tree. **`RESUME.md` is EXEMPT** (supervisor, 2026-07-28) for queue + state recording, and the
exemption **requires the push** — an unpushed RESUME is the same defect one level down, because the remote
is the recovery storage. *Reason for the exemption, worth keeping: a hold stops CHANGE and does nothing to
preserve KNOWLEDGE; by blocking the only durable place to put this queue it made the state it was
protecting less recoverable than no hold at all. **A hold must never block the recording of what is held.***

**Why the hold exists:** core repaired its own `ci/public-hygiene.sh` (`d273ab99`) after finding it could
read a scanner ERROR as CLEAN, then re-ran across branches. `dfr1195-fw-ensemble-cfg-dere` @ `5b5b55ee`
shows four blocking classes — and **that branch has never carried the gate at all** (`ls-tree` empty, zero
runs). core holds the only working instrument and owns the scan; **the response to anything already public
is Roy's call, so no cleanup, no force-push, no history rewrite, no quiet fix from here.**

**Each item carries its falsifier or proof shape, so a takeover inherits the WHY, not just the title.**

1. **FROZEN-patch warning — OWED, text drafted, not landed.** Add `docs/dfr1195-patches-FROZEN.md` plus one
   header line in each of `docs/dfr1195-{firstlight,s3-validation,ble-cargo}.patch` (patch files tolerate
   leading comments before the first `diff --git`). Content: **FROZEN at r2-core `c46383e`**; the sync
   **was live** (regenerated, byte-matched vs `git diff c46383e..HEAD`, once found stale by 87 lines) and
   was allowed to lapse **deliberately**; the regeneration history in `RESUME-archive.md` **invites
   resumption with a worked example**; re-establishing it **requires a drift gate first**; base `c46383e`
   is far behind firmware tip `c7a1d67a`, so a resumed sync starts stale while looking like continuity.
   - *Proof shape:* nothing consumes them — `ble-cargo.patch` has **zero** references; the other two appear
     only in `RESUME-archive.md` and `docs/dfr1195-first-light-findings.md`, which **cite** rather than
     apply; the single `git apply` hit is `--reverse --check`, **a recorded verification, not a directive**.
   - *Falsifier:* any script, doc or CI step that **applies** one of these patches ⇒ it is a live stale
     artifact, not an archive, and needs a gate rather than a note.
2. **`~/build-census-c7a.sh` rc discipline — APPROVED as specified, including the omission.** Capture
   cargo's rc **directly, no pipe between producer and status**; check the `cp`'s own rc; **three-state
   exit 0 pass / 1 FAILED / 2 CANNOT REPORT**; **translate** cargo's rc so `101` cannot escape wearing this
   script's contract. **NO pre-run stamp.**
   - *Demonstrated state:* `:21-22` pipes `cargo build` into `tail -2` and `:3` is `set -e` with **no
     pipefail** ⇒ **the pipeline status is tail's; a failed build reads as success.** No bad artifact
     resulted only because `rm -rf` at `:18` had already removed the stale ELF, so the `cp` had nothing to
     copy and aborted. **The currency guarantee is carried by an accident of ordering, not by a check.**
   - *Why no stamp:* `rm -rf` precedes the build ⇒ **cargo rc=0 IS the currency proof**; a stamp would cry
     STALE on a healthy repeat run.
   - *Withdrawn, do not re-assert:* any claim the build **ran cleanly** or that the ELF matches the source
     sha **by construction**. The artifact is verified by **inspection of the bytes** (supervisor
     re-derived the marker offset from section headers; 305 high-entropy 32-byte windows inside the persona
     region, **zero outside**) — **content-based verification survives a broken producer; process-based
     does not.**
3. **Exit-code contract — CLASS fix, not an instance patch.** `ci/check-vendored-vectors.sh` exits **1 for
   BOTH drift and cannot-verify** (its own usage line `:19` says so; `:77` cannot-verify, `:101`/`:105`
   drift) ⇒ a CI run without the sibling checkout is graded **DRIFT FOUND** having compared nothing, and a
   real drift is indistinguishable from an infrastructure failure. Split **0 / 1 DRIFT / 2 CANNOT VERIFY**.
   - *The class:* `ci/public-hygiene.sh` separates engine-error (`rc>1` propagates) from finding (exit 1);
     its sibling does not. **Same repo, same family, two contracts** — land ONE contract across both.
   - *Control method:* assert the **EXACT** expected code, never merely non-zero (composer caught a control
     that never executed by expecting 3 and getting 2).
4. **`gg_filter` comment + executed control** (`ci/public-hygiene.sh:107-108` @ `dc713fd`). Its rc 1 → 0
   conversion is **load-bearing in both directions**: if it propagated grep's own status, pipefail would
   surface rc 1 on the **clean no-match** path and `set -e` would abort the gate on a healthy tree. Real
   protection that **reads as accidental** — comment it and add an executed control. **Not a code change.**
   - *Proof it works today:* core's differential on a throwaway clone — invalid regex forcing `git grep`
     rc>1 ⇒ perturbed run **exit 128, no `OK` line**; unperturbed **exit 0**; vacuity guard asserted first.
     My file at `dc713fd` is sha256 `bc76ab779640e1c30da253aa5d6d99af8d354f52ff80a916809bee0f308b1bf0`,
     byte-identical to what core tested.
5. **Selftest-layer swallow idioms — residual, non-blocking.** Production sweep is fail-closed (item 4).
   The 14 `|| true` sites (`:99 :482 :488 :684 :686-691 :700 :724 :774 :794`) are in the **selftest/KAT**
   layer: a matcher that **crashes** inside a KAT expecting zero hits **reads as PASS**. The control layer
   still carries the defect the production layer fixed.

## g23 VALUE-SCRUB (2026-07-27) — LANDED, closed → `RESUME-archive.md`, "ARCHIVED 2026-07-28 (compaction, pass 2)"

- Roy's ruling stands: **keep gates public, scrub VALUES, FORWARD-ONLY.** Gate + denylist mechanism is live; `ci/public-hygiene.sh` enforces it every push. Full method, denominators and the preserve-before-scrub ordering are in the archive.
## ‼ READ FIRST — device + authorization state (ledger wins any conflict below)

- **★ NAMING (Roy 2026-07-27) — TWO AXES, don't collapse ([[carrier-role-vs-device-identity]]):** DEVICE IDENTITY = per-device, BUS-BOUND (`X1`/`D4`/`D5` = efuse MAC) — the ONLY thing to key a rig-map/persona/device-record on. ROLE = per-ensemble, NOT unique (`radar-sensor-xiao`=X1 real-hw; `complex-hive-xiao`=Android TG peer, OFF bus; `sensor-simulator-dfr1195`=D4/D5, **SIMULATED sine — fabricates readings**) — a carrier+ensemble descriptor, NEVER a key (collides when 2 share a role). "THE XIAO" RETIRED. Don't rename records by inference — mark AMBIGUOUS; resolve only where a MAC/reading/session determines it. No asserted-MAC row for an off-bus board. A SIM reading read as REAL > worse than an ambiguous name.

- **DEVICE (D5 / DFR1195 ESP32-S3):** runs **v8.7.3 at `513c949db0f9ec0eebbf7d6df3febec39561a13a`** — flashed and verified.
  The OTA-coex hang campaign is **CLOSED** (double-fault family verdict RULED AND RATIFIED).
- **★ PER-BOARD CUSTODY SNAPSHOT (2026-07-27, from RECORDS — NO live device read; digest not filename):** **D5 = `adc6cc18…cbd68`** (`d5-otarx-v873.bin`, 878864 B, v8.7.3 coex `otal2cap`; B/otafail pair `8a1ee68e`). **D4 = UNKNOWN** — no flash record in my custody; `cbd6bf67` (benchsf7) + g18 `30ffcdfd` were BUILD+ATTEST ONLY, never granted/flashed; a record-vs-metal SF mismatch (D4 ran SF12/`lora_dr=0`) is unexplained by any image I recorded flashing ⇒ NO digest asserted. **X1 = `0431d07c…6963`** (baked-A app image, 857088 B, written act 1 at literal `0x20000` app-only under grant, exit 0) — **the only board with BOTH a pinned flashed digest AND metal-confirmed identity** (3-limb control: fallback id gone, `cf1bf564` in all four emissions, build_id `otav3.A.baked.0727`). Prior/looser records: records show otal2cap v3 A `ae5fadb3` flashed to ota_0 during the OTA run + g18 x1-xiaobridge `cee2f004` build-only — defer to composer, no fabricated digest.
- **★ v8.7.3 COEX WiFi-STA ATTESTATION (PINNED, source-only, for a held Roy ruling 2026-07-27):** feature set (authoritative, `~/build-v873.sh`) = `bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive,otal2cap` (+`otafail` on B) — **`staota` ABSENT**. At **r2-core `513c949d`** (`git cat-file`-verified; `git show 513c949d:platforms/dfr1195/src/main.rs`): the infrastructure-join is `#[cfg(feature="staota")]` @934 (env creds→lab WiFi) + `#[cfg(feature="staota")]` DHCP @~987 ⇒ **NOT COMPILED**. A STA client IS compiled under `ble` (`wifi::new`, `wifi_task`@997, Station cfg) but targets the synthetic self-mesh SSID `r2-tn-form`@937 and its `connect_async()`@9024 fires only on `DATA_PLANE_JOIN`, which the in-source fw-doc @7313 states never fires when staota is absent. ⇒ **v8.7.3 coex compiles a STA client but NO infrastructure-join, and NEVER associates** — canon sec3.1.3 (bars joining infra WiFi) does not reach it. **NO sensor build was ever flashed to D4/D5** (recipe scaffold, no i2c sensor plugin binds the S3 DFR) ⇒ that WiFi question has no subject. [[marker-grep-cannot-see-comments]] (attested from the recipe + cfg-gates, not a `strings` count) [[positive-control-the-tree-not-just-the-tool]] (pinned ref, not the working tree).
- **★ BOARD-CLASS WiFi QUESTION CLOSED (supervisor 2026-07-27, ACK on the first PINNED attestation in the chain): NOT going to Roy — no bench board joins infrastructure WiFi, nothing to except.** OPEN ITEMS: (1) **D4 IDENTIFIED then SOURCE-ATTESTED (2026-07-27):** composer's passive banner read = `build_id coex.iter9.0723` — coex-FAMILY but EARLIER than the v8.7.3 (`coex.v873.0725`) I first attested; supervisor had over-generalised the v873 attestation to D4 (the "bench does X without naming the image" error) and corrected it. I attested the ACTUAL image, **PINNED: r2-core `70960dbc`** (iter-9 #d013; recipes `~/build-iter9pair.sh`/`~/build-d5-iter9.sh` carry `BID=coex.iter9.0723`→`70960dbc`). Feature set `bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive` — **no `staota`, no `otal2cap`**. Infra-join `#[cfg(staota)]`@621/675 NOT compiled; compiled STA targets synthetic self-mesh `r2-tn-form`@624; **STA NEVER associates — `DATA_PLANE_JOIN.signal()` has ZERO call sites @70960dbc** (only `.wait()`@8055; comments @761/5395/8052), `connect_async` unreachable; M8c-join-suppress `56d39498` in ancestry. **The `.40` (composer-read STA IP = the ESP-SoftAP-subnet `.40` host) = a STATIC self-assign** (`ipv4_static`@667, `<softap-subnet>.<mac_low3&0xFF>`@664, gw `.1`) — assigned unconditionally, NOT a DHCP lease, association-independent (subnet = the espressif SoftAP RFC1918 default; literal tokenized to satisfy the IP-shape gate). ⇒ HOSTILE reading (iter9 joins infra) **refuted at source**; canon sec3.1.3 does not reach D4 either. Still **NO flashed-image digest** (banner build_id ≠ a digest; no flash record) — do NOT infer one. ★ RESIDUAL RE-CHARACTERISED (source attest @70960dbc): the "D4=SF12" reading is likely a **METRIC MISREAD** (same class as the `.40`). **`lora_dr` is NOT the spreading factor** — it prints `LORA_TX_DROPPED` (main.rs:1074/215) = `lora.tx_overflow()` (:5947), a rate-based SHED counter; the in-source comment (:1065-66) says `lora_dr>0` is the expected steady state AT SF12, so `lora_dr=0` is consistent with **SF7** (no shed) or idle — the opposite of the "SF12" reading. **`benchsf7` DOES gate SF: `cfg.spreading_factor=7`@5816** (else as923 SF12 default @5798); iter9 has benchsf7 ⇒ SF7. No runtime SF override in source (boot-static). Banner "LORA-ROUTE up (SF{sf})"@5824 prints the SAME `cfg.spreading_factor` variable ⇒ banner≠code EXCLUDED for the SF value. **Real SF ground truth = D4's LORA-ROUTE boot-banner SF{n}** (composer's console read, same instrument as the build_id) — asked. If SF7 ⇒ anomaly dissolves; if SF12 ⇒ look at the 5606 `as923_nz` VERBATIM second cfg path (deliberately SF12 to match the RAK, NOT benchsf7-gated), a separate path not a benchsf7 failure. [[marker-grep-cannot-see-comments]] (2) **X1 full sha died with the overwritten grant file** — composer holds an ABBREVIATED digest only; grants now archived before overwrite (won't recur).
- **g18 (NEW, 2026-07-26):** D4 + X1 fault-forensics **rebuild at `8530327309b82fdc0707063b72a8c00c0166a9c6`** — BUILT +
  attested, **both variants ELIGIBLE=YES**. See the g18 build record below.
- **AUTHORIZATION (authoritative — the GRANT FILE is the instrument; re-read it, do not trust this line alone). CURRENT @ 2026-07-28 07:10: NO LIVE GRANT — NO DEVICE ACT PERMITTED.** The D4 read is COMPLETE and its grant **RETIRED** (hive-verified on the file: `expires=1` = epoch 1970 unsatisfiable, PLUS sentinel `artifact=__no_active_grant__` / `target=__no_active_target__` which no real command can contain — **fail-closed by BOTH clock and match**). **Supervisor adopted RETIRE-ON-COMPLETION as standing**, so a spent grant no longer lingers as a live-looking authorisation — the stale-authorisation class this lane has been sweeping for, fixed at the source. **Historical (the retired grant's bounds):** — read EXACTLY `0x18000` len EXACTLY `0x1000` (zero margin: `0x18000+0x1000 = 0x19000` = CCR1, which the length is what keeps it off), target **D4**, **OPERATOR: composer** (NOT hive), Roy authorised explicitly. `artifact=2026-07-28-antirollback-0x18000.bin`, **no `sha256` field DELIBERATELY** (a read cannot pin the digest of its own output; reported after, never asserted before). Capture lands DURABLY at `~/.local/share/r2-bench/captures/D4/` — first application of the new capture policy, NOT a scratchpad. **NO WRITE OF ANY KIND, no erase, NO REPAIR — specifically do NOT write a tagged `floor=0` over untagged bytes** (it would destroy the population evidence the read exists to collect). No other board — **D5 is a legitimate third data point and is NOT granted.**
  **X1 acts SUSPENDED, NOT REVOKED, for a MECHANICAL reason:** `_hs_authorized()` parses the whole file and the LAST `target=` wins, so two targets cannot coexist — a D4 clause under the X1 header would SILENTLY RETARGET the X1 grant. Prior grant archived COMPLETE at `grant-history/2026-07-27-x1-baked-persona-acts1-2B-2R-SUSPENDED.grant`. **Act 2B (baked-B over air) remains a LIVE DECISION, to be re-issued once core's fix lands — not withdrawn.** Act 2R moot (no image applied). **Nothing was spent under the X1 grant except act 1 and act 1b.**
  **ONE GRANT AT A TIME.** There is NO `cbd6bf67` flash grant (no D4 *flash* — brick-history board, needs Roy's explicit go regardless); any draft naming `cbd6bf67` with a HOST target is VOID (a grant target MUST be an opaque DEVICE handle, never a host). RAK relay image `858bc638` STAGED for Roy STEP3, never flashed. **Ignore any stale "on metal / grant live / flash-auth being written" language — a grant exists ONLY as a supervisor flash-authorization file.**
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

## RAK #d001 · ENSEMBLE phase · (detail → RESUME-archive.md, "ARCHIVED 2026-07-28 (compaction)")

- **RAK compact-relay #d001 CONFIRMED by measurement** (rebuild of `70f442b9` reproduces `e5c7073e`); fix = `set_relay_egress(RelayEgress::SameCarrier)`. Image **STAGED, never flashed**. **#d003 RAK FREEZE stands: no further RAK builds without a fresh order naming a pinned sha.**
- **Bench-mesh regression: UNDIAGNOSED.** The SF-split root cause was withdrawn — it rested on a `lora_dr` misread (`LORA_TX_DROPPED`, a shed counter, NOT the spreading factor). ALL-SF7 policy stands on airtime grounds regardless. Real SF is disclosed only in D4's boot banner.
- **ENSEMBLE phase — trace-first, source-only, no build (Roy directive).** Q1 score-load + Q2 §2.1.2 shared-singleton registry are **MUST-BE-BUILT on MCU** (both live only in the STD-ONLY `r2-ensemble` crate, unlinkable `no_std`). Verdict: all-as-ensembles is a PLATFORM feature first, not five scores on an existing seam. Parked pending Roy.

## ★ X1 BAKED-PERSONA OTA — ACT 1 LANDED + PASSED; ACT 2 BLOCKED (anti-rollback). Nothing touches a board until Roy is up.

**Derivation trail (attestation reasoning, A-vs-B delta accounting, tree-relationship proof, the retracted legacy-leg debate) archived verbatim → `RESUME-archive.md`, "ARCHIVED 2026-07-27 (night)".** This block is current state only.

**WHERE X1 IS NOW.** Running **baked-A**. Act 1 (composer, under grant): **857088 B written at literal `0x20000`, app-only, exit 0.** Positive control passed on **three limbs** — MAC-fallback hive_id `00fbe520` GONE; identity `cf1bf564` exact in all four emissions; build_id `otav3.A.baked.0727`. Then the single-variable proof: **reason 4 → reason 6** on a byte-identical payload, same signer, same bearer. ⇒ **`baked_persona` fixed the signer gate ON METAL; reason 4 is gone.** Board healthy, identity intact.

**★ WHAT THAT UPGRADED (static-vs-runtime, kept because the axis matters):** hive's attestation was a **STATIC** claim about the ELF and the `.bin`; the three-limb metal read is a **DIFFERENT, RUNTIME** claim about the board. Both now discharged; they were never the same claim.

**ARTIFACTS — pinned, all attested by hive, none written except A.**
| artifact | source | sha256 | size | persona @ | notes |
|---|---|---|---|---|---|
| A ELF | r2-core `4b4a71e5` | `4e595a45…49ac8` | 1363108 | `0xb904` | attestation artifact |
| **A .bin** (Roy produced, composer wrote) | ↑ | **`0431d07c…6963`** | **857088** | `0xa904` | **on X1 now** |
| B ELF | r2-core `b25a21eb` | `22e497cd…3bf3` | 1363108 | `0xb950` | built, never written |
| B .bin (Roy produced) | ↑ | `965419ba…ac5d` | 857104 | `0xa950` | staged only |

- Every image: blob **verbatim, single occurrence**, extracted-336B sha == `243ab04…426e`; **tg_pk `4e2a9a30…7706` ×2, both inside the persona region**; persona lands in **seg0 = DROM/.rodata**, loadable (PT_LOAD), not DWARF, not padded, not merged.
- **Identity:** dev TG `2e98fddb…`, tg_hash `0xcf1bf564`, blob 336 B `243ab04…426e` (SECRET-class: gitignored, digest-only, never committed). **Same blob in BOTH images** — a payload carrying a different persona is NOT authorised.
- **A and the unbaked B have IDENTICAL CODE** (`b25a21eb` = `4b4a71e5` + one comment-only commit; comment-stripped diff empty) ⇒ no hidden second cause; behaviour deltas isolate to the persona variable plus benign panic-string offsets.
- **Partition table:** `platforms/dfr1195/partitions.csv` sha256 `e0e49127…627c`, **ota_0 @ `0x20000`** — the one value both tables agree on and all act 1 needed. Both lanes use it. X1 still carries composer's xiao-wio template on-flash; writing the dfr1195 table = a SEPARATE partition-table write needing its own grant.
- **Role omission = STATUS QUO, not regression** (unset `DFR_ROLE_PATH` ⇒ derived fallback, identical to today's non-baked A).

**⛔ ACT 2 BLOCKED — anti-rollback record has no magic (core-owned defect, composer measured on metal, hive verified at `4b4a71e5`).** `read_anti_rollback` (main.rs:8076-8088) validates nothing but `0xFFFFFFFF`→0, so X1's 0x18000 (holding ESP-IDF app text, zero 0xFF ⇒ its writer never ran) yields `current_seq=1769304421` / `floor=543450482` ⇒ **every OTA refused reason 6 StaleSeq, in front of the signer gate.** `read_ota_pending` two sectors away DOES validate `OPND` — and the threat is **named in prose twice in the same file** (`:6203 CT1\0`, `:8147 RBK1` "guards an erased/foreign sector reading as valid"); **the guarded neighbour is the less critical one.** **9 of 11** config-plane readers already validate a tag ⇒ the fix is **conformance, not invention**.
- **DESIGN: SETTLED — tag-only, on AUTHENTICATION grounds** (only record identity authenticates; a 0xFF-tail test is a shape heuristic and fails the same criticism as the reader). Correct **even if** legacy records exist. hive's legacy-leg **RETRACTED**. Both halves must be invalidated together — a seq-only repair trades reason 6 for reason **12** RevokedAuthority.
- **RESIDUAL RISK: OPEN — and closable ONLY FROM SECTORS, never from logs.** Writer trace narrows the population (only a completed confirmed OTA can write one) but **is not a census**; X1 is **one** measured board; D4/D5/RAK unmeasured. Honest wording: **"no legacy record has been observed anywhere, and no exhaustive search has succeeded"** — do NOT upgrade to "none exist".

**STANDING CONSTRAINTS IN FORCE**
- **NO auto-repair before the D4 read** (pinned in the GRANT, not just the brief): the fix MUST NOT write a tagged `floor=0` over untagged bytes — *the repair would erase the question* on every board it lands on. **Order: measure D4 → fix → any repair under its own grant.** No repair grant will be written first.
- **Fail closed, but NOT silent:** `(0,0)` is the right value and the wrong UX — distinguish non-erased-untagged from erased and surface it (diagnostic, better a health field). **The countability property is worth more than the diagnostic**: it makes the affected population measurable in the field with no grant per board.
- **NO brief, ledger entry or report may cite a log archive as if one exists** (supervisor-decided, binds now; **LEDGERED: `DECISIONS.md` D-20260728-01**; supervisor's fleet IDs D-20260727-76/-77 — the ledger is authoritative, this line is a pointer). Retrospective device questions → **measure hardware** or **UNANSWERABLE**. *"The logs show no X" is not a claim.* hive swept its own 214 tracked files: **0 violations** (one pre-policy contemporaneous console read is now permanently unverifiable — flagged, not rewritten).
- **De-provision falsifier: DEFERRED, unspent, NOT scored either way** — it needs the unbaked payload to boot, and it would strand the board identity-less.
- **hive artifacts carry the defect** (it is in the source both were built from). **Attestations stand** — the artifacts are exactly what they were attested to be.

**★ D4 ANTI-ROLLBACK READ — COMPLETE; RESULT (a); GRANT RETIRED (2026-07-28).** Grant `.fleet/flash-authorization` (06:38): read EXACTLY `0x18000` len EXACTLY `0x1000`, target **D4**, **OPERATOR composer (NOT hive)**, capture → `~/.local/share/r2-bench/captures/D4/` (first application of D-20260728-01). **NO write/erase/repair of any kind**; D5 explicitly NOT granted. **D4 is NOT on the bus** (composer efuse-resolved 06:45, only X1 attached) ⇒ **read not performed, grant UNSPENT**, Roy has the cable question. **X1 acts SUSPENDED not revoked** (mechanical: `_hs_authorized()` takes the LAST `target=`, so two targets cannot coexist); **act 2B remains a live decision, to be re-issued once core's fix lands.**
- **Outcomes pre-registered before the measurement (all three informative):** **(a)** foreign/app data, few or no `0xFF` (X1's shape) ⇒ systemic, no legitimate record here either; **(b)** entirely `0xFF` ⇒ never written, migration trivially safe for this board; **(c)** **8 non-`0xFF` bytes then `0xFF` to the end** ⇒ **A LEGITIMATE LEGACY RECORD EXISTS, the downgrade window is REAL, a migration provision becomes mandatory.** Count the bytes — virgin vs written-once differs by exactly eight. **A fourth shape must NOT be forced into the nearest class.**
- **⚠ hive's stated reason for "(c) is hard to fake" was WRONG and is WITHDRAWN; the conclusion survives on a BETTER mechanism (composer refuted it BEFORE the read; hive verified the refutation on its own bins).** ~~segments pad to ≥16 so byte 8 is unreachable~~ — **FALSE:** segments are **4-byte** aligned; measured segment data-ends `0x20018` and `0xc1ee8` in BOTH images are **≡8 mod 16**. **REPAIRED MECHANISM (verified independently):** `esp_image` pads so the checksum is the last byte of a 16-multiple (+32 B hash, itself 16-aligned) ⇒ image **TOTALS ≡0 mod 16** (857088, 857104 both); **app partition bases are 64KB-aligned** ⇒ `base+total ≡0 mod 16` in all 8 combinations checked. (c) needs `end ≡8 mod 16`, which a padded image at an aligned base **cannot produce**. Stronger than hive's original: it rests on the image format + base alignment, **not on segment internals**.
- **BOUNDARY, pre-registered rather than discovered later:** the padding argument covers a predecessor that was an **ESP APP IMAGE** (X1's app-text sector is that family). It does **NOT** cover prior content from a non-image writer (raw dump, other-tool data write, hand-written blob) — if D4's sector is not app-image-derived, the argument must be **RE-DERIVED for that origin, not inherited**.
- **★ RULE BANKED (hive's, adopted by both peers): AUTHENTICATION MUST SURVIVE AN ADVERSARY; A DIAGNOSTIC ONLY HAS TO SURVIVE COINCIDENCE.** That is why the leg-retraction (a shape cannot authenticate ⇒ firmware must not accept on tail-looks-erased) and (c)-as-evidence (a human characterising a sector already in hand) stand together with no contradiction. **A retraction does not travel to every use of the fact** — the same observation can fail at one bar and pass at another.
- **✔ RESULT: D4 = (a) FOREIGN/APP DATA (composer read 2026-07-28; hive VERIFIED INDEPENDENTLY off the durable capture, every value reproduced).** `~/.local/share/r2-bench/captures/D4/2026-07-28-antirollback-0x18000.bin`, sha256 `1cbc138a…5cfb`, 4096 B; **`0xFF` count = 1 of 4096** (single byte @1536); first16 = `'03d  err=%03d  s'` (app/IDF text to the final byte); firmware view **`current_seq=543437616`, `floor=1920099616`** (recomputed LE with the `0xFFFFFFFF`→0 map: MATCH). **(c) DID NOT FIRE.**
  - **TWO BOARDS, BOTH (a) ⇒ the defect is SYSTEMIC, not an X1 quirk** — D4 also silently refuses every delivery with reason 6, and **holds NO VALID RECORD NOW**.
  - **★★ WITHDRAWN — the `0xFF` SIGNATURE IS REFUTED AT SOURCE (composer's audit; hive verified at the pinned crate).** hive had recorded and PUSHED *"D4 has NEVER had a confirmed OTA — a real record leaves ~4088 bytes of `0xFF` and there is ONE."* **That inference is DEAD.** `esp-storage 0.6.0` `storage.rs:48-72` is **READ-MODIFY-WRITE**: `internal_read` the whole 4 KB sector → overlay the new bytes → `internal_erase` → `internal_write` **the entire buffer back**. The erase is real; **the sector is NOT left blank**. ⇒ a floor committed onto a sector already holding app text yields **8 record bytes + the app text PRESERVED**, which we would have filed as **(a)**.
  - **WHAT SURVIVES, ON REPLACED EVIDENCE — the FIRST-8-BYTES test:** the record occupies bytes 0..8, and on both boards those are printable ASCII continuous with surrounding text. hive checked the discrimination rather than asserting it: D4 `0..8` = `3033642020657272` = `'03d  err'`, decoding to seq 543,437,616 / floor 1,920,099,616; against that, realistic records are **not** printable (`seq=1` → `0100000001000000`, `seq=3`+epoch floor → `0300000000f15365`, `seq=39` → `2700000001000000` — all carry `0x00`, since a monotonic counter leaves high-order zeros). Text cannot be mistaken for a record, nor a record for text.
  - **CLAIM DOWNGRADED IN SCOPE:** ~~"never had a confirmed OTA"~~ (historical, unsupported — a record could have been written and LATER OVERWRITTEN by an app flash spanning 0x18000) ⇒ **"holds no valid record NOW"** (current-state, which is what the fix decision actually needs).
  - **TAXONOMY: class (a) is AMBIGUOUS BY CONSTRUCTION** — it conflates *no record* with *record + preserved foreign data*. **Only the first 8 bytes separate them.** The `0xFF` count is demoted to corroboration about the sector's HISTORY and **must never again screen for record PRESENCE**.
  - **hive's padding argument NARROWS FURTHER:** it only ever addressed *"can foreign data mimic the ERASED-SECTOR shape (c)"*. Since (c) is no longer the only shape a record takes, it is a **narrow anti-coincidence argument about one shape**, not a general record-detection argument — a record written over app text is **invisible to shape entirely**.
  - **★ NEW CONSEQUENCE — FAIL-LOUD IS NOW REQUIRED, NOT OPTIONAL:** a board holding a REAL record over app text would, under tag-only, **silently lose a VALID floor and look identical to a board that never had one**. That is exactly the population we most need to hear about, and it is the one that disappears quietly. Recommended to core.
  - **FIX DECISION UNCHANGED** (tag-only rests on AUTHENTICATION and never on this signature); for the two measured boards no valid record exists, so tag-only costs them nothing. **Any future screening MUST use the first-8-bytes test, not the `0xFF` count.** Writer-trace population argument gains a second consistent data point and **is STILL NOT A CENSUS**. **D5 is the third and is NOT granted** (hive not asking).
  - **★ FIRST PAYOFF OF THE CAPTURE POLICY (D-20260728-01):** a second lane re-verified the measurement hours later **from durable evidence, not from a transcript**. Under the previous arrangement it would already be unappealable.
  - **⚠ DEFECT IN THE PRE-REGISTERED CRITERIA (hive, flagged for the next read):** (a) is worded *"mostly printable, few or no `0xFF`"*, but D4 is **2045/4096 = 49.9% printable** — it FAILS the "mostly printable" clause while being decisively (a) on the `0xFF` clause. **The taxonomy discriminates on `0xFF` STRUCTURE; printability is corroboration about ORIGIN.** composer's classifier printed FOURTH SHAPE on that wording and **disclosed the near-miss rather than quietly reclassifying** — correct behaviour provoked by the criteria, not a bad instrument. Recommend restating (a) as *"few or no `0xFF` (origin typically app/IDF text)"*.
  - **✔ hive's ORIGIN BOUNDARY DISCHARGED ON EVIDENCE (composer; hive re-ran it with its OWN controls):** all five distinctive strings from D4's sector — `Group key expansion`, `IGTK key expansion`, `Pairwise key expansion`, `pp rom version`, `esp_rtos::start` — occur **1/1 in BOTH** the sector (4096 B) and a known ESP app image (857088 B); **two hive nonsense controls 0/0 in both**. ⇒ D4's predecessor content **IS app-image-derived**, same family as X1's, so the padding argument applies **without inheritance**. **SCOPING, so the discharge is not itself inherited:** (i) discharged **for D4's sector, not globally** — a future (c) needs its OWN origin check; (ii) string evidence measures **origin-of-bytes**, while the padding argument additionally assumes **manner-of-write** (an image write at a 64KB-aligned base) — same-origin bytes from a raw/truncated/misaligned write would pass the string test and fail the premise. **Moot for D4** (app text to the final byte, no image ending falls inside the sector), but the two are different claims and only the first is measured.
  - **★ RULE BANKED (composer's, from their own instrument failure): A BLANK CONTROL IS NOT A RESULT.** Their first origin check returned blank for every string **including the control** — indistinguishable from a broken command; only having a control at all revealed it. **The control's job is not to validate the finding, it is to prove the instrument was alive** — so it must be able to fail loudly and be checked FIRST.
  - **✔ PROPAGATION CLOSED (2026-07-28): supervisor MOVED THE BINDING POINTER** to the image-length-padding form and **restated criterion (a) onto `0xFF` STRUCTURE**, both carrying hive's scope caveat (padding argument covers **ESP-APP-IMAGE origin only**). It took **two lanes writing** before the governing artifact changed. Prior state: **a retraction is not done until the binding pointer moves** ([[deprecation-not-done-until-pointers-move]]). Carry only the image-length-padding form.

**✔ CONVERGED (supervisor, 2026-07-28). #7 IS BLOCKED ON ONE THING ONLY: ROY'S GRANT DECISION. Everything achievable without a board is done. hive stands by.**
- **FLEET EFFECTIVE UNION = ONE HOST-LOCAL HOOK + ONE CI JOB.** core NOMINAL (zero CI) · android NOMINAL · **hive EFFECTIVE-on-push-where `setup-hooks.sh` has run, zero CI** · composer the only CI-wired gate. Everything else is *capability*, not coverage. The number is now honest, which was the whole ask.
- **The convergent finding of the night (android's sentence):** *"my gates cannot fire unless I choose to fire them"* — the same sentence as the CI gap, reached from **four unrelated directions**. Four investigations converging on one missing thing is the signal it is a real gap and not a preference. **CI goes to Roy as ONE item; hive is not opening it.**
- **hive's position on #7, needing no archaeology:** attested ELF `c5e16d6d…1eff5` ready, **ELF-only**; **the `.bin` DOES NOT EXIST and MUST NOT be produced by the flasher**; hive is **not** requesting a grant and will not produce the `.bin` unprompted. Hold.
- **Falsifiers STATED ONCE and left to sit — do not re-litigate:** (1) hive's encoding lower-bound — a canonical value whose non-hex-run encoding (truncated / re-cased / hyphenated-UUID / base64 / prose) has a home no lane gates; (2) core's CI job over a deliberately stale mirror; (3) android's leg-1 presence-versus-equality.
- **hive corrections that landed in the anneal:** route-2 CLOSED (`0fe80cd`); the "untracked detector" claim **RETRACTED** — content is tracked, only activation is per-clone, and `core.hooksPath` would have **disabled the fleet secret-scan** (81 refs vs 3), so no action was correct.

**★ (superseded by the convergence above) ANNEAL — VECTOR/UNION THREAD CLOSED FOR METHOD WORK (supervisor, on Roy's call for convergence, 2026-07-28).** Settled rules ledgered at **D-20260728-02**; no further debate without a falsifier. hive's assignment out of the anneal: **route-2 and nothing else** — and route-2 is **DONE** (`0fe80cd`).
- **hive's one-line answer to the fleet question — CORRECTED 2026-07-28 (first wording WRONG, struck):** gated column is **EFFECTIVE ON ANY CLONE WHERE `scripts/setup-hooks.sh` HAS BEEN RUN**. `pre-push.local` is a **SYMLINK to the TRACKED `.githooks/pre-push`** (`ls -l`; I read "untracked" off `git ls-files` and never checked the target — asked git about the PATH, git answered about the PATH). Content tracked, activation one documented command, **zero CI invocations** — the honest gap is CI, as it always was. **`core.hooksPath` MUST NOT be set as a remedy:** it makes git run the tracked hook *instead of* `.git/hooks/pre-push`, which **IS the fleet secret-scan** (81 secret/scan refs vs 3) — the wiring fix would silently disable the secret failsafe. `setup-hooks.sh:8` documents why it chains instead. hive never set it (local/global/effective all UNSET, verified); nothing to undo. See `D-20260728-02` for the standing rule: **prove a cross-repo defect with a DIFFERENT CONSTRUCTION than the one that reported it, and name what else the fix touches.**
- **Route-2 CLOSED:** 11 transcribed literals pinned individually + orphan check; both negative controls fire; the first version (`pinned > 0`) was a **false green** an aggregate assertion let through.
- **Union:** hive gates 4/40. Holes `0053a1b2…`=2, `851fdee3…`=1. Reconciled with composer: two values covered 3/3 by the union, **`425ed4e4…` is the one real hole** (transport-relay, gated by nobody).
- **Stated once with a falsifier, not re-litigated:** content-addressed matching sees **one encoding** — truncated/re-cased/hyphenated/base64/prose forms are invisible, so **every hole count is a LOWER BOUND**. *Falsifier:* a canonical value whose non-hex-run encoding has an ungated home.
- **★ THE ACCOUNTING THAT ENDED IT, recorded because it is the point:** every finding was real and **NOT ONE MOVED TASK #7**. The attested ELF (`c5e16d6d…1eff5`) has sat since attestation with **no `.bin`, no board, no grant**. *A correction thread is finished when the next correction costs more than the defect it finds.*

**MORNING QUEUE (nothing touches a board until Roy is up)**
1. **D4 `0x18000` sector read** (grant needed) — the only thing that moves the residual risk.
2. **core:** implement tag-only + no-auto-repair + fail-loud; `read_board_profile` `0x13000` is the other unguarded reader (cosmetic, same class, core's call on scope).
3. **composer:** durable per-board capture directory, appended-never-recreated (supervisor DECIDED yes) — every reflash currently destroys answerable history permanently.
4. Act 2 remains VOID until the floor is resolved.

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

## ⚠ GATE-AUDIT GAP (2026-07-28) — grant-authorised but NOT gate-audited; hive-measured, wider than first reported

composer-codex replay-confirmed that wrapper-script indirection bypasses the fleet firmware gate entirely (the outer command carries neither the tool token nor the artifact string). **hive measured the audit trail and it is worse than the read alone:**
- `.fleet/flash-authorization.log`: **last gate record `2026-07-27T13:48:32`**; entries dated 07-28 = **0**; after 18:00 on 07-27 = **0**; naming `2026-07-28-antirollback` = **0**; naming `x1-otav3*` = **0**.
- **⚠ DENOMINATOR CORRECTION (hive's own figure was wrong, and so was composer's):** hive first said 787 entries, composer 791 — both right about different things (`wc -l`=791, `grep -c .`=787, 4 blank lines), neither saying which. **Underneath that, the file is TWO LOGS IN ONE:** only **90** are tab-delimited gate-emitted `USED` records (first `2026-07-20T09:06:42`, last `2026-07-27T13:48:32`); the remainder is supervisor **prose narrative** (`… supervisor: #d011 grant written …`), and `grep -c USED`=132 over-counts by matching prose that mentions the word. **The machine-audit denominator is 90, not ~790.** Conclusion untouched; any claim about the trail's COVERAGE was resting on a mixed count.
- ⇒ **ACT 1 — the 857088 B WRITE to X1 — is unaudited too**, as is act 1b. Every device operation since 13:48 on 07-27 is grant-authorised but absent from the log, **including the only write in the sequence**.
- **NARROWED BY MEASUREMENT (composer), (a) NOT (b) — without excluding (b):** composer's first wrapper was created `20:29:11`, the last logged op (`dfr-table-read-0x8000`) was invoked **directly** with no wrapper ⇒ **every logged entry predates the wrapper era and every post-wrapper device op is unlogged**, so **bypass alone suffices**. Their own caution kept: this is a **correlation with no counterexample, not a replay** — no direct invocation occurred after 13:48, so the window contains **no test of the append path**. **hive narrowed (b) further, free and static:** the log and its directory are **both writable now** (`-rw-r--r--`, dir writable) ⇒ (b)'s most plausible mechanism (a permission failure swallowed by `2>/dev/null || true`) is **unsupported, though not excluded**. **The act-1 replay is FORECLOSED** — the grant file has been replaced (`target=D4`), and composer **rightly refused** to swap the live authorisation artifact to reconstruct history. **hive's route out:** `_hs_authorized()` resolves `$ws` by walking up from cwd for a `.fleet` dir, so an **isolated temp workspace** exercises the same path with its own log and the live grant untouched — one constraint, a PASS means the command actually runs, so the test command must be **inert if allowed** (shape offered, recipe not validated end-to-end).
- **AMBIGUITY NAMED, NOT COLLAPSED:** the gate's append is best-effort (`>> … 2>/dev/null || true`, auto-approve.sh ~:692), so an absent entry means EITHER the gate never ran (bypass) OR it ran and the append failed silently. composer-codex's replay evidences bypass **for the D4 read**; for act 1 hive has only the absence, and **a null does not carry its own explanation**.
- **RECURRENCE, not a one-off:** hive banked this class earlier — the gate matches **COMMAND TEXT ONLY**, so `ssh host 'bash file'`, a wrapper script, or a token living inside a file all defeat it ([[espflash-gate-bypassed-by-file-and-remote-exec]]). Two independent routes now ⇒ a design property, not a bug.
- **★ IT FAILED IN BOTH DIRECTIONS THE SAME NIGHT (hive holds both halves):** **FALSE POSITIVE** — denied hive's `espflash save-image`, a pure file write with no port and no board, which is what forced the Roy-produces/hive-attests/composer-writes split; **FALSE NEGATIVE** — passed a real device operation behind a wrapper. **A gate that blocks the harmless and admits the real trains lanes to route around it** — which is how the bypass came to exist. Fixing the matcher without fixing the false-positive side leaves the incentive intact.
- **Regression test must cover the CLASS with its own positive control** (direct / wrapper / ssh / token-in-file / env-carried ⇒ DENY; a known-bad command ⇒ DENY as the control, else the rig can pass by being dead; a benign file-only op ⇒ ALLOW to guard the false-positive side). **And make the log append fail loudly** — as written, "not audited" and "audit failed" are byte-identical in the record.
- **MEASUREMENT UNAFFECTED** (hive re-derived the D4 capture byte-for-byte): bounds, capture and result stand. This is an audit/controls finding, not a data finding. **Owner: supervisor. Not hive's to action.**

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

## ROUTE-ORIGIN-1 GROUP_MGMT carve-out — CORE-OWNED, not a hive blocker → `RESUME-archive.md`, "ARCHIVED 2026-07-28 (compaction, pass 2)"

- `router.rs` universal drop = a **CANON-vs-CANON divergence**; **core owns the fix**, hive neither edits nor gates it. Full divergence trace, the two conflicting canon clauses and the falsifier are in the archive.
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
