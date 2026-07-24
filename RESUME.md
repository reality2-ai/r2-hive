# RESUME — r2-hive

Updated 2026-07-24. `main` clean + pushed. **NEXT WORK = v8 build order (awaiting core push + supervisor pinned
sha). v7 extract CANCELLED — v8 supersedes v7 everywhere; v7 ELFs = attested reference only. v6 DOA bins
quarantined to `~/doa-v6/`.**

## Current: v8.3 `b79789c4` — BUILT + ATTESTED, awaiting extract amendment

**Rig PASS (exit=0, all 11 checks) → standing conditional fired → BUILT.** `b79789c4672795ed40487d6cd28e482d731a4c63`,
BUILD_ID `coex.v83.0724`, detached clean checkout, HEAD verified.
- **d5-otarx-v83** `f1f31854e743705cceabf1967a7a130edef3739717fa309f0463665dd08e8c77`
- **d5-otafail-v83** `4c59ae48c423e636555b0eacaeeee7299bb67290824218eda8e1d1f7db85557f`
- **d4-v83** `3f42b5f721bd5e2519ba9eccd1dfa16e9a62e013bba47ad16102a065f1ba0eec`
- **xiao-v83** `458402672a9d2a882e32d7330d3d471ee89dd5eb3d5fc579ca712ff480dc4261`

**MEASURED marker matrix** (strings-level — the attest authority, not a prediction):

| artifact | BEACON-adv-up | re-advertising | CoC-half-open | idle-watchdog | §5.4 |
|---|---|---|---|---|---|
| d5-otarx-v83 | 1 | 2 | 1 | 1 | 1 |
| d5-otafail-v83 | 1 | 2 | 1 | 1 | 1 |
| d4-v83 | 1 | 0 | 0 | 0 | 1 |
| xiao-v83 | 1 | 0 | 0 | 0 | 1 |

Attest: persona baked==input all 4 (e6108006×2/0ad4a84d/43638da0); masked distinct
5efc8097/6476f9a3/e8b7a6cd/8b0961ae; roles Sensor/Sensor/Bridge-Init/Hive; BUILD_ID v83=1 with **zero
leftovers** (v6/advwd/v7/v82 = 0 across all four); `r2_dfr1195` set_phy symbols = 0; otafail differential OK.
Checks (10) zero stale MTU literals + (11) `assert 65535 == impl 65535`.

**STREAMS AT REST VERIFIED (second-party leg, both hosts):** p1 `5d1e69ca…` 875276 B · p2a `86208e5b…`
875276 B · p2b `356c2e1c…` 875276 B · p3 `45849855…` 873772 B — alfred and tuxedo-os hashed independently,
identical, all matching the ledgered pins. Free consistency checks: p1/p2a/p2b size-identical + p3 differs =
same otarx bin re-signed 3× and otafail for P3, per phase design; each stream = its bin + a **constant 188 B**
(measured — contents of those bytes NOT verified and NOT guessed). Found only via a CONTENT search: my first
glob `~/*-v83-*.stream` returned 0 on BOTH hosts because they live in `~/v83-streams/`; I reported nothing
until the search widened. **Audit scope corrected:** `~/v83-bins/` holds 4 more 0xE9 images my scope missed,
sha-verified as DUPLICATES of v83-staging ⇒ **content-audited 37, still 0 loose**; grant-v4/v6/v7-evidence
dirs hold 0 images (streams only).

**★ BIN→STREAM IDENTITY VERIFIED — the last unmeasured link, and size-identity did NOT prove it.** Payload
region `stream[188:]` hashed per stream vs the attested bins: p1/p2a/p2b all `87360384…` = `d5-otarx-v83.bin`
**byte-identical** (not merely same-size); p3 `f5ad0535…` = `d5-otafail-v83.bin`. Preambles all differ
(b3cc115f/d00385bc/c0e80188/ae576a78), opcode `03`, ver `02` on all four.
- Combined with core's one-field header extraction, the ONLY differences across P1/P2a/P2b are header field +
  signature ⇒ **reason=4 can come only from the signer field, reason=7 only from the class field** — proven at
  byte level on BOTH halves, so neither negative control can be contaminated. Had p2a's payload differed, the
  reason=4 verdict would have been worthless.
- Re-derives the 188 offset independently of core's `1+123+64` arithmetic (payload hashes to the bin starting
  at exactly 188). Two methods, one answer.
- **Chain now unbroken and fully measured, every link by ≥2 parties:** source `b79789c4` (rig PASS + specs
  stamp + suites on the tree) → ELF `f1f31854` (hive + core attest, marker matrix cell-for-cell) → bin
  `87360384` (3-way independent derive) → stream payload `87360384` (this check + 3-party at-rest hashes).
  **What reaches D5's flash is provably the artifact the rig passed** — precisely what the gate cannot tell
  us, since it name-locks and records sha without verifying it.

**MAC-HYGIENE ARC — CLOSED SYMMETRIC, BOTH REPOS, ZERO BYSTANDER CAPTURES (2026-07-24, supervisor-ledgered).**
r2-hive: content-audited by SHAPE (generic 6-octet, tool-level control confirmed the regex was ACCEPTED, exit
0/1 never 2) → live tree CLEAN; history 48 commits / 6 messages / 25 distinct values, 8 synthetic, **17
real-looking → all 17 classified own-device or own-generated**, incl. 2 BLE random/RPA (not hardware ids) and
1 scan-adjacent but provably our own bridge XIAO (roster row carries its hive_id/role/persona-verified).
r2-core: 5 families, likewise zero bystanders. `14:b5:cd` closed as ONE shared lab fixture documented in both
repos. **Disposition: accepted-residual stands** (07-17 ruling: pushed messages = flag-don't-rewrite; rewrite
breaks every clone + pinned sha incl. the live grant pin). Nothing escalated to Roy.
- **Classification canon (joint):** shape FINDS everything, context CLASSIFIES it — enumeration never
  converges (my board-OUI list was incomplete twice). Then: **context tells you WHERE a value came from;
  ownership is a SEPARATE step telling you WHOSE it is.** Both directions need it — a `[SCAN]` line is not
  automatically a bystander capture, an inventory line is not automatically ours.
- **Disposition split:** "the board broadcasts it anyway" justifies accepted-residual for OUR identifiers
  only; a third-party capture republished in a public repo is a different class and escalates to the human.
- **Instrument canon:** a sweep must prove its own regex was ACCEPTED before its result means anything, and
  the positive-control string must match *that* pattern. `grep` here is a shell function wrapping ugrep
  (rejects backreferences, exit 2). Not-0-as-broken manufactures false alarms exactly as empty-as-clean
  manufactures false all-clears. **The controls themselves need controls.**
  [[hygiene-gate-greps-all-tracked-files]] [[safety-claims-name-what-is-enforced]]

**⚠ The grant `target=` field contains a real MAC.** Quoting it verbatim into a tracked file trips the
pre-push secret scan — it caught me doing exactly that while writing the dating rule; redacted to a
placeholder, amended, pushed clean, no leak. Warned all lanes: the env-prefix convention that injects the
target into command text also injects a MAC into every transcript/log/ledger quoting that command — same root
as the gate defect, and path-argument binding alone will NOT fix the exposure.

**BINS DERIVED** (v8.3-EXTRACT-ONLY amendment; ELF shas verified pre-extract; staged in `~/v83-staging/`, all
esp_image 0xE9, 4 distinct): d5-otarx-v83 `8736038471170906239f8a41d52cd4dadb9b263561ca4c2b24c918e3db6ed797`
(875088 B) · d5-otafail-v83 `f5ad053513fa941aa2088d02d9a8d4b45ccc34f80390da6d3e40f0aa45f6ac83` (873584 B) ·
d4-v83 `a6c603362f96bdb7c40f051972761e00791f2b5e0140eb6ee02a38839eaa2c76` (878944 B) · xiao-v83
`4ed921e2a1f0365dc84547f7502488cb4074fa08cdce306d4ac935475f203918` (864880 B). **Next:** 3-way attest.

**‼ CORRECTION — "flashable path EMPTY" was FALSE, reported twice.** The quarantine achieved no safety
property. **20 esp_image-0xE9 app images are loose in `alfred:~/` right now.** I measured
`ls ~/d5-ota-*.bin` = 0; that glob never covered `d5-otarx-wd.bin`, `d5-otarx.bin`, `d5-otarx-p1.bin`,
`*-core.bin`, `my-*.bin`, `cb87c8aa-app.bin`. Confirmed by sha: **every DOA v6 image still has a loose
byte-identical twin** (`my-d5-otarx-v6.bin` 971dfae2 == the quarantined copy; likewise 95ae7408 / d299010c /
bd58d076), and my own v4 `d5-otarx-wd.bin` 0aadecc6 == `superseded-bins/d5-ota-otarx-wd.bin`. For every file I
moved, an identical copy stayed put.
- **ROOT:** I defined the flashable path by a **filename glob** while the hazard is defined by **content** (a
  0xE9 app image). Glob ≠ the safety property — the same "scope is part of the instrument" failure as the rig,
  but committed against a SAFETY claim, and worse because supervisor and core made quarantine decisions on my
  false all-clear.
- **RESOLVED (supervisor ruling: owners sweep, content-defined).** My 6 loose images identified **by sha
  against my own reported extractions** (not by name) and moved to `~/superseded-bins/`, verified both sides,
  no delete: 0aadecc6 · 7880f533 · 1afb641c · 892504b1 · bd22d272 · ce76ea9e. Core has swept theirs.
- **CONTENT AUDIT (0xE9 by content, ANY name — restricting to `*.bin` would repeat the glob mistake).**
  Scope: `~` maxdepth 1 + doa-v6 + superseded-bins + v83-staging; **excluded** build `target/` dirs (images by
  construction, regenerated). **Content-audited: 33 images, all accounted** — HOME-ROOT 12 (1 unowned
  `cb87c8aa-app.bin` 1b8092d5 from 07-05, + 11 composer `my-*`), V83-STAGING 4, DOA-V6 4, SUPERSEDED 13.
- **SWEEP CLOSED (re-measured live, not quoted).** Composer swept its 11 concurrently; core swept its 2 and
  their cross-owned twins. **Home root is now ONE image.** Verified my side: bd22d272 / ce76ea9e none loose;
  all four DOA v6 twins quarantined; the single remainder has no byte-identical twin anywhere.
- **★ Core's generalisation, adopted:** per-owner sweeps are sufficient **only when no twin is cross-owned**
  — false an hour ago (my bd22d272 == core's `-core` bd22d272), true now. So the post-sweep invariant to check
  is not "did each owner sweep" but **"does any remaining image have a twin under different ownership"**.
- **AUDIT CLOSED — content-audited: 33 images, all accounted, 0 LOOSE** (snapshot 2026-07-24T13:38+12:00;
  scope `~` maxdepth 1 + v83-staging + doa-v6 + superseded-bins, excluding build `target/`).
  loose=0 · v83-staging=4 · doa-v6=8 · superseded-bins=21. **Conservation: 33 before, 33 after** — nothing
  lost or deleted. doa-v6 grew 4→8 correctly: each DOA sha now holds both copies (mine + composer's twin), so
  cross-owner twins are co-located by content.
- **`cb87c8aa-app.bin` provenance RESOLVED** (was "unattributable"): source ELF `~/r2-dfr1195-weave.elf` sha
  `cb87c8aa337b…` — **the filename is that ELF's sha prefix**. Measured 1b8092d5 / 863440 B matches
  `RESUME-archive.md:6459` exactly, so ledger and measurement corroborate both ways. Moved to
  `superseded-bins/` with a LABEL sidecar (name preserved — it encodes the provenance).
  **Attribution corrected (supervisor invited the check):** `:1921` does *not* record a hive espflash run — it
  records the opposite, that the harness gate **blocked** hive's `save-image` and the run was relayed to a
  human; `:6459` says "extracted by SUPERVISOR". So the file is hive-**associated** (hive ELF, hive #49 task,
  hive archive), not hive-**executed**.
- **STANDING RULE:** say "content-audited: N images, all accounted" **+ the scope + that it is a SNAPSHOT**
  (the home dir has concurrent writers — any count is stale as written). Never "path empty"; glob-based safety
  claims retired.

**‼ GRANTS ARE NOT SHA-ENFORCED — my "sha-pinned grant is the flash boundary" was FALSE** (composer found it,
I verified at source). `claude-fleet/hooks/auto-approve.sh`:
- `:618` — *"The sha256 field is RECORDED, not enforced: a remote flash (ssh to another host) gives this hook
  no way to hash the bytes that will actually be written."*
- `:615` — *"THE HUMAN REMAINS THE BOUNDARY. Do not describe this as making flashes safe."*
- `_hs_authorized()` :624-650 checks only `[[ "$c" == *"$artifact"* ]]` and `[[ "$c" == *"$target"* ]]`;
  sha256 is logged, never compared.
- **Wider than "a different path to the same file":** `c` (:676) is the ENTIRE command string, so the
  substrings may appear anywhere and **need not name the flashed file at all** — an echo, a `--output` path,
  a mentioned artifact name. The grant binds *the presence of two strings*, not the identity of the flashed
  file. (The hook lines above are stable source; the grant FIELD VALUES below are not — see the dating rule.)
- **★ DATING RULE — the grant file is MUTABLE, edited by supervisor mid-cycle; quote it only with a read-time.**
  **Read 2026-07-24 ~13:43+12:00:** `artifact=-v83`, `target=/dev/serial/by-id/<MAC redacted>-if00`; zero
  quarantined images contain `-v83`, only the 4 live v83 bins do. **Earlier that day it was `artifact=d5-ota`**,
  under which every `d5-ota-*` DOA twin *did* match by path — composer reported that truthfully and supervisor
  amended the field *because of* that finding. I read the amended file and declared composer wrong, broadcast
  to three lanes, then retracted. **A measurement refutes a claim only if both refer to the same moment.** I
  held the snapshot rule for directories and not for config, though a supervisor-edited grant is exactly as
  concurrent as the shared home dir. Adopted fleet-wide: cite read-time or the mutation-log line when judging
  any past claim about mutable state.
- **★ The prescribed env-prefix convention defeats the target check BY CONSTRUCTION.** Grants instruct
  "carry `R2_OTA_TARGET=<by-id> R2_OTA_ARTIFACT=…`". The hook reads command TEXT, not environment — so the
  prefix authorizes by *literally inserting the target string into the command*. That is how every extraction
  (mine and core's) passed the target test while touching no device. Written into the grant text, so the next
  author reproduces it unless removed from the template. Supervisor accepted in full: post-cycle hook binds
  artifact/target to **resolved path arguments** (an env assignment is not a path argument), changelog names
  it, convention dropped from future templates.
- **Do NOT "fix" this by renaming the `d5-ota-*` quarantined files** — that mechanism was never the live one;
  it would buy nothing while looking like diligence, rebuilding the false safety story a layer down.
- **Owned:** I asserted the sha-boundary claim without reading the hook, after an authority used the phrase,
  and propagated it into RESUME + three fleet messages — during the same hour I was insisting peers measure
  rather than assert. The file's own header forbids the description I gave it.
- **Consequence:** quarantine buys nothing *at the gate* — it is human hygiene only. **The real control is a
  two-party sha256 of the exact path in the flash command, immediately before flash.** Proposed also a
  mechanical tightening (bind artifact/target to a resolved path ARGUMENT, not a free substring). Hook is
  fleet infrastructure — proposed, not edited.

## Prior: v8.3 standing-conditional setup (extended rig built + proven pre-sha)

**`9b5644f9` STOPPED mid-build (supervisor).** Known-failing test inside the candidate tree:
`r2-route/tests.rs:535 assert_eq!(Transport::Udp.max_payload(), 65536)` vs `r2-transport/profile.rs:103 =>
65535`, plus stale `64 * 1024` at `r2-route/constants.rs:111/115/125`, `r2-route/SPEC.md:118`. **No v82 ELF was
ever written** (killed before any `cp`) — nothing shipped, nothing to discard.
- **I found a 4th site nobody listed:** `docs/TRANSPORT-EXPANSION-SCOPE.md:34`. **v8.3 sweep = 6 sites.**
- **MUST NOT sweep** (same literal, different domain): 64KiB `chunk_size`/read-buffers/progress-modulo in
  `ota_tcp.rs`, `ota_tcp_recv.rs`, `trouble-test`, `r2-ota`, `ota-server`; `r2-cbor/SPEC.md:39` (CBOR encoding
  table); `r2-def` plugin `max_frame_bytes`; `main.rs:1570` (8-bit truncation comment).
- **My rig PASSED that tree — two owned gaps:** (i) SCOPE (variant b again) — my FILES list is 5 files, none in
  `r2-route/`; I saw the sweep touch `profile.rs` and downgraded it to "evidence only, not a gate" without
  checking other crates. (ii) **CATEGORY (new)** — the rig is a STATIC checker: it never compiles, never runs
  tests, so a known-failing test is invisible **by construction**. Rig PASS ≠ green suite; now disclosed in
  every report.

**STANDING CONDITIONAL (supervisor):** v8.3 sha → extended rig + 3 per-class controls → PASS = **BUILD
IMMEDIATELY**, full 4-set, `R2_BUILD_ID=coex.v83.0724`, #d005 + full preflight. FAIL → report, no build.

**Extended rig (checks 10/11 + disclosure) built and negative-controlled BEFORE the sha:**
| sha | class | result |
|---|---|---|
| `41eb7af6` | A: mechanism absent | FAIL 9 |
| `1395269a` | B: no typed mask | FAIL 3 |
| `64bf5e63` | lease-OK, pre-sweep | FAIL 1 — check 10 only; **check 11 correctly PASSES** ⇒ orthogonal |
| `9b5644f9` | candidate | FAIL 2 (stale sites + real skew) |

No known-good sha exists right now — expected mid-fix; v8.3 becomes the positive control on PASS.

**★ Three defects found in my OWN new checks before use — all returned the RIGHT VERDICT on the bad tree,
which is why they nearly slipped:** (i) unscoped literal scan → 9 false positives (buffer sites — reproducing
at scale the blind-sweep hazard I'd just warned core about); (ii) unscoped extraction → `impl=0` (read `=> 0.0`
from the decay fn; 7 `TransportId::Udp` arms exist); (iii) `64 * 1024` → `impl=64`, a bogus "KNOWN-FAILING
TEST" on an internally-consistent tree. **A right verdict from a wrong measurement is still a broken
instrument** — the verdict is about one sha, the measurement carries to the next.
[[marker-grep-cannot-see-comments]]

## Prior candidate: v8.1 `64bf5e63` (lease correct; superseded by the MTU sweep)

**v8.1 candidate = `64bf5e638d24140622efe8388c1f7e31f3e9d3f2`** (supersedes `1395269a`). `preflight-v8.sh` v3
**PASS, exit=0**. §2.3A:281 satisfied: `lease.rs:91 pub accepted_mask: TransportSet` (mask as STORED,
unknown bits stripped; empty on reject) **alongside** `pub accepted: bool` and `pub effective: TransportSet` —
wired into the ACK log via `a.accepted_mask.bits()`. New KATs: `lease_ack_carries_accepted_and_effective_masks`
· `lease_beacon_is_off_because_masked_not_suppressed` · `lease_transition_detector_sees_only_real_edges` ·
`lease_never_resumes_bearers_under_an_active_holder`.

**CONTROL MATRIX (per defect class — supervisor's requirement + core's positive-control suggestion):**
| sha | class | expected |
|---|---|---|
| `64bf5e63` | known-GOOD (positive control) | **PASS** — a change breaking this is a regression |
| `1395269a` | class B: bool, no mask companion | FAIL exactly 1 (check 3) |
| `41eb7af6` | class A: mechanism absent | FAIL 7 |

**★ I ALMOST BLOCKED A CORRECT BUILD (4th variant, costliest).** check(3) required `pub accepted:
TransportSet` — I encoded **the fix shape I imagined** (retype the existing field) rather than **the
requirement** (an accepted mask exists, mask-typed). Core added a *new* field and kept the bool — better,
since "did it accept" and "what got stored" are different facts. First run said FAIL on 64bf5e63; caught by
reading `lease.rs` directly before reporting. Now asserts `pub (accepted_mask|accepted): *TransportSet`.
[[marker-grep-cannot-see-comments]]

**★ Rig v3 stripper (core's finding, load-bearing):** strings stripped FIRST (escape/raw-string aware), then
block, then line comments **incl. trailing**. v2 stripped whole comment-lines only ⇒ trailing `// …` survived
and a literal `"pub accepted: TransportSet"` would have false-PASSED check 3. Mirrors core's false-NEGATIVE
(comments-first unbalanced quotes, swallowed real code). `alfred:~/strip_src.py`.

~ Possible one-liner MTU-constant-sweep sha to follow — cheap re-run.

## Superseded candidate: v8.1 `1395269a` (accepted:bool — no accepted mask)

**v8.1 = `1395269a4059eab5e4ce2a9578db3949e63e21cc`** (+ r2-route lease layer `a46fc12d` cherry-picked).
**`preflight-v8.sh` v2 PASS, exit=0** — the real §2.3A lease is implemented, not documented:
- mask WRITE `main.rs:1224 engine.set_transport_allow_mask(transport_leases.merged(now_s))`; wiring `:845
  LeaseTable<2>` · `:1180 install(LeaseSource::OtaSession)` · `:1199 renew` · `:1215 clear(held,
  engine.transport_boot_baseline(), now_s)`
- `LeaseAck{accepted, lease_id, effective}` on ALL paths incl rejections (lease.rs:96/100/112/117/129/136)
- INTERSECTION `lease.rs:62 baseline.intersect(self.merged(now_s))`, merge `bits &= l.requested.bits()`
  (disable-only — no lease can re-enable what baseline/another lease disabled)
- **`ota_quiesce_active` REMOVED entirely** (the bare bool is gone, not patched)
- §5.2 hard filter `strategy.rs:141 !transport_allow_mask.contains(t)` + `engine.rs:317
  present.intersect(self.transport_allow_mask())`; `reap()` inside install/renew/clear (term-expiry)
- KATs: `lease_terminal_clear_restores_boot_baseline:2435` · `lease_term_expiry_restores_boot_baseline:2451` ·
  `lease_masked_bearer_is_filtered_by_route_selection:2521` · `lease_effective_is_intersection_and_ack_carries_it:2408`
  · `transport_allow_mask_is_hard_filter_before_scoring:533`
- Negative control re-run after the rig change: `41eb7af6` STILL fails 7 checks — loosening terms didn't defang it.
- **★ Rig FALSE-FAIL I caught before reporting:** v1 was main.rs-ONLY and scored this correct impl as 4 FAILS,
  because the lease landed in `crates/r2-route/`; my guessed identifiers (`*_mask`) didn't exist either. Mirror
  of the v8 false-PASS, same root — the instrument assumed the shape instead of reading it. v2 spans
  main.rs+lease.rs+engine.rs+strategy.rs+lib.rs. [[marker-grep-cannot-see-comments]]

**⚠ ONE MORE SHA COMING — `1395269a` is NOT the build target.** Specs' re-stamp found `lease.rs:27
pub accepted: bool`, but §2.3A:281 requires the **accepted MASK** (so a requester can distinguish a
TRUNCATE-stripped bit from a BASELINE-stripped one; a bool can't carry that). Confirmed independently. Core has
a one-field fix + a named KAT arm; new sha follows.
- **My rig PASSED this defect — check (3) matched the field NAME, never its TYPE.** Third variant of one root
  (instrument claims a property it never measured): v8 false-PASS by prose → v8.1 rig-v1 false-FAIL by scope →
  this false-PASS by presence-without-shape. [[marker-grep-cannot-see-comments]]
- **Check (3) hardened + discrimination proven BEFORE the fix lands** (a check written after the fix is never
  tested against the defect): now requires `pub accepted: TransportSet` + `pub effective: TransportSet`, and
  rejects `pub accepted: bool` citing §2.3A:281. `1395269a` now fails **exactly 1** check — that one, nothing
  else regressed; `41eb7af6` negative control still fails 7.

**Still NOT a build order** — needs core's new sha → my rig re-run → specs' sha-anchored re-stamp → supervisor's
explicit order.

## Superseded: v8 `41eb7af6` (CONFORMANCE-DEAD — never build)

**⛔ v8 `41eb7af6` IS DEAD — NEVER BUILD IT, in this or any future session** (supervisor: "do NOT build
41eb7af6, ever"; specs RETRACTED conformance — a private bool is not a §2.3A lease, supervisor verified canon
:275-292 directly). §2.3A is DOCUMENTED, NOT IMPLEMENTED (core-confirmed, then verified by me against code). Comment-stripped grep: `transport_allow_mask` **5 hits full-file / 0 hits
comments-stripped**; `lease_id` 0; `intersect|effective_mask` 0; `install_lease|lease_ack` 0;
`ota_quiesce_active():298-303` = `deadline!=0 && OTA_ACTIVE && now<deadline` — a plain bool, no mask. If
flashed: bearers stay canonically AVAILABLE while radios park ⇒ **R2-BEACON §3.3 + D-20260724-01 violation**,
and ROUTE §5.2 still selects the parked LoRa ⇒ **silent drops**. **Release chain to v8.1 (all four gates, in order):** core reworks the real lease → **new sha** → **specs
re-stamp conformance** (new gate — specs retracted it on 41eb7af6, so a re-stamp is now required before a
build order) → **supervisor's explicit order** (BUILD_ID `coex.v8.0724` or `.1`, same 4-set). My hardened
preflight rig carries over and runs on the new sha.

**★ MY PREFLIGHT MISS (owned) — it would have PASSED this image.** I grepped §2.3A markers, hit doc-comments,
and reported *behaviour* off prose ("lease installed on OTA_ACTIVE after CoC-up", "refreshed per inbound SDU",
"dual release incl no-progress hard timeout"). A marker grep cannot distinguish comment from code. What stopped
a non-conformant image reaching metal was the **#d005 latch** (holding for an explicit supervisor sha order
instead of core's relayed "supervisor-authorized"), not my technical check — second time that latch has paid.
[[marker-grep-cannot-see-comments]]

**HARDENED preflight = a RUNNABLE RIG, negative-control proven: `alfred:~/preflight-v8.sh <sha>`** (exit =
number of failed checks). Checks: (1) comment-stripped vs full-file DIFFERENTIAL per spec symbol — comment-only
⇒ FAIL; (2) an actual WRITE to `transport_allow_mask`; (3) `lease_id` + ACK carrying accepted+effective; (4)
`effective = INTERSECTION(baseline, leases)`; (5) quiesce predicate must consult the mask, not be a bare bool;
(6) clear restores BASELINE not 0x7F; (7) consumers (ROUTE §5.2) can read the effective mask; (8) carried-over
set (partition e0e49127 + app@0x20000, offsets 0x12000/0x1C000/0x1D000/0x1E000, set_phy live=0, §5.4) — plus
post-build BUILD_ID baked + 0 prior-version leftover, persona baked==input, masked digests distinct, otafail
differential.
- **Negative control:** FAILS 9 checks on known-bad `41eb7af6` while the genuinely-fine carried-over checks
  still PASS on that same sha ⇒ it discriminates, not blanket-rejects. An unfired check is unproven.
- **Self-audit caught a FALSE-PASS in my own rig (fixed):** v1 passed "ACK path present" on 41eb7af6 because
  bare `accepted` matched the log string `"ACL conn accepted @ {}ms"`. Comment-stripping removes `//` lines but
  **not string literals** — a claim inside `println!` is another documented-not-implemented vector. All terms
  are now scoped identifiers (`lease_id`/`accepted_mask`/`effective_mask`/`baseline_mask`), never bare words.

## v8 scope (unchanged, fires on a conformant sha + explicit supervisor order)

**v8 = OTA-session radio quiesce** (leased mask {LoRa, ESP-NOW} + power-downs) — the general form of the
coex-emission lever v6/v7 carried as the 1-line `not(otal2cap)` fakesensor gate. Roy blessed g10; core
committing. **Scope when the order lands: FULL 4-SET** — d5-otarx-v8, d5-otafail-v8, d4-v8, xiao-v8, ONE pinned
sha, ONE attest. BUILD_ID `coex.v8.####`. D4/XIAO jump straight to v8 (their v7 flashes cancelled with the
extract); per-board grants only after the D5 v8 cycle proves green. **Preflight set (staged, confirmed
correct):** partition e0e49127 map + app@0x20000; personas 0x12000/0x14000/0x17000 untouched; set_phy
source-scope (0 live call sites); §5.4 marker; BUILD_ID baked + 0 prior-version leftover; persona baked==input;
masked digests distinct; otafail differential. #d005 stands — drain inbox, explicit sha order, clean detached
byte-verified checkout.

## Current build order — v7 4-artifact set (BUILT+ELF-attested, 2026-07-24)

**BUILD ORDER #d005: PINNED `6eec53d5`, BUILD_ID `coex.v7.0724` = v6 + boot-deadlock fix.** v6 (05dba4f3) was
DOA — deterministic boot-hang: coarse-checkpoint flash-write cache-suspend deadlock (coarse_time_init didn't
seed COARSE_LAST_CKPT_S → checkpoint_tick fired a flash write on the first main-loop tick, in the boot-settling
window). v7 fix = `coarse_time_init` seeds `COARSE_LAST_CKPT_S=base` (+12 lines), deferring the first
checkpoint write a full 225s out of the boot window. **v6 bins SUPERSEDED (never flash a hanging image).**
- **4 ELFs (~alfred, off-tree):** d5-otarx-v7 `89d79329131645f476f13028f37ae6bae83bab4cb43f84a073aaa48934c5ce0d`
  · d5-otafail-v7 `7b071ff5a764cbc850a49befa41a49f0c99d290c0b70240a7efb564df63a0937` · d4-v7
  `5d6ba59ddf066f0a7a5a4991861d5a5d2298bcf8106c6befa70a9127e5530996` · xiao-v7
  `31f6b466613d50465b82c8482a84daada0ea9351b429f006be28461a3c5c63c7`.
- **Attest PASS:** persona baked==input (e6108006×2/0ad4a84d/43638da0); masked distinct
  2b9ff062/dc4baac1/f1a21abd/fc946c2a; roles Sensor/Sensor/Bridge-Init/Hive; **BUILD_ID coex.v7.0724 baked all
  4, 0 v6 leftover** (serial/HEALTH shows v7); re-adv "CoC half-open/idle, re-advertising"; §5.4 rollback marker
  all 4; otafail differential. Preflight: v7 delta confirmed (LAST_CKPT_S=base seed :5827); partition e0e49127
  + app@0x20000; set_phy source-scope 0 live call sites.
- **v7 EXTRACT CANCELLED (supervisor ruling) — v8 SUPERSEDES v7 EVERYWHERE.** No amendment issues, no v7 bins
  are made. v7 ELFs stay as **attested reference only** (2-party ELF PASS: core's 4 shas match mine exactly).
  Rationale (mine, accepted): v8 = v7 base `6eec53d5` + quiesce diff ⇒ strictly ahead on the OTA path;
  extracting v7 now would mint a superseded bin set beside the DOA v6s — the exact hazard I flagged. v7 flash
  already ran ELF-direct under grant v7; that cycle is **CLOSED at the coex boundary** (P2a/P2b PASS, P1/P3
  gated on v8).
- **ALL BINS QUARANTINED — flashable path is EMPTY until v8 attests** (supervisor-ordered, no delete, every
  move sha-verified both sides). Grant v5 also RETIRED (`.SUPERSEDED`, never issues — its content rides the
  v7/v8 lineage: rotation live on v7 metal, rollback@0x1E000 + §5.4 in the v7 grant).
  - `~/doa-v6/` = 4 DOA boot-hang bins (971dfae2/95ae7408/d299010c/bd58d076)
  - `~/superseded-bins/` = 5 (v5: 3f88fd04/bb4f50b5/d06826e4 · v4: 0aadecc6/7880f533)
  - `~/d5-ota-*.bin` count = **0**. Completeness positive-control: 4+5 = 9 = the exact pre-quarantine inventory
    total, nothing lost or stranded. ELFs left in place (only bins are flashable-by-glob).
  - **SHA-PIN affirmed as grant law** — every grant sha-locked, filename never authoritative; the v8 grant
    names sha + a quarantine-checked dir.
- Push-timing note: core's push of 6eec53d5 crossed my first fetch (branch tip briefly read 05dba4f3); re-fetch
  resolved it, HEAD verified 6eec53d5 before build. [[positive-control-the-tree-not-just-the-tool]]

## Prior build — v6 4-artifact set (05dba4f3, DOA boot-hang, SUPERSEDED by v7)

**BUILD ORDER #d005: PINNED `05dba4f3`, BUILD_ID `coex.v6.0724` = f52a0f98 v5 fixes + af17e83d unconnected
re-adv timer + 05dba4f3 set_phy removal.** 4 artifacts (-v6 names preserve v4/v5 baselines):
- **d5-otarx-v6.elf** `4fbc36d0b9fe130599ec430e1a64fc96f5cb71fc79647df785aa89c173bbb020` (= supervisor's d5-otarx-wd @v6, otal2cap)
- **d5-otafail-v6.elf** `d9398cb1a0ea05ca9e685a9169fccec2cdb6f303a54c6ab030bf01f96138699b` (= d5-otafail-wd @v6, otal2cap+otafail)
- **d4-v6.elf** `98f9fdfbc6e3a8e0d5435b4527316421680c823adac32c167a0119c72cd32d42` (initiator)
- **xiao-v6.elf** `7c7626427283f864ed45e9e50bd56700ab62223e647bfc8676a2d57897cc686d` (acceptor)
- **Attest PASS:** persona baked==input (e6108006×2/0ad4a84d/43638da0); masked distinct
  0fc99fd9/683a865c/7eed359c/887fc70d; roles Sensor/Sensor/Bridge-Init/Hive; re-adv strings "CoC
  half-open/idle, re-advertising"; "§5.4 r2.update.rollback emitted"; otafail differential; DLE
  update_data_length KEPT.
- **★ set_phy-removal preflight — SCOPE finding (naive `nm|grep set_phy` FALSE-FAILS it):** grep=3 in every
  image, but ALL 3 are non-firmware — `hci_le_set_phy_cmd_handler` (BLE controller HCI handler, in every BLE
  image) + `ieee80211_set_phy_bw`/`_mode` (WiFi PHY, unrelated). ZERO `r2_dfr1195` set_phy. The removed
  trouble_host `Connection::set_phy` is INLINED in release → not a standalone symbol in EITHER build (v4
  da70ee0e pre-removal ALSO nm=0), so nm can't discriminate. Removal is source-authoritative (0 call sites; 3
  hits = removal comments; 2M-PHY switch crashes esp-radio 0.18.0 `llc_phy_upd` assert→panic, DLE kept).
  PASSES on correct scope. [[never-conclude-from-a-null]] [[four-reachability-instruments]]
- **Preflight PASS (05dba4f3):** partition e0e49127; offsets link_key 0x1C000 / checkpoint 0x1D000 / rollback
  0x1E000 (collision-free); re-adv `READV_INTERVAL_S=10` :4219; §5.4 :1453; key-19 gate `schema>=2` :2441.
- **BINS EXTRACTED (amendment-4, dual-prefix + explicit partition e0e49127; ELF sha verified pre-extract;
  app-image only, esp_image 0xE9, 4 distinct):**
  - d5-ota-d5-otarx-v6.bin `971dfae282b2f450ea86d1a4da52305bd85b89e5ce44182a39e5f756c36c116d` (872448 B)
  - d5-ota-d5-otafail-v6.bin `95ae74089bfcd34ae7fe6bc991a81deb7ceda5c035cae49d7580f8a5f66926a7` (871008 B)
  - d5-ota-d4-v6.bin `d299010cb3bf0a8f277f0a084bdc8e5c603bdbb5ab8600eef2df07c969801aab` (878240 B)
  - d5-ota-xiao-v6.bin `bd58d0760fdd78dcea0a340894b974f3aee6c340a3a47fbdcb0dc38a3853da43` (864112 B)
  - Composer two-party independent-derives = MATCH or ABORT. NO flash/sign. (R2_OTA_TARGET gate-token carries a
    device tail → off-tree only.)

## Prior build — v5 fix triplet (BUILT+two-party-matched, grant-v5 staged)

**BUILD ORDER #d005: D5/D4/XIAO bench triplet, BUILD_ID `coex.v5fix.0724`, PINNED `f52a0f98` — ELFs BUILT +
attested; bins pending extract grant.** v5-fix bundle = beacon adv 1000ms + HB origination ttl=2 + rbid
clockless coarse-time (bake anchor + uptime + NVS checkpoint @0x1D000/225s + boot-resume-max + key-19
monotonic-max + epoch=coarse/T_rotate) + §5.4 rollback persist record **@0x1E000 (relocated)** +
`r2.update.rollback` CBOR emit from io_task next boot.
- **ELFs (~alfred, off-tree):** D5 `ca105f885ff4b8c98560a2c46dfc58604b5a0b8a13954eedd02baad296a83df7` · D4
  `32d73d83c3fb83c8c5cba1554ea17bc456ba13d148a7db5f1042585f28b1e14e` · XIAO
  `97cab1829174580c8470a1c14dff3672a04a777fbec8c92e57774488db5a9346`.
- **Attest PASS:** persona baked==input D5 e6108006 / D4 0ad4a84d / XIAO 43638da0; masked digest distinct
  7284eeb9/b0f2a288/f840957c; role RPF1 D5 b[4]=1 Sensor · D4 b[4]=2 Bridge b[6]=1 Initiator · XIAO b[4]=0
  Hive; markers COARSE_BASE_S+UPTIME_S+LAST_CKPT_S syms + "§5.4 r2.update.rollback emitted" string.
- **★ LATENT COLLISION I caught → FIXED AT SOURCE (core f52a0f98; supervisor "good catch"):** was
  `ROLLBACK_REC_OFFSET=0x1C000` double-claiming `LINK_KEY_OFFSET=0x1C000` (4KB-sector erase mutual-clobber).
  Relocated to **0x1E000** (verified free); sector map now 0x1C000 link_key(xiaobridge) / 0x1D000 checkpoint /
  0x1E000 rollback / 0x1F000 free; 0x1C000 double-claim GONE (grep -c=1). Was dormant on the triplet (no image
  compiles link_key) but latent for any xiaobridge+OTA image. Two stale shas carried it: `e1172e9f` (my killed
  build) + `7131fb9f` — both superseded.
- **Preflight PASS (f52a0f98):** partition e0e49127 ✓; 0x1C000/0x1D000/0x1E000 all in r2cfg DATA
  (0x11000..0x20000), no overlap with persona/TG/0x17000/app ✓; (a) adv 1000ms :4175 · (b) HB ttl=2 :1428 ·
  (c) coarse-time init:365+checkpoint225s:1107+key-19:3847+monotonic-max:2441 · (d) rollback read:1418+§5.4
  CBOR:1449+write@0x1E000:7615. key-19/key-18 confirm: key-19 emit structurally paired w/ HEALTH_SCHEMA_VERSION=2
  const (≥2 by construction) + receiver gate `if schema>=2` :2441.
- **BINS EXTRACTED (amendment-3, dual-prefix R2_OTA_TARGET gate-token + R2_OTA_ARTIFACT=d5-ota, explicit
  partition table e0e49127; ELF sha verified pre-extract; app-image only, esp_image 0xE9):**
  - d5-ota-d5-v5.bin `3f88fd04897d9f2c3635cca4e805958a2d3cc45a00be993471ee508ce2c84f00` (894976 B)
  - d5-ota-d4-v5.bin `bb4f50b53949ef87767c275238936a101647e81c85c248afd80b8ec8887ade46` (878096 B)
  - d5-ota-xiao-v5.bin `d06826e45448d963c239c6b70a58df97c56f56aab0fedcd893865c9a4c930711` (863936 B)
  - 3 distinct. **TWO-PARTY MATCH 3/3 CONFIRMED (composer == hive) — v5 triplet validated end-to-end.**
    **Grant v5 STAGED (3 per-board sections), issued after OTA P1-P3 completes under v4.** No hive action until
    then. (R2_OTA_TARGET gate-token carries a device tail → off-tree only.)
- Recipes (iter-9 anchored): D5 `bridge,ble,benchsf7,baked_persona,fakesensor,benchkeepalive`+cos / D4 same
  +d4-initiator.role / XIAO `bridge,ble,benchsf7,baked_persona,loratcxo,xiao,benchkeepalive`+xiao-role.
- Build hazard: `nohup` detach kills export-esp.sh (empty log); use attached ssh (harness background).

## OTA adv-wedge build (2026-07-24, grant v4 LIVE)

**ADV-WEDGE-WATCHDOG pair BUILT + ELF-attested (supervisor BUILD ORDER #d005, PINNED `e6ff5198` verbatim,
BUILD_ID `coex.advwd.0724`).** Lineage rolls up: `3c8ea9e1` CoC-tuning + `86a8b8c3` otal2cap fakesensor-gate +
`e6ff5198` ADV-WEDGE idle-watchdog. Round-2 root = adv-wedge (one aborted CoC permanently silenced D5 ADV);
watchdog re-advertises within 8s of any idle abort. Independent clone, detached HEAD=`e6ff5198`, clean, gitdir
real.
- **d5-otarx-wd.elf** `da70ee0eb822356b6f349a9bd1d7d84a78cd9fe2fb47f2e63988753681593b21` (1379948 B).
- **d5-otafail-wd.elf** `10ae4dd6e2df4f14cf0de9306088d7a317c2ae2bd1ae3460fc237568cbdc5533` (1378636 B).
- **All 3 markers binary-attested:** (a) tuning trio set_phy Le2M+update_data_length 251,2120+credits 32
  (source :4143-4151/:4439-4448; ota_receive_over_coc+serve_coc symbols present); (b) fakesensor-gate :734
  `not(otal2cap)` — DIFFERENTIAL `apiary_bus_task`=0 sym in wd vs =3 in otatune baseline (spawn DCE'd = gate
  took); (c) OTA_PROGRESS watchdog symbol + strings "OTA(L2CAP) idle-watchdog abort (no SDU" / "CoC
  half-open/idle, re-advertising". persona baked==input e6108006 (wire 0xDA73508E), role b[4]=1 Sensor;
  otafail differential elf da70ee0e≠10ae4dd6 + masked ba99ad1c(P1)≠08d271eb(P3).
- **BINS EXTRACTED (amendment-2 grant, `R2_OTA_ARTIFACT=d5-ota`, input ELF sha verified pre-extract):**
  - **d5-ota-otarx-wd.bin** `0aadecc62db7a277354274880a47003fb3ac83c3fb704df24a350b17483ee581` (869824 B, esp_image 0xE9).
  - **d5-ota-otafail-wd.bin** `7880f5335f5ece698407bbcfb3c165248e23334256254dc28d04262270dfc468` (868272 B, esp_image 0xE9).
  - differential P1≠P3 OK. **TWO-PARTY MATCH confirmed (0aadecc6/7880f533) — hive extraction validated.**
    **GRANT v4 LIVE — composer flashing + running the OTA cycle.** Nothing hive-side until the metal verdict.
    otatune + b79b4f7a baselines stay archived. Next hive action: only if the cycle surfaces an image-level
    defect (adv-wedge recurrence past the watchdog, or a data-phase occupancy drop the tuning didn't cover).
- Build hazard SOLVED: `nohup` detach kills export-esp.sh (no tty + set -e exits before cargo → empty log ×2);
  attached ssh (harness background) keeps the tty. Build only via attached ssh.
- otatune baselines d5-otarx.elf/d5-otafail.elf + b79b4f7a bins UNTOUCHED (archived discriminators).

## Prior state (iter-9)

**✅✅ iter-9 CONFORMANCE COMPLETE — 3-BOARD BAR PASS (composer co-boot 2026-07-23).** Pair `#d025` + D5
conformant reflash both green on metal. No build pending.

**3-board PASS (all 4 falsifiers clear):** 3 iter-9 conformant boards (D4 724383ea + XIAO 5fb1565f + D5-cos
a0157eb2, all bit2=0), D4 monitor-reset co-boot.
- **FA2 PASS (THE key tiebreak — my sticky-capture finding's live test, iter-9 couldn't run it):** D4 resolves
  BOTH acceptors (8c15b0c2 + da73508e) then capture-dials XIAO `8c15b0c2` = LOWEST of 2 resolvable, NOT D5.
  Two-live-acceptor lowest-hive tiebreak WORKS ⇒ NO capture bug ⇒ **iter-10 sticky-capture candidate confirmed
  NOT load-bearing** (the earlier sticky was a first-seen-single-acceptor artifact only).
- **FA1 PASS:** all elect None (D4/D5/XIAO provider=None) — zero bit2=1 leak. Conformance holds 3-board.
- D4↔XIAO sustain: D4 0x25 ×4 + keepalive ×10; XIAO 0x25 ×11 + keepalive ×31 (~2.5s bidir); accept completes.
- **FA4 PASS:** D5 unpaired — 0x25=0 (bit0 DARK, EXPECTED) + still resolvable/advertising + cosine emitting
  (APIARY ×20) + accept=0. D5 did NOT disrupt the pair.
- **FA3 note:** 1 transient XIAO 'Disrupted' in ~120s → session RE-ESTABLISHED (31 keepalives after) =
  reconnect blip, self-recovered (validates iter-8 keepalive disconnect→break→re-dial→re-establish), NOT a
  sustained wedge. Relevant to the parked re-dial/conn-watchdog thread only if it RECURS.

**Overnight posture (`#d026`, Roy: green the remaining matrix overnight):** STANDBY-READY. Discipline: NO
build until an explicit order names a sha; #d005/#d006 preflight (drain → pinned-sha detached byte-clean →
`rm -rf target` → attest) on each.

- **OTA D5 P1+P3 DELIVERED + attested (from PINNED `b79b4f7a`, hive-owned dir, coex.iter9.0723) — awaiting
  two-party verify + P1-good-first flash.** 2 clean builds, NO dev-unsigned-ota (0 hits), both role b[4]=1
  Sensor/b[6]=0, persona da73508e preserved.
  - **[1] d5-otarx-p1** `54dddb16df9f4bbf5f63fe6273975a05df2440a9e5287265831baf8895a66eba` (`~/d5-otarx-p1.elf`,
    receiver flash-base + P1 payload): persona baked==input e6108006 @48264 masked `70e3ef93`; otal2cap swap
    (ota_receive_over_coc×2 + PSM 0x00D3×2); signature-REQUIRED (verify_strict + ets_secure_boot); CONFIRM
    path present (`health PASS` + `OTA CONFIRMED` + `image Valid; anti-rollback floor committed` = core's
    P1-watch strings).
  - **[2] d5-otafail-p3** `2a4f3308c7d606bd88b36ca09a3a7ddce0f55427215dfef8975841d8f9c71198`
    (`~/d5-otafail-p3.elf`, P3 radio-dead): persona baked==input e6108006 @48168 masked `00016efb`; same swap +
    signature-required; **otafail TOOK** — P1≠P3 differential + confirm/success path DCE'd (NO health-PASS/
    OTA-CONFIRMED/image-Valid) = health min-2 provably-unmet ⇒ no confirm ⇒ bootloader auto-rollback; cfg gates
    source-verified :775 BLE_UP / :912 LORA_UP (cfg(not otafail)).
  - Composer signs both real-TG seq cur+1 (P1 = same bytes re-signed). **P1-GOOD FIRST** on a fresh D5 (boot →
    8s → health PASS → OTA CONFIRMED; no-confirm/reset-loop ⇒ STOP, do NOT proceed to P3) = composer/Roy flash.
    ef7b2d24 (418c7934) DISCARDED. [[ota-per-platform-sink]]
  - **P1 flash result (composer 2026-07-23): boots CLEAN + HEALTHY + radios-up (flash-base role OK), NO confirm
    strings — NOT a P1 defect; cycle NOT blocked (composer stopped one step early).** Source-definitive
    (b79b4f7a `ota_confirm_or_rollback_on_boot` :3657-3684): the confirm AND the anti-rollback **floor-commit**
    both live in the New|PendingVerify arm — `set_current_ota_state(Valid)` → `read_ota_pending()` →
    `write_anti_rollback(max(seq),max(floor))` → `OTA CONFIRMED … floor committed seq=N floor=F` (:3660); the
    `_` normal-boot arm (:3680) clears stale pending, commits NO floor. espflash-direct = Valid/Undefined boot =
    `_` arm ⇒ no floor by design (`read_ota_pending()=None`). Answer = **(b): floor + P3's revert target are
    established by the P1 OTA-PUSH (seq=1), the step not yet run** — espflash-base only bootstraps a running
    receiver to push TO. Sequence: base→ota_0 (done) → OTA-push P1 seq=1 (inactive slot, PendingVerify, ~8s
    deferred confirm → floor=1) → OTA-push P3 seq=2 (health-fail → activate_next → revert to the P1-confirmed
    slot). (a) self-confirm-on-healthy-boot would be WRONG (no staged seq to commit). PROCEED: ota-push P1
    --dry-run then Roy-gated metal push. PASS-BAR already revised (base-flash no-confirm = EXPECTED). Composer's
    positive-control localized it right (no false "confirm FAILED"). Core + supervisor concur.
  - **★ Signed OTA payload = the app .bin, NOT the ELF** (composer pre-push Q). ota-push --image checks esp_image
    (magic 0xE9@0, chip_id@off12); the ELF (0x7F magic) is not it. Extract via `espflash save-image --chip
    esp32s3 <elf> <bin>` / esptool elf2image (deterministic → two-party reproducible). **The signed+pinned sha =
    the .bin sha, ≠ ELF `54dddb16`/`2a4f3308`** — composer extracts (my alfred lacks esptool + espflash is
    keyword-gated), then two-party cross-check the .bin sha256 BEFORE signing (attest the delivered bytes).
    persona da73508e baked in the .bin too. Seq: fresh D5 floor=0 ⇒ P1 seq=1 (base never wrote a floor — `_`
    arm). TG_SK: ephemeral unseal + immediate shred, off-tree (composer/Roy custody).
  - **.bin EXTRACTED + attested (grant-shape auto-approved):** the espflash keyword-gate was cleared by the
    sanctioned per-op grant shape (`R2_OTA_TARGET=<target> espflash save-image …`, artifact `d5-ota` + target
    named ⇒ gate auto-approves; esptool fallback not needed). These are the SIGNED-PAYLOAD bytes
    (header.payload_hash == SHA256(.bin); ELF sha ≠ delivered bytes).
    - **P1 d5-otarx-p1.bin `bd22d272d6c7fd1179a03b18e97de84c5a6fe8ace13fd259b1793f70c41e8cee`** (897504 B, from
      ELF 54dddb16; esp_image 0xE9; persona da73508e @44168).
    - **P3 d5-otafail-p3.bin `ce76ea9e3c08c8bc828ae81c8a5473f5c38bae8d6b67b24db031e4cf6e133c39`** (895968 B,
      from ELF 2a4f3308; esp_image 0xE9; persona da73508e @44072). P1≠P3 (otafail diff preserved).
    - save-image: chip esp32s3, merge=false (app image only). **TWO-PARTY .bin MATCH CONFIRMED** — composer's
      independent extraction == my hive shas (bd22d272 P1 / ce76ea9e P3); signed-payload bytes cross-validated.
      Core = 3rd derivation on request (ELF paths handed: `/home/roycdavies/d5-otarx-p1.elf` 54dddb16 /
      `d5-otafail-p3.elf` 2a4f3308, b79b4f7a-built — the 418c7934 P1 was ef7b2d24, discarded). **Never route the
      gate for actual flash/sign — grant-gated** (supervisor). MAC in the target path off-tree.
    - **Signer mechanism (composer, HELD on supervisor):** sealed TG 730c29e7 has no raw tg.txt; composer
      recommends `tg OtaSign` in-memory unseal + a new `ota-push --signed-stream` branch (NO plaintext key on
      disk) — I ENDORSED it over my earlier tmpfs-export (stronger custody). Ratified → step = `ota-push
      --signed-stream --dry-run` first (target_class=0, target_tg all-zero). No hive dependency (same .bin
      payload regardless of signer transport).
  - **P1 dry-run STAGED byte-exact `bd22d272` (composer, path A in-memory unseal):** --signed-stream drove
    OST/ODT/OCM e2e, receiver wrote 897504/897504B = my .bin extraction validated end-to-end. Metal push HELD
    (tuxedo DOWN + operator-gated). **P2b (ClassMismatch r7) BLOCKED** on ota-sign hardcoding target_class=0
    (wildcard ⇒ can't force a mismatch; needs a --target-class override to emit e.g. bridge B52C9F26) — core's
    ota-sign + supervisor test-design, NO hive action (my §2.6 class gate is correct; the payload can't be
    built). Offered a source-confirm of the accept/reject arm if useful.
  - **P1 metal push BLOCKED — RECONCILED to core's HALF-OPEN seam; clean board fix found.** Empirical
    (composer): CoC CONNECTS (0x00D3, link up) then drops on the FIRST OST write — `ENOTCONN os error 107`, 4/4
    identical, co-located (not range). **4/4 deterministic = NOT stochastic coex contention.** Matches core's
    OWN documented seam at `ota_receive_over_coc` :7819: *"'CoC up'+'receiver up' then the client hits ENOTCONN,
    board link-layer never surfaces the drop"* = ACL goes HALF-OPEN (supervision timeout) in the first-OST
    window; board blocks in rx.receive to the 15s guard. NOT adv-during-CoC (adv suppressed, :4068 sequential
    loop) and NOT a handler-close. **★ Root of the still-firing timeout: the otal2cap CoC uses
    `L2capChannelConfig::default()` (:4123) + NO 2M-PHY/DLE — while the cocbench path TUNES `set_phy(Le2M)` +
    `update_data_length(251,2120)` + `{Every(1), 32 credits}` (:4117-4128) "to stream without credit-starvation."**
    A ~900KB stream on the untuned 1M/default CoC = slow round-trip-heavy first OST = long occupancy in the
    fragile window = supervision timeout (coex-aggravated). bit0 survived coex (1-byte/2.5s keepalive = trivial
    occupancy); a 900KB burst doesn't. **CLEAN BOARD FIX (core reflash, corrects our "no clean lever"):** port
    the cocbench L2CAP tuning to the otal2cap serve arm (:4114-4128, widen cfg(cocbench) → include otal2cap).
    cocbench PROVES it streams reliably. **FIX LANDED (core `3c8ea9e1`, parent b79b4f7a):** :4117
    `#[cfg(any(cocbench,otal2cap))]` → set_phy(Le2M) + update_data_length(251,2120) + {Every(1), 32 credits};
    diff main.rs 10+/5-, comment credits the #d026 hive-diagnosis; verified read-only. **REBUILD DOUBLE-GATED:**
    (1) composer btmon shows supervision-timeout reason **0x08** (positive control — don't build the fix on an
    unconfirmed mechanism; if ≠0x08 → hold+re-scope) AND (2) supervisor build order (#d005). On both: rebuild
    d5-otarx-p1 (3c8ea9e1+otal2cap) + d5-otafail-p3 (+otafail) in the hive-owned dir → new ELFs → grant-shape
    .bin extraction → **fresh 3-way .bin cross-check** (composer+core); b79b4f7a bins bd22d272/ce76ea9e RETIRED.
    Composer central-retries = stopgap only. Stale-hk (weave-hk/bench-D5.bin ≠ baked persona) = separate
    resolver drift, out-of-band.
  - **★ ROOT = OST FRAMING MISMATCH (my hypothesis, core-CONFIRMED at source :7866-7871). Fix = tool-side, NO
    reflash.** Central-fix made CONNECT reliable (8/8), but the b79b4f7a board dropped INSTANTLY/det 8/8 on
    first OST = NOT a supervision timeout. `ota_receive_over_coc` REQUIRES `[len u16 LE][message]` framing (:7814,
    mirrors serve_coc :3187, needed for multi-SDU reassembly); ota-push omitted the prefix → first extraction
    read len from `"OS"`=0x534F=21327>4096 → `framing desync (len=21327)` + RESP_ERR 0x0E + RETURN → close →
    ENOTCONN (instant/det/pre-data, exact; dry-run passed on a loopback not enforcing the prefix). **FIX (core
    contract call): ota-push adds the `[len u16 LE]` prefix (composer tool-side).** **NO REBUILD — b79b4f7a bins
    bd22d272 (P1) / ce76ea9e (P3) STAY VALID** (wire-framing fix, not the image). **My 3c8ea9e1 occupancy tuning
    HELD SECONDARY/armed** — deploy ONLY if a post-framing data-burst drop appears (`start seq=` then ODT-drop +
    btmon 0x08); preflight primed, no build now.
  - **★ NEXT LAYER (framing CLOSED → header VERSION SKEW): D5 serial = `verify REJECT reason=1` = BadHeader.**
    attempt-2 (composer added `[len]`) DELIVERED the 204B OST + parsed (my framing diag CLOSED); board
    verify_header REJECTED. Pinned to source: b79b4f7a vendors r2_update @ crates/r2-update/src/lib.rs with
    **`PACKAGE_VERSION=2` (:93) / `HEADER_LEN=123` (:89)**; verify_header :526 `if h.version != PACKAGE_VERSION`
    → BadHeader (reason 1, :588). **Composer signs v3 / HEADER_LEN=137 → DUAL skew** (version 3≠2 fails first;
    len 137≠123 also misaligns). Vendored-vs-live: firmware vendored r2_update v2, composer's ota-sign is v3.
    **RULED (supervisor + core veto-cleared): (a) composer emits V2 headers this cycle** — tool-side, NO
    reflash; **my bins stay valid** (composer wraps them in a v2 header). Source-airtight: v2 verify_header has
    NO check_abi_compat (v3-only) ⇒ v2/123 header matches the board's :7827 path; P2a=4/P2b=7 bars hold on v2.
    **Board v3 re-vendor = POST-CAMPAIGN backlog** (v3's abi_hash/min_core_abi gates = new failure modes,
    spec-first) — **when it runs, FOLD the 3c8ea9e1 CoC-tuning into that one rebuild.** 3c8ea9e1 tuning HELD/
    orthogonal meanwhile. Nothing hive-side now. Expect composer's v2 re-push → `OTA(L2CAP) start seq=1` +
    RESP_OK. [[shared-checkout-path-dep-coupling]]
  - **★ NEXT LAYER (v2 header CLOSED → RESP-framing asymmetry): `start seq=1` prints, then central 10s-times-out
    on the OST RESP.** NOT occupancy (pre-any-ODT-data). Confirmed at source: `ota_receive_over_coc` sends RAW
    responses — `tx.send(&[RESP_OK])` (:7920), `&[RESP_ERR,0x0E]` (:7864), the OAK ack — **no [len] prefix**,
    while the proven `serve_coc` (:4485 "Frame: [len_lo, len_hi, R2-frame]") is SYMMETRIC [len]-framed both ways
    and the OTA INBOUND requires [len]. So the outbound RESP is un-framed vs the central's recv_framed(expects
    [len]) → central stalls → 10s timeout. **3c8ea9e1 is NOT this fix** (data-burst occupancy, not reached).
    **RULED (b) tonight (supervisor): composer reads RAW RESP** (1B OK / 2B ERR+reason / OAK ack) — tool-side,
    NO reflash, bins valid. Decisive fact: the INSTALLED P1 image (b79b4f7a) serves P2a/P2b/P3 with raw RESP
    regardless of any base fix ⇒ only (b) covers old+new uniformly; (a) tonight = full chain redo (rebuild+
    re-attest+re-sign 4 pkgs+new grant) for zero gain on the installed image. **(a) canonical board-fix
    ([len]-frame RESP, agreed asymmetry vs serve_coc :4485) + 3c8ea9e1 tuning fold = ONE post-campaign
    v2→v3-re-vendor rebuild** (spec-first, backlogged together). **TEST-FIRST gate (core, evidence-driven — I
    endorse; = [[dont-let-a-fix-land-on-an-unconfirmed-mechanism]]):** do NOT pre-arm 3c8ea9e1 — occupancy is
    UNPROVEN (no ODT has ever flowed; framing/header/RESP each refuted it as the current layer). (b) unblocks →
    the 897KB ODT stream ACTUALLY RUNS → THEN btmon 0x08 during the ODT burst / ODT-drops PROVES occupancy →
    only then the bundled reflash (frame-RESP + 3c8ea9e1 tuning [+ v3-bump]) that **I rebuild d5-otarx on**. IF
    ODT flows clean ⇒ NO reflash at all (best case, fully tool-side). Discriminator: RESP arrives-but-misparse ⇒
    (b) done, proceed; ODT-burst never-completes + 0x08 ⇒ occupancy ⇒ bundled reflash. Connect-window drops
    (1-7 of 8) = separate coex connect-race (composer central-retry); 3c8ea9e1 is post-connect, touches neither
    that nor the RESP layer.
  - **★ OCCUPANCY PROVEN on metal → 3c8ea9e1 UN-HELD → tuned pair BUILT + attested (supervisor build order
    #d005).** Composer's held run: OST→RESP_OK-raw→ODT#1→OAK(cum=200) then mid-burst drop at 1/4488 chunks =
    the occupancy layer surfaced on the actual ODT burst, exactly as diagnosed. Built 2 variants on PINNED
    `3c8ea9e1` VERBATIM (no new commits, no frame-RESP — composer's raw reader intact; BUILD_ID coex.otatune.0724):
    - **d5-otarx** ELF `fe758e415c7ab29eed15928aedca38e79d53ded00770cd90b8f7c2711d043fe1` / BIN
      `1afb641c9ad35fe444453e669d83b74332f9a1ec72b95290c64cec3c5415af39` (900208 B, esp_image 0xE9).
    - **d5-otafail** ELF `5bc94781bdc9a6d318a7ecb8af467e59112cc7db4ccbcad43bae1eec0fa3fd0e` / BIN
      `892504b179e4d957112377bf18978b964f1ce7cbca09a608d539b7954432d86b` (898656 B, esp_image 0xE9).
    - **★ tuning COMPILED IN** (set_phy×3 + update_data_length×1 both, ABSENT on b79b4f7a otal2cap = the fix
      took). persona da73508e baked==input both (otarx masked df3c1bca / otafail 3417c497), role b[4]=1 Sensor,
      OTAFAIL_OK P1≠P3, PSM 0x00D3, verify_strict, dev-unsigned=0, no frame-RESP. **3-way attest PASS**
      (hive+composer+core match). **HELD unflashed:** (1) grant v3 gated on composer in-flight btmon **0x08**
      confirming the data-phase stall IS supervision-timeout occupancy (mid-burst drop = symptom; 0x08 =
      mechanism the tuning fixes) — test-first, don't land the reflash on a symptom; (2) **b79b4f7a bins
      bd22d272/ce76ea9e NOT retired — they're the on-board occupancy DISCRIMINATOR** (A/B: tuned image must
      SURVIVE the ODT burst where b79b4f7a dropped at 1/4488) — I over-stated "retired," corrected; (3)
      connect-drop 8/8 = SEPARATE coex connect-race (tuning is post-connect, can't fix; central-retry owns).
    **v3 GRANT ISSUED — composer FLASHING the tuned pair (supervisor OVERRULED btmon-first: instrument
    unavailable [setcap+flaky ssh] ⇒ blocking on it inverts test-first; the A/B IS the experiment, reflash
    cheap/reversible; occupancy = best-evidence: OAK-per-ODT :7974 refutes protocol-mismatch, cum 400<4096
    refutes flash-sector ⇒ radio-level drop).** b79b4f7a bins = ARCHIVED discriminator baseline (kept, NOT
    v3-flashable — sha-lock = tuned pair only). **FALSIFIER LOCKED (advance-gate): tuned SURVIVES past 1/4488 ⇒
    occupancy CONFIRMED, cycle proceeds; tuned DROPS same early-burst ⇒ occupancy REFUTED ⇒ btmon 0x08 MANDATORY
    + re-scope (credits/RESP-parse). My next action fires ONLY on that falsifier.** Standby for the A/B verdict.
  - **★ OWNED correction (core):** my "verify floor via HEALTH key-6 ota_status" was WRONG — key-6 is hardcoded
    0 (:3717), NOT the floor. Correct path = read NVS **0x18000** = `[seq u32 LE][floor u32 LE]`, 0xFFFFFFFF→0
    (:7285, core owns). composer verifies seq/floor at 0x18000, not the HEALTH wire.
- **Stale-tree trap RESOLVED + killed (root closed by core+supervisor):** ~/dfr1195-fw-build was an ORPHANED
  linked worktree sharing the branch ref with core's dfr1195-fw-wt — every core commit advanced the shared ref
  under the stale tree ⇒ byte-exact-PARENT "reverse-edits" (nobody wrote my files; my byte-match diagnosis was
  right, mechanism = shared-ref-advance). **Structural fix DONE:** builds now use hive-owned
  `~/dfr1195-fw-hive-build` (independent clone, `.git` = real dir verified, `git checkout --force --detach
  <sha>` always). Old dir rm'd (its pointer named core's `worktrees/dfr1195-fw-wt` admin — removed ONLY the
  duplicate, core's real worktree untouched). Stash dropped. [[offthread-consult-write-race]]
- Other anticipated: beacon-plane diffs (only if core finds emit gaps), extended-wire test image.

**D5 iter-9 conformant (from PINNED `70960dbc`, BUILD_ID coex.iter9.0723): DELIVERED 2026-07-23.** Roy
authorized the reflash; supersedes d5-cos5/`11f2d2ef`. 3 clean builds.
- d5-cos9 `a0157eb2095e960f081dd43a8b47d70770af86ea65928886ade4a04e1e271e0f` (`~/d5-cos9.elf`).
- Persona baked==input `e6108006` @47216 = wire **0xDA73508E PRESERVED UNCHANGED**; masked base `305377b5`.
- Role BAKED_ROLE_PROFILE = RPF1 b[4]=1 Sensor, **b[6]=0 AcceptorOnly** (no initiator) + role≠norole diff
  `aa71d687`. Role byte = the 48B .role record (read_role_profile :3322), NOT the 336B persona. **bit2=0 rides
  70960dbc** engine_task `NodeCaps::new(false)` — the point of the reflash.
- Wave cos≠sin diff (cos `a0157eb2` ≠ sin `57648717`) + `k_cosf` linked + WaveSourceSentant×6 = cosine at
  sentant layer. C: core1×2 + lora_route×6 + espnow×6 + apiary×6 (fakesensor). Markers: BUILD_ID + domain-sep
  + APIARY value=.

**Pair PASS recap (`#d025`, composer co-boot 2026-07-23):** D4 dials XIAO `8c15b0c2` (capture-decouple works;
D4-dials-D5 was a boot-order confound), `0x25` sustained both (D4 ×4/XIAO ×7), bidirectional keepalive 10/21
~2.5s, election Some(D5) canon-correct, XIAO bit2=0. Mechanism reads all metal-vindicated (dial≠election
decoupled, quiescent=serve_coc-sticky). Pair `70960dbc`: D4 `724383ea`/`~/d4-init9.elf`, XIAO
`5fb1565f`/`~/xiao-acc9.elf` (both two-party verified). Sticky-capture secondary = core+supervisor ruled
INTENDED (re-dial-on-lower-peer robustness, iter-10 only if a mixed live bench needs it).

**Canon (closed, cited):** sensor-bit2 RATIFIED — R2-ARCH §3.1.3 v0.17 (D-20260723-05 = #d013) + R2-BEACON §7.2
(bit2 = fixed-AP gateway only). Every MCU board incl a sensor MUST advertise bit2=0; D5-old bit2=true was a
pre-#d013 legacy artifact. (I once scored a stale bar + reopened this closed ruling — owned;
[[cite-canon-before-claiming-a-finding]] currency corollary.)

**Owned lesson:** pre-iter9 dirt in ~/dfr1195-fw-build = off-thread-consult write race (stashed main.rs
byte-matched iter-8 `351a166e` exactly), dropped per supervisor ruling; the `rm -rf target` + detached
byte-clean + positive-control preflight caught it (mandatory standing mitigation).
[[offthread-consult-write-race]] [[positive-control-the-tree-not-just-the-tool]]

Arc (history in DECISIONS.md/git): Fix C (core1 executor isolation) → tri-bearer coex `0x25` on `bee0e996` →
blerole/D4-initiator merge (`54a8a1f3`) → board-to-board iters 3-8 (#d024: rbid resolve, list-gap,
capture-gate, domain-sep, lowest-eligible dial, ap_capable=false H2-fix, accept step-log, keepalive sustain) →
iter-9 #d013 conformance (bit2=0, #d025).

## Open threads (post-campaign, not blockers)

- **sensor-provider_capable canon = CLOSED** (R2-ARCH §3.1.3 v0.17 / R2-BEACON §7.2 = #d013): MCU sensor MUST
  bit2=0. D5 reflashed to `70960dbc` (a0157eb2) closes the D4-elects-D5 wrinkle by construction (all boards
  bit2=0 ⇒ elect None). Pending only the 3-board metal re-score.
- **conn-liveness watchdog** (my `conn.next()`/`is_connected()` primitive): NOT needed — keepalive
  `tx.send.is_err()→break` covers the common case, metal showed zero half-open. Parked as backstop; core wires
  only IF metal ever shows a tx.send-succeeds half-open (session neither sustains nor returns).
- **InvalidRouteLen CLOSED benign** (attribution corrected 2026-07-23): the 2 beacon classes are **OURS**
  (5511 FNV table; supervisor REFUTED the earlier "foreign noise" attribution I carried). The :2729
  EXTENDED-decoder drops (n~29) = OUR beacon frames on the extended path; apiary DATA (READING=64cedb11)
  decodes at :2101 = safe by construction, so the benign verdict HOLDS (canon-correct drops per R2-WIRE
  L244/L250, not strictness, not real-DATA-loss) — only the source label changed (ours, not foreign). Owned a
  mechanism-direction inversion earlier (extended-mis-parses-compact). Optional :2729 log rate-limit parked
  with core (LOW/cosmetic).

## Backlog (Roy-gated, not started)

- **D5 reflash/provision**: D5 stays `11f2d2ef` (cosine ×307). Any reflash needs fresh Roy word.
- **SEN0676 radar sensor plugin** for esp32-s3-dfr1195 (UART/ADC not i2c — confirm with circuits + board.toml).
- **RAK relay-LED** (dev/bench image only, brief flash per relayed frame; heartbeat LED untouched). Low.
- **DFR1195 display mislabel** (cosmetic): screen shows 'hive' twice with different values; relabel per field.
- **RAK tx_power −9dBm** (30cm bench; as923_nz default +20 saturates RX) — core change, rak
  `lora_leaf_config:1219`.
- **AGENTS.md doc-drift**: cites `docs/dfr1195-partitions.csv`; build uses `platforms/dfr1195/partitions.csv`
  (both app@0x20000) — recommend updating.

## Standing artifacts (LIVE on alfred, secret-bearing, off-tree)

- iter-8 pair `~/d4-init8.elf` / `~/xiao-acc8.elf` (RUNG-GREEN pair, flashed).
- D5 cosine `~/d5-cos5.elf` (`11f2d2ef`) — 3rd node, cosine origin-verified ×307, powered distractor.
- Personas ~/.r2-dev-trial/: d4 (0xC434FAFC), xiao (0x8C15B0C2), d5 (wire da73508e). MACs off-tree.

## Safety

- Plain non-force pushes only. Never `--all`/`--mirror`/`refs/keep/*`.
- Three local keep refs preserve removed security material (only local copies). Do not repack/prune/expire.
- Never bypass the fleet secret scan (`ci/public-hygiene.sh`, exit status enforced); forbids MACs/device-tails
  in tracked files — keep board MACs off-tree (bit me once in RESUME).
- Firmware lives in **r2-core** (dfr1195-fw / rak4630-fw are core worktrees). Never edit core; hive
  designs/builds/attests, **core lands source**. Hive never flashes (composer/Roy flash under grants).
- NVS `0x17000` raw role-write = brick class (no role partition on default table); bake role via
  `DFR_ROLE_PATH`, never NVS-write on baked_persona images.
- Env-baked const verify = full `rm -rf target` (incremental cache poisons it) + the DIFFERENTIAL (role vs
  empty, cos vs sin), never raw-bytes-in-ELF for a const-folded value.
- Every commit needs a `Decision-Log:` trailer (`Decision-Log: none` routine). Verify ahead=0 via
  `git ls-remote origin`, not a local ref.

## Branches

- `storing-backend` — real unfinished work on an old base; needs deliberate rebase + validation.
- `hygiene-scanner-v2`, `platform-trait`, `v0.2-relay-handshake` — stale/contained; do not merge.

Key rulings in `DECISIONS.md`. Ops hazard: [[reference-xiao-boot-flush-wedge]]. Lesson:
[[shared-list-serves-multiple-consumers]].
