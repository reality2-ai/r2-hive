# RESUME — r2-hive (hive-worker)

## ✅ 2026-07-01 — #29 DONE: r2-route + r2-transport re-vendored into dfr1195-fw, BUILD-VERIFIED (commit dfad9b7)
2-crate vendor from r2-core cf2646e committed blobs (fork-immune); r2-wire + r2-fnv PINNED (frame codecs
byte-identical, interop-safe). r2-route gained immune.rs (§13.8.2 network-immune, DoS-cap, is_reachability_blocked,
§2.3B override) + EspNow→Mesh; r2-transport gained profile.rs(+libm) + mesh.rs(alloc-gated) + host_udp/tcp/udp(std,inert).
Firmware reconcile: main.rs EspNow→Mesh (2 code + 2 comments; ESP-NOW HW driver untouched); ForwardRequest gained
arrival_transport: Option<Transport> → set None at BOTH sites (r2-dataplane handle_rx_frame + main.rs io_task) =
behavior-preserving (new drop, inert = prior behavior). **VERIFIED: local xtensa build GREEN on blemesh (route+Mesh/
espnow) AND loraroute (lora + alloc-gated mesh.rs).** libm resolved as new transitive dep. Committed to dfr1195-fw
(mine only; docs/dfr1195-firstlight.patch + tools/xbuild.sh left as pre-existing non-mine churn). NOT flashed (Roy-only).
**⚠️ CORRECTED FRAMING (core hop-10, verified engine.rs):** the DoS-cap PROPER (neighbour.rs provisional-ceiling +
no-evict-authenticated) is UNCONDITIONAL — already LIVE on this build; arrival_transport=None does NOT weaken DoS
protection. arrival_transport gates ONLY the SEPARATE §2.3B reachability_override_set (bench faked-distance/no-hear
pairs via set_reachability_blocked), EMPTY by default ⇒ None is behavior-IDENTICAL today (zero risk), not merely
"preserving". FOOTGUN: the override is enforced ASYMMETRICALLY when None — OUTBOUND selection (engine.rs:716) honors
it, INBOUND ingress-drop (engine.rs:534, behind `if let Some(arrival)`) is SILENTLY BYPASSED. Both ForwardRequest code
comments now say this (comment-only fix, blemesh re-verified green).
**FOLLOW-UPS owed (re-scoped):** (1) ONLY IF using §2.3B reachability_override on-device: resolve arrival Transport
(io_task has got.3 ordinal in ble builds; u8→Transport) → Some(arrival) at the 2 sites FIRST (PREREQUISITE for
symmetric enforcement), then the RX guard. Pure DoS protection needs NOTHING — already live. (2) #20
(ConnectionlessRadio ESP-NOW/R2-Mesh) now UNBLOCKED (mesh.rs vendored). (3) build field/staota/carrier combo before a
field flash (blemesh+loraroute cover the vendored surface). Core: nothing to fix core-side (tree clean @4235bab);
offered to raise a §2.3B strict-mode/debug-assert to specs (override-set non-empty + arrival None) — I'm endorsing it.

## (historical) #29 EXECUTING — superseded by the DONE entry above
Core resolved the cascade (off-thread + live): #29 = **2-crate vendor (r2-route + r2-transport), r2-wire PINNED**.
Verified the pin is interop-safe: r2-wire frame codecs (compact/extended/transcode/types) are BYTE-IDENTICAL
firmware-vs-core; only additive alloc-gated wifi.rs differs (absent in no_std firmware). **ISOLATED HOST COMPILE
GREEN:** built cf2646e r2-route+r2-transport against firmware's pinned r2-wire+r2-fnv in a scratch workspace,
`cargo check --no-default-features` = exit 0 (proves the 2-crate vendor compiles no_std against pinned r2-wire).
**APPLIED to dfr1195-fw-wt (from cf2646e COMMITTED blobs, fork-immune):** replaced crates/r2-route + crates/r2-transport
wholesale (r2-route gained immune.rs; r2-transport gained profile.rs+mesh.rs+host_udp/tcp/udp/lora*; profile.rs sha
76038e63 == core). Kept r2-wire + r2-fnv PINNED. **main.rs reconciled:** all firmware-used r2-route/r2-transport
symbols VERIFIED present in vendored crates (LoRaRadio is a TRAIT not struct — false-alarm cleared); only break was
EspNow→Mesh — fixed 2 code refs (1424 Observation.transport, 4062 DATA_RX send) + 2 ordinal comments. `espnow_task`/
`esp_radio::esp_now::EspNow` left as-is (that's the ESP-NOW HARDWARE driver, maps to abstract Transport::Mesh).
**IN FLIGHT:** local xtensa build (toolchain present at ~/.rustup/toolchains/esp; NO alfred needed) `cargo build
--release --no-default-features --features blemesh` — this is the signature-level gate. **DO NOT COMMIT the firmware
until this build is GREEN.** If red: iterate the specific errors (residual risk = refactored r2-route signatures the
firmware calls). After blemesh green: also build `lora`/`loraroute` (exercises r2_transport::lora paths) + `field`.
Then commit dfr1195-fw + (optionally) drive Roy's flash. NOTE firmware worktree has PRE-EXISTING non-mine churn
(docs/dfr1195-firstlight.patch, platforms/dfr1195/Cargo.lock, tools/xbuild.sh) — commit ONLY my #29 files.

## ✅ 2026-07-01 — TASK #4: r2-hive BIN builds+tests GREEN vs consolidated r2-core crates (commit 478c6c8)
Surfaced from INBOX (I'd been on wasm #26 / firmware #29; this r2-hive-BIN workstream had accumulated directives).
VERIFIED ground truth before acting (didn't blind-trust hours-old directives): all 5 previously-dangling path-deps
(r2-def/r2-ensemble/r2-dispatch/r2-transport/r2-discovery) now RESOLVE (core landed them in r2-core/crates as
excluded std-only + workspace members). handshake.rs R2-TRANSPORT-RELAY **v0.2 already conformant** (NOT re-implemented
— verified: device-first CHALLENGE, single-use nonce echo-match, ≤10s CHALLENGE_TTL, ±60s stateless reject, signs
4-field `<nonce>:<trust_group>:<device_id>:<timestamp>` Ed25519, v1 legacy 3-field kept). **BUILD was RED** — 3×
E0599 `no variant EspNow for r2_route::Transport`: core's vendored r2-route applied the v0.18 EspNow→Mesh rename,
but r2-hive-bin/src/hive.rs still said `Transport::EspNow` at 3 sites (send-order list:532, try_send_on host-stub:576,
USB TransportKind map:1037). FIXED = pure source rename ::EspNow→::Mesh (discriminant 5 unchanged, wire/OTA interop
preserved per core), comments→R2-Mesh. **AFTER:** `cargo build -p r2-hive` GREEN (was exit 101); `cargo test -p r2-hive`
GREEN — 105 lib tests + all integration binaries, 0 failed. No EspNow stragglers repo-wide. Reported to supervisor.
**GOTCHA logged:** a backgrounded `cargo … 2>&1 | tail` reports TAIL's exit (0), MASKING cargo's failure — always
redirect cargo to a file + capture its own `$?` (that's how I caught the real BUILD_EXIT=101). **NEXT (owed):** apply
specs' canonical Ed25519 relay-handshake vector to handshake.rs test when it lands (specs authoring it).

## ✅ 2026-07-01 — wasm v0.4.12: near-field floor max(d,0.001) sync (commit 474fb26) — follow-up to v0.4.11
core (fleet msg) confirmed the log-distance real params (PL_ref=40, n LoRa1.5/WiFi2.35/Mesh2.85/BLE3.4) — I'd
already caught+synced those in v0.4.11. The delta I hadn't had: the NEAR-FIELD FLOOR is `max(d, RANGE_LOSS_MIN_D=
0.001)` (a numeric floor ≠ d_ref=1.0), so sub-reference d<1 gives LESS loss than PL_ref (near-field modelled), not
a PL_ref plateau. My v0.4.11 pkgs were built against a transient worktree state (floor=1.0) — correct for d≥1,
wrong for d<1.
**REFUTED via test:** re-ran my range test against current source → FAILED (range_to_loss_db(2,-5.0)=0.0 not 40)
→ proved the floor was 0.001. Rebuilt v0.4.12 against **profile.rs sha256 76038e63** (content-sha anchor).
Test rewritten to the current near-field model (sub-reference < PL_ref; monotonic↑ above d_ref; loss finite∧∈[0,160]
any input; LoRa<BLE) — value-agnostic + intentional tripwire on floor flips. Canonical: `clamp(PL_ref +
10n·log10(max(d,0.001)/1), 0, 160)`.
**VERIFIED:** host 12/12, wasm32 clean, ws-mesh e2e 3× PASS. 3 pkgs re-staged v0.4.12: web wasm **66d9fdd90491807a**
/ js **c55c6b39a0ca0bfd**; ws-mesh node wasm 66d9fdd9 (==web); + carrier-bridge. route_hops still exported.
**✅ CORRECTED (core forensics, hop-2/50):** I WRONGLY claimed core amended 5e30c49 in place. GROUND TRUTH (core's
reflog + blob check, verified): 5e30c49 is a PLAIN commit, never force-pushed/amended (reflog e75fd4a→69dc566→
5e30c49→3323f3d, all plain); its committed profile.rs blob fbc1549 == worktree (sha256 76038e63). The 1.0→0.001
flip was a real COMMIT BOUNDARY (e75fd4a floored 1.0; 5e30c49 set 0.001, byte-exact to composer theater.html +
ratified R2-TRANSPORT v0.20). What moved under my v0.4.11 BUILD was the shared WORKING TREE: the #27 off-thread
fork transiently STAGED a floor=1.0 edit (blob 6cf58f8) there, which core caught+reverted — NOT an amend by core.
Core AFFIRMED: never amends published commits; content-sha anchoring is the right robustness for path-dep builds
(keep it).
**⚠️ DO-NOT-ASSUME (corrected):** the shared r2-core WORKING TREE (what path-deps compile) can be transiently
dirtied by the #27 off-thread fork (recurring hazard supervisor tracks) — commits themselves are stable. So anchor
path-dep builds on file content-sha, not commit hash, AND prefer vendoring from COMMITTED blobs (git show <ref>:path)
over the worktree. FLOOR STATUS: 0.001 is CANON NOW (v0.20-ratified) but NOT guaranteed-final — core routed the
d_ref=1.0-vs-0.001 §2.7 floor to specs; if specs blesses d_ref, core lands a NEW commit + pings me (tripwire firing
would then be EXPECTED/coordinated, not drift). My 0.001 tripwire stands.
**Sent:** composer (corrected swap params incl 0.001 floor + v0.4.12 sha), core (v0.4.11 already had real params;
the delta was the floor; asked if 0.001 FINAL; wrongly-accused-of-amend RETRACTED after its forensics).
**#29 UNBLOCKED by core (tree stable @cf2646e, worktree==5e30c49 committed):** core offers vendor-now-on-0.001 OR
hold-for-specs-floor. Floor coupling to #29 is MINOR (verified, NOT fully orthogonal — I initially over-claimed):
r2-route ROUTING BEHAVIOR is floor-independent (decides on MEASURED RSSI, not synthetic range_to_loss), BUT the
r2-route CRATE re-exports range_to_loss_db/loss_from_range_units (lib.rs:66-67) + carries a loss-VALUES test
(tests.rs:45-53: Ble74/WiFi63.5/LoRa55/Mesh68.5/LoRa(-10)→0, all consistent with PL_ref=40 + floor 0.001). So a
future specs d_ref flip = re-touch ~5 vendored test numbers, NOT a behavior change. Decision: vendor from COMMITTED
blobs @cf2646e (fork-immune) on 0.001 canon; the DoS-cap/is_reachability_blocked/SCF-gate/spray security fixes are
the value and are floor-independent. Steps owed: diff firmware's vendored r2-route vs core committed (firmware-specific
deltas to preserve?) + EspNow→Mesh v0.18 align + rebuild on alfred + re-stage.
**SCOPED (read-only, cf2646e reachable locally):** firmware r2-route/src (13 files @dfr1195-fw-wt 6fb1579) vs core
committed r2-route/src — delta ~1482 lines: **1373 core-side / 109 firmware-side** (firmware is MOSTLY BEHIND).
9 files differ (constants/engine/hop/lib/neighbour/path/strategy/tests/transport) + **immune.rs is CORE-ONLY**
(the is_reachability_blocked/DoS module = the #29 payload). The 109 fw-side lines SAMPLED (engine.rs) look like
STALE upstream code core refactored (use-stmts, ingest_observation, select_transport), NOT embedded-local
adaptations — so a whole-crate overwrite is viable; the 109-line audit is the SAFETY GATE before overwrite.
Confirmed firmware transport.rs:22 `Transport::EspNow=5` (apply core's v0.18 →Mesh rename; discriminant 5
unchanged = label-only, wire/OTA interop preserved per core). **GATES CLEARED by core (hop-4/6):** (a) vendor r2-route
from cf2646e (byte-identical at tip fe99b56; last r2-route change 5e30c49, stable); (b) worktree clean/fork-immune now.
Floor re-touch if specs flips = just `range_to_loss_db(Lora,-10.0)` in r2-route tests.rs (PL(10) values floor-indep).
**AUDIT GATE PASSED (read-only):** the 109 firmware-side r2-route lines have ZERO embedded-local markers
(no cfg/xtensa/esp/panic/no_std) — all STALE-UPSTREAM (old import lists, pre-refactor ingest_observation/select_transport,
local transport constants core moved into r2_transport::profile). Safe whole-crate overwrite; nothing firmware-local to preserve.
**⚠️ SCOPE GREW → MULTI-CRATE CASCADE (escalated to core; HELD pending its guidance):** #29 is NOT r2-route-only.
core's r2-route now `r2-transport.workspace=true` (firmware's r2-route has no such dep), and:
  • **r2-transport** firmware copy is STALE — MISSING profile.rs (the whole log-distance model) + mesh.rs; lib/transport/tests
    differ (EspNow→Mesh). host_udp.rs is core-only but `#[cfg(feature="std")]` (lib.rs:74-75) ⇒ inert for no_std firmware (safe).
  • **r2-wire** ALSO drifted — lib.rs differs + core-only wifi.rs. INTEROP-CRITICAL (wire format; a bump must be fleet-coordinated).
    Not yet determined whether core's r2-route/r2-transport REQUIRE the newer r2-wire or compile against firmware's existing one.
  • firmware call-sites: only 2 `Transport::EspNow` in platforms/dfr1195/src/main.rs → Mesh.
**DO-NOT (until core confirms):** do NOT autonomously vendor the wire-format crate (r2-wire) — interop risk with deployed boards.
Vendor from COMMITTED blobs @cf2646e, NOT worktree. Alfred remote build required (firmware builds on neither local box).
Next focused pass AFTER core confirms the coherent snapshot set (2-crate vs 3-crate) + r2-wire interop guarantee.

## ✅ 2026-07-01 — wasm v0.4.11: route_hops + core log-distance REAL-PARAM drift-sync (commit 104dde1)
**Trigger:** composer coord-Q — supervisor wanted the directed-message feature as an "R2-TEST-SENDER PLUGIN
emitting delivered/dropped/hop-path events"; composer built it on real primitives (build_frame/route_frame/
verifyFrame) and asked if a plugin-install + event-subscribe surface is on the wasm roadmap.
**MY RULING (my repo, my call):** NO JS plugin-registration surface — a JS "plugin" forks core's Rust Plugin
trait into JS-land (one-codebase violation). The plugin+event-bus model ALREADY exists & is real in r2-engine
(register_plugin/Sentant/enqueue/drain_outbound; HB+OTA are real Rust sentants on that bus in-wasm). A
directed-send test-sender = BENCH INSTRUMENT, not production hive behavior → does NOT belong in the production
ensemble. So composer's PRIMITIVE version STANDS. Told supervisor; if it wants a REAL Rust in-ensemble plugin
that's a specs/core Q (my answer: a test instrument doesn't belong in the production ensemble).
**SHIPPED route_hops(frame)->Uint32Array (v0.4.11):** full route_stack trail [origin,…,last_hop], mirrors
frame_origin. Closes the hop-path leg → composer's event triad is 100% derivable from real primitives, zero
plugin: delivered=verifyFrame deliver:true@dest; dropped=route_frame Dropped | verifyFrame deliver:false;
hop-path=route_hops(frame). ExtendedRouteStack.{len,entries} are pub in core r2-wire → read without touching
core (one-writer respected).
**DRIFT CAUGHT (important):** my range test tripwire FIRED — core landed 5e30c49 ("real composer/specs-v0.19
params") AFTER my e75fd4a build: PL_ref moved 0(provisional)→40 dB (theater.html-matched), n-table now LoRa
1.5/WiFi 2.35/Mesh 2.85/BLE 3.4 (was my provisional 2.7/2.9/3.0/3.2), clamp [0,160]. My range_to_loss_db/
transport_profile RE-EXPORT core so they auto-track — only my TEST+doc baked the stale PL_ref=0. Rewrote range
test to assert the ratified SHAPE (monotonic; d≤d_ref→PL_ref; LoRa<BLE loss), NOT the provisional numbers →
value-agnostic (survives Roy field-anchor) but still trips on MODEL drift. Doc updated to snapshot current
values + "code is truth, doc is snapshot".
**SIDE-EFFECT UNBLOCK for composer:** core's range_to_loss_db now matches composer's theater.html BYTE-FOR-BYTE
(per core's own comment) → composer's stated trigger to swap its JS pathLossDb → my range_to_loss_db is now MET.
Told composer to refute-check (confirm its theater.html n-table == the 4-tuple; feed range_units in d_ref=1
convention) before swapping.
**VERIFIED:** host 12/12 (incl new route_hops test + drift-synced range test), wasm32 clean, ws-mesh e2e 3× PASS
(TG isolation over real WS holds). 3 pkgs re-staged v0.4.11: web pkg wasm sha **e253810a13dd320b** / js
**3cb4353c428c85df**; ws-mesh node wasm e253810a (== web); + carrier-bridge. route_hops in web d.ts confirmed.
Sent: composer (ruling + route_hops shas + swap unblock), supervisor (ruling + drift catch).
**This is ALSO the "re-stage when Roy field-anchors provisional values" pending item DISCHARGED** — core's
5e30c49 IS the anchoring event (provisional 0 → theater.html-matched real params).

## 🔄 2026-07-01 — CROSS-PROVIDER TAKEOVER (codex→claude); TWO new spec items in flight
Took over from hive-codex. Verified ground truth: r2-hive `platform-trait`@0ca53ef (clean); dfr1195-fw@52b2819
(dirty: docs/dfr1195-firstlight.patch + platforms/dfr1195/Cargo.lock + ?? tools/xbuild.sh — pre-existing churn from
prior session, NOT mine; left untouched, committing only my files).
**(A) R2-TRANSPORT v0.19 (specs 37dfc60) — range→loss LOG-DISTANCE ratified** (reverses v0.4.9's linear). core gave
the SETTLED design: PL(d)=reference_path_loss_db+10·n·log10(d/d_ref); d_ref=1 range_unit (INTERNAL); clamp ≥0 for
d≤d_ref; ★ signature STAYS range_to_loss_db(TransportId,range_units)→f32 (d_ref internal → NO downstream re-plumb);
values provisional. **core BLOCKED landing it:** (a) core's commit/fleet-send perms tightened; (b) a concurrent 'core'
FORK live-editing r2-core transport crates (#27 worktree-isolation). Fork IS driving the batch — item D (HostUdpRadio
MTU cap + AB-006) landed @8aaf01a; item C (log-distance profile.rs) STILL PENDING (profile.rs still LINEAR at HEAD).
**✅ DONE — core LANDED v0.19 log-distance (e75fd4a, CI-green); I RE-ALIGNED (v0.4.10, 6b7fc7d):** range_to_loss_db
re-exports core's now-log-distance fn (no source change to the fn — path-dep auto-followed); transport_profile JSON
re-emits reference_path_loss_db + path_loss_exponent (dropped range_loss_db_per_unit); tests → log-distance. Host
11/11 + wasm32 + ws-mesh 3× PASS. ALL 3 pkgs re-staged at 0.4.10 (web pkg/ wasm sha e1527886d87396ec / js
d956b91d07fef140; ws-mesh node wasm 4f0cbf556f93672f; + carrier-bridge). ✓ SIGNATURE UNCHANGED (d.ts):
range_to_loss_db(transport_id:number, range_units:number):number. Values
PROVISIONAL (n LoRa2.7/WiFi2.9/Mesh3.0/BLE3.2, PL_ref=0) → re-stage when Roy field-anchors; shape FINAL, signature
stable. composer told (re-consume pkg, same call site, reach-spread re-tunes, ordering BLE<Mesh<WiFi<LoRa preserved).
core ack'd. composer's own web-build into its webapp = fine (compiling≠source-edit, one-writer intact).
**#29 HOLD extended:** don't re-vendor r2-route into dfr1195-fw until r2-core HEAD STABILIZES post-fork (re-vendoring
off a live-edited tree = moving target). Unblocked-in-principle (batch in HEAD) but wait for the fork race to settle.
**(B) R2-BEACON v0.21 (specs bd32ddd) — class-id repeater→hive, ROY GREENLIT** (the wire change previously held).
role_class_hash string "ai.reality2.device.repeater"→"ai.reality2.device.hive" (class_hash 0x00FC1F17→0xBAFE8AC1;
FNV auto-derives, no hardcoded hash). Firmware DONE (main.rs:3661, commit 6fb1579), build-green, hash VERIFIED
(FNV-1a-32 of both strings = spec bytes exactly), ELF staged alfred:~/r2-dfr1195-weave.elf sha 424ec044 (this ELF
also carries the clean-reset recipe + formation-decouple + role-Hive). WIRE CHANGE: flash all role-0 boards in the
SAME window as composer's scanner cutover to 0xBAFE8AC1 (mixed-version goes dark) — Roy flashes; coordinating the
window with composer + supervisor now.
**v0.21 FLASH-GO = GREEN (composer scanner READY):** composer's scanner recognises BOTH 0xBAFE8AC1 (hive) AND
0x00FC1F17 (repeater, LABELED LEGACY alias) through the window → NO DARK GAP (reflash needn't be atomic; retire the
alias once all role-0 on v0.21). FNV TRIPLE-verified (me+specs+composer). Relayed flash-go to supervisor with 2 paths:
(a) class-id-only reflash (espflash flash --partition-table … r2-dfr1195-weave.elf, preserves persona) or (b) full
clean-reset prep (#27). AWAITING Roy's flash (Roy-only) + supervisor's path pick. Ping composer to drop the legacy
alias once reflash confirmed.
**(C) BROWSER WASM-WS module DELIVERED (335f7ba):** composer was standing by. ws-mesh/hive-ws-browser.js (ESM) +
hive-ws-browser.d.ts — the option-B binding for composer's webapp (caller passes init'd wasm module; global
WebSocket; frame_origin echo-drop → verifyFrame → route_frame). WS msg shape = binary raw R2-WIRE. Gateway = HIVE
infra (composer confirmed its bench runs no WS bearer). Also FIXED a latent node bug: frame_origin is a MODULE
free-fn (this.wh.frame_origin), was called as this.hive.frame_origin → echo-drop silently no-op'd. test-mesh 3× PASS.
**hive-codex read-only findings TRIAGED (all resolved at HEAD 1d6c6d2):** (1) class-id — v0.21 SUPERSEDED the v0.17
.repeater ruling, I shipped .hive (6fb1579); (2) Cargo.lock now consistent 0.4.9 + r2-transport present; (3) no dirty
pyc, tree clean. codex's findings were at the older 941ca60.
**#29 r2-route re-vendor NOW UNBLOCKED (core hop-3):** whole-crate re-vendor clean — DoS-cap + is_reachability_blocked
+ SCF-gate + spray all in r2-core HEAD. TODO next: re-vendor r2-route into dfr1195-fw (verify no firmware-specific
r2-route deltas to preserve) + align firmware Transport::EspNow→Mesh (v0.18) + rebuild + re-stage. Non-blocking.

## ✅ 2026-07-01T14:58:15+12:00 — v0.4.9 WASM PKGS STAGED + THEATER REGRESSION LEAD
Objective: urgent supervisor unblock for composer after `5809fde` landed `r2-hive-wasm v0.4.9` but generated wasm
packages on disk were stale. Result: generated staging outputs refreshed; no tracked code/package files changed
because these outputs are gitignored.
- **Branch/HEAD/worktree:** `platform-trait` at `7c9122e` (`docs: record class-id ruling and wasm lock`), matching
  `origin/platform-trait` before this RESUME-only update. Generated package dirs remain ignored by git:
  `crates/r2-hive-wasm/pkg`, `crates/r2-hive-wasm/carrier-bridge/wasmhive-node`, and
  `crates/r2-hive-wasm/ws-mesh/wasmhive-node`.
- **Staged package outputs:** rebuilt all three from current source: web package at `crates/r2-hive-wasm/pkg`
  and node packages at `crates/r2-hive-wasm/{carrier-bridge,ws-mesh}/wasmhive-node`. All now report
  `r2-hive-wasm 0.4.9`, have `range_to_loss_db`, `transport_profile`, and `quality_from_rssi`, and no longer export
  stale `range_to_loss`. Web package hashes: `r2_hive_wasm.js` `98e641bf`, `r2_hive_wasm.d.ts` `5c8a92ce`,
  `r2_hive_wasm_bg.wasm` `ffec64d5`, `package.json` `08ce6a53`. Node package hashes:
  `r2_hive_wasm.js` `0cb104c6`, `r2_hive_wasm.d.ts` `c6cd3940`, `r2_hive_wasm_bg.wasm` `ffec64d5`,
  `package.json` `8b0a4e70`.
- **Verification:** `wasm-pack build --release --target web`; `wasm-pack build --release --target nodejs --out-dir
  carrier-bridge/wasmhive-node`; `wasm-pack build --release --target nodejs --out-dir ws-mesh/wasmhive-node`; direct
  Node require check proved `version()=="0.4.9"`, `transport_profile(2)` JSON, `range_to_loss_db(2,10)`, and
  `quality_from_rssi(-65)`; `node ws-mesh/test-mesh.js` PASS; carrier router test-vector PASS. Only observed warning
  was the pre-existing `r2-wire::hmac::EXT_AUTH_MAX` dead-code warning plus wasm-pack version/LICENSE notices.
- **Composer theater lead sent:** composer-side `webapp/theater.html` and `webapp/bench-sim.html` still import
  stale `/webapp/wasmhive` `range_to_loss`; their checked-in `webapp/wasmhive` copy lacks `range_to_loss_db` and
  `transport_profile`. That is the strongest current lead for Roy's missing event visualisations: stale wasm import
  can abort module init before animation/event wiring starts. Composer owns that repo; do not patch it from r2-hive.
- **Event-driver context sent to composer:** packet/relay flow is driven by `route_frame(...).sends[]` and each send's
  `kind`/`target`/`frame`; delivery confirmations are driven by `verifyFrame(frame).deliver` rather than
  `route_frame().outcome` because route forwarding is separate from local delivery; sentant/app/OTA arms are driven by
  `tick()` and `deliver_event(frame)` returned `frames`/`progress`. Migration hazards to check in composer:
  `range_to_loss` -> `range_to_loss_db(id, units)`, use `transport_profile(id)` fields for physics, and preserve
  numeric transport-id handling for Mesh id `5`.
- **Changed files:** this `RESUME.md` entry only. Generated wasm outputs are on disk for local staging but ignored.

## ✅ 2026-07-01 — TAKEOVER CLEANUP: class-id ruling + wasm lockfile hygiene
Objective: finish the interrupted handoff after specs ruled the v0.17 class-id question and hive-codex found dirty
generated/lockfile state. Pre-cleanup ground truth: `platform-trait` at `5809fde` (ahead of origin), with
`RESUME.md`, `crates/r2-hive-wasm/Cargo.lock`, and tracked generated
`crates/r2-hive-wasm/carrier-bridge/__pycache__/r2-carrier-bridge.cpython-314.pyc` dirty.
- **Specs ruling recorded:** R2-RUNTIME v0.17 role label rename remains label-only for beacon class identity:
  `ai.reality2.device.repeater` / class_hash `0x00FC1F17` STAYS. Do not rename the class-id to `.hive` without a
  future explicit wire-change ruling.
- **Lockfile fixed:** regenerated `crates/r2-hive-wasm/Cargo.lock` from the wasm crate so it matches
  `r2-hive-wasm v0.4.9` and includes the new `r2-transport` path dependency. The prior dirty lockfile had only
  advanced to `0.4.8`; do not commit that stale state.
- **Generated churn cleaned:** restored the tracked `__pycache__/r2-carrier-bridge.cpython-314.pyc` to HEAD. The
  bytecode change was generated cache churn, not source.
- **Verification this turn:** `cargo generate-lockfile` in `crates/r2-hive-wasm`; `cargo test` PASS (11 passed,
  1 ignored; only pre-existing `r2-wire::hmac::EXT_AUTH_MAX` dead-code warning); `cargo build --target
  wasm32-unknown-unknown` PASS; `wasm-pack build --target nodejs --out-dir ws-mesh/wasmhive-node` PASS; `node
  ws-mesh/test-mesh.js` PASS (B delivered signed HB over real WS, C wrong-key rejected). Final diff should be
  `RESUME.md` + `crates/r2-hive-wasm/Cargo.lock` only.

## 🔵 2026-07-01 — #26 CURRENT STATE (my deliverables IN; cross-integration remains)
r2-hive-wasm v0.4.9. My #26 half is delivered + green (host 11/11, wasm32 clean, WS mesh 3× PASS):
1. **WS binding PROVEN** over a real WebSocket (ws-mesh/: gateway + hive-ws + test; ae5b739) + **refuter-fixed**
   (941ca60): localhost-bind boundary (was binding 0.0.0.0!), keyless-hive warning, own-echo drop via frame_origin.
2. **§2.7 TransportProfile IMPORTED** from core's r2-transport, single-sourced, wasm-clean (5809fde): exports
   transport_profile(id) + range_to_loss_db(id,units) [core's CANONICAL linear per-transport-slope, replaced my
   provisional log-distance] + quality_from_rssi (byte-exact). Composer's sim reads the SAME physics = no drift.
3. **EspNow→Mesh v0.18** rename aligned (78a31a8). **Role Repeater→Hive** v0.17 done (52b2819, firmware).
core landed ITS half: 7f31dab (canonical profile + host-UDP ConnectionlessRadio). REMAINING for #26 DONE =
composer wires its browser app to the WS gateway (its bench server per core's ruling) + core's host-UDP binding
integration + a live multi-hive-over-real-sockets demo (the composer/core join). WS-seam peer-refute PASSED.

## ✅ 2026-07-01 — ROLE RENAME Repeater→Hive (R2-RUNTIME v0.17) + core WS-design APPROVED
**Role rename (dfr1195-fw 52b2819, build-green):** specs R2-RUNTIME v0.17 (Roy) — canonical roles = sensor/HIVE/
bridge/receiver; role-0 Repeater→Hive (LABEL only). Renamed Role enum variant + label()→"hive"; wire byte 0 +
from_wire + behaviour UNCHANGED; "repeater"=descriptive alias. **KEPT** the R2-BEACON §8.1 class-id string
"ai.reality2.device.repeater" (wire class_hash 00FC1F17) to honor "no wire change"; **specs ruled it STAYS
.repeater** (no `.hive` class-id rename in v0.17). Recipe ELF re-staged (alfred:~/r2-dfr1195-weave.elf sha
1c66026c). RPF1 role bytes unchanged (0=Hive), so the prep recipe is unaffected.
**core APPROVED WS-TRANSPORT-BINDING.md (all 4):** (1) TransportProfile→r2-transport (there's an uncommitted
profile.rs WIP core will adopt+commit as canonical; import THAT byte-exact — HOLD until core pings field names/path);
(2) WS binding = **B** (JS-carried, my rec) confirmed, reserve A; (3) exports confirmed (quality_from_rssi byte-exact
to core's transport.rs, zero drift; range_to_loss provisional until specs ratifies values = one-line swap);
(4) gateway = **composer's** bench server (my ws-mesh/gateway.js = reference/test-harness). HOLD WS route in/out wiring
on core's committed-struct ping (as planned — no fork).
**SCF-suppression catch → CANON:** specs R2-ROUTE v0.46 §3B (6a953cf) — SCF has_viable MUST require confidence >
NEIGHBOUR_PROVISIONAL_CEILING (authenticated liveness); conjecture TN-L0-XT-AB-006 open; core wiring SCF-gate to
is_authenticated. Folds into #29.
**#29 r2-route re-vendor = WHOLE-CRATE (core ruling), AFTER core lands the v0.46/v0.47 batch** (DoS-cap 0df4646 +
is_reachability_blocked + SCF-has_viable-gate + spray-rank) → vendor ONE coherent HEAD. core pings when committed.

## ⏳ 2026-07-01 — #26 real WS+UDP transports (supervisor GO; core-seam-blocked for WS binding) [task #26]
GOAL: r2-hive-wasm stops using the in-process virtual-mesh → meshes over REAL sockets (browser/WS + host/UDP) =
the production no-radio hive. Two bindings of ONE carrier-independent transport profile (specs R2-TRANSPORT v0.16
§2.7, bcb1a37 — schema gathers EXISTING per-transport params; only range→loss is new/PROVISIONAL; staleness_timeout
DERIVED = -ln(min_conf)/λ; guard LoRa.λ<WiFi.λ<BLE.λ). DIVISION: core leads host-UDP (ConnectionlessRadio over
UdpSocket, d0f1864 — NOT landed yet, core at session-limit until 12:30 Pacific/Auckland); I own WASM-WS binding +
wiring both into route in/out + the §2.7 exports.
**DONE — WASM-WS BINDING PROVEN END-TO-END (ae5b739, crates/r2-hive-wasm/ws-mesh/):** r2-hive-wasm now meshes over a
REAL WebSocket (not the in-process relay) = the browser half of the production no-radio hive. Zero-dep WS broadcast
gateway (ESP-NOW-shared-bearer analogue) + hive-ws client wiring route in/out to a real socket (Node global WS;
verifyFrame deliver-gate + route_frame forwarding). test-mesh PROVES it: 3 hives, A+B share TG key, C wrong key
(same tg_hash) → A's SIGNED heartbeat crosses real WS → B delivers (hmac_ok), C REJECTED (TG isolation over the
socket). 3× PASS. Option B (JS-carried, my rec); gateway+wiring survive core's A/B choice. wasm-node build gitignored
(rebuild: `wasm-pack build --target nodejs --out-dir ws-mesh/wasmhive-node`). GOTCHAS caught: route_inbound_sync is
forwarding-ONLY (self-addressed→Dropped is correct; delivery=verifyFrame, a SEPARATE layer); verify method is
`verifyFrame` (camelCase js_name) not verify_frame (a swallowed-throw made a false-positive isolation PASS until fixed).
**DONE (seam-independent, committed 6df4060, v0.4.7, 9/9 tests):** the two §2.7 exports composer+core wanted —
`quality_from_rssi(rssi_dbm)` (§2.5 −50→1.0/−80→0.0 clamp) + `range_to_loss(distance_m, path_loss_exp,
ref_loss_db_1m)` (PROVISIONAL log-distance, caller-supplied steepness, range emergent at −80dBm). Same physics
field+sim share (§2.7 one-source). Composer told.
**HELD on core (queued, →core inbox for 12:30):** (1) WHERE the shared TransportProfile struct lives (r2-transport?
import-not-fork); (2) host-UDP ConnectionlessRadio interface — should WASM-WS impl the SAME trait or ride the wasm's
existing SyncTransport seam?; (3) confirm export sigs. NOT building the WS binding until the seam is confirmed
(avoid forking core's transport architecture / building the wrong layer). NOTE: current wasm route_frame-in +
sends-out is already transport-agnostic (JS carries them); the "in-process mesh" is composer's router.js relay →
real-WS-mesh likely = a WS gateway + router.js glue + profile metadata, NOT a wasm-core rewrite (confirm division).
**DESIGN PROPOSAL written** (docs/WS-TRANSPORT-BINDING.md, 5a3d31f) — spec-first §2.7; pointed core at it. The ONE
decision for core: WS binding = my rec is deliberate asymmetry (host=core's Rust UdpRadio ConnectionlessRadio;
browser=JS-carried over SAME wire+profile — wasm sync-route boundary makes web_sys async↔sync bridge not worth it;
route_frame-in/sends-out already IS the binding) vs full-symmetry option A (Rust WsRadio via web_sys, I'll build if
core prefers). Unify the PROFILE not the socket layer. Gateway (broadcast relay) = layer-agnostic infra either way
(hive's or composer's bench server? — confirm). NEXT on core's seam confirm: struct in r2-transport → WS binding →
attach profile to links → peer-refute → hosted-green.

## ✅⏳ 2026-07-01 — FORMATION-DECOUPLE firmware DONE + build-verified; PENDING peer-refute [task #28]
Firmware path of the carrier nbrs=0 root cause. core's API contract (via supervisor, r2-dataplane 140da84):
if/else — verified→`accept_keepalive`, unverified→`ingest_observation` (both exist in vendored r2-route). SHIPPED
**dfr1195-fw c5ccdd3**, TWO bugs fixed (both present, both found via the clean-reset build-verify):
1. **EMIT (the real root):** HB header `flags {mcu_origin:true,..Default}` → has_route=FALSE while route=Some;
   `sign_extended` PRESERVES flags (doesn't force has_route) → emitted HB decoded ORIGIN-LESS even under multitg →
   dropped at ROUTE-ORIGIN-1A → NO neighbour ever formed. Fixed has_route:true (mirrors core encode_keepalive fix).
   [Corrected my earlier WRONG claim that emit already set has_route — verified, didn't assume.]
2. **RX decouple:** ANY decoded HB → `engine.ingest_observation(obs)` (TG-agnostic §2.1 link, relay-viable, nbrs>0);
   couple_ok(GroupHmac) gates ONLY accept_keepalive+duty+seen+PCO/rate; seq/dc parsed ONLY in verified branch;
   delivery stays classify(auth&&addressed). is_reachability_blocked OMITTED (not in vendored r2-route; bench mask
   off) → r2-route re-vendor follow-up. DoS-band (provisional low-conf upsert) = core's flagged NOT-YET, noted in-code.
Build-verified xtensa (carrier,multitg,field,routetest green, 1.32MB). **Recipe ELF RE-STAGED** with this fix
(alfred:~/r2-dfr1195-weave.elf sha 52da8eae) — ESSENTIAL, pre-fix boards form 0 nbrs.
**REFUTER-PASSED (verdict in):** decouple logic CLEAN — Angle-3 H9 intact (DG-1/duty/seq stay verified-only);
Angle-1 trust PASS (delivery HMAC-gated; phantom can't become a directed hop — try_directed needs a PATH entry,
ingest_observation touches only the neighbour table). #28 = DONE. Two refuter-confirmed issues = the KNOWN DoS-band,
NOT decouple defects → follow-ups:
- **[#29] r2-route RE-VENDOR:** dfr1195-fw r2-route PREDATES core's DoS-cap 0df4646 (provisional-ceiling +
  no-evict-authenticated = Angle-2 flood-evict fix) AND lacks is_reachability_blocked. Cherry-pick of 0df4646
  CONFLICTS (engine.rs+tests.rs diverge from core lineage) → coordinated whole-crate re-vendor needed (core owns
  r2-route; don't hand-fork). Asked core the clean path (→inbox). NON-BLOCKING (no adversary on bench) → post-run.
- **Angle-1 SCF-suppression sub-case** (spoof origin=D → has_viable(D)=true → suppresses SCF buffering, fr4 path):
  one-line note sent to specs for the DoS-band normative (SCF reach should require authenticated liveness).

## ⚠ 2026-07-01 — CARRIER FLASHED + LIVE on Alfred; R2RX works, PARTICIPATION blocked (TG-key mismatch) — diagnosed
Carrier flashed (role=STA fw=leaderless-0.4). R2RX reception WORKS (real over-the-air frames). But can't verify/
deliver: nbrs=0 dlv=0 blk=43+ synced=false, DROP NoViableNeighbour, DELIVER-BLOCKED tg_ok=TRUE hmac_ok=FALSE.
**DIAGNOSIS (file:line):**
- **"in-TG" is NOT a TG-id** — it's the demo Event PAYLOAD (`main.rs:1301 payload=b"in-TG"`, 696e2d5447=ASCII).
  The frames are the demo ORIGINATOR Events.
- **Q3 (own events alternate hmac BAD/good) = DELIBERATE, not a bug:** `main.rs:1300 good = ev_seq%2==0` + `:1325
  signer = if good {group_hmac} else {bad_hmac}` (bad_hmac=`[0xFF;32]`, :823) = a deliver-gate PROOF feature. Nodes
  run the same fw → ~50% of their Events are deliberately bad → correctly blocked (most of blk=43).
- **Q1 real blocker:** dlv=0 ⇒ even the GOOD-key (even-seq) Events fail → carrier group-hmac key ≠ nodes' good hk.
  tg_ok=TRUE (`deliver-gate :1751 target_group==my_tg_hash`) = SAME tg_hash but DIFFERENT hk = provisioning/key
  mismatch. (demo-fallback = shared `TG_HK_DEMO=[0x5C;32]`+`MY_TG_HASH`, :134/180 — only if ALL unprovisioned.)
- **Q2 (nbrs=0) = downstream:** under `multitg` the HB is HMAC-signed (`:1011`) → carrier HB-verify fails on the
  nodes' HBs (key mismatch) → no neighbour coupled → nbrs=0 → DROP NoViableNeighbour. Single root cause.
**SHIPPED (r2-core dfr1195-fw @55a8a45):** carrier now ALWAYS signs with the real TG key (force `good=true` under
`carrier`; default keeps the alternating proof). Stops the carrier emitting 50% bad frames + cleans Q3. xtensa-
green (carrier+default); ELF re-staged ~/r2-dfr1195-carrier.elf (tuxedo+Alfred). NECESSARY-not-sufficient.
**STILL NEEDED (asked supervisor):** the hk MISMATCH fix — need the fact: nodes UNPROVISIONED (demo) or
PROVISIONED (persona)? → either erase 0x12000 on all (shared demo [0x5C;32]) OR provision/serial-PROVISION the
carrier with the nodes' hk (serial PROVISION cmd @0x14000 needs `multitg` in the carrier build). Nodes likely also
need alternating-hmac-off for full participation (re-flash) — confirm acceptable. VISIBILITY (R2RX) works now.
**2026-07-01 NEXT-STEP DISPATCHED:** Roy picked 'have hive check'. Gave supervisor the non-destructive read cmd
(relay→Roy): `espflash read-flash 0x12000 0x200 node-persona.bin --port <NODE …F4:12:FA:52:99:28-if00>` (NOT the
B6:0A:A0 carrier — composer holds it). 0x200 = EXACTLY the firmware's read window (read_persona reads 512B @0x12000,
main.rs:1923/1943; persona CBOR ~336B + trailing 0xFF). read-flash is READ-ONLY (resets node→ROM briefly, rejoins).
INTERPRET: all-0xFF/00 ⇒ demo-unprovisioned [0x5C;32]; CBOR map byte + ascii tg_id ⇒ REAL persona (= hk source /
or STALE if hk≠nodes'). AWAITING the `xxd` dump → then I give the exact ONE-command alignment (provision-carrier-to-
match / erase-all-to-demo / serial-PROVISION). Supervisor expects a REAL persona (fresh-demo carrier couldn't verify
their good-key frames at all) — dump disambiguates real-vs-stale.
**SECURITY BRANCH on the dump (supervisor + composer's flag — apply when it lands):** extract the 32B hk + classify.
DEMO (all-0xFF/00 / no persona ⇒ [0x5C;32]) = THROWAWAY key → MAY be web-served (composer can hand it to the wasm
bridge's setGroupHmac over the wire). REAL (CBOR persona hk) = a LIVE GroupHmac secret → MUST NOT be web-served;
deliver it to the bridge out-of-band (local file / env), never over composer's web channel. The classification picks
BOTH the alignment command AND the key-serve path. Coordinate composer on key-serve + the carrier hk-alignment (the
bridge's WasmHive.setGroupHmac gets the SAME hk the nodes use). Standing: keep peer-refuting the deliver-gate.


## ⏳ 2026-07-01 — #26 FRONT HALF: wasm TG-member group-hmac + bridge control channel [hive @47590b1 + @3a3af06]
Composer (carrier-as-bridge weave) asked for 2 mechanisms to weave browser/IP wasm hives into the boards' ONE TG mesh:
**(2) wasm TG join [r2-hive-wasm v0.4.6 @47590b1]:** `WasmHive.withGroupHmac(id,hk,tgHash)` ctor + `setGroupHmac(hk,tgHash)`
runtime join/leave + `verifyFrame(frame)->{keyed,tg_ok,hmac_ok,deliver}` = the REAL deliver-gate (firmware main.rs:1751-2:
tg_ok=target_group==tg_hash||0, hmac_ok=verify_extended). build_frame/build_heartbeat/start_ota/ensemble frames SIGN
(sign_extended, firmware :1011-13) + stamp target_group=tgHash when a member. hk = persona's 32B SYMMETRIC GroupHmac key
(NOT withOta's Ed25519 tg_pk — TWO keys). No key = legacy TG-agnostic sim (unchanged). Real r2-trust dep (default-features
=false = member-only, no keyholder/getrandom). **wasm32-unknown-unknown RELEASE build GREEN** (r2-trust wasm-clean). Test
`group_hmac_frame_crossing_same_key_delivers_wrong_key_rejects`: same hk->deliver; same tg+wrong hk->tg_ok:true hmac_ok:
false deliver:false (= live carrier symptom); join/leave flips. ⚠ deliver-gate SECURITY-CRITICAL -> **peer-refute OWED**
before #26 'done' (API shape stable, only hardening). hk VALUE pending Roy's persona dump (value-independent API). Ties to
the SECURITY BRANCH above: the bridge's setGroupHmac gets the SAME hk the nodes use (demo=web-serveable, real=out-of-band).
**(1) bridge --control [@3a3af06]:** closes the gap (--participate only ingested from serial). `--control` reads bridge
STDIN: `RX <hex>`->carrier hive router (relay/dedup/re-flood, repeater) ; `TX <hex>`->INJECT verbatim to serial
(transparent egress, honors --participate). Functional-tested (RX/TX/read-only-gate/bad-verb). README control table added.
Notified composer with exact signatures. REMAINING #26: WS + UDP transports + carrier multi-transport gateway.

## ✅ 2026-07-01 — THEATER ORACLE: neighbour/path classifier getters [hive @664e8b3, r2-hive-wasm v0.4.5]
composer's next theater arm (conj 100/103 mobile-vs-infra classify + evict-at-floor/rediscovery; 200/204 used-path-
wins/idle-decays). Read-only over EXISTING r2-route state — no engine change. New WasmHive methods:
- `neighbours()` → JSON `[{hive_id,viable,confidence,last_seen,class:infra|mobile,duty,fade_remaining}]`. `viable` =
  `is_viable(FORWARDING_CONFIDENCE_FLOOR=0.1)` — SAME floor the forwarder uses (r2-route engine.rs:607/648) = engine
  truth. `class`=MobilityClass (decay-λ). `fade_remaining`=secs to floor (`neighbour_fade_remaining`, t=ln(conf/floor)/λ).
- `paths()` → JSON `[{destination,next_hop,confidence,last_updated,sample_count}]` (conj 200/204).
- `decay(now)` → real decay_neighbours+decay_paths; needed because confidence rises only on observation, falls only on a
  decay tick → drag-out-of-range = stop route_frame + decay(now)↑ → confidence falls/viable→false/evict; fresh frame=rediscovery.
- directed_via/flooded oracle = ALREADY in route_frame return (outcome=Directed+send target / outcome=Flooded). No new getter.
Test neighbour_oracle_learns_then_fades_below_floor (learn→viable→decay→evicted). wasm32 + 7 host tests green.

## ✅ 2026-07-01 — HW CLEAN-RESET PREP RECIPE (Roy KARAWHIUA / aggressive reset) — build-verified
**Deliverable:** exact Roy run-sheet to reset all DFR1195 dev boards to one image + one fresh throwaway TG.
**Q1 build-verify (on alfred, NOT asserted):** combined image FAILED first build — fr4 role/SCF telemetry
(msg.scffwd/silence/hold) calls `emit_msg` which was `routetest`-gated; every metal fr4 build pulled routetest
transitively so field/fr4-standalone was never built. **FIXED durably** (dfr1195-fw `4771e94`: emit_msg now
`any(routetest,fr4)`). RECIPE IMAGE = `carrier,multitg,field,routetest` → CLEAN, 1.32MB ELF, staged
`alfred:~/r2-dfr1195-weave.elf`.
**PATH = PERSONA bundles, NOT serial-PROVISION** (caught via composer): PROVISION@0x14000 sets target_group=RAW
tg_id (no FNV); composer wasm+tooling use tg_hash=FNV-1a-32(tg_id); PERSONA sets board tg_hash=FNV(tg_id) → MATCHES.
composer's `gen-persona --emit-weave-key` builds persona-<mac>.bin@0x12000 + weave-hk (wasm serve), e2e-verified.
field OK (persona present → not INERT). routetest = composer's live msg.* route-walk telemetry.
**ROY RUN-SHEET (per board, by-id; all espflash=Roy) — CORRECTED for the persona-clobber trap:** 0. composer
gen-persona → persona-<mac>.bin+weave-hk. 1. `espflash erase-flash`. 2. `espflash flash --chip esp32s3
--partition-table ~/dfr1195-partitions.csv ~/r2-dfr1195-weave.elf` ← **--partition-table MANDATORY** (else app→
0x10000 spans+clobbers persona@0x12000 + won't boot; app must be ota_0@0x20000). 3. `espflash write-bin 0x12000
persona-<mac>.bin`. 4. (opt) `write-bin 0x17000 role.bin` (RPF1 48B: 0=Repeater 1=Sensor 2=Bridge 3=Receiver;
omit→Repeater). 5. composer serves weave-hk→wasm setGroupHmac + bridge --participate. CSV staged alfred:~/dfr1195-
partitions.csv. erase-flash wipes bootloader too; step2 rewrites bootloader+parttable+app (self-contained).
**BLOCKING (asked composer):** per-mac personas MUST share {tg_id,hk,tg_pk} + DISTINCT master_secret → distinct
hive_id (hive_id=FNV(master_secret,tg_id); shared master_secret=identical hive_id=routing collapse). GO on confirm.
**ALFRED BUILD CAPABILITY (new):** rsync worktree → alfred:~/dfr1195-fw-build/ ; `source ~/Development/homelab/
export-esp.sh && cd platforms/dfr1195 && cargo +esp build --release --no-default-features --features <set>`. esp
toolchain + espflash + 4 boards on alfred. Can now build-verify firmware combos remotely (not just static analysis).
4 board ports: 50:23:E4, 50:26:98, 52:99:28, B6:0A:A0(carrier). See [[dfr1195-firmware-bench-workflow]].

## ✅ 2026-07-01 — WEAVE Qs answered + #26 r2-trust portion found DONE
Composer's carrier-as-bridge weave Qs (via supervisor), both verified in r2-hive-wasm src + 6 host tests green:
- **(b) GroupHmac/TG-key API ALREADY EXISTS** (no new code): `WasmHive.withGroupHmac(hive_id,hk,tg_hash)` /
  `setGroupHmac(hk,tg_hash)`. hk = persona's 32B group HMAC key (≠ withOta's Ed25519 tg_pk). Set → build_frame/
  build_heartbeat/ensemble SIGN via `sign_extended` (wire-identical to fw main.rs:1011) + stamp target_group →
  DFR nodes verify. Inbound: `verify_frame()` runs real `verify_extended` deliver-gate → {keyed,tg_ok,hmac_ok,
  deliver}. WEAVE needs setGroupHmac(nodes_hk,…) = the SAME hk as the carrier hk-alignment in flight.
- **(a) Arbitrary inject:** path-1 WORKS NOW — router calls `hive.build_frame(target,event_hash,payload,seq)` →
  INJECT (signed if keyed) = host-originated-arbitrary. path-2 (VERBATIM external browser bytes relayed as-is) =
  ~10-line bridge control-input add (parent stdin/FIFO/socket → 'INJECT <hex>' straight to serial), on request.
  Firmware INJECT = uart_rx_task parse_inject_hex → DATA_TX → ESP-NOW egress.
  **UPDATE 2026-07-01:** path-2 ALREADY SHIPPED (--control channel, control_reader). Re-verified functional:
  STDIN 'RX <hex>' → router relay (participate-gated via router_reader); 'TX <hex>' → verbatim 'INJECT <hex>' to
  serial (participate-gated). JSON {kind:control,verb,hex,routed/sent}. py_compile clean. Composer told → wires its
  client→server WS to the bridge stdin. Activation still gated on Roy's persona hk + the REAL-vs-DEMO serve branch.
  **DEPLOY-SYNC 2026-07-01:** Alfred runs the bridge from alfred:~/carrier-bridge/ (a SEPARATE copy, not a checkout).
  composer found it STALE (pre---control) + refreshed from repo; I verified BYTE-IDENTICAL after (sha256 match both
  files). Re-scp+sha-verify on every bridge change — I own Alfred deploy-sync. See [[carrier-bridge-alfred-deploy]].
- **#26 STATUS UPDATE:** the 'real r2-trust (TG/GroupHmac/deliver-gate)' portion of #26 is ALREADY DONE in wasm
  (real r2_trust::GroupHmac + sign_extended outbound + verify_extended inbound, exported + tested). **#26 remaining
  = WS + UDP transports ONLY.**
- **#26 VIRTUAL-TRANSPORTS scope (from core, 2026-07-01):** mostly-COMPOSITION not net-new — the route engine
  already treats each Transport type faithfully (a sim presenting as Transport::Lora/Ble/Wifi inherits the real
  routing math, isomorphism free). Exists to compose: §2.6 ConnectionlessRadio seam, per-transport MTU/power/jitter
  tables, LoRa ToA+duty+MTU math, harness faked-distance. NET-NEW ~1.5-2.5d: per-radio profile structs single-sourced
  from those tables + a UDP-backed ConnectionlessRadio. FLAG: wasm can't open UDP → profile is carrier-independent, a
  wasm node carries the SAME profile over WebSocket. core wants me to confirm host-UDP-first vs wasm-browser lead when
  I open #26 (I'll do BOTH bindings — supervisor pinned wasm-hive as browser/WS AND host/UDP). Ack'd core.
  **2026-07-01 core follow-up:** specs is PINNING the transport-profile field schema now; core will have the profile-
  table shape ready to coordinate. #26 DELIVERABLE flagged by core: r2-hive-wasm must EXPORT quality_from_rssi +
  range→loss for composer. is_reachability_blocked = grab on next r2-route re-vendor (§2.3B faked-distance ingress
  gate; flagged in core's ingest_observation caller-contract doc). Ping core the lead binding when I open #26.

## 📋 2026-07-01 — LoRa-into-bench SCOPE (Roy multi-transport direction; READ-ONLY, #16/#22)
**KEY FINDING: board-side LoRa is ALREADY BUILT + METAL-PROVEN — integration, not net-new dev.**
- (1) SX1262 driver/wiring DONE: core r2-sx1262 (impl LoRaRadio) present+current on dfr1195-fw (595ea65 RXEN,
  0cb30b2 AS923). DFR1195 integrated SX1262 pins CONFIRMED: SPI3 SCK7/MISO5/MOSI6 NSS10 BUSY40 RST41 RXEN(GPIO42
  host RF-switch) DIO1=4; 8MHz Mode0; wairoa_as923_nz 916.8MHz. RxenRadio newtype = thin RF-switch seam. XIAO+Wio
  variant (DIO2 RF-switch) also wired (main.rs:565-616).
- (2) LoRa+ESP-NOW dual-radio + R2-ROUTE auto-bridge DONE = 'bridge' feature (TN-FR-2). Per-transport TX chans
  (DATA_TX_LORA vs DATA_TX); engine auto-bridges (best_transport→Hop{nbr,transport}, no bridge code); transport-
  agnostic dedup → exactly-once crossing. Data-plane = LoRaTransport::service() (lora_transport.rs+lora_airtime.rs).
- (3) FLAG not net-new: loraroute (=lora+routetest+r2-transport/alloc) + bridge (un-gates ESP-NOW). METAL 2026-06-23:
  FR-1 PASS, FR-2 PASS (12 events crossed exactly-once), FR-4 SURVIVED-METAL (see [[lora-message-passing-metal]]).
**REMAINING COST = integration:** xtensa build on alfred only (no local build-verify); multitg required for LoRa
routing (TG key NVS@0x14000); RIG-PINNING — bench-default consts hardcode tuxedo D1-D4 hive_ids (remap+rebuild for
alfred X1-X4, OR use 'field' role-profile@0x17000); live = a bench-cycle (flash+prov2+run), gated composer bench-ssh.
**#26 tie-in:** wasm SIM heterogeneous bench would tag Transport (r2-route enum exists) to SHOW LoRa-vs-ESPNOW links.
**CORE CONFIRMED (re-vendor CLEAN, zero breaking):** diffed core HEAD 274941f vs dfr1195-fw vendored state — lora.rs
(LoRaRadio seam) / transport.rs (Transport trait) / lora_transport.rs (service()) / lora_airtime.rs all BYTE-IDENTICAL.
Two ADDITIVE-only deltas, harmless: (1) r2-transport 'mesh' module = §2.6 ConnectionlessRadio/MeshTransport (NOT on LoRa
path); (2) r2-sx1262 with_dio2_as_rf_switch() ctor (board uses RXEN → ignore). → ZERO dev cost to refresh LoRa; cost =
rig-remap + xtensa-on-alfred + bench-cycle. SCOPE CLOSED.
**BONUS #20 UNBLOCK:** §2.6 ConnectionlessRadio/ConnectionlessMeshTransport (ESP-NOW connectionless bearer) is NOW on
core HEAD → #20's 're-vendor to 0df6feb' gate is effectively MET. #20 buildable whenever prioritized.

## ✅ 2026-07-01 — CORE-SYNC §5.5 inv-5 (reject-while-pending) [hive @c7978c5]
Core type-enforced §5.5 invariant-5 (r2-core e921622): `ImageSink::pending_seq()->Option<u32>` (default None) +
`ApplyError::PendingUpdate{pending_seq,this_seq}` — `SignedOtaApply::start()` rejects unless new seq STRICTLY > staged
pending seq. NON-BREAKING for sim: MemSink keeps default `pending_seq()=None` (no pending window, exempt). Only hive
adaptation: `apply_reason()` match is exhaustive (no wildcard) → added arm `PendingUpdate => 0x71` (retry-after-reboot,
distinct from sink/capacity 0x70). r2-hive-core (8) + r2-hive-wasm (5) tests green. ACKed core.
**OWED on board FirmwareSink:** override `pending_seq()` → staged-but-unconfirmed seq (anti_rollback::load_pending
equiv) so `start()` enforces inv-5 for the board automatically; `apply_reason` already maps it. No separate begin-gate.

## ✅ 2026-07-01 — OTA-IN-WASM: pure OTA plugin+sentant (increment-3) + wasm nodes OTA each other [task #25 DONE]
**Directive:** wasm hives ACT LIKE REAL HW incl OTA; the wasm OTA-as-plugin+sentant IS the increment-3 PURE OTA form
(one piece of work advances both). core CONFIRMED the OTA stack runs wasm32 (r2-update verify-only, no getrandom) +
flagged the combined-graph build-verify (DONE @77c8621). TEST/validation — NOT a substitute for the held codex
refute of `ota_receive_over_coc`.
**DELIVERED @f7a0f0d (r2-hive-wasm v0.4.0):**
- `r2-hive-core::ensemble` (shared): `FlashSink` trait (the ONLY per-platform seam) + `MemSink` (wasm in-mem
  image). `OtaPlugin<S:FlashSink>` impl `r2_engine::Plugin` — OST→`verify_header`, ODT→`pv.update`+`sink.write`,
  OCM→`pv.finish`+`finalize`, reusing r2_update verify_header/PayloadVerifier/Ed25519/4-gate/anti-rollback
  VERBATIM (verify-before-write: a bad image never finalizes). Buffers `r2.update.progress`, drained via `poll()`.
  `OtaSentant` (control): OST/ODT/OCM→PluginCall, re-broadcast PROGRESS. Event hashes: OST=0xe9444700
  ODT=0xeb1afc1f OCM=0xe21d2c8b PROGRESS=0x7b241625 (HB=0x67ec1945). progress payload=[phase][done BE32][total
  BE32][reason]; phase 0=START_OK 1=DATA 2=VERIFIED 3=APPLIED 0xFF=REJECT.
- `r2-hive-wasm` v0.4.0: `WasmHive::withOta(hive_id, tg_pk)` (OTA-capable receiver), `startOta(target, pkg)`
  (updater → OST/ODT*/OCM frames, chunk 200), `deliver_event` now runs the full bus cycle (loops poll_plugins+tick
  so multi-progress OCM=VERIFIED+APPLIED both surface) → returns `{frames:[…]}` incl progress.
- **VERIFIED:** ota_plugin_verifies_and_applies (real signed pkg → APPLIED + image written) + rejects_tampered +
  rejects_replayed_seq + **ota_over_wasm_mesh_e2e** (updater.startOta→receiver.withOta.deliver_event→APPLIED).
  wasm32 + host workspace clean; startOta/withOta in web .d.ts. composer has the live API + hashes + progress shape.
**NEXT PHASE [task #26]:** full-real-stack production no-radio hive (web/WS + UDP) + refutation instrument — real
r2-trust in wasm (TG/GroupHmac, derive_peering_keys, deliver-gate, L5) + real WS + UDP transports (coordinate
core's udp). Radio-less tier (MCU=radio / host+browser=IP), reaching radio hives via the Alfred carrier.
**OTA codex refute (ota_receive_over_coc) STILL HELD — separate from this wasm validation.**

### convergence-v2 @e9e2775 — STATE B (authoritative final): core's SignedOtaApply orchestrator
Supervisor CORRECTED the v1 ruling (the "use FirmwareSink / ignore apply.rs / verify-only" msg was a
STALE-CHECKOUT read of a53a07b). AUTHORITATIVE = STATE B (OTA_PLUGIN_SHAPE.md @a97ac8d): core owns BOTH the
verify primitive AND the canonical `r2_update::apply::SignedOtaApply<S: ImageSink>` orchestrator (the
verify-before-write RCE-guard ordering is SHARED in core, NOT re-implemented per platform). Converged onto it:
- `MemSink` impls `r2_update::apply::ImageSink` (begin/write/activate); board esp_ota_* impl = firmware (a)-refactor.
- `OtaApplier<S: ImageSink>` buffers OST/ODT/OCM (= CMD_START_SIGNED datagram-framed) → on commit runs
  start(verify_header 4-gate/Ed25519/anti-rollback + PT_FIRMWARE_FULL type-gate + begin) → feed(verify-then-write/
  chunk) → finish(hash-confirm THEN activate). Bad image never activates. Early verify_header on OST = fast reject.
- `OtaSentant<S>` owns the applier + broadcasts r2.update.progress (dropped the r2_engine::Plugin indirection).
- **Borrow note (flagged to core):** SignedOtaApply borrows &mut sink + finish consumes self → can't persist across
  discrete EventBus events; wasm BUFFERS-then-applies-on-commit, MCU streams the SAME orchestrator. Shared ordering.
- NO wire/API change → composer UX + minted pkg stay valid. Tests: ota_applier_verifies_and_applies / rejects_
  tampered / rejects_replayed_seq / ota_over_wasm_mesh_e2e green; wasm32 from-source + host clean.
**MINTED for composer's live demo:** `~/r2-staota-artifacts/ota-test-pkg.bin` (1187B = header123‖payload1000‖sig64)
+ `ota-test-pkg.tg_pk.hex` (tg_pk 5f671329…945b), on TUXEDO + Alfred. Re-mint: `cargo test mint_ota_artifacts --
--ignored` in crates/r2-hive-wasm. composer's from-source wasm build FIXED (FlashSink removed).
**SignedOtaApply codex refute (core-side) + ota_receive_over_coc refute (hive-side) gate METAL separately.**

### A7/A8 type-confusion fix + composer finishers @11c5156 (v0.4.1)
core (verify-don't-assume) found my v1 OtaPlugin OST omitted the payload_type gate (a signed DIFF/RECOVERY would
install as FULL = RCE-class). RECONCILE: the LIVE path is already v2 (SignedOtaApply), whose `start()` gates
`payload_type != PT_FIRMWARE_FULL` BY CONSTRUCTION (apply.rs:99) — so 'ruling B' was already satisfied; v1 inline
is gone. Added belt-and-braces: `OtaApplier::on_ost` rejects DIFF/RECOVERY EARLY + regression test
`ota_rejects_type_confusion` (signed DIFF → REJECT, never activated). Gate now at BOTH early-OST + commit-time
SignedOtaApply. The CLAUDE/codex OTA refute should target the SignedOtaApply path (e9e2775+), not the v1 orphan.
**composer finishers DONE:** (1) `deliver_event` returns STRUCTURED progress —
`{"frames":[…],"progress":[{phase,bytes_done,bytes_total,reason},…]}` (fixes composer's all-0 compact-frame
decode). (2) signed test pkg staged (above). composer can now render APPLIED + REJECT(tampered/unsigned/DIFF).

### Claude OTA-refuter findings — 2 regressions FIXED + tested @a56c1bc (v0.4.2); F3 → core
The refuter confirmed core's SignedOtaApply SEQUENCE sound (verify-before-write/type-gate/hash-before-activate);
the 3 findings were all in MY hive OtaApplier ADAPTER seam (gaps the orchestrator can't close for the caller):
- **F1 (HIGH) anti-rollback floor never advanced** — on_ocm dropped AppliedUpdate → cfg.current_seq frozen →
  REPLAY + DOWNGRADE (defeats §10.1#3). FIXED: on_ocm advances cfg.current_seq=applied.seq + authority_epoch_floor
  BEFORE APPLIED, resets per-transfer state. Test `ota_advances_floor_blocks_replay_and_downgrade`. (Board
  persists floor→NVS; sim = cfg-in-RAM node-session floor.)
- **F2 (MED/HIGH) unbounded ODT buffer + lost TOO_BIG** — OOM via replay-OST-then-flood. FIXED: on_ost rejects
  payload_len>OTA_MAX_IMAGE(4MB); on_odt rejects buf+chunk>total → closes transfer. Test `ota_bounds_odt_buffer`.
- **F3 (LOW) no abort() on reject** — ImageSink (core trait) has no abort → partial staging left; mitigated by
  MemSink::begin-clears-next-attempt (never read/activated). FLAGGED core to add ImageSink::abort (+capacity).
OtaConfig gained `authority_epoch_floor`. 7 ensemble tests + wasm e2e green; wasm32+host clean. These GATE METAL
(Roy-gated) — closed except F3-pending-core. Refuter should re-run on a56c1bc.

### convergence-v3 @fc291da (v0.4.3) — core folded F1/F2/F3 INTO the orchestrator (un-skippable)
core updated `r2_update::apply::ImageSink`: `capacity()` (F2 → orchestrator rejects oversized before begin,
`ApplyError::CapacityExceeded`), `current_seq_floor()` + `activate(&AppliedUpdate)` that MUST persist the floor
(F1 → orchestrator does the commit-time anti-rollback re-check, the SINK persists), `abort()` on every post-begin
failure (F3). All 3 are now STRUCTURAL in core. Converged hive:
- MemSink impls the new trait; the anti-rollback floor LIVES IN THE SINK (current_seq_floor/activate-persists),
  not my adapter. Dropped OtaConfig.current_seq + my manual on_ocm floor-bump + manual abort (orchestrator+sink
  do them). `OtaApplier::ctx()` reads current_seq from sink (the trait invariant) + returns `DeviceContext<'static>`.
- KEPT hive-side: the pre-start buffer bound (payload_len > sink.capacity() at OST + buf>total at ODT) — my
  event-driven adapter buffers in RAM BEFORE OCM, so the orchestrator's commit-time capacity check is too late to
  stop the buffer OOM; the early bound guards the RAM buffer. (Flagged this to core.)
- 3rd reject arm minted: `ota-test-pkg-diff.bin` (signed payload_type=0x02 → A7/A8 REJECT), tuxedo+Alfred.
Net: F1+F2+F3 closed structurally in core + the buffer guard hive-side. 7 ensemble tests + wasm e2e green.
composer has all 4 demo arms (APPLIED + tampered/unsigned/wrong-TYPE reject). SignedOtaApply codex refute (core)
gates METAL.

### refuter RE-VERDICT (a56c1bc) + follow-ups @83f2b91 — F1+F2 GENUINELY closed; board-brick contract documented
Claude OTA refuter re-ran: F1+F2 genuinely closed for host/wasm (no TOCTOU, OOM-bounded-before-growth, the 3 tests
exercise REAL exploits: capture-replay / signed-downgrade / flood / type-confusion). ONE new BOARD-ONLY finding
(gates METAL): my F1 commits the floor at apply-time = correct for SIM, but the BOARD ImageSink::activate MUST
DEFER the NVS floor commit to BOOT-CONFIRM (stage pending+(seq,hash); bump persisted floor only after confirmed
boot + §5 health check; cf. linux ota_tcp_recv.rs:606-613) — immediate persist strands a failed-boot image below
the floor = remote BRICK. FIXED: corrected the MemSink::activate contract comment (sim=immediate-RAM right; board
MUST boot-confirm) so the firmware (a)-refactor doesn't inherit the brick reading. MINOR done: 2 tests now assert
reject-REASON bytes (StaleSeq 6 / LengthMismatch 2); noted 4MB=sim ceiling, board=~1.5MB ota_1 slot.
**OTA-in-wasm FULLY CLOSED.** The board OTA (a)-refactor (port ota_receive_over_coc → this ensemble OtaApplier +
a boot-confirm-staging FirmwareSink→ImageSink) is owed when firmware OTA is built; contract baked into the comment.
### .progress reason-byte fix @41ae9e4 (v0.4.4) + core boot-confirm contract ACK (fdb9d74)
composer (5-arm falsification theater on the real wasm receiver — full/tampered/wrong-key/DIFF/replay all probe-
verified) found the structured `.progress` reason read 0 for the 3 OST-TIME rejects (only OCM-time tampered=5
surfaced). Root cause: after an OST reject (header_ok=false), trailing ODT/OCM frames emitted reason 0 → the
bench's LAST .progress entry overwrote the correct reason. FIXED: sticky `last_reason` re-emitted on every trailing
frame of a dead transfer (cleared at next OST); reset→clear_transfer + a reject() helper. Now all 5 arms surface
the reason: tampered=5(hash) / wrong-key=3-4(sig/signer) / DIFF=1(BadHeader A7-A8) / replay=6(StaleSeq) / full=
APPLIED. Test `ota_reject_reason_propagates_to_trailing_frames`. 8 ensemble tests + wasm e2e green.
core ACK (fdb9d74): the boot-confirm contract = exactly what I'd documented (sim immediate, board stage-pending+
confirm-on-boot, authority_epoch immediate, current_seq_floor returns CONFIRMED). No sim change. Board contract
baked in the comment for the (a)-refactor.

### OTA-in-wasm: COMPLETE (v0.4.4). Canonical SignedOtaApply; A7/A8 + F1/F2/F3 + reason-display all closed;
### 8 ensemble tests + wasm e2e; composer's 5-arm theater green. Board OTA (a)-refactor owed when firmware OTA built.

**NEXT: #26** full-real-stack wasm hive — real r2-trust (TG/GroupHmac/deliver-gate, no-RNG verify paths first;
key-minting needs injected RNG) + WS + UDP transports + the carrier multi-transport gateway (tier-fusion).

### convergence-v1 @1a8f7a9 — applied core's OTA-plugin ruling (OTA_PLUGIN_SHAPE.md a53a07b) [SUPERSEDED by v2]
core RULED the canonical OTA-plugin shape; supervisor CORRECTED the doc (IGNORE the experimental
`r2-update::SignedOtaApply`/`ImageSink` orphan — it breaks r2-update's verify-only layering; r2-update stays
VERIFY-ONLY; the EXISTING `r2-hive-core::ota::FirmwareSink` is the one canonical seam). Converged: dropped the
ad-hoc `FlashSink` I'd introduced → `OtaPlugin<S: ota::FirmwareSink>` (slot_capacity/begin/write_chunk/finalize/
abort); MemSink impls FirmwareSink (wasm RAM); board esp_ota_* impl = the firmware (a)-refactor later (one plugin,
sink swaps). Sequence per doc §2: verify_header → TOO_BIG precheck → begin → per-chunk{PayloadVerifier::update THEN
write_chunk} → finish → finalize; `sink.abort()` on EVERY reject. NO wire/API change → composer's OTA UX (ecbad9f)
stays live (OST/ODT/OCM = CMD_START_SIGNED datagram-framed; verify contract = r2-update verbatim). RNG note (core):
verify/deliver-gate/membership = no RNG (my OTA path is verify-only); in-wasm key-MINTING (provisioning/TG-join)
needs caller-injected RNG (getrandom-js browser / seeded ChaCha for deterministic refutation runs) → lands in #26.


## ✅ 2026-07-01 — UNIFIED ENSEMBLE increment-1: HB sentant on the EventBus (shared core + wasm) [task #25]
**Directive (Roy/supervisor):** make wasm-sim hives run the SAME basic ensemble as the DFR1195 (sentants/plugins on
the r2_engine EventBus — HB + provisioning/TG + OTA plugin+sentant), over the wasm virtual-mesh bearer. The wasm
OTA-as-plugin+sentant IS the pure increment-3 OTA form (one piece of work advances both). Coordinate core (OTA
mechanics) + composer (UX). NOT a substitute for the held codex refute of ota_receive_over_coc.
**FEASIBILITY PROVEN:** r2-engine (EventBus) + r2-update (OTA verify) BOTH build wasm32-clean.
**INCREMENT-1 DONE (@693853e):**
- `r2-hive-core::ensemble` (NEW, shared across wasm/Linux/ESP32) — `HbSentant` impl `r2_engine::Sentant`: on a host
  `TICK` it broadcasts a heartbeat (payload = hive_id BE32 = firmware HB wire form). `TICK_HASH`/`HEARTBEAT_HASH`.
  r2-engine added as a no_std+alloc dep of r2-hive-core. Test `hb_sentant_emits_on_tick`.
- `r2-hive-wasm` v0.3.0 — `WasmHive` now hosts an `EventBus` with the HbSentant = UNIFIED node (routing via
  `route_frame` + ensemble via `tick(seq)->{frames:[hex]}` / `deliver_event(frame)->event_hash`). So a wasm node
  ORIGINATES its HB via the same sentant the board runs. Test `ensemble_tick_emits_heartbeat_to_peer` (A.tick→HB
  frame→B's ensemble sees HEARTBEAT_HASH). Host workspace no-regression; new API in web .d.ts. composer notified.
**NEXT — OTA plugin+sentant (increment-2/3, the pure OTA form):** ASKED CORE (fleet ask, reply→inbox): canonical
OTA plugin shape? where does the shared OTA plugin live (r2-hive-core::ensemble vs r2-update helper)? **FlashSink
trait seam** so ONE OtaPlugin drives real-flash on the board + a memsink in wasm (I lean yes). Build after core's
ruling: OtaPlugin (verify_header/PayloadVerifier/Ed25519, OST/ODT/OCM, 4-gate/anti-rollback) + OtaSentant in
r2-hive-core::ensemble → wasm nodes OTA each other (software e2e) → same plugin compiles into firmware = the #19
(a)-refactor. HELD on core's answer + the ota_receive_over_coc refute (this is TEST/validation only).


## ✅ 2026-07-01 — r2-hive-wasm v0.2.0: in-wasm R2-WIRE encode helpers (composer's bench-sim ask)
composer's browser wasm-SIM (de95e1e, webapp/bench-sim.html) is FUNCTIONING on r2-hive-wasm @71b2b32 — N WasmHive
nodes flood real frames over a virtual mesh, headless-verified (floods=5, real loop-prevention). They asked for
per-node frame origination (so each node floods its OWN HB with proper origin, not the fixed aa→bb test vector).
**SHIPPED `6f3b96a` (v0.2.0):** `WasmHive.build_heartbeat(seq)->Uint8Array` (origin=self in route stack, payload=
self hive_id BE32 = firmware HB wire form) + `build_frame(target_hive,event_hash,payload,seq)->Uint8Array` (generic
Event). Both use the SAME `r2_wire::encode_extended` the firmware uses ⇒ sim traffic WIRE-IDENTICAL to real-HW (sim
+ carrier tier speak the same bytes). r2-wire promoted dev-dep→dep. version()→"0.2.0". Verified: `encode_helpers_
roundtrip` (A's HB/Event parse+route on node B) + wasm32 green + API in web .d.ts. Notified composer; offered
build_reply / TG-tagged HB variants. composer also wiring the carrier-bridge (R2RX→wasm→INJECT) host-reader into
the same bench view = real-HW carrier tier + wasm-sim rendering together.


## ✅ 2026-07-01 — host CARRIER-BRIDGE: DFR1195 carrier ↔ wasm-hive ↔ R2 mesh (loop CLOSED, staged on Alfred)
**Supervisor DO:** (i) scp carrier ELF→Alfred, (iii) write the host-bridge (R2RX→wasm-hive route→INJECT) with the
DTR hazard "impossible to get wrong"; + confirm the running boards already ESP-NOW-mesh+HB (→ carrier flash alone
= heartbeat-visibility).
**(i) DONE:** `r2-dfr1195-carrier.elf` scp'd → `Alfred:~/` (verified). Alfred has espflash+node+python3, and 4
Espressif USB-JTAG boards (50:23:E4 / 50:26:98 / 52:99:28 / B6:0A:A0) + 1 Arduino Leonardo.
**MINIMAL-PATH = YES:** deployed firmware DOES ESP-NOW-mesh + emit lub-dub HBs (`espnow_task`+`io_task`). So ONE
Roy cmd gives real-HW heartbeat-VISIBILITY, no node reflash: `espflash flash --monitor --chip esp32s3
~/r2-dfr1195-carrier.elf` streams `R2RX`+`ESP-NOW peer MAPPED` live. (Assumes running boards = default ch1 mesh,
not staota — SELF-CONFIRMS on flash. Did NOT pre-open any running board = the un-recoverable bricking risk, and
pointless since flash self-confirms.)
**(iii) BRIDGE DONE — committed r2-hive `010aa0d` (`crates/r2-hive-wasm/carrier-bridge/`), staged
`Alfred:~/carrier-bridge/`.** Architecture chosen FOR the DTR mandate: **Python parent OWNS the serial port
DTR/RTS-safe** (pyserial `dtr=False`/`rts=False` set BEFORE open, never toggled, ABORTS if it can't) = the ONLY
thing touching the port; **Node child = pure wasm-hive router, NO serial access → physically cannot brick**. Loop:
`R2RX <hex>` → `router.js` (wasm-hive `route_frame`) → `INJECT <hex>`. `--participate` OFF by default (logs
would-be injects; safe unattended). Vendored pyserial (pure-python, no pip/sudo) + wasmhive-node pkg shipped in
the bundle (gitignored in-repo; recreate per README — both on Alfred).
- **VERIFIED on Alfred:** `--selftest` runs there (node + vendored pyserial OK); positive loop proven with a REAL
  R2-WIRE frame pair → `Flooded sends=1` + `INJECT 0441…bba1f5ed00` (host hive `a1f5ed00` appended to route stack
  = it relayed). Test vector in the bridge README.
- **render handoff:** sent composer the stdout line format (OTA-RX peer-MAPPED / FRAME / [router] route / INJECT)
  + offered a JSON-lines mode. Earlier `scratchpad/r2-mesh-read.py` = the standalone DTR-safe reader (visibility
  only); the bridge supersedes it for the full loop.
**NET EOD:** heartbeat-visibility = Roy's ONE flash command; full participation = + the bridge. Everything staged
on Alfred for Roy's remote session. Carrier flash is remote-viable (no BOOT button — task-#14 proof). Task #23 +
the bridge = DONE pending Roy's flash. OTA-refute still HELD (no findings).


## ✅ 2026-07-01 — CARRIER firmware (Roy's all-radio-via-MCU bench): transparent serial↔ESP-NOW radio-modem
**Supervisor/Roy ask:** designate ONE DFR1195 as Alfred's MCU CARRIER (serial↔mesh bridge) so Alfred JOINS the R2
mesh as a real node (not a passive BLE scanner). The concrete enabler for real-HW heartbeat-visibility AND the
TCP↔radio gateway the wasm-hives need. Scope-then-build; Roy flashes (Roy-only).
**SCOPE finding:** no MK-DONGLE / R2-USB-relay-node crate exists, but the gap was SMALL — the ESP-NOW mesh+relay
(`espnow_task` + `io_task` RouteEngine) is built + metal-proven; the serial command bridge (`uart_rx_task`:
IDENTIFY/PROVISION/MASK/SENDTO) exists; hex-frame-over-serial egress is already a codebase convention (health
telemetry consumed by composer's serial-reader). Carrier = those + two thin legs.
**BUILT — `carrier` feature, r2-core branch `dfr1195-fw` @`d332251` (pushed). Transparent radio MODEM** (Roy's
exact model: carrier = Alfred's radio; ALFRED's hive does the routing/dedup; the DFR is the antenna):
- EGRESS (`espnow_task`): every received over-the-air R2-WIRE frame → host as `R2RX <hex>` line, emitted BEFORE
  local routing (`emit_carrier_rx`, one atomic println). `can_hear` still gates (a bench mask, if any, shapes it).
- INJECT (`uart_rx_task`): `INJECT <wire_hex>` → decode (`parse_inject_hex`) → `DATA_TX.try_send` → `espnow_task`
  ESP-NOW-broadcasts VERBATIM. ACK `INJECT-OK len=N` / NAK `INJECT-ERR bad-hex|queue-full`. line buf 160→600B
  under carrier (full 256B frame = "INJECT "+512hex). ch1 default (no `staota` ⇒ no lab-WiFi dependency).
- **VERIFIED:** `cargo build --release --features carrier` xtensa-GREEN (only pre-existing dead-code warnings);
  default `--release` still GREEN = **NO regression**. ELF staged `~/r2-staota-artifacts/r2-dfr1195-carrier.elf`
  (1.3 MB). EOD-flashable.
**4 NODE-BOARDS (the over-the-air mesh):** run the EXISTING heartbeat mesh build — NO new firmware. Flash
`--features ble` (ESP-NOW mesh + lub-dub HB; add `benchkeepalive` for watchable 8s keepalive). ALL 5 boards on
ch1. Do NOT USB-multiplex them (fakes the mesh). HEARTBEAT-VISIBILITY works EGRESS-ONLY (Alfred decodes R2RX, no
key). For Alfred to PARTICIPATE (inject HBs the nodes' deliver-gate accepts) all 5 must share the TG — simplest =
all unprovisioned (demo-TG via mac_low3 fallback) + Alfred uses the demo GroupHmac key.
**LOOP-CLOSER (asked supervisor whose it is — composer owns Alfred-side host, but the wasm-hive is mine):** a tiny
host bridge = read tty `R2RX <hex>` → `WasmHive.route_frame` → `sends[]` → `INJECT <hex>` to tty = the TCP↔radio
gateway uniting THIS turn's two deliverables (wasm-hive + carrier). Held pending the ownership answer to avoid
duplicate work with composer's sim. Task #23 = DONE (pending Roy-flash + host-bridge wiring).
**REMOTE-FLASH UNLOCK (Roy is AWAY from the bench — no physical access, no power-cycle, no BOOT button):**
- (a) AUTO-RESET FLASH = **YES, no button**. ESP32-S3 native USB-Serial-JTAG enters ROM download via the host's
  USB-CDC DTR/RTS sequence = exactly espflash's default reset. PROOF on these boards: task-#14 = a console-OPEN
  alone already drops a running board into download (rst:0x15 via DTR/RTS), so the full espflash sequence flashes
  remotely with certainty. Roy SSH→Alfred: `espflash flash --monitor --chip esp32s3 r2-dfr1195-carrier.elf`.
  Self-healing: `--after hard-reset` boots the new app; the carrier image carries the ca24915 clear-at-boot.
  ⇒ real-HW unblocks TODAY if Roy can reach Alfred. (ELF is on TUXEDO — needs scp→Alfred.)
- (b) EXISTING SERIAL TELEMETRY = **YES** (interim signal, no flash): running boards println! 'ESP-NOW peer MAPPED
  hive=.. mac=..' (= real over-the-air HB reception) + health-hex + liveness. ⚠ But opening the tty asserts
  DTR/RTS on most tools → the SAME task-#14 path drops the (older, pre-ca24915) board into download = silent, and
  Roy can't power-cycle. So reads MUST de-assert DTR+RTS before open. **Wrote a safe reader**
  `scratchpad/r2-mesh-read.py` (pyserial, dtr=False/rts=False-before-open, tags peer-MAPPED, decodes R2RX/health
  hex) — handed to composer (who holds the ttys). Offered to scp it.
- (c) carrier = built+staged (above).


## ✅ 2026-07-01 — current-TN WASM-HIVE delivered (crates/r2-hive-wasm) for composer's EOD bench sim
**Supervisor EOD ask:** composer is adapting workshop's wasm-hive (simpler TN) for a v1 sim today; the UPGRADE =
my one-codebase no_std hive → wasm on CURRENT TN crates, so the sim can run REAL current-TN. "produce/point-to a
current-TN wasm-hive build … but DON'T block composer's v1 on it." Prioritised BEHIND OTA-refute-response (which
is gated — no findings landed yet).
**DELIVERED — new crate `crates/r2-hive-wasm` (committed `71b2b32`, pushed platform-trait):**
- Thin wasm-bindgen browser host over the SAME `r2_hive_core::sync_host::route_inbound_sync` core the Linux host +
  ESP32-S3 firmware run (r2-route/r2-wire). NO fork — identical current-TN routing.
- API: `new WasmHive(hive_id)`; `hive.route_frame(source_hive, kind, frameBytes, now, dice) -> JSON
  {outcome, sent, sends:[{kind,target,frame(hex)}]}`. kind = R2-TRANSPORT §2.2 id (0=Ble 1=Wifi 2=Lora 3=Internet
  4=Usb 5=EspNow 6=Udp). Plus `provisional_id_mac(mac)` + `version()`. CaptureTransport (mirror of sync_host test
  StubTransport) records the engine's would-send frames; the sim IS the network (moves `sends` between nodes).
  Topology is LEARNED: route a frame FROM a node (immediate_source observation) before addressing TO it.
- **Workspace-EXCLUDED** (root Cargo.toml `exclude=["crates/r2-hive-wasm"]`) — std + wasm-bindgen, wasm-only — so
  host build/CI never compiles it for a non-wasm target. Confirmed via `cargo metadata` (not a member). ZERO
  host-CI impact. pkg/ + target/ gitignored (only source committed: Cargo.toml/lock, src/lib.rs, .gitignore).
- **VERIFIED (conjecture→refutation):** (1) `cargo build -p r2-hive-wasm --target wasm32-unknown-unknown --release`
  green; `wasm-pack build --target web` → 33KB wasm + JS glue. (2) node smoke (nodejs target, scratchpad): wasm
  loads; `provisional_id_mac` == a JS FNV-1a reference of the canonical addr ⇒ r2-route/r2-fnv id-core executes
  CORRECTLY in wasm; garbage→`NotR2Wire` JSON, no panic; WasmHive lifecycle ok. (3) host `cargo test` (rlib;
  wasm-bindgen attrs inert off-wasm): positive relay → Directed/Flooded with `sends` JSON populated (target +
  non-empty hex). Build command in the crate's lib.rs doc header.
- **Honest gap:** positive Flood/Directed is proven on HOST (route_frame wrapper) + the engine-runs-in-wasm is
  proven via FNV; I did NOT hand-craft a valid R2-WIRE frame to drive a positive case THROUGH wasm (composer's sim
  will). Residual wasm-only risk ≈ nil (same compiled core; boundary marshalling proven). Open offer to composer:
  add in-wasm R2-WIRE frame ENCODE helpers so the sim needn't hand-craft bytes.
- Sent composer (artifact+API+build cmd) and supervisor (delivery+CI note). Task #22 = DONE.
- **CI note:** `.github/workflows/ci.yml` triggers only on push:main / PR→main, so NO hosted run fires for
  platform-trait by design (the known CI-gap = a morning item, NOT introduced here). Local verification stands.

## ✅ 2026-07-01 — owed task-#4 cleared: r2-hive build+test GREEN vs consolidated r2-core; relay-v0.2 confirmed done
**Build/test (tip a038435):** `cargo build --workspace` clean; `cargo test --workspace` = ~200 passed / 0 failed /
3 pre-existing ignores (r2_hive lib 105, r2-hive-core 26, + 12 integration suites). All 5 vendored r2-core crates
(def/ensemble/dispatch/transport/discovery) resolve from ../r2-core/crates; r2-discovery stubbed transports compile
(runtime-noop as flagged by core). My wasm-crate exclude introduced ZERO regression (workspace unaffected). Result
reported to supervisor (the owed task-#4 build/test result).
**Relay v0.2 — already DONE (verify-then-record via git, NOT re-done):** R2-TRANSPORT-RELAY v0.2 device-side
challenge-response landed in `40eaf0e` (feat(compat): v0.2 device-first relay handshake) + `04d19cc` (nonce CSPRNG
routed through Platform seam) + `c5aec3e` (recv loops survive transient errors). handshake.rs reads inbound
{type:challenge,nonce}, echoes it, signs Ed25519 over `<nonce>:<trust_group>:<device_id>:<timestamp>` (4-field),
stateless ±60s timestamp fast-reject retained. specs ruled the Ed25519 primitive CORRECT (the §3.2 'HMAC' wording
was the spec defect, fixed in v0.2). So relay-handshake conformance = settled PASS, no further hive change.
**Net OPEN items (unchanged):** OTA-refute (#19, gated — no findings landed yet; triage+respond on arrival) +
metal e2e (Roy-flash-gated). §2.6 ESP-NOW bearer (#20, re-vendor-gated). Everything else this turn = delivered.


## ✅ 2026-06-30 — staota.0630.1659 VALIDATED on metal + 2 post-validation fixes committed (NOT yet staged)
**.1659 VALIDATED (supervisor + composer):** D3 provisioned is ALIVE + BEACONING — wire 46dbf1ae, fw
staota.0630.1659, §7 BLE BEACON adv up, LoRa SF7/916.8 up. My INERT-revert diagnosis held; the provisioned path
works. blank-INERT was benign (confirmed). Remaining provisioned-board issue: D3's LCD DARK even when alive (see
dark-LCD below).
**Three fixes committed on `dfr1195-fw` (xtensa-green, DESK-VALIDATION-REQUIRED, NOT staged to artifacts — .1408
lesson: build-green ≠ boot-green for this region). They form a coherent next rev; STAGING DECISION is with
supervisor (keep .1659 as known-good baseline vs stage a new rev for desk-validation):**
- `bf205d5` — moved `esp_rtos::start` ABOVE the §3.5 INERT block. Fixes the INERT liveness DEADLOCK (Timer::after
  ran before the embassy time-driver was registered → one boot burst then hang). Verified staota-DFR + bench +
  staota-XIAO. Also gives INERT post-init context to RE-ADD the in-INERT console receiver later (deferred).
- `ca24915` — clear `force_download_boot` at app boot. Core-confirmed: that RTC bit is STICKY by design (ROM never
  auto-clears) → after one reboot_to_download, ANY later reset (console-open chip-reset/brownout/WDT) re-enters
  ROM download FOREVER. Clear-at-boot makes it one-shot. Highest-value half of the USB-JTAG finding.
- `6323f29` — B5 §7 BLE beacon class_hash = role device-class hash BIG-ENDIAN (was my_tg_hash.to_le_bytes() — a
  clear-text TG-identity leak + wrong byte order; specs ruling R2-BEACON v0.12 §7.4.0/§7.4.1). Widened
  role_class_hash/fnv1a32 cfg lora→any(lora,ble); pass class_hash:u32 into ble_task. Per-role wire values:
  repeater C60DD3A9, sensor 991DB9AF, bridge D81020E4, receiver A5A3980C (all big-endian). Flagged composer to
  update verify-board.py to the spec value. LoRa beacon was already correct.
**USB-JTAG console-open reset (supervisor's big finding) — joint answer w/ core:** console-open → 'rst:0x15
USB_UART_CHIP_RESET → boot DOWNLOAD' = ESP32-S3 ROM host DTR/RTS download trigger + (on boards that ran
reboot_to_download) the sticky force_download_boot bit. NOT my app code. Core: no esp-hal disable for the host
trigger (raw PAC write only; it disables over-USB auto-reset, reboot_to_download replaces it); eFuses off-limits
(permanent). PLAN (core's order): clear-at-boot DONE → composer re-tests console-open → add PAC register-disable
ONLY if it still resets. **RE-IMAGE ESCAPE GAP (answered to composer):** depends how the board entered download.
PRIMARY path (esptool DTR/RTS auto-reset enters download — works remotely, = the console-open-reset behavior):
force_download_boot NOT set → `--after hard-reset` boots app → clear-at-boot fires → NO gap, no tool change.
reboot_to_download path (bit SET): EN-toggle hard-reset preserves the always-on RTC bit → re-enters download;
escape via (i) POWER-CYCLE (clears always-on RTC; recommended) or (ii) tool register-clear (read-modify-write)
RTC_CNTL_OPTION1 @0x6000_8128, force_download_boot = bit0 — CONFIRMED vs esp32s3-0.30.0 PAC (base 0x6000_8000 +
offset 0x128; SVD-derived). So the gap is reboot_to_download-only; composer owns flash-board.sh's choice
(documenting power-cycle as default). Observe beacons by BLE scan, NOT console-open (still resets).
**DARK-LCD on provisioned D3 (task #13): RESOLVED = NON-BUG.** Roy clarified D3's screen shows content; the "dark"
was only while D3 sat in the BOOTLOADER (no app running). Provisioned app renders fine. Firmware confirms: 0xFF
(erased, what a DFR's 0x13000 has) → `b[0] != 0x00` → has_screen=TRUE → display inits. NOT board-profile. Do NOT
add a DFR 0x13000 write. Task #13 REFRAMED → LCD 'TN READY' status-screen redesign. **DONE (64bc0be):** 6-line
render — L1 'R2 TN READY' / L2 'hive <id>' / L3 '<role> fw<rev>' / L4 'BLE+ LoRa+ TG+' (new BLE_UP/LORA_UP
atomics) / L5 'nbrs:N ADV+' / L6 sync. Human label 'D3' on L2 = pending composer NVS-write coordination
(proposed 0x18000 [magic LBL1][len][utf8≤15]); Roy's display-form pref (D3 vs D3+hex) pending. Ships bundle-only.

**NEXT-REV BUNDLE (ONE OTA rev, supervisor-sequenced; .1659 held as baseline tonight) — readiness tracker:**
| piece | state |
|---|---|
| INERT esp_rtos reorder | ✅ bf205d5 |
| clear force_download_boot | ✅ ca24915 |
| class_hash structure (role-class, BE) | ✅ 6323f29 |
| class_hash canonical strings (v0.16 §4.1) | ✅ 765c948 (ai.reality2.device.*; repeater 00FC1F17 / sensor 43895E89 / bridge B52C9F26 / receiver 17F3554A BE) |
| LCD TN-READY + human-label | ✅ 64bc0be render + 712fc34 NVS-label read (composer confirmed + writes 'D3' @0x18000 [LBL1][len][utf8]; L2 hex-fallback) |
| Company-ID 0xFFFF prepend | ✅ 5e57aeb (was THE beacon-regression root cause: omitted prepend → 0x01B2 off-by-2; now §7.3 [FF FF][magic 0xB2 @ AD-off 4]) |
| BLE address opacity | ✅ 11d99bc (opaque per-boot HWRNG random, static-random type, NOT wire_id-derived; composer's RBID-resolver VERIFIED 2 ways → bench ID survives) |
**STATE: 8/8 COMPLETE + SHIP-GATE CLOSED.** All firmware done (tip 11d99bc), xtensa-green. Resolver gate
SATISFIED: composer's rbid-resolver is live+verified (D3+D5 resolve via rbid, address-independent, webapp-side)
AND the firmware rbid EPOCH IS PINNED AT 0 (hardcoded `let epoch: u64 = 0`, no rotation) so composer's static
epoch-0 table holds → clear to ship. SSID-rebuild = DROPPED (OTA rides BLE→transient-SoftAP, not permanent-STA).
READY to stage DFR+XIAO the instant supervisor gives the OTA-or-desk word.

## ► 2026-06-30 — NEXT PHASE: OTA DELIVERY (BLE-negotiate → transient SoftAP) + L2CAP throughput bench
**OTA model (supervisor, spec-grounded — R2-UPDATE / R2-BLE / R2-WIFI §3.3):** NOT permanent-STA. Flow = BLE
discovery+negotiate (#ota_query/#ota_info, RBID-lower-initiates §4.3) → firmware >1KB escalates #wifi_req→
#wifi_offer{ssid,psk,ip,port,ttl}→ RECEIVER brings up a TRANSIENT ad-hoc SoftAP (R2-WIFI §3.3, 120s TTL) → push
signed image TCP :21043 → #wifi_done teardown. Small <1KB on L2CAP CoC 0x00D2; 0x00D3 OTA reserved/fallback.
- **OTA MODEL PIVOTED → single-canonical BLE-L2CAP (ADR-BLE-006), NOT WiFi-STA/transient-SoftAP.** The bench
  proved ESP↔ESP L2CAP works → OTA rides the 0x00D3 CoC: reuse `ota_recv_signed` (CMD_START_SIGNED, verify-before
  -write, 4-gate, Ed25519, R2-UPDATE v0.6) OVER the CoC. Signed core reusable; adapt TCP→CoC [len BE] §3.1.2.3.
- **★ ROY: OTA = PLUGIN + SENTANT in the BASIC ENSEMBLE (boundary rule: everything is plugin+sentant EXCEPT the
  core network stack). RUNTIME GAP CONFIRMED [#19/#21]:** the firmware is a MONOLITHIC EMBASSY APP — has core's
  network stack (RouteEngine/r2_route + r2_dataplane + r2_wire + r2_trust + r2_transport + r2_discovery) but NO
  sentant/plugin runtime (no r2_engine EventBus / sentant host / plugin registration / basic ensemble). OTA today
  = a standalone embassy task. **FORK posed to supervisor:** (a) PURE = build on-device sentant/plugin runtime +
  basic ensemble FIRST (large, core-gated — asked core if r2_engine is no_std-capable [#21]), then OTA plugin+
  sentant; (b) INTERIM (my rec) = BLE-L2CAP OTA receiver NOW as an embassy task (ota_recv_signed over 0x00D3 +
  #ota_* + composer's push_ota_l2cap) = 'OTA from now on' fast, refactor to plugin+sentant later. Complex work
  identical; only the control wrapper differs.
- **★ FINDING (overnight) + INCREMENT 1 DONE (34fd380): NO RE-VENDOR needed for the runtime.** core confirmed
  r2-engine is no_std-ready; VERIFIED on-device: r2-engine is workspace-local + no_std+alloc at the CURRENT base
  (c46383e) — added it as an optional firmware dep (feature `otaengine`) + a minimal EventBus embassy task
  (EventBus::new + tick + poll_plugins + drain_outbound), LINKS GREEN on xtensa (default unaffected). So the
  on-device sentant/plugin runtime needs NO re-vendor → the PURE plugin+sentant OTA is buildable at the current
  base (resolves the interim-vs-pure fork toward PURE). The re-vendor is ONLY for the §2.6 ESP-NOW bearer (#20) +
  #9/#12/#13 — SEPARATE from OTA. **INCREMENTS 2-4 (next):** (2) OTA PLUGIN — Plugin::execute(write-chunk/verify/
  activate), reuse ota_recv_signed's verify-before-write/4-gate/Ed25519; (3) OTA SENTANT — thin #ota_* control on
  the bus; (4) BRIDGE — network deliver_out→Event→sentant, drain_outbound→egress, + the L2CAP-0x00D3 CoC → the
  OTA plugin's chunk input. e2e w/ composer's push_ota_l2cap = metal (Roy AM). PARKED for Roy AM: flashing/e2e +
  the re-vendor (separate). EventBus API (base): register_sentant/register_plugin(Box<dyn>), tick, poll_plugins,
  drain_outbound→Vec<QueuedEvent>; Plugin::execute(cmd,&[u8])->PluginResult + poll; Sentant::handle_event(&Event,
  &mut ActionBuf). Ref: crates/r2-engine/src/conformance.rs.
- **★ OTA RECEIVER BUILT (supervisor decision (b)) — increments 2a+2b DONE, e2e image staged, NEEDS-REFUTATION.**
  2a `8fb0010` `ota_receive_over_coc` (feature `otal2cap`) = the clean reusable CAPABILITY: verify-before-write /
  4-gate / Ed25519 reused VERBATIM from ota_receiver; transport→0x00D3 CoC; R4→implicit CoC-peer-binding; FUNCTION
  form (not a Plugin struct) → no OtaUpdater-lifetime issue. 2b `b5e7abb` = thin embassy harness (device advertises
  opaque-addr + accepts 0x00D3 → ota_receive_over_coc; clean entry → #ota_* sentant later, zero complex-work
  change). xtensa-GREEN: default+otal2cap+cocbench+full field,loraroute,multitg,staota,otal2cap. STAGED e2e:
  ~/r2-staota-artifacts/r2-dfr1195-DFR-otal2cap-e2e.elf (conformance §7 + OTA). E2E (Roy AM): flash → PROVISION
  (verify_header needs tg_pk) → composer push_ota_l2cap (signed, matching TG key) per-SDU OST/ODT/OCM over 0x00D3 →
  verify-before-write→activate→reboot→confirmed-boot commit. ⚠ **NEEDS-REFUTATION** (opposite-provider review of
  ota_receive_over_coc + metal e2e) before production/done. R4-binding PRE-REFUTATION (supervisor's concern: 2nd-CoC
  hijack?): structurally PREVENTED — HostResources<_,1,1> = max 1 BLE connection (2nd central can't establish) +
  the provider loop is serial (accept→ota_receive_over_coc-blocks-till-close→loop) → never a concurrent CoC; the
  single peer = the session. Refute fires after core's immune-monitor review (~midday). e2e MTU≈245 (pool251−6,
  default cfg) → composer ODT chunk ≤200 v1; signing = composer provisions the receiver TG + signs with that key.
  (a)-refactor = engine-host it (increment 1 #34fd380
  proved r2-engine on-device, no re-vendor). WIRE NOTE (specs 27b7dec): #wifi_offer→#wifi_ack (0x98465EE1, schema
  {0:ip,1:port,2:already_connected}) — NO firmware impact (the L2CAP-direct receiver has no #wifi_* frames); applies
  only to the FUTURE #ota_*/#wifi_* SoftAP-escalation layer (if built). GOTCHA: event-name hashing is NOT bare FNV
  (FNV('wifi_ack')=0xF78B4D12 ≠ 0x98465EE1) → use the canonical r2_engine/r2_wire event-hash helper + specs' values.
  ── superseded scoping (the Plugin-struct port; supervisor chose the
  cleaner module form above): impl Plugin for OtaPlugin: execute(cmd,data) dispatch — START(cmd, data=
  123B header+64B sig) → build DeviceContext (read_persona tg_pk + read_anti_rollback seq/floor) → r2_update::
  verify_header → PayloadVerifier::new; CHUNK(data=off+payload) → pv.update THEN sector-buffered write to the
  inactive slot; COMMIT → pv.finish (BEFORE activate) → OtaUpdater activate + write_anti_rollback (monotonic);
  ABORT/timeout → reset. Reuse r2_update crypto (verify_header/PayloadVerifier) — DO NOT rewrite. CHALLENGES: (a)
  OtaUpdater borrows &mut flash + &mut tbl — the plugin must OWN flash+tbl+the in-flight region/pv/secbuf(4KB)/
  written/payload_size/session-owner across execute() calls (the streaming locals → struct fields); (b) keep R3
  (every chunk within declared total; commit only when written==total) + R4 (session bound to one owner) gates;
  (c) verify-before-write invariant (no byte boots until finish() Ok). **DOCTRINE: peer-refute before 'done'** —
  this is security-critical (Ed25519 verify, anti-rollback, slot activate); NOT rushing it at the tail of the
  overnight marathon without a refutation pass. RECOMMEND a focused/peer-refuted build (flagged to supervisor).
  Then INCREMENT 3 = OTA SENTANT (thin #ota_* control → PluginCall) + 4 = network/bus bridge + L2CAP-0x00D3 feed.
- **★ THROUGHPUT BENCH [task #18] — v1 RAN: 11 KB/s; TUNED build staged (faf7213), awaiting re-run.**
  Roy ran the corrected bench (D1=RECEIVER/D3=PUSHER, read off LCD): **11 KB/s** default config. ROOT CAUSE
  (verified): trouble_host DEFAULT 80ms conn interval (connection.rs:208) — interval-starved, not a deeper bug.
  TUNED build (faf7213): interval 80ms→7.5ms (~10x), 2M PHY (set_phy Le2M), DLE 251 (update_data_length), L2CAP
  credits 32 + eager-return. Staged ~/r2-staota-artifacts/r2-dfr1195-cocbench-tuned-{RECEIVER,PUSHER}.elf; flash
  D1=RECEIVER/D3=PUSHER; read 'COCBENCH N KB/s' off LCD. EXPECT 100s of KB/s if interval-dominated (my conjecture);
  <30 → deeper cap (pool/credit or stop-and-wait push needs pipelining). The OTA-carrier (single-canonical L2CAP)
  call HINGES on the tuned number. Don't rewrite §3.1.3 until it lands (C/R). NOTE (data plane = ESP-NOW; L2CAP is
  the OTA/control carrier — this informs OTA speed only). **CI: firmware is xtensa no_std = NOT hosted-CI-covered;
  verified LOCAL-xtensa-green all combos. r2-hive platform-trait not CI-triggered; old main failures pre-date me.**
  --- earlier (superseded): BUILD CORRECTED + STAGED (24a35f8) ---
  First cocbench (0efe84c) couldn't run (un-gate→both boards drain/none push; opaque broke connect). FIX: manual
  role flag `cocbench_provider`=RECEIVER (advertise@BENCH_ADDR+drain) vs plain `cocbench`=PUSHER (connect@BENCH_ADDR
  +push); fixed BENCH_ADDR (no provisioning); LCD L1 shows 'COCBENCH N KB/s' (read off-screen, no console-reset).
  STAGED: ~/r2-staota-artifacts/r2-dfr1195-cocbench-{RECEIVER,PUSHER}.elf (distinct). Sent supervisor per-board
  espflash (D1=RECEIVER F4:12:FA:50:26:98 / D3=PUSHER F4:12:FA:B6:0A:A0). Conformance bundle UNAFFECTED (joiner
  path unreachable there under the un-gate). PENDING: Roy flashes both → metal KB/s → I analyze vs §3.1.3 (C/R).
  ~~`cocbench` feature~~ (superseded by the corrected build above):
  (xtensa-green: minimal `cocbench` + `staota,cocbench`): reuses the ble connect plumbing, cfg-swaps served fn
  (serve_coc→coc_bench_*) + PSM (0x00D2→0x00D3). provider(M7_PROVIDER_HIVE)=coc_bench_drain RECEIVER; joiner=
  coc_bench_push PUSHER (1.3MB / 240B chunks / Instant→KB/s). v1=default L2capChannelConfig. RUN (procedure sent
  supervisor+composer): two S3, ONE = M7_PROVIDER_HIVE, flash `--features cocbench`, BLE-connect→push→console
  'COCBENCH … = N KB/s' (console-open resets once→reboots→reruns→prints). composer holds ttys → metal run pending.
  Sweep 2M PHY/DLE/conn-interval/MTU-MPS/credits + BLE-only-vs-coex arm = follow-up. Gates the Roy data-plane call
  (L2CAP-bulk vs SoftAP vs ESP-NOW) → bench BEFORE the OTA wrapper. Don't rewrite §3.1.3 until the number lands
  (C/R). My read (BlueZ-confound=Linux host not BLE physics; ESP-NOW better general data plane) UNPROVEN until the
  metal number. v2 idea: render KB/s on the LCD (no console-reset to read). **SUPERVISOR
DECISION: HOLD .1659, DON'T stage —
deliver the FULL bundle via OTA, not a piecemeal desk session.** Rationale: OTA not ready (composer design-only)
→ shipping 6 now = a desk session + the 2 fast-follow = a 2nd session = more desk work for no urgency (bench
works fine on .1659; observer tolerates both company-id forms; download landmine not triggered). composer is
building OTA path + RBID-resolver + NVS-label so the FULL 8 ships via OTA. Fallback = ONE desk session for the
full 8 ONLY if OTA can't be readied. **DO NOT stage an artifact until supervisor gives the OTA-or-desk word.**
composer HAS both inputs (RBID algo + NVS-label proposal) → not blocked on me. When composer's resolver matches +
label offset acked → I implement the firmware halves (opaque random-NVS address + L2 label read) → 8/8 via OTA.
Re-vendor onto 0d1f308 = SEPARATE pass AFTER this rev validated. composer's Q1 console-open re-test gates whether
the PAC register-disable joins this rev or a later one.

**BEACON CONFORMANCE-HARDENING (post-validation, multi-item — composer on-air decode + specs v0.15/R2-BLE v0.12):**
D3's .1659 beacon had 3 AD issues, all now understood:
- class_hash value+endianness — B5 (6323f29) fixes the STRUCTURE (role-class, big-endian). BUT specs v0.15 says the
  class STRING must be reverse-DNS `ai.reality2.device.*`, so my `r2.*` strings are WRONG → asked specs for the
  authoritative set+vectors → will recommit role_class_hash (fixes BOTH §7 BLE + §8.1 LoRa). [task #15, blocked specs]
- Company-ID 0x01B2 (magic 0xB2 + ver 0x01 packed in the company-id slot) omits §7.3's 0xFFFF — observers key on
  0xFFFF → LIKELY the ORIGINAL beacon-'regression' root cause. Fix = prepend 0xFFFF. HELD pending Roy's a/b (specs).
  composer tolerates both forms meanwhile. [task #17, held Roy]
- BLE address opacity — specs v0.15 §7.4.0 inv.4: address MUST be identity-independent. ble_task builds it from
  my_hive (low 4 = wire_id) → leaks stable id, defeats RBID rotation. Fix = random opaque address. SEQUENCED with
  composer (their bench reads wire_id from the address → they add RBID-resolution first). [task #16, seq composer]
**NEXT (the remaining big item): re-vendor onto core consolidation tip (0d1f308)** — #9 arrival_transport
=Some(rx_via), #12 telemetry consume (neighbour_score/neighbour_fade_remaining), #13 §2.3A beacon_emit_transports
mask-gating, B2 non-connectable beacon. (B5 class_hash = DONE standalone, 6323f29.) Do this as a SEPARATE focused
pass AFTER the 3-fix batch is desk-validated — do NOT bump the core base on top of un-validated changes; the
re-vendor changes the validated artifact base + needs core's consolidation tip confirmed.

## ⚠️ 2026-06-30 — INCIDENT (RESOLVED): .1408 BOOT-FAILED on D5 (INERT path) — FIX = staota.0630.1659 (VALIDATED above)
**FIX SHIPPED (`dc78b90`, staota.0630.1659, SUPERSEDES .1408):** reverted the in-INERT console-receiver to the
proven liveness-only INERT (removes the early `UsbSerialJtag::new` — the boot bug). Kept the un-gated §7 beacon,
A4/B3, and reboot-to-download (command-only, now reachable only via uart_rx_task = post-init = safe). A FRESH
board's INERT path is now IDENTICAL to the pre-.1408 staota that DID boot on D5 → high confidence. Awaiting Roy's
desk re-test (load .1659 → INERT-liveness → download-mode-provision 89e83d99 → provisioned → beacon → scan).
DEFERRED: in-INERT REMOTE provisioning (console-store on a fresh board) — re-add AFTER esp_rtos::start (post-init
context) + desk-validate. Fresh boards provision via download-mode meanwhile.

### 2026-06-30 — D3/.1659 METAL READ (supervisor): blank-INERT is EXPECTED, NOT a fault + NEW DEFECT found
Supervisor flashed D3 (B6:0A:A0) with .1659 --flash-only (unprovisioned): enumerates on USB (CPU stage running),
but BLANK LCD + NO LED + console SILENT (0 bytes/35s incl. RST taps), and — crucially — STABLE on USB / NOT
boot-looping (unlike .1408). My read (verified vs source + the artifact's compiled `field` strings):
- **BLANK LCD + NO LED = EXPECTED for field-INERT, not a red flag.** INERT halts at main.rs:187-223; LCD init
  (read_board_profile L234) + LED config (LEDC/GPIO21 L319) both run AFTER it → an unprovisioned field board never
  reaches them.
- **NEW DEFECT (root of the silence, structurally confirmed):** the INERT loop awaits `Timer::after(6s)` at L221,
  but `esp_rtos::start()` (registers the embassy time driver) is at L307 — AFTER the loop. So a field-INERT board
  prints ONE boot burst (ota_slot_report + §3.5 UNPROVISIONED-FAIL-CLOSED + first INERT beat) then DEADLOCKS on a
  timer that never fires. Liveness is a single boot burst, NOT a repeating 6s stream → composer greys it after 12s.
  This MATCHES D3's signature (stable USB, not looping, silent+dark = parked in the deadlock). Does NOT match a
  boot failure. So .1659 is very likely booting D3 correctly into INERT.
- **DECISIVE TEST (supervisor running):** provision D3 --in-download. Expect LCD+beacon → .1659 good. If still dark
  → deeper bug, escalate.
- **FIX (converges with the deferred in-INERT-receiver re-add): move `esp_rtos::start` ABOVE the INERT block** so
  embassy_time is driven inside INERT (repeating liveness works) AND the post-init context lets the in-INERT
  console-receiver be safely re-added. Single reorder fixes both. Pending .1659 confirmation + desk-validate.
**DO NOT flash .1408 to a FRESH/unprovisioned board (use .1659).** Metal result (supervisor): D5 (the only board imaged with
.1408) boot-loops/goes silent — drops USB-JTAG + stays absent, 0 passive console bytes, no BLE beacon. The other 9
(older firmware) are stably present (clean differential = firmware regression in .1408).
- **ROOT CAUSE (high confidence, structural — NOT yet metal-confirmed):** the firmware is `#[esp_rtos::main]` and
  inits esp-rtos/embassy + esp-radio at main.rs:331 — AFTER the §3.5 INERT block (187-245). My console-receiver
  constructs `UsbSerialJtag::new(p.USB_DEVICE)` at main.rs:200 = the PRE-esp-rtos/esp-radio-init window. The PROVEN
  non-inert usb_rx (line 489) is built AFTER that init; the ORIGINAL Timer-only INERT (which D5 ran) never built a
  UsbSerialJtag there. So grabbing/re-initing the USB-JTAG too early disrupts esp-println's USB-JTAG → 0-bytes /
  USB-drop symptom. **INERT-PATH-ONLY:** a PROVISIONED board skips line 187, so the post-331 provisioned path
  (un-gated beacon + reboot-to-download, both NON-boot-path) is unaffected.
- **WORKAROUND for the beacon test (NO REBUILD):** download-mode-provision D5 with the EXISTING .1408 (esptool
  write persona@0x12000 in the same BOOT session) → boots PROVISIONED → skips INERT → ble_task → beacon → scan.
  Confirms root-cause-INERT-only AND validates the beacon. Sent to supervisor.
- **FIX (track 2, restores remote/console provisioning — pending, CANNOT metal-test myself):** reorder so the
  console-receiver runs AFTER esp-rtos/embassy init but BEFORE radio bring-up (keeps fail-closed), OR build usb_rx
  once post-init + share it INERT↔uart_rx_task. Requested a BOOT-LOG from D5 (does the banner print before going
  silent? pinpoints UsbSerialJtag::new vs elsewhere) to confirm before shipping. Needs desk-validation.
- **SAFE FALLBACK build available on request:** revert the INERT console-receiver to the proven liveness-only loop
  + keep un-gated beacon + drop reboot-to-download = guaranteed-booting beacon image (download-mode-provision for
  the test). Not built yet (workaround covers the beacon test); ship if supervisor wants a clean baseline.
- LESSON: .1200/.1404/.1408 were xtensa-BUILD-green but NEVER metal-booted before D5 — the INERT path (esp-rtos
  ordering) only fails at runtime. The de-risk gap: build-green ≠ boot-green for early-init peripheral grabs.

## ► 2026-06-30 — REBOOT-TO-DOWNLOAD (field re-flash recovery) DONE+GREEN — NEW REV staota.0630.1408 (D5's desk image)
Supervisor bumped this to FIELD-CRITICAL (Roy: no BOOT button in the field; D5's stuck flash proves it). ROOT
CAUSE: the running app — incl. the §3.5 INERT/console-liveness loop — HOLDS the USB-Serial-JTAG, so a host
download-reset can't get through → remote re-flash futile without a BOOT press.
- **Firmware `7f079bd` (dfr1195-fw):** new console command `DOWNLOAD` (alias `REBOOT-DOWNLOAD`), handled in BOTH
  uart_rx_task AND the §3.5 INERT loop (D5 is stuck in INERT — must work there). `reboot_to_download()` sets
  `esp_hal::peripherals::LPWR::regs().option1().force_download_boot()` (RTC-domain, survives reset) +
  `software_reset()` → ROM enters download mode, taking over the USB-JTAG the app held → remote espflash re-flash.
  Form (ii) per supervisor: deterministic (board self-enters download; the reset stops the app = solves the hold
  root cause), NOT (i) release-only. Build-verify GREEN: field,loraroute,multitg,staota / staota / nobt.
  **Self-review fix `f8425ee` (in .1408, not .1404):** the uart_rx_task `is_persona` dispatch guard matched
  REBOOT but not plain DOWNLOAD → reboot-to-download via the `DOWNLOAD` token worked in the INERT loop (calls
  handle_persona_cmd unconditionally) but was IGNORED on a running/provisioned board (only `REBOOT-DOWNLOAD`
  worked there). Added DOWNLOAD to the guard → both tokens work in both contexts (matters for field re-flash of
  PROVISIONED boards). supervisor confirmed form (ii) + the sequencing where the one desk flash both bootstraps
  D5 and validates reboot-to-download in a single visit.
- **NEW ARTIFACTS staota.0630.1408 (REPLACE .1200)** at `/home/roycdavies/r2-staota-artifacts/` (DFR + XIAO,
  creds baked, ~1330792B). Content = console-receiver + un-gated beacon + reboot-to-download. THIS is D5's
  desk-flash image → the desk BOOT-press becomes the LAST physical touch (future re-flash/OTA-recovery = send
  `DOWNLOAD` remotely). Beacon test UNAFFECTED (reboot-to-download dormant unless commanded). Per-carrier flash
  cmd unchanged (point at the .1408 elf).
- **DESK-VALIDATE before field reliance (HONEST caveat — the one path I can't metal-test: espflash gate + remote
  boards):** at D5's desk visit, after flashing .1408, send `DOWNLOAD` and confirm espflash reaches the
  sw-triggered download mode OVER USB-JTAG; BOOT-button fallback if S3 force_download_boot lands UART-only.
- Reported supervisor + composer (console-provision.py UNCHANGED — DOWNLOAD is separate from REBOOT). Beacon-
  hardening (B2/B5/§2.3A) + re-vendor (#9/#12) still POST-beacon-test.

## ► 2026-06-30 — r2-hive DEPLOYABLE NODE was BROKEN vs r2-core-consolidation — FIXED + GREEN (task #4 closed)
Verify-don't-assume paid off: actually built+tested the deployable node (not assumed) and found it did NOT
compile against the current local r2-core (branch `r2-core-consolidation` @ 5450cdc — which r2-hive's path-deps
build against; NOTE origin/main does NOT yet have this change). Core's §2.3B work (`bf1bf3b`) added a REQUIRED
field `arrival_transport: Option<Transport>` to `ForwardRequest`, silently breaking BOTH downstream constructors:
- `crates/r2-hive-core/src/sync_host.rs:198` (host sync-tier forward)
- `crates/r2-hive-bin/src/router.rs:254` (host router forward)
- **FIX (`dcb1f10`):** both set `arrival_transport: None` = BEHAVIOUR-PRESERVING (engine.rs:492 skips the §2.3B
  arrival-reachability drop when None). NOT a silent faked-distance enablement on the host tier. sync_host has the
  arrival `transport` in scope, so the host COULD enable §2.3B by passing Some(transport) — left None as a
  deliberate decision FLAGGED FOR CORE (asked: should the host sync/router tier enforce §2.3B, or is faked-distance
  mesh/firmware-only?).
- **NOW GREEN:** `cargo test --workspace` (stable toolchain, default features) = ~200 tests pass, 0 failed, incl.
  the relay-handshake v0.2 challenge-response conformance. ALSO verified the `ble,lora` radio-deployment variant
  builds clean (EXIT=0; host libdbus present; 1 benign pre-existing unreachable-log warning in the LoRa rx loop,
  not my change) — so NO further consolidation API-drift hides behind the radio features.
- **SAME break as the firmware** re-vendor onto 0d1f308 (identical None fix queued there). Reported to supervisor
  (task #4 closed) + FYI'd core (a required-field addition breaks all downstream ForwardRequest constructors;
  suggested #[non_exhaustive]+Default for future additive-non-breaking changes; flagged the consolidation→main
  merge will need this fix). DO-NOT-ASSUME: r2-hive currently builds ONLY against the consolidation branch (which
  has arrival_transport); it would FAIL against origin/main (no such field) until consolidation merges.

## ► 2026-06-30 — ADVERSARIAL REFUTATION of the receiver-staota work (peer-refuted; 2 fixed, 2 batched, 1 escalated)
Closed the doctrine's "peer-refuted before done" gap on 30e0ff5 (console-receiver) + aa9088f (beacon un-gate):
ran an INDEPENDENT read-only adversarial reviewer (fresh agent, tasked to BREAK them; not opposite-provider — a
codex-twin pass would be stronger, noted). 5 findings, all triaged vs ground truth. The in-flight beacon-test
artifacts (aa9088f) are UNTOUCHED — fixes are committed but not rebuilt; they bundle into the next staota.
- **FIXED NOW (`6df9d0c`, build-green field,loraroute,multitg,staota / staota / nobt; no beacon-payload change):**
  - **A4** (chunk robustness): handle_persona_cmd now requires PERSONA BEGIN before any chunk/END (begin_seen flag
    threaded through both call sites) + RESETS the accumulator after END → no stale-accum re-parse / cross-record
    append. New ACK `PERSONA ERR no-begin` (composer suffix-matches `PERSONA ERR`).
  - **B3** (vacuous guard): the `debug_assert` was a release no-op (shipped artifact) AND tautological (adv[4]
    just assigned 0xFF). Replaced with a release-EFFECTIVE runtime log-guard (`BEACON-GUARD FAIL` if plen==0).
- **ESCALATED — spec-first (asked specs, awaiting inbox):**
  - **B5 (medium, spec+privacy) — RULED by specs (authoritative; firmware fix, NOT a spec change). TWO bugs:**
    BLE §7 beacon at main.rs:2651 does `class_hash: my_tg_hash.to_le_bytes()` — WRONG on both axes:
      (1) VALUE: `class_hash` MUST = the DEVICE-CLASS hash `role_class_hash(profile.role)` (FNV-1a-32 of the class
          string, §4/§7.3/§7.4) — same value the LoRa §8.1 + mDNS §8.4 profiles carry. `tg_hash` mis-populates the
          field AND violates R2-BEACON Design Principle #1 ("signpost, not passport: NO trust-group identity in the
          advert"). A clear-text rotation-invariant TG hash is a GROUP correlator (re-links all TG members across
          every rbid epoch — §6.1/§8.1.2/§6A.2 below-membrane leak); my un-gate amplifies it all-boards-always-on.
      (2) ENDIANNESS: `to_le_bytes` is a SECOND independent bug — §7.4.1 mandates BIG-ENDIAN (`uint32_be`); even
          after the value fix, LE byte-reverses the field and fails cross-impl decode + the §9 vectors.
    FIX (post-test pass): thread the class_hash into ble_task like LoRa (compute `role_class_hash(profile.role)` in
    main, pass in), emit BIG-ENDIAN. specs landed R2-BEACON v0.12 §7.4.0 "Field privacy invariants (BLE)" (commit
    72a2c69, hosted-CI verify pending per specs' honesty caveat) hardening this bug-class. Relayed ruling to
    supervisor (specs' own relay hit the self-msg channel bug).
- **BATCHED — POST-TEST BEACON-HARDENING PASS (beacon-payload/behaviour changes; don't rebuild mid-test):**
  - **B2 (medium):** every board now advertises `ConnectableScannableUndirected` → a central can connect-and-hold
    to SUPPRESS a board's beacon (DoS) + force serve_coc. Fix: advertise the pure beacon NON-connectable for the
    un-gated (non-blemesh) path; keep connectable only where the CoC is actually used (blemesh). Also add a backoff
    to the `accept()` Err arm (currently a tight re-advertise spin, unlike the advertise() arm's 1s sleep).
  - Do B5 + B2 together with the §2.3A per-available-transport beacon mask-gating (all beacon-emit changes).
    §2.3A MASK-GATING API LANDED (core 50d73fa, CI-green): `engine.beacon_emit_transports(present: TransportSet)
    -> TransportSet` (also on DataPlane) = present ∩ effective §2.3A mask (baseline ∩ lease) = the canonical
    transports I MUST beacon on. BINDING: pass the board's PHYSICALLY-PRESENT transport set; map each returned
    Transport to its profile (BLE→§7, LoRa→§8.1, IP→§8.4 mDNS); a masked/absent transport → no beacon there (flip
    the mask → beacon stops, by construction). Replaces the current "advertise whenever `ble` is up" with
    mask-driven emit. NOTE: §2.3B beacon-RX INGRESS gate (#13's other half — drop beacons from a faked-unreachable
    peer) is NOT in 50d73fa; still spec-blocked on 2 pins (stable-link-address keying R2-TRANSPORT v0.7 + RBID §6
    canonization). So #13 EMIT = ready (un-gate done + this mask-gating API); #13 RX-gate = spec-blocked.
- **ACCEPTED-RISK / FOLLOW-UP (recorded, no immediate change):**
  - **A2 (medium):** the persona receiver is parse-only (r2_trust::parse_persona does CBOR-decode + derive, NO
    signature verify — cert key-4 parsed then ignored, persona.rs:33 "may be ignored v0.1"; firmware admits
    cert-sig verify is a follow-up, main.rs:168) AND is wired into uart_rx_task (RUNNING boards), so momentary USB
    access to a deployed node can re-home its identity unauthenticated. This is the INTENDED v0.1 model (console =
    local-trust management plane) AND composer REQUIRES the running-board path (re-provision deployed boards). The
    real gap = the documented cert-sig verify follow-up; until then console==full-trust. FYI'd composer.
    **RESOLVED by composer's decision (2026-06-30):** console==full-trust CONSCIOUSLY ACCEPTED for the bench
    (console is LOCAL to Alfred, same local-trust as prov2.py's group-key, never over-air). Do NOT gate INERT-only
    — the running-board re-provision path is a WANTED FEATURE (re-home deployed boards). The required hardening =
    the cert-validation follow-up (parse_persona must verify cert key-4 vs tg_pk) — CORE-OWNED (r2-trust). FLEET
    FLAG: cert-validation MUST land before console-store is relied on in ANY untrusted-physical-access (field)
    setting (momentary USB = re-home = the risk). Bench (Alfred-local) proceeds as-is. FYI'd core (owns the fix).
    No firmware change needed from hive.
- **ATTACKED, NO DEFECT (verified):** A1 (no write-anywhere — offset is always a compile-time constant, never
  console-derived), A3 (all buffers bounds-checked before indexing — no OOB/panic), A5 (fail-closed intact — no
  radio/mesh before a validated persona), A6 (no p.USB_DEVICE double-take — diverging branch), B1 (advert built
  unconditionally), B4 (blemesh preserved).

## ► 2026-06-30 — RECEIVER-STAOTA DELIVERED: console-persona-receiver (#14) + un-gated §7 BLE beacon (#13) — DONE+GREEN, ARTIFACTS STAGED
Supervisor+composer GO (the gating deliverable for Roy's BLE-beacon test). Both features built, xtensa-green,
committed, pushed on `dfr1195-fw`; both staota artifacts rebuilt with creds and staged on Alfred. ONE bootstrap
full-flash per board now delivers BOTH the beacon (to test) AND remote-provisioning-forever (no more download mode).
- **Firmware HEAD (`dfr1195-fw`, base r2-core c46383e):**
  - `30e0ff5` console-persona-receiver (#14) — `handle_persona_cmd` (PERSONA BEGIN / PERSONA <128hex>×N / PERSONA END
    → parse_persona-validate → store@0x12000 → read-back → ACK `PERSONA OK <hive>`; RPF1 <96hex>→@0x17000;
    BOARDPROF <4hex>→@0x13000; REBOOT→software_reset). WHITELISTED offsets, each VALIDATED. Wired into BOTH
    `uart_rx_task` (running boards) AND the §3.5 INERT loop (fresh boards — usb_rx constructed in the diverging
    branch, no double-take). uart_rx_task line buffer 128→160B. Fail-closed preserved (local USB only, validate-
    before-write). Framing locked with composer's console-provision.py 311866c.
  - `aa9088f` un-gated §7 BLE beacon (#13 emit) — EVERY board advertises encode_advert (was am_provider==
    M7_PROVIDER_HIVE only). `advertise_beacon=true` for all non-blemesh ble builds; blemesh keeps the data-CoC
    provider/joiner split. REGRESSION-GUARD: debug_assert the advert is a built R2-BEACON AD + every board logs
    `BEACON adv up (§7, hive .. rbid ..)`. BINDS core's r2_discovery::beacon (no reimplement).
- **Build-verify GREEN (xtensa):** #14 across field,loraroute,multitg,staota / field,loraroute,multitg / staota /
  nobt. #13 across field,loraroute,multitg,staota / xiao,field,loraroute,loratcxo,multitg,staota / blemesh / staota.
- **ARTIFACTS (staged, BUILD_ID `staota.0630.1200`, creds baked from ~/.config/r2-composer/wifi.env):**
  `/home/roycdavies/r2-staota-artifacts/r2-dfr1195-DFR-staota.elf` + `…-XIAO-staota.elf` (~1330KB each, NOW-stamped;
  NOTE: `cp` is aliased `-i` — staged with `\cp -f`). Partition table: `docs/dfr1195-partitions.csv`.
- **PER-CARRIER FULL-FLASH CMD (supervisor/Roy runs it — espflash gate blocks hive+composer; VERIFY board identity
  from boot banner / by-id MAC FIRST). Chained no-reset so the old app never boots mid-sequence (no write-bin hang);
  erase clears STALE config (persona/runtime-TG@0x14000) → clean console-provision:**
  `espflash erase-region --port $PORT --before default-reset --after no-reset 0x12000 0xE000`
  `espflash flash --port $PORT --before no-reset --after hard-reset --partition-table docs/dfr1195-partitions.csv <DFR|XIAO .elf>`
- **CRITICAL TEST ORDERING (verified: main.rs:187 INERT halt diverges, ble_task spawns at :505):** an UNPROVISIONED
  board boots INERT and does NOT spawn ble_task → does NOT advertise the beacon (no identity to beacon = correct R2).
  Sequence: erase+flash → INERT (receiver live) → composer console-provision.py installs persona → REBOOT →
  provisioned → ble_task → `BEACON adv up` → BLE-scan sees it. So flash → provision → THEN scan.
- **Follow-ons (NOT in this deliverable):** #13 §2.3B-on-beacon RX-gate (link-address keyed, R2-BEACON v0.9 — needs
  core's beacon-ingress hook); reboot-to-download (secondary); #9 faked-distance re-vendor; #12 RouteEngine real-
  weights telemetry. DO-NOT-ASSUME: the §2.3A per-available-transport mask-gating of the beacon EMIT still layers in
  with transport_allow_mask (right now the beacon advertises whenever `ble` is up, not yet mask-gated).

## ► 2026-06-30 — RE-VENDOR onto 0d1f308 DE-RISKED (trial worktree, isolated — dfr1195-fw + staged artifacts UNTOUCHED)
Autonomous de-risk of the post-staota core-dependent work block (#9 faked-distance + #12 telemetry + #13 RX-gate).
Done in a THROWAWAY worktree so the in-flight beacon flash (dfr1195-fw @ aa9088f, c46383e-based artifacts) is not
disturbed. Result: the re-vendor is a KNOWN, PROVEN-CLEAN one-shot — no ambiguity, no surprises left.
- **TARGET UNAMBIGUOUS = `0d1f308`** (tip of `origin/r2-core-consolidation`). Verified ancestry: `bf1bf3b` (#9
  §2.3A boot-baseline + §2.3B virtual-reachability), `41a3a3f`, AND `c46383e` (current firmware base) are ALL
  ancestors of 0d1f308; and 0d1f308 holds both the #12 accessors and the faked-distance hooks. So ONE re-vendor
  enables #9 + #12 + (check) #13 together. RESOLVES the old #9 "re-vendor onto 41a3a3f vs 0d1f308" ambiguity →
  use 0d1f308 (it subsumes 41a3a3f). UPDATE: the re-vendor target is the consolidation TIP, which ADVANCES as
  core lands more — now ≥`50d73fa` (beacon_emit_transports §2.3A API) on top of 0d1f308 (telemetry accessors) on
  top of bf1bf3b (arrival_transport). At re-vendor time target the CURRENT tip + RE-CONFIRM the clean rebase (the
  trial proved 0d1f308 clean; re-verify the newer tip since core keeps landing commits).
- **REBASE PROVEN CLEAN:** `git rebase --onto 0d1f308 c46383e` over the firmware branch = 22 commits replayed,
  ZERO conflicts.
- **ONE BUILD FIXUP (caught now, not as a post-test surprise):** 0d1f308's `ForwardRequest` gained
  `arrival_transport: Option<Transport>` (core bf1bf3b §2.3B). Firmware construction at `main.rs:~1551` must add
  `arrival_transport: None` = BEHAVIOUR-PRESERVING (engine.rs:492 `if let Some(arrival)` → None skips the §2.3B
  drop; the re-vendor itself must NOT change runtime behaviour). With that line = **build GREEN**
  (field,loraroute,multitg,staota, 19 warnings = same as current). RECIPE: rebase --onto 0d1f308 + that one line.
- **#12 accessor signatures CONFIRMED in 0d1f308** (match core's message byte-for-byte): `neighbour_score(&self,
  hive_id:u32, transport:Transport)->Option<f32>` (engine.rs:361), `neighbour_fade_remaining(&self,
  hive_id:u32)->Option<f32>` (engine.rs:379, NO `now` arg), + 3 guard tests (tests.rs:800/821/837).
- **SEQUENCING (do NOT re-vendor yet):** re-vendor changes the artifact base → keep dfr1195-fw stable at aa9088f
  until the beacon flash/test CONFIRMS the c46383e-based artifacts on metal. THEN: re-vendor (recipe above) →
  enable #9 (set `arrival_transport: Some(rx_via)` from the got.3 RX carrier + reachability-lease surface +
  two-gate ingress incl. neighbour-refresh ingest-gate + boot-baseline + CAP=32) → #12 (consume neighbour_score
  at the placeholder `w=1.0` main.rs:~1401 + extend the NBR-TBL emit main.rs:~1114) → #13 §2.3B-on-beacon RX-gate.
- **Trial worktree removed after recording; nothing committed to a real branch.** DO-NOT-ASSUME: line numbers
  (1551/1401/1114) are pre-re-vendor; re-confirm after the rebase replays.
- **REBOOT-TO-DOWNLOAD (follow-on) — feasibility researched, deliberately NOT implemented (well-justified defer):**
  MECHANISM (esp-hal 1.1.1, no high-level API): raw PAC write `RTC_CNTL.option1().modify(|_,w|
  w.force_download_boot().set_bit())` ("force chip entry download boot by sw") then `esp_hal::system::
  software_reset()`. WHY DEFERRED (not laziness): (1) UNVERIFIABLE by me — espflash/download gate blocks hive, and
  the boards are ~30km remote; (2) HIGH RISK if wrong — a board sent to a download mode that espflash-over-SSH
  CANNOT reach (the original contention problem that birthed console-provision) is STRANDED with no app running =
  worse than INERT, needs physical access (Roy is 30km away). MUST be metal-validated on a physically-accessible
  board (confirm espflash can reach the sw-triggered USB-JTAG download mode over the link) BEFORE any remote use.
  The console-persona-receiver already covers the immediate need; reboot-to-download is the riskier last-resort
  recovery path. Matches supervisor's "secondary later / FOLLOW-ON".

## ► 2026-06-30 — INERT-LIVENESS FIX DONE+GREEN (firmware a2f1718→93453de) + latent emit_msg regression fixed
Supervisor+composer GO'd the inert-liveness fidelity fix; LANDED at `93453de` (build-green xtensa across
field,loraroute,multitg / field,loraroute,bridge,multitg / routetest / loraroute / nobt). r2-hive recovery patch
refreshed (reverse-apply OK).
- **Inert-liveness:** the §3.5 fail-closed INERT loop (main.rs ~185) now emits — every ~6s (under composer's 12s
  grey threshold) — a HEALTH line (build_health: wire_id=mac_low3, tg=0, ip=0.0.0.0) + a `role=inert` status
  line, + the human notice every ~30s. An unprovisioned field board now shows as a LIVE-INERT node on composer's
  dashboard instead of being invisible. FAIL-CLOSED FULLY PRESERVED: serial-println ONLY — no radio TX, no mesh
  Event frame, no TG adoption; tg=0/ip=0 honestly mark no-TG/no-net. composer's reader already parses HEALTH/
  status so it "just works".
- **Latent regression FIXED (honest self-catch):** a2f1718 (per-hop k4) had pinned emit_msg's map element-count
  `n` to u64 via `as u64`, breaking Encoder::map(usize) in the FIELD/r2-cbor combos — which were NOT in a2f1718's
  5-combo verify (a real gap in that verification; the field combos use r2-cbor's map(usize), the verified combos
  either cfg'd emit_msg out or used a u64-accepting map). Restored `n` to type-inferred (mut + +=). Lesson: the
  per-hop verify should have included a field combo; it does now.
- **Pre-existing (NOT my regression, NOT in scope):** plain `field` (no routetest) does not compile — field/fr4
  SCF code calls emit_msg/ROUTETEST_HASH/mesh_broadcast which are routetest-gated, so `field` has ALWAYS required
  routetest (ships as field,loraroute,…). Noted, not "fixed" (field-alone is not a shipped combo).
- **Bench unblock decision (Roy's call, supervisor relaying):** PROVISION the 10 boards (mint personas, one bench
  TG = a real 10-node mesh) vs demo/bench-build reflash. The inert-liveness fix makes inert boards visible
  REGARDLESS of that call. composer derives device→IP from r2.hb.health key3 for OTA push (see #11/#17).

## ► 2026-06-30 — BENCH ZERO-TELEMETRY DIAGNOSED (my INERT halt) — fix path sent, decision pending (SUPERSEDED ↑)
Composer's full-check: 10 ESP32 USB-powered but a 30s /r2 sample saw ZERO r2.hb.health/status/beacon/msg.*.
ROOT CAUSE (firmware ground truth) = my own R2-PROVISION §3.5 fail-closed INERT halt (main.rs ~185):
`#[cfg(field)] if persona.is_none() { loop { println!("§3.5 INERT…"); Timer 30s } }` runs BEFORE io_task/
ota_task/render-loop are spawned, so a FIELD build + UNPROVISIONED board emits ONLY a boot banner (gone before
the reader attaches) + one INERT line / 30s → none of the telemetry the orchestrator parses. Working as designed
(fail-closed) but reads as a dead bench.
- GATING FACTS (answer to composer's Q): on a NORMALLY-RUNNING board the idle heartbeat is ALREADY UNGATED —
  status ~2s (main render loop, line 653) + HEALTH ~6s (io_task `fire_seq % 3`, line ~1111). NEITHER is
  routetest-gated. msg.* IS routetest-gated (per-Event traffic — correct). [⚠ CORRECTION 2026-06-30: my
  "beacon is LoRa-only → N/A on WiFi ESP32" claim here was WRONG — see the BLE-BEACON GAP entry below; there IS a
  BLE-beacon advert but it's gated to am_provider==M7_PROVIDER_HIVE, never generalized.] So idle liveness is not
  the problem; the INERT halt suppressing ALL tasks is.
- FIX PATH sent to supervisor+composer (2026-06-30): (1) IMMEDIATE no-fw — PROVISION each board (persona.bin
  @0x12000 + reboot) OR flash a NON-field bench build (demo-TG fallback emits idle telemetry out of the box,
  fastest). (2) FIRMWARE FIDELITY FIX (build on GO) — emit a minimal idle HEALTH/status FROM the INERT loop
  (role=inert marker; radio OFF, no TG, fail-closed FULLY preserved) so an unprovisioned board shows as a
  live-INERT node, not invisible (Roy 'bench mirrors real state'). DECISION PENDING: are the bench boards meant
  to be field (→provision) or bench-build (→reflash)? + do they want fw-fix (2)? Do NOT weaken fail-closed
  (radio/TG stay off); the fix only ADDS a liveness line.

## ► 2026-06-30 — PER-HOP RX TRANSPORT TELEMETRY (supervisor-elevated, core test dep) — DONE+GREEN
Firmware `dfr1195-fw` at `a2f1718`; r2-hive recovery patch refreshed at `2108576`. Supervisor elevated per-hop
transport-tagged telemetry from path-animation polish to a CORE TEST DEPENDENCY (the bench must visualise REAL
link-strength-through-usage, which only real observed per-hop traffic can drive). Observability only — no spec gate.
- **What landed (Phase A):** `msg.rx` now emits `{0:id,1:at,2:from_hop,3:origin,4:transport}`. New key `4` =
  the `r2_route::Transport` ordinal of the carrier the frame was RECEIVED on. Numbering is the canonical 7-bit
  space (`transport.rs`: Ble0/Wifi1/Lora2/Internet3/Usb4/EspNow5/Udp6 == `transport_allow_mask` bits), so bench
  per-link counts map 1:1 to host mask semantics.
- **Tap (core-confirmed):** all inbound radios coalesce through one `DATA_RX` channel — so the RX carrier was
  being lost there. Added a 4th `MeshRxFrame` field stamped per-feeder (espnow_task=EspNow, lora_task=Lora,
  blemesh CoC=Ble) + the io_task UDP select-arm=Udp; threaded to `emit_msg` k4. This is core's flagged
  handle_rx/DATA_RX site. NO wire/on-air change; the tag never re-enters the air.
- **Why rx-side is sufficient:** every received frame = one real `(from_hop, transport)` link traversal, so
  rx counting fully measures traffic crossing each link (Roy's link-strength-through-usage signal) with no
  multi-carrier ambiguity. `emit_msg` change is ADDITIVE (keys 0-3 unchanged) → composer's `/r2` parser keeps
  working and adopts k4 when ready.
- **Build-verify:** `cargo build --release` GREEN (xtensa esp32s3) across `routetest` / `loraroute` / `blemesh`
  / `nobt` / default — covers all three feeders + both sides of the routetest gate.
- **Caveat:** `msg.*` telemetry is `routetest`-gated (the regime composer's bench runs in). Broadening to ALL
  traffic is a separate, more invasive scope call — flag before doing it.
- **Phase B (scoped, NOT built):** egress-carrier tag on `msg.tx`/`msg.relay` (per-carrier emit in
  `mesh_broadcast`, since a bridge fans out ESP-NOW+LoRa). Only needed if the bench wants the SEND-side carrier;
  rx-side already counts every link. Also pending: composer's item (2) per-device transport-mask ENFORCEMENT hook
  at the DATA_RX/handle_rx seam (waits on core's runtime mask shape + composer ping).
- **Coordination:** notified supervisor (done), composer (the exact k4 shape for the /r2 parser), core (tap +
  numbering confirm; offered BIT vs ordinal). Do not assume composer has adopted k4 yet.

## ► 2026-06-30T06:26:56+12:00 — DOCTOR-ONLY FINAL IDLE REFRESH
Objective: doctor-only durable handoff refresh after stopped-lane fleet activity. No code/content edits; update
`RESUME.md` only if ground truth shows stale current state, then commit/push and idle.
- **Branch/HEAD/worktree:** r2-hive is on `platform-trait`, clean and in sync with `origin/platform-trait`.
  The authoritative current HEAD is whatever `git rev-parse HEAD` / `origin/platform-trait` shows — do NOT
  trust any frozen hash written in this file, since each RESUME refresh is itself a doc-only commit that
  advances HEAD. The recent chain of doc-only hygiene commits is
  `a10d63f`→`18e3b1c`→`e422250`→(this refresh); none of them touched repo source. The substantive firmware
  work lives in the sibling `dfr1195-fw` worktree, not here.
- **Firmware worktree state:** `/home/roycdavies/Development/R2/dfr1195-fw-wt` is on `dfr1195-fw` at
  `54973b9ba17a` (`feat(dfr-ota): R2/R3/R4 OTA-receiver hardening (specs-sanctioned)`), matching
  `origin/dfr1195-fw`, with exactly one dirty file: `M docs/dfr1195-firstlight.patch` inside that sibling
  worktree. No platform source diff was observed there this turn. Do not "clean" that core-owned worktree from
  r2-hive.
- **Transport allow-mask status:** implemented in r2-hive host/sync/local-mgmt and currently verified. Tracked-file
  check shows `crates/r2-hive-bin/src/mgmt/transport_policy.rs`, `mgmt/api.rs`, `mgmt/mod.rs`,
  `crates/r2-hive-bin/src/hive.rs`, `crates/r2-hive-core/src/sync_host.rs`, and the focused integration tests are
  all tracked. `rg` confirms `mgmt/mod.rs` exports `transport_policy`, `mgmt/api.rs` dispatches
  `r2.mgmt.transport.allow_mask.{state,set,clear}`, `HiveState` delegates the effective mask to
  `route_engine.transport_allow_mask()`, and host sends check the mask before physical egress. Targeted gates run
  at current HEAD all PASS:
  `cargo test -p r2-hive-core route_respects_transport_allow_mask_before_sync_send -- --nocapture`;
  `cargo test -p r2-hive-core route_drops_when_mask_removes_only_sync_candidate -- --nocapture`;
  `cargo test -p r2-hive --test transport_integration transport_allow_mask_filters_host_send_before_physical_egress -- --nocapture`;
  `cargo test -p r2-hive --test mgmt_integration transport_allow_mask_mgmt -- --nocapture`. Only observed warning:
  pre-existing `r2-wire` dead-code warning for `EXT_AUTH_MAX`.
- **DFR/ESP32 patch + partition status:** r2-hive `docs/dfr1195-firstlight.patch` still byte-matches
  `git -C /home/roycdavies/Development/R2/dfr1195-fw-wt diff c46383e..HEAD -- platforms/dfr1195/Cargo.lock
  platforms/dfr1195/Cargo.toml platforms/dfr1195/build.rs platforms/dfr1195/src/main.rs
  platforms/esp32/sdkconfig.defaults`, and reverse-apply check in the firmware worktree PASSes. Source config
  remains custom-partition canonical: `platforms/esp32/sdkconfig.defaults` has
  `CONFIG_PARTITION_TABLE_CUSTOM=y`, `CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"`, and
  `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`; `platforms/esp32/partitions.csv` has `otadata@0xf000`,
  `ota_0@0x20000 size 0x1E0000`, and `ota_1@0x200000 size 0x1E0000`. Generated ESP-IDF `out/sdkconfig` also
  shows rollback enabled, anti-rollback not set, `TWO_OTA` not set, and custom table enabled. The prior ESP32
  build artifact still exists:
  `platforms/esp32/target/riscv32imac-esp-espidf/release/r2-esp32` = 3,698,964 bytes, mtime
  `2026-06-28 07:50:37 +1200`. I did NOT rerun the ESP32 build this turn; current `esp-idf-sys` output has no
  copied `out/partitions.csv`, so the known custom-partition copy race/workaround is still a real build caveat.
- **Known external-gated items / no local code-only action:** ESP32/DFR OTA confirmed-boot and rollback still need
  metal/network validation; radarprobe remains blocked on Roy-side physical/model facts (continuity RO->GPIO44,
  DI->GPIO43, DE-RE->GPIO6, MAX485 5V/GND, radar model/datasheet); CCR1 remains composer-contract/emitter gated;
  ESP-IDF custom partition handling still needs a portable fix or documented repeatable workaround; transport
  allow-mask firmware role-profile ingestion, per-hop telemetry tags, and bench metal validation were not added by
  the host/sync/mgmt patch and remain scoped to later contract/bench work. Do not re-adopt ESP-IDF
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` unless the image shrinks below 1 MiB or a different built-in table is proven.
- **Paused-branch note:** `crates/r2-hive-core/src/record_store.rs` is not part of current `platform-trait`; it
  belongs to the paused `storing-backend` branch at `478203a`. Treat any RecordStore seam notes as branch-scoped
  unless that branch is explicitly resumed.
- **Verification this turn:** `git status --short --branch`; `git log -5 --oneline --decorate`; `date -Iseconds`;
  focused `git ls-files`/`rg` wiring checks; the four targeted cargo tests above; sibling firmware
  `git status`/`git log`; patch `cmp` byte-match and reverse-apply check; ESP32 sdkconfig/partition/artifact
  inspection; `fleet inbox | tail -80` confirming the doctor-only refresh request. No full workspace test or fresh
  ESP32 build was run because this is a RESUME-only doctor refresh.

## ► 2026-06-30 — DOCTOR HYGIENE / MARKER WORDING CLEARED
Objective: resolve fleet-doctor handoff hygiene only: inspect stale marker wording in `RESUME.md`, verify the
old `transport_policy.rs` untracked/unwired blocker against disk, and avoid code changes. Result: **DOC HYGIENE
ONLY** on branch `platform-trait`; pre-edit worktree was clean at `41eed45`.
- **Transport-policy blocker status:** resolved. Ground truth: `git ls-files --stage` shows
  `crates/r2-hive-bin/src/mgmt/transport_policy.rs`, `crates/r2-hive-bin/src/mgmt/api.rs`, and
  `crates/r2-hive-bin/src/mgmt/mod.rs` are all tracked. `rg -n "transport_policy|TransportPolicy|transport policy" .`
  shows `mgmt/mod.rs` exports `pub mod transport_policy;`, `mgmt/api.rs` imports it and dispatches
  state/set/clear event classes to it, and integration tests reference the same module.
- **Marker cleanup:** replaced remaining stale marker wording in old handoff notes with concrete
  `follow-up`, `remains open`, or completed-task language. No active technical follow-up was removed; the FR-2
  firmware work, AP-failover WiFi layer, and LED-init work remain recorded as open where they were already
  described.
- **Changed files:** `RESUME.md` only.
- **Verification:** narrow doc checks only: `git status --short --branch`; fleet-doctor marker scan of
  `RESUME.md`; `rg --files | rg 'transport_policy\.rs$|transport_policy'`; tracked-file check via
  `git ls-files --stage`; wiring check via `rg -n "transport_policy|TransportPolicy|transport policy" .`. No
  cargo tests are needed for this docs-only hygiene change.
- **Do not assume:** this entry does not re-verify the previously green transport-policy cargo gates, metal bench,
  or firmware patch application; it only records current tracked/wired handoff state and removes stale marker
  wording.

## ► 2026-06-29 — BENCH PHASE-2 TRANSPORT-DISABLE WIRING / IMPLEMENTED+GREEN
Objective: wire the now-unblocked Phase-2 node-wide egress transport software-disable policy in r2-hive without
inventing hive-local routing semantics, then verify and push. Result: **IMPLEMENTED** against core's canonical
`r2_route` API on branch `platform-trait` (pre-work HEAD `852e03b`; this RESUME entry is in the transport-policy
implementation commit).
- **Verified authority before coding:** r2-specifications clean on `spec-conformance-v0.2` at
  `45b8a507e731aeeaae124f263f0809c4116502c5`; R2-TRANSPORT §2.3A says `transport_allow_mask` is `0x7F`
  default all-on, node-wide, egress-only, disable-only, leased/acknowledged/clearable, local-authority-only by
  default, not gossiped/mesh-written; R2-ROUTE §5.2 applies it as a hard candidate filter before scoring;
  R2-RUNTIME §3.2.2 lists it as an optional role-profile knob. r2-core clean on `r2-core-consolidation` at
  `7c0320eaa9ca49e26dcb2d4ae4fb27fd6af405cb`; `c2737b9` exposes
  `RouteEngine::{transport_allow_mask,set_transport_allow_mask_bits,clear_transport_allow_mask,set_transport_allowed,transport_allowed}`
  over the canonical 7-bit `TransportSet`, and `DataPlane` delegates to the same surface. No r2-core files were
  edited.
- **Host/state wiring:** `HiveState` now keeps only local ACK/state lease metadata; the effective policy remains
  single-sourced in `route_engine.transport_allow_mask()`. Added `transport_policy_snapshot`,
  `set_transport_policy_lease`, and `clear_transport_policy`. `send_to_hive_via` now snapshots the core allow mask
  and skips disabled transports before any physical WS/UDP/BLE/LoRa/USB-dongle send attempt. This covers local
  sends that do not pass through `RouteEngine::plan_forward` first; route-engine planned egress already gets the
  core hard filter before scoring.
- **Mgmt surface (local only, no mesh mutation):** new UDS/loopback mgmt event classes:
  `r2.mgmt.transport.allow_mask.state`, `.set`, `.clear`. Requests are R2-WIRE extended frames with CBOR payloads:
  `state {0:cid}`; `set {0:cid,1:mask_uint8,2:lease_id_uint,3:source_text}`; `clear {0:cid,1:lease_id_uint?}`.
  Set ACK returns `{0:cid,1:requested_mask,2:accepted_mask,3:effective_mask,4:all_mask,5:lease_id,6:source,7:true}`.
  State/clear return `{0:cid,3:effective_mask,4:all_mask,7:active_bool}` plus lease fields `{1,2,5,6}` when active.
  Unknown bits are acknowledged via core truncation (e.g. requested `0x82` → accepted/effective `0x02`). A second
  different lease gets `r2.mgmt.event.error` code `lease_conflict`; clearing without a lease id is the local
  force-clear. Mgmt-only daemon state returns `unsupported` rather than silently unknown.
- **Sync/no_std proof:** `r2-hive-core::sync_host::route_inbound_sync` still delegates to the caller's
  `RouteEngine`; focused tests set the core mask directly and prove (a) masked higher-scoring LoRa is not sent
  while WiFi remains viable, and (b) a masked only-candidate drops without egress. No firmware source or
  `docs/dfr1195-firstlight.patch` changed; firmware/host boundaries preserved.
- **Changed files:** `crates/r2-hive-bin/src/hive.rs`,
  `crates/r2-hive-bin/src/mgmt/{api.rs,mod.rs,transport_policy.rs}`,
  `crates/r2-hive-bin/tests/{mgmt_integration.rs,transport_integration.rs}`,
  `crates/r2-hive-core/src/sync_host.rs`, and `RESUME.md`.
- **Verification:** targeted tests PASS:
  `cargo test -p r2-hive-core route_respects_transport_allow_mask_before_sync_send -- --nocapture`;
  `cargo test -p r2-hive-core route_drops_when_mask_removes_only_sync_candidate -- --nocapture`;
  `cargo test -p r2-hive --test transport_integration transport_allow_mask_filters_host_send_before_physical_egress -- --nocapture`;
  `cargo test -p r2-hive --test mgmt_integration transport_allow_mask_mgmt -- --nocapture`.
  Full gate PASS: `cargo test --workspace` (105 r2-hive lib tests, 20 mgmt integration tests, 4 transport
  integration tests, all other workspace tests/doc-tests green; one pre-existing ignored router authenticated-dedup
  fixture remains ignored). `git diff --check` PASS. `cargo fmt --all --check` is NOT a valid repo-local gate today
  because it tries to format/check the sibling `r2-core` path dependency and reports pre-existing r2-core rustfmt
  drift; the new `transport_policy.rs` was rustfmt'd directly and unrelated rustfmt churn was reverted.
- **Refutation / peer challenge:** asked core for an adversarial API/semantics check. The direct off-thread answer
  hit the provider spend-limit message, but supervisor relayed the peer-review result: specs-codex found no spec
  gaps; core-codex found one concrete WIP blocker, to ensure `transport_policy.rs` is tracked and that `mgmt/mod.rs`
  + `mgmt/api.rs` dispatch it. That blocker is resolved by the final staged file set before commit.
- **Composer/bench next endpoint:** composer should drive the local UDS management socket (default
  `r2_hive::default_socket_path()`, usually `$XDG_RUNTIME_DIR/r2-hive.sock` or `/tmp/r2-hive-<uid>.sock`) with
  `r2.mgmt.transport.allow_mask.set {0:cid,1:mask,2:lease_id,3:"composer:bench-phase2"}`. For "disable LoRa only",
  send mask `0x7B` (`0x7F & !Transport::Lora.bit()`). Clear with
  `r2.mgmt.transport.allow_mask.clear {0:cid,1:lease_id}` or omit key `1` for local force-clear. Do not send this
  as a mesh `r2.api.event.send`; mesh-received frames intentionally do not mutate the policy.
- **Do not assume:** this is host/sync/mgmt enforcement only. No firmware role-profile ingestion of
  `transport_allow_mask` was added in this patch, no per-hop telemetry tags were added, and no metal bench was run
  because no core-crate pin/bump or firmware artifact changed in r2-hive.

## ► 2026-06-29 — BENCH PHASE-2 TRANSPORT-DISABLE RECHECK / BLOCKED-ON-HIVE-CALLABLE CANONICAL API
Objective: re-check the stale transport-disable hold after specs/core landed the Phase-2 policy commits, then either
wire the smallest hive integration or record the precise blocker. Result: **NO HIVE CODE WIRING YET**; the spec is
now ratified locally, and core has a lower-level `r2-dataplane` `PhyMask` setter, but current hive code has no
callable canonical 7-transport policy surface without inventing a hive-local clone.
- **Verified local ground truth:** r2-hive `platform-trait` was at `eeee933` with only this `RESUME.md` dirty;
  r2-specifications was clean on `spec-conformance-v0.2` at `45b8a507e731aeeaae124f263f0809c4116502c5`;
  r2-core was clean on `r2-core-consolidation` at `c5d0be8df05e99c2fa9f9540400752f29890e7f6`. The DFR firmware
  worktree remains `dfr1195-fw` at `54973b9` with only its nested `docs/dfr1195-firstlight.patch` dirty, so do not
  assume that worktree already tracks core `c5d0be8`.
- **Spec surface now landed:** `d55577c` adds R2-TRANSPORT §2.3A `transport_allow_mask` over the canonical §2.2
  7-transport bitmask (`0x7F` all-on), node-wide, egress-only, disable-only, leased/acknowledged/clearable, local
  authority by default, and not advertised/gossiped. R2-ROUTE §5.2 now says the mask is a hard filter before
  scoring. R2-RUNTIME §3.2.2 adds optional role-profile `transport_allow_mask`.
- **Core surface now landed:** `4ca1364` adds `r2_dataplane::{PHY_FLRC, PHY_LORA, PHY_ALL}` and
  `DataPlane::{egress_enabled_mask,set_egress_enabled_mask,set_egress_phy_enabled,egress_phy_enabled}`. The mask
  is applied inside `r2-dataplane` to `handle_rx_frame` relay output and `poll_keepalive` output, and it strips
  unknown bits. This is lower-level physical-carrier policy (`PHY_ALL == PHY_FLRC|PHY_LORA`), not the canonical
  `Transport` `0x7F` mask by itself.
- **Blocker verified in code:** `rg` over current core found no `transport_allow_mask`, route-engine policy setter,
  or `select_transport`/`RouteEngine::plan_forward` parameter for the 7 canonical `Transport` bits. `r2-route`
  still selects from `NeighbourEntry.transports`, MTU, link quality, and strategy only. Current r2-hive does not
  depend on `r2-dataplane` in its host crates; `rg r2_dataplane` in r2-hive hits only a process-hygiene comment and
  the firmware patch artifact. The DFR firmware source imports only `encode_dc_seq_cbor`, `frame_fingerprint`,
  `parse_dc`, and `parse_seq` from `r2_dataplane`; it does not instantiate `DataPlane`, `handle_rx_frame`, or
  `poll_keepalive`, so there is no existing object to call the new setter on.
- **Why no hive patch this turn:** wiring Linux/cloud `HiveState::send_to_hive_via` or
  `r2-hive-core::sync_host::route_inbound_sync` would require a new hive-owned 7-bit mask/lease manager and a
  mapping to `Transport::{Ble,Wifi,Lora,Internet,Usb,EspNow,Udp}` outside core's landed API. Wiring the DFR patch
  directly would require either migrating the firmware io loop onto `r2_dataplane::DataPlane` or fabricating a
  local `Transport`→`PhyMask` policy adapter. Both would create semantics the user explicitly barred.
- **Smallest unblocked path once core/supervisor picks it:** either (A) core exposes the canonical
  `transport_allow_mask` as a shared policy type/manager and route/host filter API over `r2_route::Transport`
  bits, then hive wires `HiveState`, `sync_host`, UDS/loopback mgmt ACKs, tests, and role-profile ingestion; or
  (B) firmware first migrates the DFR io path to the landed `r2-dataplane` two-entry-point contract, then hive can
  set `DataPlane::set_egress_enabled_mask()` at the physical-carrier boundary and separately reconcile the
  spec-level `Transport` mask mapping. Until then, keep the policy local-only; mesh-received frames MUST NOT
  mutate it.
- **Peer/refutation:** asked core whether a host-wide `Transport` policy API exists or whether only the
  `DataPlane` `PhyMask` setter landed; the off-thread answer was the provider spend-limit message, so no peer
  challenge was available. Confidence is from local disk inspection only.
- **Verification this turn:** `git status --short --branch` in specs/core/hive; `git show --stat` for
  `d55577c`, `45b8a50`, `4ca1364`, `c5d0be8`; spec reads of R2-TRANSPORT §2.3A, R2-ROUTE §5.2, and R2-RUNTIME
  §3.2.2; targeted `rg`/`sed` inspections of `r2-dataplane`, `r2-route`, hive `HiveState`, hive `sync_host`, and
  the DFR firmware worktree. No cargo tests were run because this turn intentionally makes a docs/handoff-only
  blocker update.
- **Changed files:** `RESUME.md` only. Do not add hive-local transport-mask semantics or mesh-remote control
  frames to bypass the missing shared API.

## ► 2026-06-28 — DFR FIRMWARE PRE-METAL HARDENING (refutation-review items, supervisor GO) — DONE+GREEN
Worktree `dfr1195-fw` HEAD `54973b9`. Three refutation-review items implemented + build-green at `428f81c`
(field,loraroute,multitg / nobt / radarprobe / field,loraroute,bridge,multitg), then R2/R3/R4 OTA-receiver
hardening landed at `54973b9` with commit-recorded `cargo build --release` GREEN (xtensa esp32s3, 13.54s).
Patch refreshed (`docs/dfr1195-firstlight.patch`, c46383e..HEAD = 16 commits). Metal validation of the OTA
round-trip remains bench-network-gated.
1. **§3.5 fail-closed is now INERT (not advisory).** Under `field` + no valid persona: HALT before any TG/
   radio/task setup — no demo-TG adoption, no radio/HB/beacon/io spawns (was only a louder println). Bench
   builds (no `field`) keep the demo fallback. (main.rs persona-boot block.)
2. **OTA confirmed-boot (mirror r2-core confirm_or_rollback_on_boot).** New `ota_confirm_or_rollback_on_boot()`
   at boot: ota_state ∈ {New,PendingVerify} → §5 health-gate → mark Valid (confirm) OR Invalid + roll back to
   prev slot + reboot. OCM marks the activated slot `New` (esp-idf set_boot semantics). Uses esp-bootloader-
   esp-idf 0.5.0 current_ota_state/set_current_ota_state (source-verified — 0.5.0 resolved, NOT the 0.2.0 I
   first read). Health-check is minimal "booted past init"; richer §5 self-test = follow-up.
3. **After-confirm seq-floor (R2-UPDATE §5.1).** Floor no longer bumped at OCM-activate — OCM STAGES (seq,
   floor) to a new OTA-pending NVS sector @0x1A000; the live anti-rollback floor commits ONLY at confirmed-
   boot after the §5 gate. Kills the v0.21 brick-defect (a bad image can't raise the floor) — this CLOSES the
   FORKS.md "OTA anti-rollback floor ORDERING" fork (impl done; metal-validate when the OTA round-trip unblocks).
4. **OTA receiver R2/R3/R4 hardening (specs-sanctioned receiver robustness, not binding ratification).** R2:
   30s inactivity timeout abandons a stalled in-flight OTA session. R3: `payload_size = vh.payload_len`, ODT
   rejects off+len beyond the declared payload, and OCM commits only when `written == payload_size`. R4: ODT/OCM
   are bound to the authenticated OST sender address; foreign chunks/commits are dropped silently. Verify-before-
   write + New/PendingVerify confirmed-boot lifecycle intact.
NVS map now: persona@12000 / board@13000 / tg@14000 / mask@15000 / sendto@16000 / role-profile@17000 /
anti-rollback@18000 / CCR1-reserved@19000 / ota-pending@1A000. ⚠ crash-on-boot auto-rollback still needs
CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE in the composer-staged bootloader (deployment follow-up; core owns it).
- **CORE PARTITION RULING LANDED:** keep custom `partitions.csv`; do NOT switch to ESP-IDF built-in
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` (deploy-invalid: 1 MiB slots, current image is ~1.6 MiB). Core confirmed
  custom CSV supplies the needed `otadata` + two OTA slots + rollback-enable. Remaining non-metal diagnostic:
  esp-idf-sys custom-partition copy race still needs a portable fix or documented workaround; do not re-litigate
  `TWO_OTA` unless the image shrinks below 1 MiB or another built-in table is proven.
- **TAKEOVER HYGIENE (hive-codex, 2026-06-29; pre-edit r2-hive HEAD `e27b56e`):** rechecked r2-hive clean on
  `platform-trait`; firmware worktree at `54973b9` with only its nested `docs/dfr1195-firstlight.patch` dirty.
  Regenerated the r2-hive recovery artifact from `c46383e..HEAD` over the owned firmware paths and found
  r2-hive's `docs/dfr1195-firstlight.patch` stale by 87 lines (missing `54973b9` R2/R3/R4 OTA hardening), then
  refreshed it. Composer telemetry answer: firmware emits `r2-dfr1195: msg.* <hexcbor>` over USB serial; composer
  has already forwarded/used normal `msg.tx/rx/relay/delivered` as the `/r2` orchestrator `msg.*` stream for
  step-a/happy-path/E2/E3. Earlier SCF one-shot evidence used raw serial because of a one-shot orch WS gap, so
  Phase 2 path animation can consume `/r2` for normal lifecycle, but should keep raw serial as the diagnostic
  fallback for rare SCF-gap captures until composer confirms the gap is closed. Changed files this turn:
  `docs/dfr1195-firstlight.patch` and `RESUME.md`. Verification: regenerated-patch byte-match PASS;
  reverse-apply in `/home/roycdavies/Development/R2/dfr1195-fw-wt` PASS; `git diff --check` PASS. No full
  workspace tests run because this is a docs/artifact-only refresh.

## ► CURRENT 2026-06-27 — RADAR BRING-UP (Modbus-RTU PROBE, Roy chose PROBE-to-discover; ULTRACODE on)
First REAL sensor. Build+flash a Modbus-RTU PROBE firmware to the radar XIAO to discover the radar protocol
empirically (baud + slave-addr + register map), → then build the real radar driver + sentant on the sensor ensemble.
- **RADAR XIAO IDENTITY-VERIFIED (safety gate):** MAC **1c:db:d4:5b:8a:60**, esp32s3 rev v0.2, 8MB, **ttyACM12**
  (by-id `usb-Espressif_USB_JTAG_serial_debug_unit_1C:DB:D4:5B:8A:60-if00`), port FREE. It is the ONLY
  Espressif NOT in {triplet 14:C1:9F../E8:3D..E5:20/D8:3B.. + spare E8:3D..DB:44 + 5 DFR F4:12:FA:*}. FLASH
  ONLY this by-id path (ttyACMn remaps — verified the trap; Alfred has 11 Espressif boards now).
- **PROBE LOGIC:** Modbus-RTU master over XIAO UART→RS-485 transceiver; sweep baud {4800,9600,19200,38400,
  115200}×slave-addr (1 first, then 1..247 subset); on CRC-valid response → dump holding(fn 0x03)+input(fn
  0x04) regs 0..63 + device-id (fn 0x2B/0x0E); print over USB serial. Report baud+addr+register-map.
- **RS-485 PINS RECEIVED (Roy, 2026-06-27):** MAX485 transceiver. RADAR_UART_TX=**GPIO43** (D6 → MAX485 DI),
  RADAR_UART_RX=**GPIO44** (D7 ← MAX485 RO), RADAR_DE_RE=**GPIO6** (D5, DE+RE tied; HIGH=TX, LOW=RX). Radar
  self-powered 12V (live slave answers). OUTPUT on USB-CDC console ONLY (the GPIO43/44 UART IS the RS-485
  bus — never log to it). GPIO43/44 = esp32-s3 default UART0 pins BUT console rides USB-Serial-JTAG (free);
  use UART1 via GPIO-matrix to avoid any UART0 console remnant. radarprobe gates OFF LoRa so GPIO6 (=DFR LoRa
  MOSI) won't collide. Half-duplex: DE/RE HIGH before TX, HOLD until UART TX-COMPLETE, then LOW for RX (the
  brick gotcha — get esp-hal tx-done detection right; core advising). Flash NO LONGER pin-blocked — gated only
  on the design workflow finishing + build-green; re-confirm identity (1c:db:d4) at flash.
- **IN FLIGHT (2026-06-27):** Workflow `wk6evtri0` (radar-probe-design: research→adversarial-verify→synthesize
  the esp-hal UART half-duplex DE/RE + Modbus-RTU + firmware-integration spec; API-drift-hardened since it
  bit us 3× this session). Fork-asked core for the esp-hal UART TX-complete/baud-reconfig/UART-peripheral
  gotchas. NEXT: implement the `radarprobe` feature + probe task per the synth spec, build-verify xtensa, hold flash.
- **PROBE BUILT + FLASHED + RUNNING (worktree `3bc56d1`+parity-sweep).** `radarprobe` cargo feature
  (standalone RS-485 Modbus master on UART1, radio stack OFF, USB-CDC output). Design via Workflow
  `wk6evtri0` (source-verified esp-hal API: flush()=tx-idle mod.rs:850/906, apply_config live baud sweep;
  adversarial-verified Modbus CRC poly 0xA001) + core's UART gotchas. esp-hal flush/spawn(Result)/Config
  builders all source-confirmed. CRC self-test PASSES on metal. Flashed to radar XIAO 1c:db:d4 (identity
  re-confirmed via board-info).
- **FORMAT-EXHAUSTIVE SWEEP = FULLY NULL (escalated to Roy).** 21 combos (parity {N,E,O} × baud
  {2400,4800,9600,19200,38400,57600,115200}, 8 data /1 stop), Roy's pins (TX=43/RX=44/DE-RE=6): ALL
  START→DONE clean, ZERO responses, ZERO garbage, no panic. Probe FUNCTIONAL (CRC-selftest PASS). The
  CLEAN-silence across the WHOLE format space ⇒ UART RX received NOTHING ⇒ radar never got our request
  (TX-path) or isn't transmitting. Firmware's safe space EXHAUSTED. Sweep log: scratchpad/radar-sweep.log.
- **REMAINING = PHYSICAL (Roy's bench) — escalated.** (1) TX/RX wiring vs MAX485 DI/RO (the ambiguity Roy
  flagged) — ⚠ I will NOT blind-swap in firmware: if GPIO44 is wired to RO (an output), driving it as TX =
  output-contention = HW-damage risk; the swap must be a WIRING change or confirmed first. (2) DE/RE pin
  (is D5=GPIO6 right?) + polarity (standard tied DE-high/!RE-low ⇒ HIGH=TX is what I use). (3) radar 12V on
  + A/B actually landed on the MAX485 A/B. (4) is it genuinely Modbus-RTU (vs a proprietary/streaming
  protocol or a different bus)? — radar MODEL/datasheet would pin the real baud/addr/protocol.
  AWAITING Roy: confirm wiring/power OR the radar model. Next firmware experiment (only after Roy OKs the
  wiring): TX/RX-swapped re-flash. Probe + parity-sweep already committed (worktree).
- **POWERED RE-RUN (battery on) = STILL FULLY NULL** + **PASSIVE LISTEN-ONLY phase = NONE at every baud.**
  Added a safe RX-only listen phase (DE/RE low, never drives the bus) to catch a STREAMING/non-Modbus radar
  + test the RX path. Result: ZERO bytes received passively at ANY baud (9600..2400), AND the active Modbus
  sweep null again. DECISIVE: the UART RX (GPIO44←MAX485 RO) gets NOTHING under any condition, and the radar
  is NOT streaming. Firmware has exhausted BOTH active (format space) + passive (listen) testing → the issue
  is PHYSICAL, not firmware/format. ESCALATED to Roy, prioritized: (1) SWAP A/B bus wires (most common RS-485
  fix; reversed A/B ⇒ MAX485 receiver outputs nothing valid ⇒ clean silence) ; (2) verify continuity RO→GPIO44
  (RX) / DI→GPIO43 (TX) / DE-RE→GPIO6 ; (3) confirm the radar is actually transmitting (LED/scope) ; (4) radar
  MODEL/datasheet (protocol/baud/addr + any wake/init command; may not be Modbus). Probe is fully built +
  metal-proven-functional (CRC-selftest PASS); ready to re-run the instant a physical variable changes.
- **A/B SWAP (Roy) = STILL FULLY NULL** (both A/B orientations now tested). 7 listen-NONE + 21/21 Modbus
  combos, zero response/garbage. So A/B polarity is NOT it either. Firmware DEFINITIVELY EXHAUSTED (active
  format space × both A/B orientations + passive listen). RX path delivers zero bytes regardless ⇒ a BROKEN
  SIGNAL LINK or POWER/PROTOCOL issue. NARROWED next steps (Roy's bench, escalated): (1) CONTINUITY-meter
  RO→GPIO44(RX) [prime — RX path] / DI→GPIO43(TX) / DE-RE→GPIO6 ; (2) MAX485 POWER — Vcc=5V (not 3V3) + GND
  landed? (a MAX485 needs 5V; unpowered/3V3 transceiver = dead bus) ; (3) the A/B pair actually on the
  MAX485 A/B terminals? ; (4) **RADAR MODEL/DATASHEET** (highest value) — confirms Modbus-vs-proprietary, the
  real baud/addr/register-map, AND any WAKE/INIT command (a radar needing an init sequence never answers a
  blind read). Firmware side COMPLETE; no further probe iteration until a physical variable changes or the
  model lands. Logs: scratchpad/radar-{sweep,sweep-powered,listen,abswap}.log (all null).
- **COMPANION AUDIT (hive-codex, 2026-06-27):** git state clean on `platform-trait` before work; firmware
  worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` clean at `9fe219d` (base `c46383e`). Found a
  durable-handoff gap: r2-hive's `docs/dfr1195-firstlight.patch` did not include the radarprobe commits even
  though the firmware worktree did. Refreshed the patch artifact from
  `git -C ../dfr1195-fw-wt diff c46383e..HEAD -- platforms/dfr1195/Cargo.lock platforms/dfr1195/Cargo.toml platforms/dfr1195/build.rs platforms/dfr1195/src/main.rs platforms/esp32/sdkconfig.defaults`
  (intentionally excluding the nested `docs/dfr1195-firstlight.patch` inside the firmware worktree). Verified:
  `rg radarprobe docs/dfr1195-firstlight.patch` now hits; `git apply --reverse --check
  /home/roycdavies/Development/R2/r2-hive/docs/dfr1195-firstlight.patch` passes in the firmware worktree. No
  firmware source changed this turn; only the r2-hive patch artifact changed. Hygiene note: global
  `git diff --check` reports three trailing-whitespace warnings inside the generated patch artifact itself
  (`+ ` blank source lines); left intact so the patch remains a faithful diff of the firmware worktree.
  Coordination note: `fleet ask hive` could not get a substantive challenge because the base provider hit the
  org monthly spend limit; sent a heads-up anyway. Do not assume the scratchpad radar logs exist in this
  checkout (`scratchpad/` absent here).
- **COMPANION RE-CHECK (hive-codex, 2026-06-27):** objective remains patch/handoff hygiene only; no firmware
  iteration while the radar result is blocked on physical checks or a radar model. Verified branch
  `platform-trait`; r2-hive HEAD `225b8f4`; firmware worktree clean at `9fe219d`. Re-ran:
  `rg radarprobe docs/dfr1195-firstlight.patch` (hits the feature, GPIO43/44/6, passive listen, parity sweep)
  and `git -C /home/roycdavies/Development/R2/dfr1195-fw-wt apply --reverse --check
  /home/roycdavies/Development/R2/r2-hive/docs/dfr1195-firstlight.patch` (PASS). `git diff --check` still
  reports the same three trailing-whitespace warnings inside the generated patch artifact only; intentionally
  not normalized. `scratchpad/` is absent in this checkout. Coordination: `fleet ask hive` returned the org
  monthly spend-limit message, but `fleet inbox hive-codex` later had a base-hive ACK confirming the firmware
  worktree is stable, radar bring-up is paused on Roy-side physical/model input, and there is no patch-artifact
  race. Next action remains Roy bench: continuity RO->GPIO44 / DI->GPIO43 / DE-RE->GPIO6, MAX485 5V+GND,
  actual radar model/datasheet. Do not assume a firmware TX/RX swap is safe; driving GPIO44 if it is wired to
  MAX485 RO can contend outputs.
- **SECURITY CRITICAL CLOSED (hive-codex, 2026-06-27; security commit `d48094f`, patch-artifact commit
  `d13a12d`, pre-fix HEAD `225b8f4`):** verified and fixed
  the reported unauthenticated public management WebSocket. `/r2/mgmt` now has three gates: default daemon bind
  is loopback (`127.0.0.1`); non-loopback bind requires explicit `--allow-public-bind`; even with that opt-in
  the management WS is not mounted on non-loopback listeners, so local control is UDS/loopback-only by
  construction. The WS upgrade now requires a valid active `r2_web_session` cookie and rejects cross-origin
  browser upgrades. Web auth now enforces revocation inside `verify_cookie_header`; web plugins fail closed
  when `web_auth` is missing unless the operator explicitly sets `--web-dev-mode`. Install/package defaults
  changed to loopback; Docker keeps public container bind only with explicit `--allow-public-bind`.
  Changed security files: `crates/r2-hive-bin/src/{main.rs,hive.rs,web.rs,web_auth.rs,config.rs,mgmt/ws.rs}`,
  `crates/r2-hive-bin/tests/{web_auth_integration.rs,web_plugin_integration.rs,web_plugin_load.rs}`,
  `install.sh`, `Dockerfile`, `README.md`, and `crates/r2-hive-bin/packaging/defaults/hive.toml`.
  Verification: `cargo test -p r2-hive` PASS (105 lib + all integration/doc tests); `bash -n install.sh` PASS;
  `RUST_LOG=info target/debug/r2-hive --bind 0.0.0.0 --port 0 --no-mgmt --no-usb` exits before listen with the
  expected non-loopback refusal. `cargo test --workspace` still fails only at the pre-existing lower-priority
  red test `r2-hive-core::sync_host::tests::route_relays_to_known_neighbour` ("expected a relay decision, got
  Dropped") that supervisor already called out; critical mgmt-WS surface is closed. `git diff --check` still has
  only the known generated-patch whitespace warnings in `docs/dfr1195-firstlight.patch`.
- **CODEX REVIEW CLEANUP COMPLETE (hive-codex, 2026-06-27; branch `platform-trait`, test-fix commit
  `aba0ab7`, pre-cleanup HEAD `8531935`):** supervisor asked to close the three remaining codex-review items.
  Verified current code first: web-auth revocation is enforced in `web_auth::verify_cookie_header` by checking
  the active device ledger (`is_known_device`) after cookie signature/expiry validation; web plugins fail closed
  with `503 web auth not configured` when `web_auth` is absent unless explicit `--web-dev-mode` is set. Those two
  MED items were already closed by the security commit `d48094f` and are covered by
  `web_auth::tests::revoked_device_cookie_is_rejected`, `web_auth_integration::revoked_cookie_is_rejected`,
  `web_auth_integration::missing_web_auth_fails_closed_by_default`, and
  `web_auth_integration::explicit_dev_mode_serves_with_warning_header`. Fixed the remaining RED test in
  `crates/r2-hive-core/src/sync_host.rs`: `route_relays_to_known_neighbour` now builds a conformant extended
  fixture with `route_stack[0] = source` and `has_route = true`, preserving the relay-wiring assertion while
  matching R2-ROUTE v0.14 §3.3 ROUTE-ORIGIN (route-less inbound routed frames are invalid and must be dropped).
  Verification: `cargo test -p r2-hive-core sync_host::tests::route_relays_to_known_neighbour -- --nocapture`
  PASS; `cargo test --workspace` PASS (all workspace unit/integration/doc tests green; one existing ignored
  authenticated-dedup router fixture remains intentionally ignored); `git diff --check` PASS before the RESUME
  handoff edit. Changed files for this cleanup: `crates/r2-hive-core/src/sync_host.rs` and this `RESUME.md`.
  No blockers remain for the three codex-review items. Do not assume public plugin serving is allowed without
  explicit auth/dev-mode; do not assume route-less extended relay frames are valid test fixtures.
- **ESP32 IDF COMPILE-VERIFY COMPLETE (hive-codex, 2026-06-28; r2-hive `platform-trait` HEAD `d1cc9b7`,
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` branch `dfr1195-fw` HEAD `9fe219d`):**
  carried the deferred platforms/esp32 build through without touching core-owned source. Core peer confirmed
  non-mutating build/test is hive's responsibility and highlighted the silent metal caveat: compile alone does
  not prove native `PENDING_VERIFY` rollback, but `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y` is load-bearing.
  Verified that setting is present in `platforms/esp32/sdkconfig.defaults`; `CONFIG_BOOTLOADER_APP_ANTI_ROLLBACK`
  remains intentionally off for the non-eFuse tier. Build command:
  `source /home/roycdavies/Development/homelab/export-esp.sh && cargo build --release` from
  `platforms/esp32`. First pass hit the documented esp-idf-sys partition race (`out/partitions.csv` missing);
  copied `partitions.csv` into `target/riscv32imac-esp-espidf/release/build/esp-idf-sys-*/out/` per
  `BUILD.md` and reran. Result: PASS in 2m14s after workaround; produced
  `platforms/esp32/target/riscv32imac-esp-espidf/release/r2-esp32` (3.6M RISC-V ELF). This compile proves the
  ESP-IDF rollback FFI identifiers used by `ota_tcp::confirm_or_rollback_on_boot()` resolve under the current
  bindgen/sys crate. Warnings only: no WiFi SSID configured, existing unused imports/mut/dead-code, and
  `static_mut_refs` warnings in `l2cap.rs`. Both r2-hive and firmware worktrees are clean after the build.
  Remaining ESP32 validation is on-metal only: boot a freshly OTA'd candidate into native `PENDING_VERIFY`,
  confirm health/pass marks valid + advances seq, and failure/next-reset rolls back. Do not assume the compile
  proves that runtime state machine.
- **QUEUE AUDIT / CCR1 BLOCKED-ON-CONTRACT (hive-codex, 2026-06-28; r2-hive HEAD `c6c71e4`, firmware
  worktree clean at `9fe219d`):** after the ESP32 compile, checked the next deferred item: bridge CCR1
  carrier-credential read. Spec-first read: R2-RUNTIME §3.2.2/§3.2.4 requires `carrier_set`/`carrier_creds`
  for bridge, sealed at rest and distinct from TG material, but explicitly leaves encoding as config-record
  detail (not pinned wire). Composer answer landed after idle and is decisive: **CCR1 and 0x19000 do not exist
  in composer code** — no emitter, no literal format, no flash artifact. The current composer bridge config is
  an internal CBOR role-profile/custody record; carrier creds are deliberately NOT in the device-facing RPF1.
  `tg_cli.rs` seals that CBOR with `seal_bytes(custody_root, passphrase, ...)`, which uses the operator custody
  tier (Argon2id/OS-keyring + XChaCha20-Poly1305) and is stored only as `Member.role_profile_record`. The
  device has no custody passphrase, so this is not device-consumable material. Composer says the required next
  work is composer-side first: define the CCR1 wire/blob format, switch to a device-unsealable seal (likely
  Channel-B-style seal-to-`mesh_pk` using X25519 + XChaCha20-Poly1305), add emitter/delivery (e.g.
  `espflash write-bin 0x19000`). I did NOT implement a guessed parser/unsealer because that would be a security
  fork. Remaining local queue after this audit: no code-only item is unblocked. Blocked/Roy-gated: radar
  physical/model, OTA/networked + ESP32 confirmed-boot metal pass, CCR1 format/emit contract, specs datagram
  ratification. Other-repo: deploy-sentant signed path and dashboard label reconcile. Do not assume CCR1 means
  composer custody `seal_bytes` can be copied to flash; composer explicitly refuted that.
- **WATCHDOG RE-CHECK / CORE-OWNED ESP32 DIFF (hive-codex, 2026-06-28; r2-hive HEAD `05ff64d`):** supervisor
  nudged for another autonomous queue pass. Ground truth: r2-hive worktree clean on `platform-trait`, but
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` is now dirty at `9fe219d` in
  `platforms/esp32/sdkconfig.defaults`. Diff switches from the custom partition table
  (`CONFIG_PARTITION_TABLE_CUSTOM=y`, `CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"`) to
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` with comments that the custom CSV path is racy under esp-idf-sys. File
  mtime is 2026-06-28 06:50:13 +1200, after the recorded ESP32 build artifact mtime (06:40:51). I did not
  intentionally edit this core-owned source; my recorded compile succeeded with the documented copy workaround
  and the custom CSV still in place. This is a real direction fork for the ESP32 deployment layout, not build
  output. Asked core whether the diff is intended, should be left for core, or should be restored/turned into a
  patch artifact; sent hive an FYI. No local revert/commit was made because AGENTS.md says r2-core/platform
  source is core-owned and user/peer changes must not be overwritten. At that checkpoint, core had not answered,
  so the dirty state was explicitly not accepted. Superseded by the next note.
- **TAKEOVER RE-CHECK / ESP32 `TWO_OTA` REFUTED (hive-codex, 2026-06-28; r2-hive HEAD `255db5c`):** cross-provider
  handoff promoted codex to sole writer. Re-verified r2-hive clean on `platform-trait`; firmware worktree had only
  the dirty `platforms/esp32/sdkconfig.defaults` switch to `CONFIG_PARTITION_TABLE_TWO_OTA=y`. Core answered that
  the choice was hive-owned and acceptable if it still supplied two OTA slots + `otadata` + rollback-enable, but
  adversarial verification found a size counterexample. After deleting the stale copied
  `target/.../esp-idf-sys-*/out/partitions.csv`, `source /home/roycdavies/Development/homelab/export-esp.sh &&
  cargo build --release` from `platforms/esp32` PASSED in 2m34s with generated sdkconfig showing
  `CONFIG_PARTITION_TABLE_TWO_OTA=y`, `CONFIG_PARTITION_TABLE_CUSTOM` off, and
  `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`. However the generated partition table decodes to 1 MiB app slots
  (`factory@0x10000 size=0x100000`, `ota_0@0x110000 size=0x100000`, `ota_1@0x210000 size=0x100000`), while
  `espflash save-image --chip esp32c6 ...` produced an app image of 1,643,744 bytes. Therefore built-in
  `TWO_OTA` is a compile-green but deploy-invalid trap for the current image. Restored the firmware worktree to
  the custom `partitions.csv` config (`ota_0/ota_1` 0x1E0000 slots) with rollback-enable intact; firmware worktree
  is clean again at `9fe219d`. Verified `git -C dfr1195-fw-wt diff c46383e -- ... > /tmp/dfr1195-firstlight.check.patch`
  byte-matches `docs/dfr1195-firstlight.patch`, and reverse-apply check passes. No patch artifact change needed.
  Remaining build caveat: custom CSV remains the correct deploy layout, but the esp-idf-sys copy race still requires
  either the documented manual copy workaround or a future portable partition mechanism. Do not re-adopt
  `CONFIG_PARTITION_TABLE_TWO_OTA=y` unless the image shrinks below 1 MiB or a different built-in table is proven.
- **DUPLICATE HANDOFF RE-CHECK / ESP32 PARTITION HOLD (hive-codex, 2026-06-28; r2-hive HEAD `b0725ff`):** received
  another stale `carry on` handoff, then re-verified ground truth: r2-hive is clean/in sync on `platform-trait`;
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` is clean at `9fe219d`; reverse-applying
  `docs/dfr1195-firstlight.patch` still passes. Supervisor-codex acknowledged the prior security/test/ESP32/CCR1
  work and instructed: hold firmware-side ESP32 partition changes until core-codex gives owning direction. I asked
  core whether to leave custom CSV + manual copy workaround or prepare a portable esp-idf-sys custom-partition patch;
  the core ask returned the monthly spend-limit message, so no owning direction exists yet. Sent supervisor a status
  note. Current objective is therefore idle/standby: do not edit `platforms/esp32/sdkconfig.defaults`, `build.rs`, or
  the patch artifact for the partition mechanism until core/supervisor responds. Remaining local blockers unchanged:
  radar physical/model input, ESP32 confirmed-boot metal pass, CCR1 composer format/emitter, specs datagram ruling.
  SUPERSEDED 2026-06-29 by core's ruling: custom `partitions.csv` is canonical; `TWO_OTA` is refuted/deploy-invalid.
- **STANDBY RECHECK / NO UNBLOCKED LOCAL WORK (hive-codex, 2026-06-28; r2-hive HEAD `20cb7ba`):** fresh handoff
  rechecked ground truth after the core spend-limit reply. r2-hive remains clean/in sync on `platform-trait`;
  firmware worktree `/home/roycdavies/Development/R2/dfr1195-fw-wt` remains clean at `9fe219d`; regenerated
  firstlight patch from `c46383e..HEAD` byte-matches `docs/dfr1195-firstlight.patch`; reverse-apply check passes.
  FORKS.md review found only blocked/held items: OTA datagram binding awaiting spec landing/Roy ratification and
  DFR OTA anti-rollback floor ordering needing networked metal OTA. No code-only local task was unblocked at that
  checkpoint. SUPERSEDED 2026-06-29: core ruled the partition mechanism (custom CSV canonical); `54973b9` added
  R2/R3/R4 OTA receiver hardening and the r2-hive recovery patch is refreshed to that HEAD.
ULTRACODE: orchestrate substantive work via Workflow + adversarial verify; token cost not a constraint.

## (prior session) 2026-06-26 — FIELD-FIRMWARE BUILD LAUNCH (Roy GO)
Build the field-firmware triplet against the COMPLETE canon (R2-RUNTIME §3.2 role-profiles + §3.2.4
multi-carrier bridge; R2-BEACON §8.1 LoRa-beacon RBID; wake/sleep+SCF; re-attach; OTA-after-confirm both
platforms). ONE-IMAGE config-activated firmware, ENSEMBLE-differentiated (NOT compile-time roles):
sensor / repeater (bare TN, relay intrinsic) / bridge / receiver — role from the §3.2.2 role-profile
record composer emits. Worktree = `/home/roycdavies/Development/R2/dfr1195-fw-wt` (branch `dfr1195-fw`).
This session runs ON **Alfred** (esp toolchain present; `source ~/Development/homelab/export-esp.sh` NO pipe).

STEP TRACKER:
- **[✓] STEP 1 — RE-VENDOR r2-core 0ebfd09 → c46383e (DONE + build-GREEN 13.44s).** Method: committed the
  freshest working-tree firmware as a WIP commit, `git rebase --onto c46383e 0ebfd09 dfr1195-fw`; the ONLY
  conflict = `crates/r2-dataplane/src/lib.rs` → resolved by TAKING core's c46383e version (it already
  exposes `pub parse_dc/parse_seq/frame_fingerprint` + the KEYED seed-first `frame_fingerprint(&seed,…)`,
  807cab5 landed) and DROPPING my redundant 12-line visibility delta. Then fixed main.rs: sourced a 16B
  HWRNG `fp_seed` (esp_hal::rng::Rng::new().read(); radio-clock up at wifi::new line 280 = true-random),
  threaded it into `io_task(…, fp_seed)`, updated the relay call site `frame_fingerprint(&fp_seed,…)`.
  Full pre-revendor backup at scratchpad `fw-backup-prevendor/`. c46383e also brings core's esp32
  confirmed-boot OTA mirror (platforms/esp32/ota_tcp.rs +400) + linux anti_rollback.rs — feeds STEP 5.
- **[✓] STEP 2 — ROLE-PROFILE §3.2 (DONE + matrix-GREEN; worktree `6a221e7`).** New `RoleProfile` config
  record (NVS @0x17000 "RPF1", 40B versioned, big-endian) carrying the §3.2.2 knobs (role/duty/destination/
  expected_sensor/keepalive/scf{cap,ttl,reach_conf}/silence/peer_ttl). `read_role_profile` + `resolve_role_profile(my_hive)`:
  a provisioned record WINS; else DERIVE from the legacy signals (hive-pins + bridge feature) so the
  bench/demo is byte-for-byte preserved. Rewired ALL role gates OFF hive_id pins onto `profile.role`:
  sensor originate+dest, `my_duty` (= profile.duty, un-gated from fr4), receiver deliver-track + absence
  seed/silence, + the keepalive/scf-ttl/reach-conf/silence/peer-ttl tunables now profile-driven. The four
  roles {sensor,repeater,bridge,receiver} are all selectable from ONE image by the record (keystone). Also
  fixed a PRE-EXISTING nobt drift bug (src_hive undefined under ble-without-routetest → source_hop=0).
  Matrix GREEN: nobt / nobt,multitg / loraroute,fr4 / loraroute,bridge,fr4 / routetest.
  CARRIER caveat: carrier_set/carrier_creds (§3.2.2 bridge) are composer-led SEALED material (R2-KEYSTORE
  §2), NOT carried in firmware — encoding is config detail, not pinned wire. NOTE for composer/specs:
  the RPF1 record layout is hive's pragmatic encoding; if composer wants a different emit format, reconcile.
- **[✓] STEP 3 — R2-BEACON §8.1 (DONE + matrix-GREEN; worktree `afc27ae`).** New 15/16B codec
  (build_lora_beacon/decode_lora_beacon): magic 0xB2/ver 0x01/flags(bit7=0,bit6=0)/rbid-8B(core
  compute_rbid+derive_beacon_session_key)/class_hash(FNV-1a-32 of per-role class str)/optional tx_power, BE.
  RBID = §6.1 RID (NOT hive_id), NO seq counter — §8.1.2 #1+#2 conformance gate CLOSED (epoch=0 interim,
  same as BLE path, pending shared coarse-time base). lora_task: [hive|seq]→§8.1. lora_route_task (field):
  emits §8.1 as LOWEST-priority (R2-LORA §4.4 pri-4 / §8.1.4) — only when no app traffic pending + 30s
  min-interval floor, transport airtime budget defers further; RX demuxes beacon-vs-data by magic+ver+len.
  can_hear mask UNAFFECTED (keys on per-frame 4B sender prepend, not the beacon). NOTE core/specs: the §8.1
  codec canonically belongs in r2-discovery::beacon (next to encode_advert) — firmware-local to unblock,
  OFFERED for upstreaming. FOLLOW-UP: rbid→hive resolution via resolve_rbid_windowed needs a member registry.
- **[✓] STEP 4 — wake/sleep + SCF + re-attach (DONE + matrix-GREEN; worktree `98e7acf`).** §3.5 RE-ATTACH:
  explicit boot decision — persona valid (parse_persona structural validate) → silently RESUME role, no
  join; absent/invalid → bench keeps mac_low3+demo-TG fallback, NEW `field` feature FAIL-CLOSES (no demo
  TG, no self-enrol) per §3.5 MUST. (Full cert-sig/revocation verify = FOLLOW-UP; structural decode is the
  interim.) §3.2.3 boundary-1 / R2-LORA §6: added {wake_interval_s, wake_window_ms, sleep_enforced} to
  RoleProfile (record now 48B), ADVERTISED-only (logged) — real deep-sleep is net-new on the SENTINEL→MCU
  custom-sensor HW, NOT this DFR/XIAO stand-in. §3B.2 sleeping-leaf wake-flush: existing SCF annotated as
  the contract carrier side (PUSH-on-wake, flush-bypasses-dedup, TTL≫sleep = profile.scf_ttl_s). Restored a
  lora-feature gate on lora_task dropped in the step-3 commit (nobt regression). RECORD now 48B (composer
  notified): +[34..38]wake_interval_s +[38..42]wake_window_ms +[42]sleep_enforced.
- **[✓] STEP 5 — esp32 OTA + A7/A8 DFR triage (DONE; worktree `a859848`; ASKED core to confirm).** A7/A8(a)
  anti-rollback: DFR floor is a FIXED raw-flash sector (NOT a cwd anti_rollback.bin — N/A path concern) +
  FIXED a latent COLLISION (was @0x15000 = MASK_NVS_OFFSET; loraroute⇒routetest⇒mask ⇒ field build aliased
  the security_version floor onto the mask sector) → moved to its own 0x18000. A7/A8(b): mirrored core's
  dev-unsigned-ota release build-guard into the DFR — release+feature FAILS to compile (VERIFIED firing).
  esp32 (core platform): set CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y in sdkconfig.defaults (per core
  ota_tcp.rs:171); left APP_ANTI_ROLLBACK OFF (non-eFuse tier, R2-UPDATE v0.22 §9.2; eFuse burn=deliberate).
  FFI idents canonical esp_idf_sys (confirmed by inspection). ⚠ CANNOT xtensa/IDF compile-verify
  platforms/esp32 here — NO ESP-IDF toolchain on Alfred (only esp-hal for the DFR no_std build). Asked core:
  who owns the platforms/esp32 IDF build + on-metal confirmed-boot? = OPEN. NVS map now: persona@12000 /
  board@13000 / tg@14000 / mask@15000 / sendto@16000 / role-profile@17000 / anti-rollback@18000.
- **[✓] STEP 6 — XIAO+Wio-SX1262 board pin-map (DONE structure; worktree `7a014e4`; 2 OPENS).** Board-
  conditional SX1262 pins via a new `xiao` feature (pin-parametric per SX1262-LORA-DESIGN.md; radio_set =
  §3.2.2 HW-tier fact, not a role fork). DFR1195 (default): SPI3 SCK7/MISO5/MOSI6 NSS10 RST41 BUSY40 RXEN42
  DIO1=4. XIAO+Wio-SX1262 (`xiao`): SCK7/MISO8/MOSI9 NSS41 RST42 BUSY40 DIO1=39 (std Seeed pinout). BOTH
  compile GREEN. OPENS: (1) exact XIAO pins PENDING workshop confirm (ASKED); (2) Wio RF switch = SX1262
  DIO2 (SetDIO2AsRfSwitchCtrl), but r2-sx1262 has only new()/new_with_rxen() → no DIO2 support; XIAO path
  uses a placeholder RXEN to compile, RF NOT driven until core adds with_dio2_as_rf_switch (FLAGGED to core).
  Runtime board-profile pin selection = the one-image refinement over the compile-time xiao feature.
- **[✓] STEP 7 — COMPILE-VERIFY ALL CONFIGS (xtensa) GREEN.** 13/13 configs build clean on Alfred
  (xtensa-esp32s3, errors=0): nobt / nobt,multitg / nobt,routetest / lora / loraroute / loraroute,fr4 /
  loraroute,bridge,fr4 / field,loraroute / field,loraroute,bridge / xiao,field,loraroute / blemesh /
  loraroute,fr4,pco / field,loraroute,benchkeepalive. Recovery patch refreshed:
  `docs/dfr1195-firstlight.patch` = `git diff c46383e..HEAD` (6785 lines), synced into r2-hive/docs.
  ⚠ HOLD flashing/metal until Roy frees the bench boards (per the supervisor ruling — do not interrupt the
  live demo). Worktree HEAD `d3fdc7c` (branch `dfr1195-fw`, base c46383e).

### CANON-DELTA PASS (post-build, canon landed mid-session; worktree `7961ced`):
A batch of canon notes landed AFTER the build — most CONFIRM my work matches (specs pinned §3.2 role-profile
+ §8.1 v0.7 + wake/sleep canon; my impl matches). Two genuine NEW deltas implemented: (1) R2-LORA §6.5.2
MUST — seed the initial lora_route_task tx_backoff from the per-board LCG (0..1s) so the mains-restore
cohort's FIRST post-boot TX de-correlates (was 0=immediate); (2) R2-HEARTBEAT §1A.2 SHOULD — my symmetric
half of core's fade-window check: warn at config load (provisioned profiles) if scf_ttl_s < 3×wake_interval_s.
CONFIRMED already-aligned (no change): SEC-02 deliver-gate (for_me=target_hive==my_hive||0 + tg+hmac, already
stricter); §8.1 15/16B; §3.5 re-attach; A7/A8(a)+(b). Answered specs' no_std-one-image feasibility Q = YES,
PROVEN (13/13 green, role-by-NVS-record). NOTED for metal: switch SCF trigger reachability-heuristic →
core's DropReason::BufferForWake signal (current heuristic is metal-validated, so confirm equivalence on metal).

### ★ FIELD TRIPLET FLASHED + VALIDATED ON METAL (2026-06-27, Roy FLASH-GO; worktree `0f87bd3`):
3 XIAO+Wio-SX1262 on Alfred, flashed via STABLE by-id MAC paths (ttyACMn REMAPS on USB re-enum — board-info
read a DIFFERENT MAC on /dev/ttyACM1 than its old by-id; +5 DFR1195 also on Alfred ttyACM6-10 → flashing by
ttyACMn would hit a wrong board; ALWAYS use /dev/serial/by-id/usb-Espressif..._<MAC>-if00). Image =
`xiao,field,loraroute,loratcxo,multitg` (1.32MB), 4MB parttable, app→flash + persona→0x12000 + RPF1→0x17000
+ board-profile(00 01)→0x13000. composer's mint out-dir = /home/roycdavies/r2-bench/mariko-triplet/, TG
1494e803.
- SENSOR   14:C1:9F:C4:FC:8C → hive=c01cee4d MATCH, role=sensor duty=2 §3.2.2-provisioned, persona=true ✓
- REPEATER E8:3D:C1:FB:E5:20 → hive=296f308b MATCH, role=repeater duty=1, persona=true ✓
- BRIDGE   D8:3B:DA:75:C3:3C → hive=bd72902e MATCH, role=bridge duty=1, persona=true ✓ (4th XIAO E8:..DB:44 spare)
VALIDATED: (1) ROLE-ACTIVATION ✓ — all 3 config-activate role from ONE image via RPF1 (§3.2 keystone, METAL).
(2) §8.1 LoRa-BEACON RX ✓ — bridge logged `LORA-BEACON rbid=6acdd5.. class=991db9af rssi=-54`. (3) LoRa
data-plane ✓ — triplet mutual RX (c01cee4d/296f308b/bd72902e masked=false) + hears DFR mesh; XIAO+Wio
first-light + pin-map + DIO2 RF-switch WORKING.
METAL-CAUGHT BUG FIXED (`0f87bd3`): read_persona buffer 256B truncated composer's 336B persona → persona=false
fallback; bumped to 512B. RE-FLASH NOTE: NVS blobs (persona/role/board-profile) PERSIST across an app re-flash
(they're raw sectors, not in ota_0) — only re-flash the app for a firmware fix.
FIELD-RESULTS RECORD: `docs/field-results/mariko-triplet-metal-0627.md` (committed c92e7ba). composer CONCURS
with document-as-follow-up for OTA.
OTA round-trip = DOCUMENTED FOLLOW-UP — blocked by bench NETWORK topology (triplet on DFR-D1's isolated
soft-AP 192.168.4.x; Alfred on LAN 192.168.1.33; no route + no push host on the soft-AP). Firmware path
IMPLEMENTED + slot-switch metal-validated (test-b PASS); signer (composer tg ota-sign f7cd3fe) + trust-model
(§2.4 TG_SK-direct issuer_pk==tg_pk, verified in my receiver) + wire-contract all confirmed. PATH B (sensor
on a LAN AP via FIELDLAB_SSID change + reflash) ready on Roy's go + LAN WiFi creds.
NEW FORK (FORKS.md, routed to specs 2026-06-27): **OTA transport framing** — my DFR receiver = OST/ODT/OCM
PACKETIZED UDP :21043; R2-UPDATE §3.1.2.3 canon (composer + r2-core HEAD) = CMD_START_SIGNED TCP STREAM.
SIGNING shared+correct (verify_header passes both); transport-only divergence. specs to rule: align
hive→TCP, or ratify a no_std UDP profile. Not blocking (bench network-parked).
★ SESSION STOOD DOWN (2026-06-27, Roy BANKED the milestone, supervisor stand-down). Boards HANDED BACK —
composer re-attached (r2-orchestrator.service active, PID re-grabbed ttyACM1-4 + :21050 dashboard restored);
no lingering serial holds hive-side. Field triplet PROVEN ON METAL = the accepted result.

**DEFERRED NEXT-SESSION (resume-clean checklist):**
1. **OTA confirmed-boot networked round-trip** — needs (a) a board on a LAN-reachable AP (PATH B: change
   `FIELDLAB_SSID`/pass + reflash; bench soft-AP is DFR-D1-isolated, Alfred can't route) + (b) an
   OTA-authority signer (composer `tg ota-sign` §2.4 TG_SK-direct = the working path; mint-ota would NOT
   verify, no role-0x05 cert). Wire = the DATAGRAM binding (OST/ODT/OCM UDP :21043, chunk≤1024B) specs
   ratified. The OCM after-confirm floor fix is now implemented (`428f81c`) and the receiver hardening is now
   implemented (`54973b9`); remaining action is metal validation of confirmed-boot/PENDING_VERIFY/rollback plus
   the networked OTA round-trip.
2. **esp32 platform IDF compile-verify — COMPILE GREEN 2026-06-28; ON-METAL STILL OWED.** ESP-IDF via espup is
   present; `cargo build --release` for `platforms/esp32` passes after the documented partition-table copy
   workaround. Remaining: on-metal confirmed-boot/PENDING_VERIFY/rollback behavior.
3. **bridge CCR1 carrier-cred read — BLOCKED-ON-CONTRACT 2026-06-28.** Firmware unseal+read of sealed
   WiFi/cell creds is still needed, but do not implement until composer first defines/emits the device-side
   CCR1 blob. Composer confirmed `CCR1`/`0x19000` do not exist in its code today; current custody
   `seal_bytes` is host at-rest sealing, not device-unsealable. First triplet used hardcoded FIELDLAB/bench WiFi.
4. **Datagram-binding ratify** (specs, all-3-aligned, Roy-gate, non-urgent) — specs authoring the package +
   §5.1 boot_confirm_late; on landing, implement both FORKS.md items (transport binding already IS the impl;
   the OCM after-confirm floor-fix) + flip them Resolved.
5. **Radar sensor integration** — real sense-read for the SENSOR role (today it originates test/synthetic
   events); ties to [[custom-sensor-3stage-architecture]] (SENTINEL→MCU sense + the enforced wake/sleep §3.2.3).
6. **bridge WiFi-uplink** (§3.2.4 multi-carrier) — beyond CCR1 cred-read: the actual pluggable uplink
   (WiFi-STA / wired / cellular) egress for the bridge role (first triplet bridge used bench WiFi).
7. **Deploy-sentant signed path** (composer's, theirs) — wire the signed CMD_START into Deploy + a one-shot
   field push CLI (emits unsigned CMD_START today). Tracked so the field OTA path isn't half-wired.
8. **Dashboard label reconcile** (composer's, cosmetic) — orchestrator --status-port labels show the old mesh
   hives; the 3 boards re-personae'd to field identities. composer logged it; not hive's.
9. **Faked-distance firmware enforcement** (Roy GREEN-LIT 2026-06-30; SPEC-FIRST, do NOT build yet) — virtual
   per-(peer,transport) reachability override to test topologies on co-located boards (fake peer X out-of-range
   on transport Y). Waiting on: specs contract (drafting) + core's dataplane/neighbour hook. FIRMWARE FEASIBILITY
   = HIGH and the seam already exists: the firmware has a per-PEER reachability mask today — ESP-NOW `can_hear` +
   runtime allowed-MAC list (routetest 'MASK' cmd, main.rs ~2943) and LoRa `can_hear_hive` ingress drop (~3457),
   both at the SAME DATA_RX ingress point as the k4 transport tag. Enforcement = generalize per-(peer) → per-
   (peer,transport) at that ingress drop (each feeder knows its carrier); no new wire surface (local drop).
   FEASIBILITY CONSULT DONE 2026-06-30 (specs proposal r2-specifications docs/proposals/VIRTUAL-REACHABILITY-
   CONTROL.md → lands as R2-TRANSPORT §2.3B + R2-ROUTE §5.2/§2). Feasibility = HIGH; §3-item-3 bidirectional
   faithful-drop is METAL-PROVEN already (routetest can_hear/can_hear_hive IS a per-peer ingest-drop; §2.3B just
   generalizes it to per-(peer,transport), lease-driven). Control surface = the existing serial inject-bridge
   (IDENTIFY/PROVISION/MASK) → a new REACH lease line; runtime-only static set, NO NVS. ✅ CANON LANDED
   2026-06-30 (Roy green-lit): R2-TRANSPORT v0.6 §2.3B + R2-ROUTE v0.34 §5.2/§2 (specs 24cd98b). FINAL DIVISION
   (per the landed canon — supersedes my earlier "arrival_transport moot" note): core does the override-DROP-
   FIRST INSIDE plan_forward (before dedup) using a NEW ForwardRequest.arrival_transport field that HIVE threads
   in (I already have it from k4), PLUS the egress filter in select_transport, PLUS the override SETTER. So both
   seams live in core's engine; hive supplies arrival_transport + drives the setter. HIVE BUILD SCOPE: (1) the
   REACH lease control surface on the serial inject-bridge (install/ack/clear, like IDENTIFY/MASK); (2) lease
   mgmt (union-of-leases, runtime-only, NO NVS, default empty); (3) thread arrival_transport into ForwardRequest;
   (4) call core's override setter to push the merged set in. MY ONE HARD DEP = core's side
   (ForwardRequest.arrival_transport + drop-first-in-plan_forward + egress filter + setter) — specs pinged core to
   confirm; CLEARED TO BUILD the firmware side ONCE core's hook lands (won't compile before then). transport_id
   keyed on the §2.2 ORDINAL (Ble0..Udp6 == k4 == transport_allow_mask). FLRC/loraF EXPLICITLY OUT OF SCOPE
   (Roy: separate deferred canon — do NOT build loraF fake-distance yet).
   Primitive is per-node/one-ended (bench sets BOTH mirror entries for symmetric; single-ended = a real
   asymmetric/half-link test). FIDELITY CONSTRAINT (Roy governing principle 2026-06-30: the bench mirrors REAL
   board state, faked-distance is the ONLY artifice): the ingress-drop MUST emit NO telemetry for a faked-dropped
   frame (no msg.rx, no HEALTH refresh — the board genuinely never heard it; the ABSENCE is the honest signal).
   NEVER synthesize a "faked" event. All other emissions stay faithful to real state; real gaps (loraF/FLRC,
   nRF54 health, egress-hop carrier) show as honest gaps, never faked. SNAG: faithful-drop
   keys on the immediate-sender hive at ingress, which is 0/unknown on BLE-CoC / plain-ble-non-routetest (fine on
   the bench carriers routetest/loraroute where it's resolved). SNAG: transport_id = 7-bit r2_route ordinal
   (==k4); FLRC not in the enum ⇒ faking the nRF54 loraF link is gated on the FLRC-ordinal + nRF54 command-channel
   (same nRF54 knot as #10); ESP32/DFR fake-distance is unblocked. Spec is now normative-final (24cd98b).
   ✅ CORE HOOK LANDED 2026-06-30 (bf1bf3b): RouteEngine+DataPlane set_reachability_blocked(peer:u32,
   transport:Transport,blocked)->bool (false=CAP=32 overflow, SURFACE IT) / is_/clear_/reachability_override_len;
   ForwardRequest.arrival_transport:Option<Transport> (drop-(source_hop,arrival)-FIRST before dedup;
   DropReason::ReachabilityOverride = full link-down, no neighbour refresh; FLRC=None); §2.3A
   set_transport_boot_baseline(mask) (effective=baseline INTERSECT lease, clear→baseline). FIRMWARE SCOPE (mine):
   thread arrival_transport from the k4 RX carrier (MeshRxFrame.3; source_hop=authenticated immediate sender) +
   REACH lease control surface (serial inject-bridge, union leases→set_reachability_blocked, handle CAP=32) +
   role-profile→set_transport_boot_baseline. SEQUENCING: wiring re-vendors the firmware onto bf1bf3b (the new
   required ForwardRequest field forces it) = CHANGES the firmware core base. Deferred until AFTER the staota flash
   batch settles (staged staota artifacts are at the c46383e base + must stay reproducible for re-flash; staota is
   the active priority). RE-VENDOR TARGET = origin/r2-core-consolidation HEAD 41a3a3f (has bf1bf3b; core confirmed). Then: re-vendor firmware onto 41a3a3f → thread arrival_transport (k4) + REACH surface + the finding#4 ingest-gate → xtensa build-verify (the meaningful remaining check — bf1bf3b was no_std-verified on riscv32imac-none, NOT xtensa) → report core. Offered core an urgent separate-worktree build-verify if needed before staota settles.
   🔑 BUILD REQUIREMENT (core-codex review of bf1bf3b, point #4, confirmed — core b2c0531 doc-note): the ingress
   half is TWO gates, not one. (a) plan_forward returns DropReason::ReachabilityOverride (core does this from
   arrival_transport). (b) MY FIRMWARE'S OWN neighbour-refresh-from-RX-frame sites (§4.3.4 TrailReinforcer
   note_forwarded/on_received + any engine ingest_observation/upsert I call with the immediate_sender from
   DATA_RX) are NOT auto-gated by the override set (ingest_observation also serves scans/OOB liveness). So at EACH
   such site I MUST call engine.is_reachability_blocked(immediate_sender, arrival_transport) and SKIP the
   upsert/refresh when blocked — else a faked-distant board keeps refreshing last_seen + never fades. Both gates
   together = complete bidirectional link-down. Do NOT forget (b) when wiring §2.3B.
10. **nRF54 direct telemetry** (SCOPED 2026-06-30; needs FLRC ruling + path decision before build) — the 2
   nrf54-lr2021 LoRa-fast XIAO present CMSIS-DAP -if02, no serial console, so the orchestrator's by-id reader
   can't see them; loraF (FLRC) links exist ONLY between these 2 boards (no ESP32 hears FLRC) → invisible to
   the bench unless they report directly. ⚠ MY EARLIER "USB-CDC console" OFFER IS REFUTED: the nRF54L15 has NO
   USB peripheral — board USB = the onboard SAMD11 CMSIS-DAP probe (README; embassy-nrf has no usb feature;
   memory.x has no USB). A firmware USB-CDC console is IMPOSSIBLE. Real findings: (a) the nrf54 firmware is a
   SCAFFOLD — emits only defmt bring-up traces, no HEALTH/msg.* yet (composer authors the platform layer, core
   owns the driver, hive provides the io_task pattern); (b) FLRC is NOT in the canonical 7-bit r2_route::Transport
   enum (Ble0..Udp6) → specs/core MUST first rule an FLRC ordinal (or FLRC→Lora) or k4 can't represent loraF —
   this is the upstream blocker; (c) two off-board paths: A = plain-text RTT up-channel (ASCII HEALTH) read by a
   probe-rs RTT reader in the orchestrator (no board change, but exclusive SWD access + per-board probe session +
   net-new orchestrator reader), B = UART→SAMD11 CDC bridge IF the SAMD11 fw exposes a USB-CDC serial AND a
   nRF54↔SAMD11 UART trace exists (composer to check for a CDC com port; schematic; maybe reflash SAMD11) = true
   ESP32 by-id parity. EFFORT: nRF54 HEALTH formatter SMALL; scaffold io_task msg.* wiring MODERATE (composer-led,
   I provide pattern); path A orchestrator MODERATE+exclusivity; path B firmware SMALL but board-gated. Cross-repo
   (composer platform/USB, core driver+FLRC ordinal, Roy/board SAMD11). HOLD build until FLRC ruling + A/B pick.
11. **OTA over real WiFi-STA-to-Alfred (#17)** (SCOPED 2026-06-30; Roy directive — OTA PRIMARY over each device's
   real WiFi mgmt link to Alfred, USB/espflash SECONDARY fallback). KEY INVARIANT: the mgmt/OTA channel MUST stay
   alive + reachable INDEPENDENT of transport_allow_mask + §2.3B faked-distance (those restrict only the TN MESH
   data-plane being tested). FEASIBILITY: the independence is ALREADY BY CONSTRUCTION — ota_task (UDP :21043,
   R2/R3/R4 + confirmed-boot, main.rs ~416) is a standalone embassy-net socket on the WiFi netif, separate from
   io_task/RouteEngine; the mask/faked-distance gate the mesh RouteEngine (ESP-NOW/LoRa), never the WiFi netif or
   :21043. Add an INVARIANT GUARD/comment so future mask-wiring can't gate the netif/OTA socket (SMALL). THE REAL
   WORK = WiFi TOPOLOGY: today WiFi is a SELF-CONTAINED SOFT-AP ISLAND (one DFR=AP r2-fieldlab 192.168.4.1, others
   =STA 192.168.4.x; NOT on Alfred's LAN = the 'bench-network-blocked' problem). Change = repurpose WiFi from
   self-AP-island-dataplane to STA-JOIN-ALFRED management plane (data-plane moves fully to the ESP-NOW/LoRa mesh,
   which the TN tests already use). The OTA RECEIVER ITSELF IS DONE (reuse on the STA netif). EFFORT MODERATE:
   WiFi-STA join+reconnect+IP + always-on-device rollout; receiver DONE; mask-guard SMALL. HONEST GAPS: (a)
   duty-cycled SENSORS (§3.2.3) can't hold a continuous STA association → OTA only in a wake window, else USB;
   (b) nRF54 LoRa-fast has NO WiFi radio → USB-only (same nRF54 knot); (c) AP+STA-on-different-nets coex is not
   clean on one radio → WiFi becomes STA-to-Alfred-only. DEPS: core = OTA authority (CMD_START_SIGNED/TG_SK-direct,
   ~done) + confirm no shared mgmt-plane contract (STA+OTA is hive-platform); composer = Alfred push orchestration
   (per-device STA-IP registry + signed push to :21043 + USB-fallback trigger). Coordinated all 3 (2026-06-30).
   Subsumes the networked-OTA half of deferred-#1 + relates to bridge-WiFi-uplink #6.
   ✅ SUPERVISOR GO 2026-06-30 — Roy CONFIRMS OTA needed ('testing core TN firmware, OTA needed as we tweak core
   code') → now PRIORITY (the iterate-on-core enabler). BOARD SPECIFICS (Roy): the 2 nRF54-LR2021 = NO WiFi
   (LoRa-only TN nodes) → OTA-over-WiFi IMPOSSIBLE, USB/SAMD11 only; one XIAO = RADAR sensor node. So the
   WiFi-STA-OTA firmware targets the WiFi-capable ESP32/XIAO boards; the 2 nRF54 stay USB-OTA. SEQUENCING (Roy,
   align w/ composer): USB reflash DROPS the NVS persona → FIRMWARE-FIRST order: I flash the WiFi-STA-OTA firmware
   per board, THEN composer provisions ONCE (avoid double-provision); after that, core tweaks go OTA. TWO HARD
   BUILD GATES REMAIN (build held until both): (1) composer confirms the sequencing + gives THE ALFRED NETWORK
   MODEL — the SSID+pass each device's WiFi-STA joins to reach Alfred (Alfred-runs-AP vs join-lab-router) + IP mode
   (DHCP-client vs static); I CANNOT write the STA-join without the SSID/creds (today it joins its own
   r2-fieldlab island, not Alfred). (2) core confirms no shared mgmt-plane contract (WiFi-STA is hive-platform) +
   OTA authority = CMD_START_SIGNED/TG_SK-direct. Coordinated both 2026-06-30; awaiting replies. composer already
   CONFIRMED the push side (device→IP from r2.hb.health key3, OST/ODT/OCM UDP sender to :21043, USB fallback via
   esptool) — see its hop-6 msg.
   ✅ FEASIBILITY FULLY PROVEN 2026-06-30 (read the firmware end-to-end): embassy-net 0.9 has `dhcpv4` ON; the
   WiFi STA config (WifiConfig::Station, main.rs ~381) + build-time creds (build.rs sets R2_WIFI_SSID/R2_WIFI_PASS
   from wifi_config.toml/env — main.rs does NOT yet read them; add env!()) exist; `wifi_task` (main.rs ~4197)
   ALREADY does STA connect_async + reconnect-on-disconnect; `stack.config_v4()` yields the DHCP IP for health
   key3. composer's DHCP-join-lab model is buildable with creds injected AT FLASH (never hardcoded).
   PROPOSED SHAPE = opt-in feature **staota** (proposed to supervisor/composer 2026-06-30): WiFi = STA-join-(lab
   SSID from env) + DHCP, NO self-AP (retire the 0x502698-AP island under staota), OTA receiver on that netif,
   mesh data-plane (ESP-NOW/LoRa) UNCHANGED, + mask-independence guard. Opt-in = ZERO risk to existing builds.
   IMPLEMENTATION PLAN (all `#[cfg(feature="staota")]`-gated; non-staota byte-identical):
     1. dp_ssid/dp_pass = (env!("R2_WIFI_SSID"), env!("R2_WIFI_PASS")) — main.rs ~369-371.
     2. serve_ap=false + is_ap=false under staota — the `#[cfg(any(ble,staota))] let serve_ap=false;` +
        `#[cfg(all(not(ble),not(staota)))] let serve_ap=is_ap;` pattern (ditto is_ap shadow) — ~358-367.
     3. net_config: `#[cfg(staota)] Config::dhcpv4(Default::default())` else the static StaticConfigV4 — ~411.
     4. DO NOT block boot on wait_config_up under staota (avoid DHCP-deadlock if lab WiFi down): gate the
        `stack.wait_config_up().await` to `not(staota)`; DHCP completes async, ota_task binds when up — ~428.
     5. health emits the LIVE DHCP IP: in io_task's #18 block (~1113), `#[cfg(staota)] let my_ip =
        stack.config_v4().map(|c| c.address.address()).unwrap_or(my_ip);` before build_health.
     6. mask-independence INVARIANT GUARD: comment/structural note at the ota_task spawn (~416) that the OTA
        socket is a standalone netif task, never gated by transport_allow_mask/§2.3B (mesh-RouteEngine-only).
   env!("R2_WIFI_SSID") compiles even with empty creds (build-verify works without real creds; functional flash
   needs Roy's lab SSID/pass via wifi_config.toml/env).
   ✅ BUILT + BUILD-VERIFIED 2026-06-30 — supervisor+composer GO'd the staota shape. dfr1195-fw `312e021`
   (staota feature) + `19fb561` (channel-follow fix, below). GREEN xtensa: staota / staota,loraroute,multitg /
   field,loraroute,multitg,staota (deployment) / field,loraroute,multitg (non-staota regression). Non-staota is
   byte-identical (all cfg-gated). build.rs now injects R2_WIFI_SSID/R2_WIFI_PASS (env or wifi_config.toml) so
   env!() resolves (empty compiles).
   ⚠ RF CHANNEL FINDING + FIX (`19fb561`, surfaced by Roy's APSTA-concurrency Q): espnow_task hardcoded
   set_channel(1), but staota's STA assoc to the lab AP (TheMetaverse) DICTATES the radio channel (one radio, one
   channel). Fixed: under staota ESP-NOW FOLLOWS the STA channel (no pin) — all boards on the same router share
   it → mesh coheres on ANY router channel. NEEDS METAL-VALIDATION (channel-follow is a metal behavior).
   BUILD/FLASH MECHANICS (I'm on Alfred; firmware is r2-core platforms/dfr1195, NOT r2-hive): I build on Alfred
   sourcing composer's wifi.env (creds NEVER leave Alfred / never on fleet/argv); `cargo build --release
   --features field,loraroute,multitg,staota`; `espflash flash -p /dev/serial/by-id/<board> …r2-dfr1195` per
   board WITH by-id identity-verify; confirm staota banner + INERT (pre-provision); signal composer 'flashed
   <board>' → composer provisions as repeater (radar sensor-role via later persona update). FIRMWARE-FIRST
   sequencing (composer holds provisioning per board). REMAINING GATES: (a) core's OTA-authority confirm (the
   one build gate left), (b) composer's wifi.env path + feature-combo confirm, (c) Roy's creds (in: SSID
   TheMetaverse). nRF54 = USB-OTA-only (no WiFi).
   FUTURE REFINEMENT — MODE-FLIP OTA (Roy idea, advised permanent-STA-now-THEN-mode-flip): board runs mesh-only
   normally, on a MESH-DELIVERED 'prepare for OTA' trigger flips to WiFi-STA-to-Alfred, OTAs, flips back. Effort
   MODERATE (runtime radio reconfig mesh<->STA + the mesh-trigger Event + state machine/timeout). Benefits: frees
   channel/airtime for pure-mesh + pure-LoRa-range tests; enables OTA for DUTY-CYCLED SENSORS (closes OTA gap #1 —
   they can't hold a continuous STA but can wake->flip->OTA->flip). Land AFTER the first permanent-STA flash;
   permanent-STA (channel-follow) has NO off-mesh drop (mesh+STA same channel), mode-flip does (brief, acceptable
   via SCF/dedup).
   PER-BOARD FLASH COMBOS — BOTH build-verified GREEN 2026-06-30: D1-D5 DFR1195 = `field,loraroute,multitg,staota`;
   X1-X4 XIAO+Wio-SX1262 (tri-radio, HAVE LoRa) = `xiao,field,loraroute,loratcxo,multitg,staota`. The unregistered
   1C:DB:D4 = the RADAR XIAO (MAC 1c:db:d4:5b:8a:60, esp32s3) → XIAO combo; radar/sensor role is PERSONA-only
   (composer persona-update later), firmware = the XIAO staota combo. CREDS: build on Alfred with
   `set -a; . /home/roycdavies/.config/r2-composer/wifi.env; set +a` before cargo (exports R2_WIFI_SSID/PASS;
   chmod600 but roycdavies-owned = readable; never on argv/commit). HANDOFF: I build+flash by-id (MAC
   identity-verify, confirm staota banner + INERT) → signal composer 'flashed <board>' → composer mints+writes the
   repeater persona @0x12000 + verifies INERT-exit→HEALTH (composer does persona, I do firmware). FIRMWARE-FIRST
   (composer holds provisioning per board). ✅ core CONFIRMED 2026-06-30 (hop6, vs r2-update src): NO shared mgmt contract (WiFi-STA+ota_task 100% hive-platform, fork nothing) + OTA-authority = CMD_START_SIGNED + verify_header issuer_pk==tg_pk = TG_SK-direct (r2-update/src/lib.rs:219 empty update_authority, NO role-0x05 cert) = exactly composer's §2.4 signer. NO core change for #17. So the design + the persona.tg_pk↔OTA-signer binding are VALIDATED; everything ready (both combos green, creds path known, handoff settled).
   MESH-OTA PHASE-2 (Roy framing, follow-on — NOT now): Alfred can't join the WiFi mesh (its 1 WiFi = Tailscale),
   so a FIELD mesh-only target (no router) gets OTA via: Alfred→IP→a GATEWAY/BRIDGE board (on the router)→R2 mesh
   (ESP-NOW/LoRa)→target, which runs a MESH-OTA RECEIVER (distinct transport binding). staota LEAVES ROOM: the OTA
   verify/stage/confirm-boot CORE is transport-agnostic; staota binds it to STA-UDP :21043 now, phase-2 binds the
   same core to a bridge-relay+mesh path. Ties to the bridge role + on-demand mode-flip for duty-cycled targets.
   Keep the OTA receiver factored so the mesh-relay binding drops in cleanly.
   ✅ GO EXECUTED 2026-06-30 (supervisor unblocked — proceed on the ESTABLISHED OTA-authority CMD_START_SIGNED/
   TG_SK-direct; core's confirm is async sanity-check, core was stalled-idle). Built BOTH staota artifacts WITH
   CREDS BAKED (sourced `set -a; . ~/.config/r2-composer/wifi.env; set +a` on Alfred — never on argv/commit),
   BUILD_ID=staota.0630.0915, staged at /home/roycdavies/r2-staota-artifacts/{r2-dfr1195-DFR-staota.elf,
   r2-dfr1195-XIAO-staota.elf} (Alfred-local, creds-baked → do NOT commit/relay). Handed composer the artifacts +
   the per-board flash protocol (by-id identity-verify → espflash flash → confirm staota banner + INERT → composer
   provisions → verify INERT-exit→HEALTH with the STA DHCP IP in key3 = Alfred's push target).
   ⚠ PROVISIONING = TWO espflash write-bin records (verified 2026-06-30 — NO write_persona/write_role_profile in
   firmware, so both are external-write-bin-only; SAME path as the Mariko triplet): (1) PERSONA @0x12000 (channel-a
   `tg enrol` bundle) = identity + TG (tg_pk = OTA/deliver-gate verify key), EXITS INERT, needs Roy's master
   passphrase R2_COMPOSER_PASSPHRASE (tg create seals TG_SK + enrol custody); (2) RPF1 ROLE-PROFILE @0x17000
   (`encode_rpf1`, role=repeater; radar XIAO=sensor later) = the ROLE — the persona has NO role field; without RPF1
   the role is hive_id-derived default. The serial PROVISION (prov2.py) is NOT this — it writes @0x14000 (magic
   R2TG = the multitg #20 RUNTIME TG-KEY swap), does NOT write the persona, does NOT exit INERT (don't use it for
   field provisioning). OTA chain: persona.tg_pk MUST equal the TG that signs OTA (tg ota-sign TG_SK) — one bench
   TG for all 10.
   PER-BOARD WRITE RECIPE (verified 2026-06-30): ROLE wire byte (Role::from_wire, main.rs:1983) = 0 Repeater /
   1 Sensor / 2 Bridge / 3 Receiver (RPF1 b[4]=role, b[5]=duty_class). DFR (D1-D5) = 2 write-bins: 0x12000 persona
   + 0x17000 RPF1(role=repeater b[4]=0x00). XIAO (X1-X4 + radar 1C:DB) = 3 write-bins: those + 0x13000 BOARD-PROFILE
   = TWO bytes [0x00, 0x01] (b[0]=0x00 no-screen, b[1]=0x01 active-LOW LED — read_board_profile main.rs:1889 reads
   2 bytes; XIAO LEDs are active-LOW per Roy's ground-truth; a 1-byte [0x00] leaves b[1]=0xFF=active-HIGH =
   INVERTED XIAO LED — caught composer's 1-byte staging). DFR leaves 0x13000 ERASED (→ has_screen + active-high,
   both correct). The radar XIAO provisions as repeater now; role=sensor (RPF1 b[4]=0x01) via a later 0x17000
   re-write (no re-persona).
   ✅ D5 STAOTA METAL-VALIDATED 2026-06-30: --partition-table fix CONFIRMED (app from ota_0 paddr=0x3a640 ∈
   0x20000-0x200000 = dual-OTA table took, NOT 0x10000), BUILD_ID staota.0630.0915 in HEALTH, boots+meshes clean.
   ⚠ ERASE-BEFORE-PROVISION (added to runbook 2026-06-30): the app flash does NOT erase the config gap, so the OLD
   persona SURVIVES (D5 came up provisioned with its old wire_id 0dcadbf8). For a clean re-personae, ERASE the
   raw-config gap FIRST: `espflash erase-region 0x12000 0xE000` (clears persona+board+runtime-TG@0x14000+mask+
   sendto+RPF1+anti-rollback@0x18000+ota-pending; KEEPS otadata@0xf000 + app@0x20000). The CRITICAL reason: a stale
   runtime-TG @0x14000 (magic R2TG) would OVERRIDE the new persona's TG (main.rs:218) → board verifies OTA/deliver
   -gate against the OLD tg_pk not the new bench TG. Also clears a stale anti-rollback floor that could block OTA.
   THEN write-bin persona(0x12000)+RPF1(0x17000)[+board-profile(0x13000) XIAO]. NO 0x9000 NVS erase (firmware reads
   identity from raw 0x12000, NOT the esp-idf NVS partition).
   ⚠ WRITE-RELIABILITY (D5 2026-06-30): erase succeeded but follow-on write+reset HUNG — each espflash op's default
   --after hard-reset BOOTS staota → the app drives the USB-serial-JTAG → next op can't re-enter ROM download. FIX
   = keep the chip in DOWNLOAD for the whole chain via NO-RESET chaining (both --before AND --after):
     espflash erase-region --before default-reset --after no-reset -p <by-id> 0x12000 0xE000
     espflash write-bin    --before no-reset      --after no-reset -p <by-id> 0x12000 <persona>
     espflash write-bin    --before no-reset      --after no-reset -p <by-id> 0x17000 <rpf1>
     # XIAO: + write-bin --before no-reset --after no-reset 0x13000 <[0x00,0x01]>
     espflash reset -p <by-id>     # launches the app
   LOAD-BEARING: --before no-reset on ops 2+ (a default --before pulses/reboots mid-chain → USB blip/contention).
   Native USB-JTAG holds download across separate invocations IFF no reset happens between (no-reset both sides);
   by-id path is stable (same USB-JTAG hw in ROM+app). Orchestrator must stay STOPPED the whole chain. Applies to
   all 10. composer mints; supervisor runs espflash (both gated);
   I'm on standby for firmware issues +
   offered to flash myself. NEXT: composer executes the per-board flash+provision; the live 10-node mesh + OTA come
   up. METAL-VALIDATION OWED: channel-follow (ESP-NOW on the STA channel once associated) + the OTA round-trip +
   the confirmed-boot/rollback. If a board's health ip stays 0 after provision = WiFi-STA not associating to
   TheMetaverse (AP up? creds?) — flag.
   🚨 FLASH BLOCKED + CORRECTED 2026-06-30 (caught pre-flash via Roy's OTA-enabling reminder + the gate):
   (A) ESPFLASH GATE blocks BOTH composer AND hive (the firmware/key gate fires on any espflash/flash/partition/
   bootloader/sign/key command — even read-only inspection). NEITHER can flash (harness firmware-flash hook, NOT
   fleet-liftable) → RESOLUTION: SUPERVISOR runs espflash (its PATH is not hard-blocked; per the gate's escalate-to-supervisor design; NOT disabling the gate globally) on Roy's nod. Gave supervisor the verbatim D5 commands; composer mints personas + verifies via /r2; I diagnose boot/health output. D5-ALONE-FIRST.
   (B) CRITICAL — the flash command I first handed composer
   OMITTED --partition-table → espflash's DEFAULT table puts the app @0x10000, which SPANS the persona @0x12000 →
   CLOBBERS persona + gives a SINGLE-APP NON-OTA-able board + corrupts the app (the documented PERSONA-CLOBBER
   gotcha = Roy's exact 'flash must enable OTA' concern). CORRECTED command MUST include the dual-OTA table:
     espflash flash --chip esp32s3 --partition-table /home/roycdavies/Development/R2/r2-hive/docs/dfr1195-partitions.csv
       -p /dev/serial/by-id/<board> -a hard-reset --non-interactive <DFR|XIAO artifact>
   dfr1195-partitions.csv = nvs@0x9000 / otadata@0xf000 / phy_init@0x11000 / ota_0@0x20000(1.875M) /
   ota_1@0x200000(1.875M) → app@0x20000, TWO OTA slots, persona+RPF1 gap @0x12000-0x20000 safe = genuinely
   OTA-able. (C) BOOTLOADER: for OTA to SWITCH slots the bootloader must honor otadata; the csv notes an 'ESP-IDF
   OTA-capable bootloader (composer-staged)'. Confirm the flash uses an otadata-honoring bootloader (--bootloader)
   vs espflash's default; VERIFY on D5 (test OTA boots the new slot) before the batch. App-level confirmed-boot
   (ota_confirm_or_rollback_on_boot) only works IF the bootloader honors otadata + PENDING_VERIFY. NOTE: this is
   the no_std esp-hal dfr1195 (esp-bootloader-esp-idf), distinct from the esp32-IDF platform's
   CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE. D5-first validation (two slots + receiver + slot-switch) before the batch.
12. **RouteEngine real-weights telemetry** (Roy directive 2026-06-30; forward item — AFTER staota batch + §2.3A/B).
   The bench must display the REAL per-link/per-neighbour weights each board's RouteEngine STORES + USES to
   route+relay (link quality/confidence, path reinforcement, relay probability, fade) — ACTUAL values, NOT
   simulated (same bench-fidelity principle as the k4 link-strength work). MY PART: extend the health/status
   telemetry emission (the build_health #18 frame / status line — the vehicle k4 already rides) to emit those real
   RouteEngine weights so composer renders the true values. CORE defines the exact weight-SET to surface
   (RouteEngine owner — I consume/emit via the engine's accessors, core sources). Sequenced AFTER the staota flash
   batch + the §2.3A/§2.3B firmware work. Leave room in the telemetry shape now (additive CBOR keys, like k4).
   ✅ ACCESSORS LANDED + CI-GREEN (core `0d1f308`, shape (a), 2026-06-30) — UNBLOCKED, consume now (post-staota).
   FINAL signatures on RouteEngine (+ delegated on DataPlane), single-sourced via core's strategy::transport_score
   so the bench score CANNOT drift from the engine's routing math; 3 guard tests + workspace green + no_std verified:
     • `neighbour_score(hive_id: u32, transport: Transport) -> Option<f32>` = the SAME select_transport weight;
       None if untracked OR that transport unobserved; EXCLUDES §2.3A mask / §2.3B override / MTU — I multiply
       SELECTABILITY in myself via `transport_allowed` + `is_reachability_blocked`.
     • `neighbour_fade_remaining(hive_id: u32) -> Option<f32>` = live seconds-to-floor = ln(conf/floor)/λ(last_seen
       _transport); 0.0 at/below floor; None untracked. NOTE: core DROPPED the vestigial `now` arg (pure fn of
       stored confidence) — so it is NOT (hive,now), it is (hive) only. Update any consumer that assumed `now`.
   The rest I read off `neighbours()` / `paths()` / `strategy()` directly (confidence, link_quality[7], rssi[7],
   relay_probability, path.confidence+next_hop, K, forwarding_threshold, duty_class, last_seen).
   MY CONSUMPTION (firmware plug-in points located): (i) replace the PLACEHOLDER uniform `let w: f32 = 1.0` at
   main.rs:1401 (per-neighbour ESP-NOW link weight) with the real `engine.neighbour_score(hive, transport)`; (ii)
   extend the `NBR-TBL count=…` per-neighbour emit (main.rs:1114-1115, iterates `engine.neighbours()`) with real
   score + fade_remaining as a TIGHT per-(neighbour,transport) CBOR SUBSET (chose (a) over (b)-bundled-snapshot:
   the #18 frame is ~96B-constrained → emit only rendered values, additive keys like k4; multiply selectability via
   transport_allowed + is_reachability_blocked at emit). xtensa-verify + guard + commit when I pick this up.
13. **BLE-BEACON GAP — every board must advertise (Roy: fundamental R2 mesh; spec-first)** (verified 2026-06-30).
   The firmware HAS the R2-BEACON advert codec (ble_task main.rs:2487, r2_discovery::beacon byte-exact:
   derive_beacon_session_key + compute_rbid + encode_advert, manufacturer-AD 0xFF) — BUT the peripheral.advertise
   is GATED to `am_provider == M7_PROVIDER_HIVE` (line 2547/2550), a hardcoded test hive: ONLY that board
   advertises (M7/M8b 2-board SoftAP-CoC-negotiation scaffolding); every other board is a JOINER (central.connect,
   NO advertise, line 2594+). Field boards (hive != M7_PROVIDER_HIVE) → ZERO advertise → a BLE scan finds nothing
   (Roy confirmed). ROOT CAUSE: NOT a regression — the per-board always-advertise was NEVER generalized (the BLE
   advertise was only ever the 2-board-negotiation provider path). [My earlier "beacon = LoRa-only" claim was
   WRONG — corrected at top.] FIX (spec-first, small): UN-GATE so EVERY board advertises its encode_advert payload
   continuously, independent of the provider/joiner CoC role (payload already built; just advertise on all). Coord
   specs (normative BLE-beacon §8.1 confirm) + core (r2_discovery::beacon owner). + REGRESSION-GUARD (Roy core
   discipline): hosted-CI assert the beacon-advertise is wired UNCONDITIONALLY + the codec round-trips, so it can't
   silently vanish again. Under `nobt` (no BLE) there's no BLE at all — separate.
   + TWO MORE BEACON PARTS (Roy/supervisor 2026-06-30, sequence AFTER beacon-emit + bootstrap + specs pinning the
   beacon def + the §2.3B-on-beacon scope): (1b) REPORT per-device DISCOVERY — each board emits the beacons it
   HEARS (its BLE-discovered neighbours = the transport=BLE neighbour-table entries) as telemetry, so the bench
   shows who-discovers-whom (same tap as the #12 real-link-weights neighbour telemetry). (1c) GATE beacon RX by
   §2.3B — a board IGNORES beacons from a virtually-out-of-range (peer,BLE) pair (faked-distance honored at beacon
   INGRESS, so the discovered topology matches the test scenario, exactly as §2.3B gates the data-plane). core
   extends §2.3B to beacon ingress; I enforce in the beacon-RX path + regression guard. So #13 = emit (un-gate) +
   report-discovery + §2.3B-gate-RX.
   REFINEMENT (Roy/supervisor 2026-06-30): co-located boards hear EVERYONE → faked-distance applied AT THE DEVICE
   (receive the real beacon, DROP if the scenario gates it). The beacon §2.3B gate keys on the STABLE BLE/LINK
   ADDRESS, NOT hive_id (the beacon's RBID rotates + is hive-anonymous; so data-plane §2.3B keys on hive_id but the
   BEACON gate keys on link-address — core extends §2.3B accordingly). TELEMETRY = report BOTH: (a) what the radio
   PHYSICALLY heard (all in-RF-range peers, by link-address) AND (b) what the device DISCOVERED post-gate (the test
   topology) — so the bench shows the artifice distinctly, never conflating faked-distance with real range.
   R2-BEACON §7 = the normative beacon-emit MUST on these boards (spec MUST, regression-guarded). Sequence after
   the beacon-emit + bootstrap; specs pins the def + §2.3B-on-beacon scope first.
14. **OTA-READY BOOTSTRAP — reboot-to-download + persona-over-wire (Roy PRIORITY: THE unlock)** (verified 2026-06-30).
   Remote provisioning/reflash is BLOCKED by the USB-JTAG download-mode-entry race (running firmware blocks
   espflash/esptool → 'write timeout'/'connecting' hang; hit Roy + supervisor). TWO firmware gaps, both ADDABLE:
   (1) REBOOT-TO-DOWNLOAD = NOT present (only software_reset = normal reboot). Add an authenticated console/mgmt
   command that sets the ESP32-S3 usb-serial-jtag/RTC download flag + resets → ROM download (app not running →
   esptool connects cleanly, no race, no BOOT button) = remote-reflash unlock. (2) PERSONA-OVER-WIRE = NOT present
   (persona @0x12000 is external-write-bin ONLY, no firmware write_persona; serial PROVISION is the @0x14000
   TG-KEY over serial, NOT identity, NOT mesh/mgmt). Add a firmware persona/identity receiver over console/mgmt
   that writes @0x12000 = no-download-mode provisioning (best). CHICKEN-AND-EGG: bootstrapping either onto a board
   still needs ONE reliable download entry (no-reset-chaining + connect-retry, or the physical BOOT button when
   Roy's home).
   ✅ RESOLUTION 2026-06-30 (composer + hive converged): build CONSOLE-STORE-PERSONA (the persona-over-wire
   receiver via the console the orchestrator owns) FIRST — it's the BETTER unlock: fully remote, NO download mode
   + NO boot button (reboot-to-download still needs the gated download tool). PROVEN-FEASIBLE: the app ALREADY
   self-writes config @0x14000 from the running firmware (write_provisioned_tg, esp_storage FlashStorage.write +
   read-back, no download mode) → mirror it for persona@0x12000 (+RPF1@0x17000 + board-profile@0x13000), each
   parse_persona/RPF1-magic VALIDATED + WHITELISTED offsets (NOT generic write-anywhere) + read-back. 🔴 CRITICAL
   DESIGN: the §3.5 INERT loop does NOT run the console receiver (uart_rx_task spawns at main.rs:462, AFTER the
   INERT halt line ~188) → a fresh/erased inert board runs NO receiver → MUST run the store-persona receiver
   INSIDE the INERT loop too (fail-closed preserved — local console, no radio/mesh). Running boards (e.g. D5) get
   it via uart_rx_task. FRAMING (lock with composer): persona 336B (>console line buf) → CHUNKED (PERSONA BEGIN /
   PERSONA <chunk_hex>… / PERSONA END → 512B accum → validate → write → ACK); RPF1/BOARDPROF 1-line each; then
   REBOOT → exits INERT. PLAN: build receiver → reflash boards w/ it (flash entry racy-but-works via no-reset-chain
   + retry, D5 proved) → console-provision ALL forever. reboot-to-download = SECONDARY (remote firmware reflash).
   Asked supervisor GO to build (firmware change + reflash-all implication). xtensa-verified + regression-guarded.
(Deferred list aligns with supervisor's 2026-06-27 stand-down enumeration; items 9-14 added 2026-06-30.)

### BUILD COMPLETE — all 6 steps + compile-verify GREEN. ON-METAL OWED (boards held):
- The field triplet (sensor/repeater/bridge/receiver) needs an on-metal run once Roy frees ≥2 boards:
  role-profile activation (provision an RPF1 record @0x17000, confirm role behaviour), §8.1 beacon RX
  resolution, §3.5 re-attach, OTA confirmed-boot round-trip.
- COORDINATION RESOLVED (2nd batch): composer ADOPTED RPF1 byte-exact (40B then 48B, encode_rpf1 2d1bd25);
  sent composer the XIAO board.toml GPIO map (SCK7/MISO8/MOSI9 NSS41 RST42 BUSY40 DIO1=39, RF-sw=DIO2,
  TCXO-DIO3-1.8) + 4 RPF1 answers: dest/expected_sensor=0 OK for first triplet; bridge carrier-creds sector
  RESERVED @0x19000 ('CCR1' format) but firmware read/unseal = §3.2.4 FOLLOW-UP (first triplet uses bench
  WiFi); .role blob written RAW to flash @0x17000 (not an NVS partition image). composer CONVERGED:
  board.toml [pinout] landed (8e2b2f9, matches my map); delivery = `espflash write-bin 0x17000 <file>.role`;
  composer's Mariko orchestrator side COMPLETE+green (RPF1 v2 48B emit + §3.2.4 carrier+seal + deploy-set).
  Remaining XIAO check = Seeed schematic-PDF confirm = METAL-BRING-UP item (verify MISO/MOSI on first
  XIAO LoRa light; not blocking). **core CONFIRMED the XIAO
  RF-switch WORKS with Sx1262::new()** (DIO2 keyed unconditionally in configure(); 88f549f added
  with_dio2_as_rf_switch alias) — dropped the false "RF not driven" caveat (worktree HEAD updated). **specs
  landed R2-RUNTIME v0.12 §3.2** stating one-image config-activated PROVEN, citing this build. NVS map now
  ends: role-profile@17000 / anti-rollback@18000 / (reserved) carrier-creds@19000.
- Cross-fleet OPENS (replies in): **core RULED** sdkconfig+FFI correct, NVS-collision N/A for esp32
  (namespaced API), and **platforms/esp32 IDF build + on-metal confirmed-boot is HIVE's** → I must install
  ESP-IDF (espup) to compile-verify platforms/esp32 (Alfred has only esp-hal/xtensa) = OWED. core's
  r2-sx1262 DIO2-RF-switch support = still open (flagged). **workshop CONFIRMED** the XIAO pins vs
  meshtastic seeed_xiao_s3 variant.h (my map was right) — confirm vs Seeed schematic before canon. composer
  = RPF1 emit (48B) + board.toml = queued. §8.1 codec OFFERED to core for r2-discovery::beacon upstreaming.
- SEPARATE TRACK (not firmware): repoint r2-hive-bin/Cargo.toml path-deps at r2-core's now-landed
  r2-def/r2-dispatch/r2-ensemble/r2-transport/r2-discovery (core msg 21:27) — awaiting core 'build green' go.
Canon refs read + pinned: R2-RUNTIME §3.2.1–3.2.4, R2-BEACON §8.1.1–8.1.4. Gap-analysis input doc =
`docs/field-firmware-role-prep.md`. Shorter cycles; update this tracker each step.

---

## (PRIOR) 2026-06-26 — LoRa PHASE 0 (does LoRa survive #20?)
**Re-oriented after a /clear (context-saturation stall).** #20 hardening CLOSED; my DFR signed-OTA
receiver DONE+committed (r2-hive `434132e` + `5c93026`). **TASK NOW = LoRa PHASE 0** (supervisor-directed,
I LEAD): the one test telling us what survived #20 — does LoRa still work on CURRENT firmware (HEAD,
post-#20/hardening)?
1. Build CURRENT unified firmware with `loraroute` feature (full = `nobt,loraroute,loratcxo,multitg`).
   Firmware worktree = `/home/roycdavies/Development/R2/dfr1195-fw-wt` (branch `dfr1195-fw`, was `0ebfd09`).
   Build on Alfred: `source ~/Development/homelab/export-esp.sh` first (xtensa linker).
2. Flash 2 DFR1195s AS923-NZ wairoa (R2-LORA §2.1/§3.1 = TN-FR-1 config). DFR boards are on **tuxedo**
   (`ssh tuxedo`); by-id ports from composer at flash-time. XIAO can't run LoRa (no SX1262).
3. Re-run heartbeat-sync + TN-FR-1 neighbour-discovery/`directed_via`; confirm mutual-RX + HB-sync hold.
**REPORT:** PASS = LoRa survived #20 → restore → Phase 1 parity. FAIL = regression to localise. Framing:
conjecture/refutation, TN-FR-1 re-asserted on current firmware.

### ☑ CHECKPOINT (2026-06-26 ~02:30 NZ) — Phase 0 metal HELD by supervisor; build-PASS = the accepted result.
**SUPERVISOR FINAL CALL:** stand down on Phase 0 metal. BUILD-PASS IS the Phase 0 result that matters —
*LoRa survived #20, confirmed.* Metal mutual-RX + HB-sync is a CONFIRMATION that waits for a clean window
(Roy/composer freeing a 2nd board, or the demo ending) — do NOT interrupt Roy's live demo, do NOT grab the
1 free port, STOP queuing composer. Everything staged at `tuxedo-os:~/phase0/` for an instant run when a
window opens. **Two follow-ups queued (both no-rush, both confirmed to core):**
1. **frame_fingerprint seed-first sig (core 807cab5):** my call-site is main.rs:1403 (A1 option-c
   FingerprintCache). Worktree base (0ebfd09) still has the OLD 4-arg sig → NO break now. When core advances
   the worktree base to include 807cab5: update :1403 to `frame_fingerprint(&seed, fr_origin, msg_id,
   payload, hmac_tag)` + source a 16B secret seed from the ESP32-S3 HWRNG (esp_hal Rng/Trng) for
   DataPlane::new + the call (NOT derived — guessable). Interim [0;16] = sound.
2. **Field-firmware prep (supervisor-offered) — GAP ANALYSIS DELIVERED.** The supervisor (NOT specs) owns
   the field-firmware canon, and it's NOT yet authored (only `docs/planning/FIELD-SENSOR-FIRMWARE.md` plan
   exists) → my current-firmware ground truth is its authoring input. Wrote the full answer in
   `docs/field-firmware-role-prep.md` + sent the supervisor (a)-(e): roles=FOUR (receiver=terminal
   sink/display+absence-track, distinct from bridge=transit); NO config-struct today (role = hive_id-match ×
   features, all hardcoded consts — listed the knobs+values); 8B beacon = my_hive(u32 BE)++seq(u32 BE),
   separate from §12.6 HB (keep distinct, evolve beacon into R2-BEACON §8); per-role deltas; join = persona
   @0x12000 persists, re-attach silently resumes, **NO self-enrol**. TWO new-behaviour flags for canon:
   (i) sensor duty-cycle ADVERTISED not ENFORCED (no real wake/sleep yet); (ii) no autonomous enrol.
   **OWNERSHIP CLARIFIED:** **specs** is the actual canon AUTHOR (it owns R2-LORA/R2-BEACON/R2-ROUTE;
   already landed R2-ROUTE §13.4 + R2-LORA §9.1 LoRa-no-sender-quota; will author R2-BEACON §8 + the
   role-profile) and was EXPLICITLY blocked on hive's gap analysis. Sent the full analysis to BOTH supervisor
   AND specs (specs' earlier fork-ask predated the analysis). **NEXT GATE:** specs pins R2-BEACON §8 + the
   role-profile struct/enum → THEN I implement against the pinned canon (NOT a guessed struct). Both replies
   pending. (Attribution quirk post-account-B: specs↔supervisor msgs sometimes mislabel sender — content is fine.)

### PROGRESS (2026-06-26 ~01:50 NZ):
- **BUILD-LEVEL VERDICT = PASS.** Built current firmware `nobt,loraroute,loratcxo,multitg` on Alfred —
  13.4s, ZERO errors, 24 dead-code warnings only, fresh ELF
  `dfr1195-fw-wt/platforms/dfr1195/target/xtensa-esp32s3-none-elf/release/r2-dfr1195` (1065112B, 01:44).
  LoRa firmware survives #20 at source level (no API-drift from r2-dataplane/route/wire consolidation,
  dc re-emit, H9-secure HB-rx, A1 reconcile). **GOTCHA:** must `source ~/Development/homelab/export-esp.sh`
  WITHOUT a pipe (piping source = subshell = PATH lost → "linker xtensa-esp32s3-elf-gcc not found").
- **BENCH IS LIVE — not a hardware gap.** The `tuxedo` ssh alias is a DEAD tailnet node (7d offline) =
  my timeout. Rig moved to **`tuxedo-os`** (100.90.50.112). All 5 DFR1195 enumerate; TN-FR-1 rig present
  + provisioned Jun22: D1 50:26:98=ttyACM0 (480e900e orig), D2 b7:90:10=ttyACM1 (2cab5f69),
  D3 b6:0a:a0=ttyACM4 (f91c8911), D4 52:99:28=ttyACM3 (06ae082b), D5 50:23:E4=ttyACM2 (0dcadbf8).
- **FLASH PAYLOAD PRE-STAGED** to `tuxedo-os:~/phase0/` = {espflash 4.4.0 (tuxedo-os has none), ELF
  `r2-dfr1195-loraroute`, `dfr1195-partitions.csv`}. espflash runs natively there.
- **GATE = port-release (REFINED ~02:1x NZ).** Orchestrator RESTARTED → PID 3197; now holds
  ttyACM0/2/3/4, leaves **ttyACM1 (D2 2cab5f69) FREE**. Only ONE of two needed ports free → can't run
  mutual-RX yet (needs 2 boards that hear each other; originator role NOT required — any pair works).
  Queued composer TWICE for a 2nd port (unanswered, busy/offline). ESCALATED to supervisor →
  **SUPERVISOR RULING (resolved): hive = STAND BY.** The metal-run is gated on Roy's live demo holding the
  ttys; do NOT interrupt it. Hold until composer/Roy frees ≥2 boards (then run instantly). (Overnight freeze
  was account A's weekly cap; now on account B, fresh budget.) Run script
  is staged at `tuxedo-os:~/phase0/phase0-run.sh` (hardcoded D1 ACM0 + D2 ACM1 — EDIT ports if a different
  pair is freed). **NEXT when 2 ports free:** ssh tuxedo-os, flash both with
  `~/phase0/espflash flash --chip esp32s3 --partition-table ~/phase0/dfr1195-partitions.csv --port <by-id>
  -a hard-reset --non-interactive ~/phase0/r2-dfr1195-loraroute` (partition-table = persona@0x12000 survives),
  monitor both for boot `DEV <maclow3> hive=` + mutual-RX + heartbeat-sync + neighbour-discovery, then
  RESTORE baseline + tell composer to re-attach.
Refs: [[lora-message-passing-metal]], [[dfr1195-firmware-bench-workflow]]. Shorter cycles + /clear when prompted.
(Everything below this block is PRIOR state — kept for recovery.)

---

Updated 2026-06-24 (owned by hive). Master save (read-only ref):
`r2-fleet/fleet-context/FLEET-CONTEXT-SAVE.md` (moved from claude-fleet, now tooling-code-only).

**Role + normative policy** (do-NOT-fork-per-target, authority chain specs→core→hive, before-editing,
stop conditions, no-go): **→ [AGENTS.md](AGENTS.md)**. Live spec-vs-impl forks: **→ [FORKS.md](FORKS.md)**.
This file is **STATE-ONLY** — running state, in-flight work, the session arc. (Policy moved to AGENTS.md per
the F8 process-hygiene split, 2026-06-25.)

**Current branch:** `platform-trait` (local + pushed, HEAD `ce80733`). Built atop the v0.2 work (`0aa6ab7`).

## PCO FIRMWARE MIGRATION SESSION (2026-06-24) — bundle built-green, AT THE FLASH-WINDOW
Spec-first migration of the DFR1195 firmware to **R2-HEARTBEAT v0.5** + an **Occam mesh-retire**, plus the nRF54
data-plane seam. Firmware lives in the **dfr1195-fw-wt WORKTREE** (`r2-core/platforms/dfr1195`); r2-hive holds only
the PATCH (`docs/dfr1195-firstlight.patch`) — the commits below are r2-hive patch-snapshot commits.

**THE BUNDLE (built-green PRE-FLASH, all pushed):**
- `0ad8566` §1A phase-lock -> OPTIONAL: leaderless-PCO (coupling-nudge + rate-consensus + period-jitter-off) goes
  behind an OPTIONAL `pco` feature; DEFAULT = free-run + loose period-jitter + β=0 = the §1A loose-jittered
  keepalive (the FR-1-REL POS-arm, already metal-tested -> a default-flip of TESTED code). Retired loosehb+rateoff.
- `d7507cd` §3B.1 power_state advertise (emit): HB byte 8 = self-asserted availability class, tier-aware (AlwaysOn
  DFR / Intermittent fr4-SENSOR-D1). **FORMAT SUPERSEDED:** specs caught byte-8 FORKS R2-WIRE §12.6 (HB payload is
  a CBOR MAP). Unified pass = re-emit as CBOR key `dc` (RENAMED duty_class — avoids the R2-BEACON §7.2.1 battery
  power_state collision), DROP the redundant 4B origin + fw_ver. The CBOR re-emit + byte-8 REVERT is HELD until
  specs lands §12.6 (see NEXT #2).
- `20703ab` §1A.1 RATE-DECOUPLE (the delicate one): the ~2s phase oscillator still drives fire_seq (the originate
  cadence + LED beat) UNCHANGED, but the keepalive HB-EMIT is throttled to KEEPALIVE_PERIOD_MS=30_000 (the §1A.1
  tunable knob; supervisor-confirmed 30s = "tens of s", DG-1 silence ~90s) — un-conflates liveness from the
  demo/proof signal. pco = every-beat (phase-lock); blackout test arm = every-beat (throttle cfg-gated out).
- `3095804` + `cef7516` Occam MESH-RETIRE (NOT a deletion — HELD+flagged as a compound-gate refactor): step 1 =
  excise the lora_mesh_task fn+spawn (the safe sliver, mutually-exclusive with loraroute); step 2 = ATOMIC
  compound-gate refactor dropping the loramesh/lorareach features — loramesh lived in the FR-2-bridge/ESP-NOW SPAWN
  SELECTORS (main.rs:346/:412/:2893), and since loramesh was NEVER set in any flashed config, not(loramesh)≡true
  everywhere -> each gate-simplification is a VERIFIED NO-OP. lorareach (§4.2 PCO reachback) retired -> simple
  phase-error.
- `ce80733` benchkeepalive feature (OFF by default): KEEPALIVE_PERIOD_MS 8s under the feature else 30s ship —
  ship-safe + reproducible + format-agnostic (dominates the uncommitted-binary option) for bench watchability.
- `7b3cfe3` chore: gitignore `prebuilt/` (14MB binaries out of git history).

**NO-OP INVARIANT (the load-bearing safety claim):** every FLASHED config spawns IDENTICAL tasks after the
gate-refactor — verified per-config (nobt/routetest->espnow; loraroute->LoRa leaf no espnow; loraroute+bridge->
espnow re-enabled; blemesh->neither). The bench is the EMPIRICAL test of this conjecture; if the demo regresses it
REFUTES "the migration preserves the demo" -> spec-first fix, no papering.

**BUILD MATRIX = 7 configs GREEN (errors=0):** fr4 / loraroute+bridge / loraroute / nobt+routetest / nobt /
blemesh / fr4+pco.

**BENCH/SHIP BINARIES STAGED** (supervisor: "you build both"): 6 release ELFs + app-.bin (OTA) + a merged sample at
`prebuilt/bench-bundle-0624/` (GITIGNORED, local-only — the committed artifact is the SOURCE/benchkeepalive feature,
NEVER the binaries) = {leaf (D1/D2 loraroute) / bridge (D3 loraroute+bridge = FR-2) / recv (D4 routetest)} x
{ship 30s / bench 8s}.

**FLASH-WINDOW: OPEN (Roy GO, boards free).** composer flashes/OTAs + monitors the ttys; I (firmware owner)
INTERPRET the 3 verdicts: (a) FR-2 bridge survives, (b) LED-sync + FR-4 NO-REGRESS [the critical one], (c) keepalive
fires + silence-detectable (8s bench). AWAITING composer's serial output to interpret per-item; then SHIP (30s)
binaries onto demo-correct boards.

**SESSION-RESTART RECOVERY:** a post-/compact degradation was cleared by a mid-session restart; the clean 7-config
matrix build (errors=0) + the bundle proved the recovery (supervisor: "welcome back").

**REMAINING / NEXT (priority order):**
1. BENCH-VERIFY (in progress with composer) — interpret (a)/(b)/(c), confirm the ship binaries go on demo boards.
2. duty_class CBOR re-emit — parse §12.6 `dc` on receive + call core's `set_neighbour_duty_class` + REVERT byte-8
   (`d7507cd`); GATED on specs landing the unified §12.6/§1A/§3B.1 pass.
3. r2-dataplane module (POST-bench) — NEW crate `r2-core/crates/r2-dataplane` (no_std; deps r2-route+r2-wire+
   r2-trust; core's location call). hive-OWNED: types `DataPlane`/`RxDisposition`/`PhyMask` + `handle_rx_frame` +
   `poll_keepalive`, factoring the bench-VALIDATED dfr RX logic; UNBLOCKS core's nrf54 gateway `handle_rx` body.
   PhyMask = u8 platform-agnostic egress bitmask (the plan_forward-egress->bit map is the PLATFORM adapter);
   deliver_out = RAW channel push (NOT through r2-dispatch — std/above-boundary). core registers + wires.
4. LED-flash-out (gate the FIRE-driven LED behind pco; coordinate with composer's bench LED-sync check) +
   sensor-piggyback (§1A.1, the SENSOR tier piggybacks liveness on sense-wake).

**KEY DECISIONS this session:** spec-first throughout (read §1A/§3B.1 before coding); HELD-and-flagged TWICE
(mesh-retire = compound-gate refactor not a deletion; power_state byte-8 forks §12.6) rather than blind-executing;
committed-feature > uncommitted-binary for bench (dominates both options); push-per-green-step (standing order).
Deep context in the memory files: occam-hb-simplification, r2-hive-multi-target-goal, lora-message-passing-metal,
linux-hive-deliver-gate-gap.

## OVERNIGHT AUTONOMOUS CAMPAIGN (2026-06-22, supervisor grant; Roy winding down)
Per supervisor: continue the TN metal refutation campaign autonomously — SPEC-FIRST on any weakness
(route to specs, queue for Roy, NO canon mandate overnight), RESTORE the 2-TG baseline after each run
(protect the live demo), commit auditable field.* records, tick off survived refutations, keep this file
current, don't wait per-conjecture.
- **TN-FR-2 (LoRa<->ESP-NOW gateway / DG-2 #16) = PASS / metal-green (2026-06-23).** field.* =
  `docs/field-results/lora-fr2-0623/TN-FR-2.json` (+ raw serial). 4 DFR, ONE TG 'wairoa' (3932969629,
  composer-prov2'd): D1=origin(480e900e) ->LoRa-> D2=router(2cab5f69) ->LoRa-> D3=BRIDGE(f91c8911, dual-radio
  SX1262+ESP-NOW) ->ESP-NOW-> D4=receiver(06ae082b). PROVEN: **D4 (ESP-NOW-only) DELIVERED 12 distinct Events
  that originated at D1 over LoRa (dlv=11) = the Event CROSSED LoRa->ESP-NOW**; the engine AUTO-BRIDGES — D3
  directed_via next_hop=06ae082b x11 (transport-aware best_transport picks the ESP-NOW egress, NO bridge
  routing code); dedup-once-across (D3 DROP-Duplicate x36, each msg_id delivered once = DG-2, dedup keys on
  frame-carried origin, transport-agnostic); bidirectional (D4 replies retrace ESP-NOW->D3->D2->LoRa->D1,
  D1 reply-DELIVERED x12); forced multihop (D1 masks D3-direct x48 via hardcoded can_hear_hive -> D1->D2->D3).
  Delivery ~63% (vs FR-1's 11% — the fast ESP-NOW leg). Firmware (eed35f9): `bridge` feature + PER-TRANSPORT TX
  channels (DATA_TX_LORA vs DATA_TX) + mesh_broadcast (bridge pushes BOTH carriers). Baseline restore in
  progress (composer reflash+reprovision+reattach-5). NEXT: FR-2b = TRUE LoRa<->WiFi/UDP gateway w/ PI5 (Linux
  r2-hive RECEIVER over real WiFi, composer pre-provisioned pi5 keystore); then FR-4 capstone (role sim +
  TN-FR-1-REL loose-jittered-HB two-arm). See [[lora-message-passing-metal]].
- **TN-FR-2 (LoRa<->WiFi gateway / DG-2 #16) = UNBLOCKED + DESIGNED, build pending composer's board map (2026-06-23).**
  core CONFIRMED (DG-2/BL-300/BL-301 sim-validated): (1) **dedup is transport-agnostic** — DedupCache keys on
  (frame-carried origin, msg_id) ONLY, so a LoRa-received frame re-forwarded on WiFi is NOT re-delivered/looped
  (dedup on RECEIVE; engine excludes the inbound source_hop from the flood set). (2) **MTU = handle-the-reject**:
  engine select_transport uses the FLAT LoRa MTU (222) but the DRIVER transmit() rejects > the actual lora_mtu(SF,BW)
  (e.g. 51@SF12) — so the bridge MUST check lora.send()/transmit() result and DROP that egress on reject (BL-301;
  never truncate/fragment, R2-TRANSPORT §2.2). (3) **the engine AUTO-BRIDGES**: NeighbourEntry.transports is a
  bitmask; plan_forward returns Hop{neighbour,TRANSPORT} and picks egress per hop — NO bridge routing code.
  FIRMWARE DESIGN (the bridge node = composer's D3, on both LoRa + the WiFi-island carrier):
  - Run BOTH carriers (lora_route_task + the WiFi-island carrier) feeding the SHARED DATA_RX; pass frame-carried
    origin (TN-FR-1 proved). Airtime-gate the LoRa egress via service(now_ms)+set_neighbour_count (WiFi->LoRa
    Events DEFER under load, not drop). Drop-on-LoRa-MTU-reject.
  - **KEY ARCH CHANGE**: DATA_TX is a CONSUMING channel (each frame -> ONE carrier), so it does NOT broadcast on
    both. Need PER-TRANSPORT TX routing: either split into DATA_TX_LORA + DATA_TX_WIFI (each carrier drains its
    own) with io_task pushing per advice's egress transport (Hop.transport for Directed; BOTH for Flood), OR a
    transport selector on DATA_TX. This honors core's Hop{transport} auto-bridge. Leaf nodes (LoRa-only, WiFi-only)
    use just their one channel.
  - **TRANSPORT-TAGGED INGEST**: the HB ingest_observation currently HARDCODES transport=EspNow (main.rs ~954);
    thread the ingress transport through DATA_RX (add a tag to MeshRxFrame) so the bridge's neighbour table tags
    LoRa-neighbours vs WiFi-neighbours correctly = what makes plan_forward's auto-bridge work (directed). Flood
    bridging works WITHOUT it (broadcast both + dedup), so a flood-first proof is the lower-risk first run.
  composer's FR-2 DEFS (RECEIVED, locked; full defs catalogue/topologies/wairoa-fr4/, this = fr4 minus the
  WiFi-router): **D1=origin (480e900e), D2=LoRa-router (2cab5f69), D3=BRIDGE (f91c8911, SX1262 LoRa + onboard
  WiFi), RECEIVER=PI5 (ssh pi5, Linux r2-hive over WiFi/Internet = the marae hub).** PATH: D1 ->(LoRa)-> D2
  ->(LoRa)-> D3[bridge] ->(WiFi)-> PI5. MASK: D1->[D2]; D2->[D1,D3]; D3->[D2(LoRa),PI5(WiFi)]; PI5->[D3]. ONE
  TG 'wairoa' spanning both islands (gateway test, not isolation — the bridge carries the GroupHmac across;
  keys ~/.r2/group-keys.json#wairoa, composer provisions/hands over). composer PROVISIONS + builds the gateway
  dashboard view; hive builds bridge/leaf fw + flashes + runs via ssh. **SCOPE NOTE: the WiFi side is a REAL
  WiFi link to a LINUX r2-hive (PI5), NOT ESP-NOW — so D3's 2nd carrier = onboard WiFi/UDP to PI5, and PI5 runs
  the r2-hive Linux/std build as a 'wairoa' routing RECEIVER (its RouteEngine delivers + the receive-flash
  logs). Bigger integration than DFR-only FR-1.**
  OPEN PREREQ (asked composer, queued): how D3 reaches PI5 over WiFi in r2-hive's model — UDP broadcast on a
  shared LAN (D3 STA + PI5 on one router/AP)? D3 joins a PI5 AP? which port / the existing wifi.rs UDP path? +
  confirm PI5 runs r2-hive Linux as the wairoa routing peer. Don't build D3's WiFi carrier blind = spec-first.
  FIRMWARE FOLLOW-UP (board-map-independent, do in the FR-2 build): (a) transport-tagged DATA_RX ingest — construct
  Observation with the REAL ingress transport (Transport::Lora vs Wifi) instead of hardcoded EspNow (main.rs
  ~954); core confirmed engine auto-populates NeighbourEntry.transports + plan_forward picks egress (dual-homed
  D3 = both bits on one entry, best_transport per-MTU). (b) msg.* telemetry over /r2 — PINNED schema (R2-CBOR,
  event NAME discriminator, compact-int body): msg.tx{0:id,1:from,2:to} / msg.rx{0:id,1:at,2:from_hop} /
  msg.relay{0:id,1:at,2:next_hop(0=flood)} / msg.delivered{0:id,1:at,2:dup}; id=loraroute msg_id stable across
  the 4 (routed to specs to pin). (c) LED on_received receive-flash + relay-flash (composer 👍). PROOF target:
  Event D1 -> D2 -> D3 -> PI5 delivered EXACTLY-ONCE across the bridge (DG-2 dedup-once, transport-agnostic).
  Reliability (loose-jittered-HB + retransmit) = TN-FR-4 capstone two-arm (specs TN-FR-1-REL). See [[lora-message-passing-metal]].
- **TN-FR-1 (BL-200-over-LoRa MESSAGE-PASSING) = PASS / metal-green (2026-06-23).** field.* =
  `docs/field-results/lora-fr1-0623/TN-FR-1.json` (+ raw serial). Routed Events A->B->C over LoRa on 3 DFR
  (A=480e900e, B=2cab5f69, C=f91c8911 — all TG-A), MASK-forced multi-hop: **C DELIVERED A's REQUESTs via B
  (dlv=2), directed_via B (next_hop=C for A->C, next_hop=A for the replies), exactly_once (B DROP-Duplicate
  x4), reply retraced C->B->A and DELIVERED at A, LED fires on receipt.** Baseline (2-TG demo) restored
  (reattach-5, health 200). KEY METAL LESSONS: (1) the released D1/D2/D3 originator is **480e900e** (MAC
  50:26:98), NOT 0dcadbf8 (that board, MAC 50:23:E4, stays in the demo) — re-keyed the MASK + auto-origin.
  (2) build needs **multitg** so all 3 use the NVS-provisioned TG-A key (else C can't HMAC-verify A's Event).
  (3) **synchronized-fire collisions** on the half-duplex air dropped most frames (B's TX reached A/C ~1/100s
  under lockstep); an **ALOHA TX-jitter (0-300ms) in lora_route_task** decorrelated TX starts enough to prove
  the path. RELIABILITY FINDING: per-msg delivery ~2/19 at SF7 w/ always-on tight PCO -> the reliability
  fix = Roy's refinement (HB as LOOSE jittered BACKGROUND path-maintenance, lower rate) + retransmit; feeds
  TN-FR-4. CORRECTNESS proven; the data-plane (core's LoRaTransport::service + frame-carried origin) holds.
  Firmware below ⬇ (loraroute) was the staged build; this run added the jitter + 480e900e re-key + multitg.
- **TN-FR-1 firmware (loraroute) — built atop the staged work below (2026-06-23).**
  Roy's #1: route an Event A->B->C over LoRa on 3 DFR1195, MASK-forced multi-hop (A can't hear C), validate
  directed_via B + exactly_once@C + LED-flash on RECEIPT (not heartbeat). The DEFERRED CSMA/heartbeat-mesh
  redesign is NOT this. Built a new **`loraroute`** feature (= `lora` + `routetest` + `r2-transport/alloc`):
  - Uses core's READY `LoRaTransport::service(now_ms)` data-plane (continuous-RX + TX-pacing + §4.2/§4.3
    airtime-gating, defer-not-drop) instead of the naive half-duplex `lora_mesh_task`. New `lora_route_task`
    drains DATA_TX -> LoRa, feeds RX -> DATA_RX; carries ALL frames (Events, not HB-only like loramesh).
  - Thin **`RxenRadio`** newtype impls `LoRaRadio` to toggle the DFR1195 RF switch (GPIO42 HIGH-RX/LOW-TX)
    around transmit/listen/standby — keeps the RXEN concern in the per-platform layer (LoRaTransport is
    chip-agnostic). The one-codebase seam.
  - **4-byte immediate-sender hive PREPEND** per LoRa frame = the LoRa analogue of ESP-NOW's L2 src MAC on
    a MAC-less broadcast medium: feeds the hive-based `can_hear_hive` MASK (hardcoded A={B} B={A,C} C={B},
    no fragile tty provisioning) forcing A->B->C, and threads the true RELAYER as src_hive into DATA_RX for
    the §4.3.4 TrailReinforcer.
  - **ForwardRequest.origin = frame-carried originator** (was hardcoded `0`) — the BL-200/M-ESPNOW-3 fix,
    core-confirmed: per-(origin,msg_id) dedup is what makes exactly_once + directed_via hold multi-hop.
  - **LED flashes on DELIVERED receipt** (RECEIPT_SIGNAL; heartbeat envelope suppressed under loraroute).
  - Board A auto-originates REQUEST->C at boot (loraroute default SENDTO) = self-contained 3-board run.
  BUILD GREEN: `cargo build --release --features nobt,loraroute,loratcxo` -> ELF staged (983KB) on alfred,
  ready to flash. NOTE: the `dfr1195-fw-wt` worktree is a SEPARATE stale clone of r2-core — I synced its
  `crates/r2-transport/src/{lora_transport,lora,lib}.rs` to canonical core (commit 027a912, airtime-gating)
  to get `service(now_ms)`/`set_neighbour_count`/`lora_mtu`. Patch regenerated: `docs/dfr1195-firstlight.patch`.
  BLOCKER (NOT idle): composer can't release the DFR ttys on tuxedo — the `reattach-dfr-45.sh` ssh is
  approval-gated, needs the operator or Roy's morning. composer pings `dfr-fr1-off` when 0 holders. THEN:
  flash 3 DFR (A=0dcadbf8, B=2cab5f69, C=f91c8911), watch C's LED flash on each routed message, capture
  directed_via/exactly_once serial -> commit `field.*` TN-FR-1, restore baseline. Ladder after: TN-FR-2
  (LoRa<->WiFi gateway, DG-2), TN-FR-4 (role-based sensor/router/receiver Wairoa sim).
- **DONE: BL-200 RESOLVED + PASS/metal-green** (one-line reply-msgid u16-dedup collision; fix=shared
  `r2_route::trail::reply_msg_id`, commits up to `9fe9068`; §4.3.4 vindicated, §4.6-MUST refuted; baseline
  restored-clean 5/5 DFR multitg). Metal field.* count: BL-100 survived, BL-200 resolved-pass.
- **DONE: BL-103 SURVIVED** (`3a32856`). §2.5 neighbour eviction+rediscovery holds on real ESP-NOW: silent
  board EVICTED from the route-engine nbr table (conf->0.01), ACTIVE neighbour RETAINED (selective, not a
  flush), returning board REDISCOVERED fresh. Method: fastevict route-clock x20 (1800s horizon->seconds) +
  blackout[60,150)s + NBR-TBL telemetry, 3 isolated XIAO (MASK->NVS). TUNING: x120/x40 amplified conf-
  variance (evicted active too); x20 = clean contrast. Reused real engine decay_neighbours. No spec weakness.
  field.* = TN-L1-IT-BL-103.json. Baseline restored. **3 metal field.*: BL-100 survived, BL-200 resolved-pass,
  BL-103 survived.**
- **DONE: WiFi HB-sync SURVIVED** (`c4082c0`, TN-L0-IT-HBSYNC-WIFI). Leaderless PCO converges over WiFi/UDP
  (3 XIAO SoftAP star, X1=AP via AP_MAC_MATCH flip, spread_ms->0-4ms, synced=true) = engine is TRANSPORT-
  AGNOSTIC (ESP-NOW + WiFi). **4 metal field.*: BL-100, BL-200, BL-103, HBSYNC-WIFI.**
- **BLE 2-board sync BLOCKED** (finding): blemesh M8b negotiation hardcodes M7_PROVIDER_HIVE=0x0dcadbf8 (a
  fixed test peer) -> elects an absent provider for arbitrary pairs -> no CoC. Needs generalizing; BLE is
  L0-2-node-only regardless -> BLE-mesh = WAIROA-7 queued for Roy.
- **⚠️ X4 (2c81b4a3) NEEDS A POWER-CYCLE (Roy, morning):** its USB-JTAG de-enumerated during the WiFi run
  (port vanished from /dev/serial/by-id); X1/X2/X3 restored fine to multitg (one-off X4 USB casualty, not a
  defect). X4 is OFFLINE / stuck on the WiFi build until physically re-plugged. The 5 DFR + 3 XIAO are clean.
- **🔦 LoRa FIRST LIGHT ACHIEVED (`7387686`) — TOP priority, the Wairoa rung is ALIVE.** Bidirectional
  LoRa between 2 DFR1195 SX1262 radios: D2 RX from=480e900e (rssi-44 snr12), D1 RX from=2cab5f69 (rssi-45
  snr13), clean 8B payload every cycle. Wired core's r2-sx1262 onto the DFR1195 via esp-hal (SPI3 SCK7/
  MISO5/MOSI6 + NSS10-CS + BUSY40/RST41 + RXEN42 + Delay; Sx1262::new().with_tcxo(V1_8)) + a concrete-typed
  lora_task (configure->listen->loop{poll RX; TX beacon}, RXEN HIGH-RX/LOW-TX). VALIDATED on RF: TCXO DIO3
  1.8V PLL-lock, RXEN42 polarity, DIO1, full driver API, wire (sync0x21/916.8MHz), RSSI/SNR. BENCH config
  (overrides, NOT defects): SF7 (SF12 ~2s ToA vs ~3s windows = partial-catch CRC-err at 30cm = timing
  artifact; SF7 ~40ms clean) + tx_power -9dBm (30cm; deployment +20/+22). field.* = LORA-FIRSTLIGHT.json.
  Baseline restored (D1+D2 multitg). **5 metal results: BL-100, BL-200, BL-103, HBSYNC-02/wifi, LoRa-first-light.**
  NEXT (supervisor ladder): (1) core's RXEN driver param (drop manual toggle); (2) LoRa MESH = bridge
  io_task (PCO + r2-route) to the LoRa carrier (like espnow/blemesh) = multi-board LoRa heartbeat+routing;
  (3) SF12 real-distance range test; (4) cross-transport LoRa<->WiFi gateway (DG-2 #16 = HBSYNC-07 coherence).
- **LoRa MESH = PARTIAL-FINDING (`b872008`, HBSYNC-02 transport=lora).** Built loramesh (io_task PCO+routing
  bridged onto the LoRa carrier via half-duplex lora_mesh_task, ESP-NOW gated off). PCO syncs TIGHT pairwise
  over LoRa (D2 e=0.001 spread=2ms = engine+bridge WORK) but the 3-board mesh doesn't SUSTAIN (nbrs->0):
  (1) LoRa airtime (130ms+ SF7) uncompensated in the PCO phase = §4.2 reachback the interop spec flagged
  for LoRa, METAL-CONFIRMED (D1 spread 245ms desync) -> routed SPEC-FIRST to specs/core; (2) naive bridge
  floods all traffic over the slow half-duplex link -> HBs starved. NEXT: §4.2 airtime-comp (specs/core +
  lora_airtime::time_on_air_ms — asked core if landed) + hive carrier traffic-shaping (HBs-prioritized,
  ToA-aware) -> clean LoRa mesh -> SF12 range -> LoRa<->WiFi gateway (DG-2 #16). **6 metal results: BL-100,
  BL-200, BL-103, HBSYNC-02/wifi, LoRa-first-light, LoRa-HBSYNC-partial.**
- **HBSYNC-03 sustain re-run (§4.2+shaping) = NOT green yet — deeper finding (`4700c0a` has §4.2+shaping+
  lorareach).** Ran specs' 2x2: arm2 (shaping+§4.2) + arm1 (shaping-only) BOTH = no 3-board reception
  (nbrs=0). Debug PROVED HBs TX'd fine (b0=0x29 mt=5 txd=true), Events dropped -> NOT shaping/§4.2. ROOT
  CAUSE = my naive half-duplex lora_mesh_task poll-loop (drain DATA_TX + poll RX + 10ms yield) has an
  RX/TX listen-window timing flaw -> radio misses peers multi-board. NEXT BUILD = redesign lora_mesh_task
  per core's CONTINUOUS-RX / event-driven / ToA-aware pattern (DIO1-IRQ RX + listen-before-talk/CSMA for
  the synchronized-fire collision; asked core for a reference shape). HB on metal = 30B unsigned (nobt),
  §4.2 ToA used 62B -> use actual frame_len. §4.2+shaping are correct components (kept). Baseline restored.
- THEN (per supervisor): cross-transport LoRa<->WiFi gateway (DG-2, #16); BLE-mesh 'perhaps' (WAIROA-7);
  LR2021 (composer leads). SECONDARY: WiFi MASKED routing (IP-MASK port; specs queued BL-203/200-over-wifi/
  BL-000/AB-000/BL-001) + BL-100 demote sweep (#13). M-ESPNOW-3 (carry frame-origin->ForwardRequest.origin,
  core contract confirmed engine.rs:56-64; + H1 authenticate route_stack[0]) = canonical BL-200-class kill.
  SIM-ONLY (specs): BL-204 idle-fade, L2-XT-BL-200, silence-is-signal (~40000s idle).
- **M-ESPNOW-3 follow-up:** carry frame-origin in the relay frame -> ForwardRequest.origin -> r2_route
  (origin,msg_id) dedup = the canonical fix that kills the origin-degraded class (beyond BL-200).


## Active (besides the branch) — priorities per Roy (2026-06-16)
- **NEXT TRACK — TN REFUTATION MATRIX (hive = METAL runner).** Roy's big campaign: every
  routing+message-passing edge case across ALL transports, conjecture/refutation, coverage dashboard.
  Axes: topology(L0 full/L1 multihop/L2 SCF-beyond-radio/L3 partition+heal) × scope(intra/inter-TG) ×
  trust-plane(above/below-TG) × payload(events/data) × transport(BLE/WiFi/ESP-NOW/LoRa/UDP) + edge cases.
  Flow: specs authors matrix+schema (IN PROGRESS) → core sim-tier harness → **hive runs the METAL tier on
  the 9 co-located boards spanning all radios** (`field.*` = metal only). **SPEC-FIRST INVIOLABLE:** weakness
  found → note + route to specs BEFORE any code. CLEAR until the matrix lands; supervisor points me at the
  first tranche. Prereq proven: 9-board co-located 2-TG ESP-NOW mesh LIVE. See memory
  [[tn-refutation-matrix-campaign]].
- **METAL TIER LIVE — FIRST field.* RESULT LANDED (`34aef54`).** TN-L2-IT-BL-100 (RSSI-σ mobility
  classifier, tier=hardware-exclusive) **SURVIVED on real ESP-NOW.** Built the `rssicls` firmware (real
  per-recv RSSI from r.info.rx_control.rssi → rolling per-neighbour σ → §2.4 classify σ<5dBm=Infra →
  feed obs.mobility; r2-route does the differential decay). 3 XIAO captured ~11min hearing the live
  9-board mesh: 49/49 settled (n≥20) readings σ<5dBm (min 0.19 / max 3.94 / mean 0.94) → classifier
  holds; the prior FINDING B refutation did NOT reproduce. ROBUSTNESS finding flagged to specs (worst
  link 3.94dBm ≈1dBm headroom = thin margin → metal evidence for the §2.4 hysteresis/stationary-margin
  fix). Auditable record + raw serial: `docs/field-results/TN-L2-IT-BL-100.json` (specs' capture schema).
  **NEXT: BL-200 wiring** (the first ROUTING field.*) — TrailReinforcer (`r2_route::trail`, core 7201d02)
  synced into the worktree + compiling; wire topology-mask + reply-send (normal routing + reply_marker) +
  the 3 reinforcer call-sites + decay_paths-from-tick + directed_via/exactly_once telemetry → run on 5
  ESP-NOW boards. Then BL-103 (eviction+rediscovery, reuses directed_via telemetry + blackout arm).
  KEY: metal REUSES r2-route::RouteEngine + r2_route::trail = field.* validates the REAL engine+policy.
- **BL-200 (first ROUTING field.*) DONE — PARTIAL / sim-vs-metal DIVERGENCE (`8480089`).** 5 DFR ESP-NOW,
  routetest build (full BL-200 firmware: topology MASK + §4.3.4 TrailReinforcer + A->D origin + reply
  emitter; commits 71f4f82/34efe11/141e6ad/d98fc64). PROVEN on metal: directed_via converges adjacent-to-dest
  (R2->D 20/20, flood->directed over time) + exactly_once@D (20x1) + alt-X no-steal. REFUTED: end-to-end —
  upstream A->R1, R1->R2 STAY FLOODING after 20 clean reply round-trips. The §4.3.4 reply-confirmed trail
  forms at the hop adjacent to dest (unambiguous reverse link D->R2) but NOT upstream where the reply floods
  back over un-converged paths (strong-reinforce sees varying senders -> path-to-D never concentrates).
  Routed SPEC-FIRST to specs + core (spec refinement: pin reverse next-hop? / refutation / hive wiring).
  Record: `docs/field-results/TN-L1-IT-BL-200.json` + raw serial. PROCESS: first run contaminated (demo
  lowest-hive emitted 49 Events) -> gated demo off under routetest + dropped <64,64,64> workaround (core
  9497a60 made trail generic) -> clean re-run. Baseline RESTORED (5 DFR -> multitg, rejoined TGs).
  **2 metal field.* results: BL-100 survived, BL-200 partial-divergence.** NEXT: BL-103 (eviction+rediscovery,
  reuses directed_via telemetry + blackout arm); re-run BL-200 if specs/core refine §4.3.4. LESSON: the
  metal tier earns its keep — it found a real sim-vs-metal divergence the sim 8/8 could not.
- **BL-200 RESOLVED (`bdc4d3b` fw + `bc6e029` field.*=resolved-pass).** The divergence was a ONE-LINE
  FIRMWARE BUG, not a spec gap. Root cause (metal-pinpointed via instrumented RT-DBG of core's 3 bits
  contains/sender/path-conf): the reply REUSED the request's msg_id + dedup keys on `(msg_id as u16)` ->
  reply collided with the already-forwarded request -> DROP Duplicate at every relay -> reply died at the
  hop ADJACENT to dest (still reinforced via on_received-BEFORE-dedup) -> never propagated upstream = the
  exact "adjacent-converges, upstream-floods" signature. Ruled OUT: spec gap, CAP (256>>~28), broadcast-
  overhearing (MASK isolates; core's sim silent/converged-everywhere). FIX = distinct reply msg_id
  `h.msg_id | 0x8000` (LOW-16 since dedup truncates — a first 0x8000_0000 attempt still dropped, caught on
  metal). VERIFIED isolated 5-DFR: R1->R2 directed_via, path-conf 0.66->0.96 (was flooding); R2->D 0.984;
  D exactly_once. §4.3.4 ADEQUATE (specs+core agreed). ENABLERS: MASK-NVS @0x15000 + SENDTO-NVS @0x16000
  (defeat the capture serial-open DTR-reset that wedged earlier runs) + a tight composer handshake (zero
  race). LESSON: metal found+pinpointed+FIXED a bug the SIM STRUCTURALLY COULD NOT (no u16-dedup-truncation
  nor on_received-before-dedup model). Instrument-first + spec-first prevented a canon change for a wiring
  bug. **3 metal field.*: BL-100 survived, BL-200 resolved-pass.**
- **🎉 9-BOARD CO-LOCATED CROSS-HOST MESH LIVE (0622.1517, serial-verified).** Roy directive: bring the
  4 XIAO ESP32-S3 on **alfred** into the leaderless mesh with tuxedo's 5 DFR1195. DONE. Built the SAME
  `nobt` leaderless-0.4 firmware ON alfred (esp toolchain; `source ~/Development/homelab/export-esp.sh`
  for the xtensa-esp-elf gcc — NOT `~/export-esp.sh`), flashed all 4 XIAO via espflash + the 4MB OTA
  partition table (`r2-hive/docs/dfr1195-partitions.csv`) + board-profile `0x00 0x00 @0x13000`
  (has_screen=false, led_active_low=false). Per board: ttyACM1 14:C1:9F:C4:FC:8C→af1464f4 · ttyACM2
  E8:3D:C1:FB:DB:44 · ttyACM3 D8:3B:DA:75:C3:3C→2c81b4a3 · ttyACM4 E8:3D:C1:FB:E5:20→998de7fc.
  RESULT: all 4 XIAO `synced=true nbrs=8` — each hears the other 8; peer maps include ALL 5 tuxedo DFR
  hive_ids (50:23:E4=0dcadbf8, 52:99:28=06ae082b, B6:0A:A0=f91c8911, B7:90:10=2cab5f69, 50:26:98=480e900e).
  spread 749ms→0-3ms cross-host (alfred+tuxedo, SAME ROOM) + cross-arch (XIAO+DFR1195) — RF is board-to-board,
  host-agnostic, exactly as Roy predicted. **XIAO LED = NO code change:** GPIO21 is hardcoded for BOTH
  carriers + polarity DEFAULTS active-HIGH (read_board_profile) = exactly what the XIAO external LEDs need;
  a per-target LED change would have DIVERGED the build and split the mesh. **8MB vs 4MB:** XIAO flash=8MB,
  DFR=4MB; used the 4MB table for production-parity (meshing unaffected by unused upper flash) — revisit an
  8MB layout (`docs/dfr1195-partitions-8mb.csv`) at the OTA phase.
- **STEP 3 — 2-TG per-TG keying firmware: IMPLEMENTED + COMPILES (committed; metal proof pending composer).**
  Behind a new `multitg` feature (live `nobt` demo byte-for-byte unaffected; BOTH `nobt` and `nobt,multitg`
  build green on alfred/xtensa). **Inc1 (`6e2eeca`) runtime PROVISION receive:** uart_rx_task reads the board's
  OWN USB-serial RX (composer SECURITY correction — the secret GroupHmac key must NOT go on the air like the
  IDENTIFY mesh-frame; point-to-point USB only) → `r2_trust::provision::parse_provision(line, my_wire=my_hive)`
  (core `0b44e56`, USED not re-implemented) → `write_provisioned_tg` persists {magic,tg_id,32B key} raw @0x14000
  (own 4KB sector; read-back verified) → `PENDING_PROVISION` hands the key to io_task → swaps live GroupHmac +
  target_group (no reboot); boot restores from NVS (overrides persona/demo). `tg_id`==`my_tg_hash` (fnv1a_32(UUID)
  decimal = frame target_group). ACK on serial: `PROVISION-APPLIED wire=<8hex> tg_id=<dec>` / `PROVISION-ERR`.
  **Inc2 (`5678837`) HB-signed + verify-gated coupling:** the heartbeat pulse is now `sign_extended(group_hmac)`'d
  and the io_task couple-gate flips from plaintext `target_group==my_tg_hash` to `verify_extended(&m,&group_hmac)`
  (specs §6.3 — coupling REQUIRES a GroupHmac-verified pulse). A TG-A node fails-verify a TG-B pulse → no couple
  → 2 independent sync clusters on shared RF = the logical-partition proof. **HB wire change → all-9 coordinated**
  (a multitg node won't couple to an unsigned nobt pulse → a 2-board multitg pair SELF-ISOLATES from the nobt
  mesh = a clean self-contained test). **Board→TG split (composer-confirmed):** TG-A=177560432 {D1 480e900e, D2
  2cab5f69, D3 f91c8911, X1 998de7fc/ACM4, X2 c2106bd5/ACM2}; TG-B=1584099016 {D4 06ae082b, D5 0dcadbf8, X3
  af1464f4/ACM1, X4 2c81b4a3/ACM3}. **NEXT (coordinated w/ composer):** flash a 2-board multitg pair (proposed
  ACM2=TG-A + ACM1=TG-B alfred XIAO) → composer provisions direct-to-tty → confirm NO cross-TG coupling, then
  re-provision same-TG → confirm coupling (minimal refutation), then all-9 rollout. BLOCKER: composer's
  orchestrator holds all 4 alfred XIAO ttys (the alfred dashboard feed) — it must release ports before I flash.
- **STEP 3 — METAL-VALIDATED (`4614a7a`, alfred XIAO pair, test keys over direct USB).** **Inc1 PROVEN
  end-to-end:** PROVISION-APPLIED with the correct 32B key (fingerprint key0=cc key31=cc xor=00), live
  GroupHmac+target_group install w/o reboot, NVS persist + boot-restore (`PROVISIONED TG restored from NVS
  — tg_id=1584099016`). **Inc2 verify-gate PROVEN by two controls:** POSITIVE (same key → couple) via the
  persona key (nbrs=1 when both multitg+unprovisioned); NEGATIVE (TG-A vs TG-B provisioned → HB-DBG
  `verify=false` → nbrs=0, no coupling, self-isolated from the 7 nobt boards too) = the cross-TG isolation.
  The provisioned-same-key positive is logically identical to the persona positive; composer's reliable
  provision_bridge completes it for the record. **METAL-FOUND BUG FIXED:** IDENTIFY-era uart_rx line buffer
  was `[u8;64]` → truncated the ~94B PROVISION line (key cut → BadKeyLength) → bumped to `[u8;128]`.
  **HARNESS LESSON:** my raw-tty `printf` PROVISION writes are UNRELIABLE (USB-CDC, no flow control —
  identical write = APPLIED on one board, BadKeyLength on another via byte-drop); the clean positive-control
  + all-9 rollout go through composer's reliable provision_bridge (hive flashes, composer provisions). Use
  `/dev/serial/by-id/` paths (ttyACMn renumbers on reset). **Restored ACM1+ACM2 → nobt + erased provision
  NVS → 9-board mesh WHOLE again (ACM1 nbrs=8 synced=true verified).** Commits: `6e2eeca` Inc1, `5678837`
  Inc2, `4614a7a` buffer-fix. See memory [[dfr1195-firmware-bench-workflow]].
- **CLEAN 2-TG PROOF (composer-driven) + ALL-9 ROLLOUT DONE.** composer drove the clean cross-TG proof via
  its reliable writer (prov2.py: OPOST-clean + my 128B buffer): PHASE A (X2=TG-A, X3=TG-B → both nbrs=0,
  isolated) + PHASE B (re-provision X2=TG-B same as X3 → both nbrs=1, COUPLE) = isolate↔couple driven
  purely by the GroupHmac key. Then on Roy's direct GO, the ALL-9 ROLLOUT: handshake = composer releases
  ports → hive foreground-flashes → composer provisions. hive flashed ALL 9 to the uniform multitg build
  `0622.1624mt9` (4 alfred XIAO local; 5 tuxedo DFR via `ssh tuxedo-os` with espflash binary + ELF + csv
  pre-staged in /tmp — tuxedo has no toolchain). composer provisions per fleet.json (TG-A 5 / TG-B 4) +
  renders. **HOST FACT:** this session runs ON alfred; tuxedo-os is remote (DFR-5 host, no espflash).
- **🎉 CROSS-HOST 2-TG HEARTBEAT LIVE (goal #14, metal) — directive→plan→canon→sim 10/10→metal→LIVE.**
  composer provisioned all 9 + reattached; live /r2 verdict: TG-A(177560432)={X1,X2,D1,D2,D3} all nbrs=4
  (fully coupled, cross-host alfred+tuxedo); TG-B(1584099016)={X3,X4,D4,D5} coupled (2 full + 2 marginal-RF).
  CROSS-ISOLATION CLEAN: TG-A sees 0 TG-B, TG-B sees 0 TG-A — the GroupHmac partition holds on ONE shared
  9-board ESP-NOW mesh, cross-arch (XIAO+DFR). Residual = bench RF (TG-B's 2 marginal members want the
  powered hub for tight convergence; the partition is clean). **XIAO LED FIX (Roy ground truth):** the 4
  XIAO LEDs are ACTIVE-LOW (roster said active-HIGH = WRONG) → wrote board-profile [0x00 0x01] @0x13000 on
  all 4 (byte1=0x01=active-low firmware convention; verified X3 read-flash=00 01 + boot led_active_low=true
  + TG key survived @0x14000). hive writes the polarity byte (composer's board.toml byte1 convention is
  OPPOSITE). DFR-5 = active-high (untouched). See memory [[dfr1195-firmware-bench-workflow]].

- **#1 LEAD TRACK: first real-hardware TN test on the DFR1195 rig.** Critical-path doc DELIVERED +
  CORRECTED (`45a7194`, `docs/hardware-tn-test-critical-path.md`). **TWO boards now live on tuxedo-os:
  ttyACM0 (S3 rev v0.1, MAC …26:98) + ttyACM1 (S3 rev v0.2, MAC …90:10)** — enough for hive-to-hive
  (field.lab milestone). Confirm port before flashing each. Milestone = two DFR1195s exchange one
  routed R2-WIRE frame over real radio, AND the first USB image already ships a working OTA receiver +
  2-slot partition table (Roy standing req — every later update wireless). Shortest path = WiFi-UDP first
  (core wifi.rs) → board↔board (Stage B) → wireless OTA round-trip (Stage B', composer F5 ota_push ↔ my
  OtaReceiver) → LoRa (Stage C, true infra-less TN). **SoC CONFIRMED ESP32-S3** (DFRobot wiki + SKU
  SKU_DFR1195_LoRaWAN_ESP32_S3 = ESP32-S3-WROOM-1-N4 Xtensa, 4MB, SX1262). Target xtensa-esp32s3-none-elf
  (espup Xtensa fork — the HARDER path), espflash --chip esp32s3. **I briefly mis-ID'd it as C6 from
  core's skeleton (which conflated DFR1195 with DFR1117 Beetle C6) — corrected; lesson: verify SoC vs the
  primary source, not a downstream artifact.** **BLOCKERS: (1) physical — Roy provides 2× DFR1195 (S3) +
  2.4GHz WiFi + espup-toolchain perm (+ LoRa antennas/region for C); (2) core must RE-TARGET its
  platforms/dfr1195 skeleton esp32c6→esp32s3 (flagged — its structure reuses, chip layer changes).**
  workshop's firmware/esp32-s3 is now the on-point board reference (GPIO/partitions/USB-JTAG/espflash
  mechanics/OTA self-proof). composer's S3 board.toml + 4MB OTA bound = RIGHT (un-flagged my churn).
  - **D3b division of labor AGREED with core** (Roy made the radio drivers core's top priority):
    **core OWNS** r2_transport::Transport bindings (wifi/ble/lora seam), peers.rs resolution, the SX1262
    LoRaRadio impl, and authors a first-draft esp-wifi/embassy-net bringup against the S3 pins. **hive
    OWNS** esp-hal chip/clock/heap init, esp-wifi controller + STA assoc, embassy-net Stack, flash/monitor
    loop, host-loop wiring (route_inbound_sync + sync→async bridge), the **esp-storage FirmwareSink** impl
    (OTA flash A/B + set-boot for my OtaReceiver), and metal validation + defect loop (core can't
    compile/flash — author→hive-flash→defect). **Pins:** core's matrix (esp-hal 0.23/esp-hal-embassy 0.6/
    esp-wifi 0.12/embassy-net 0.6/esp-alloc) with chip feature **esp32s3** + target xtensa-esp32s3-none-elf;
    reconcile on first metal build. **Authoring order:** WiFi-UDP → OTA → SX1262 LoRa; BLE deprioritized.
    **SX1262 = wrap a mature crate (lora-phy/sx126x) behind the LoRaRadio trait** (robustness > 'fully
    ours' for the greenfield longest-pole radio).
  - **⚡ FIRST LIGHT ACHIEVED** (`599f11b`, `docs/dfr1195-first-light-findings.md` + `dfr1195-firstlight.patch`).
    esp-hal **1.x** no_std firmware BUILDS (Alfred) → FLASHES (tuxedo ttyACM0 via SSH) → BOOTS → serial:
    "r2-dfr1195: FIRST LIGHT" + alive loop, booted from **OTA ota_0** (flashed WITH the 2-slot partition
    table → OTA-laid-out from first flash, Roy's req). **Descriptor blocker SOLVED:** esp-bootloader-esp-idf
    **0.5.0** (not 0.2.0) + esp_app_desc!(). Validated bare-metal matrix: esp-hal 1.1.1 / esp-alloc 0.10.0 /
    esp-backtrace 0.17.0 / esp-println 0.15.0 / esp-bootloader-esp-idf 0.5.0. Done in a git **worktree**
    (`~/Development/R2/dfr1195-fw-wt`); patch handed to core.
  - **⚡ WiFi/embassy MATRIX RESOLVED + COMPILES** (worktree Cargo.toml; memory [esp32-wifi-embassy-matrix]).
    The blocker was NOT a version bump: esp-wifi→**esp-radio** rename (esp-wifi 0.15.x links-collides on
    xtensa-lx-rt ^0.20 vs esp-hal 1.1.x ^0.22), scheduler esp-hal-embassy→**esp-rtos** (superseded, wanted a
    private esp-hal feature). VERIFIED set (resolves + compiles xtensa, 58s, 241K ELF): esp-hal **1.1.1**
    (unchanged) / esp-rtos 0.3.0 (esp32s3,embassy,esp-radio) / esp-radio 0.18 (default-features=false,
    esp32s3,wifi) / esp-alloc 0.10 / esp-bootloader-esp-idf 0.5.0 / embassy-net **0.9.1** / embassy-sync 0.7 /
    embassy-executor 0.10 (default-features=false) / embassy-time 0.5 / xtensa-lx-rt 0.22. **DRIFT flagged to
    core:** wifi.rs targets embassy-net 0.6 → needs same-day turn to **0.9** (IpEndpoint::from + UdpSocket::new
    /Stack lifetime). **NEXT (field.lab):** migrate main.rs bare-metal→esp-rtos/embassy async + esp-radio STA +
    embassy-net Stack, re-enable mod wifi (once core's wifi.rs@0.9), spawn udp_writer_task, wire RouteEngine →
    board A originates → board B receives+relays (dedup/TTL/spray). network-OTA receiver rides the same tier.
  - **🎯🎯 FIELD.LAB DONE — first routed R2-WIRE frame board↔board on REAL HARDWARE** (`a99313b`). WiFi-up
    smoke PASSED (soft-AP r2-fieldlab 192.168.4.1 ↔ STA .2, role auto-by-MAC), then the routed frame: board A
    (hive 502698) originates an R2-WIRE *extended* Event over real WiFi radio → board B (b79010) decodes +
    `r2_route::RouteEngine::plan_forward` + **DELIVERED msg_id=7..13 ttl=4 'hello-TN'** + **DEDUP** the
    duplicate. Stack: esp-radio 0.18/esp-rtos 0.3/embassy-0.9, one combined recv/send UDP socket task (port
    21042), static IPs. **HW finding (confirms core's B1):** RELAY ≠ DELIVERY — first cut let plan_forward's
    relay verdict (Drop NoViableNeighbour on a 2-board leaf) mask delivery; separated → delivers. Boards: my
    field.lab pair = ttyACM0(AP 502698)/ttyACM1(STA b79010), by MAC via /dev/serial/by-id; workshop's 3
    DFR1195s = ACM9/10/11.
  - **🎯 THE FLEET WORKS — synced LED heartbeats over TN** (`cb8fa14`). Both boards run a leaderless
    Mirollo-Strogatz pulse-coupled oscillator: fire = LED beat + broadcast R2-WIRE `Heartbeat` frame;
    receiving the peer's fire = advance-only phase nudge. Initialized 1.1s apart → phase-lock ~60ms apart
    (proven coupling: crystal drift <1ms/26s). Serial: AP `HB phase 0.97->1.00` then `FIRE` (pulse triggers
    fire); STA convergence `0.70->0.82->0.97->lock`, `synced false->true`. Clock = embassy_time (esp-rtos
    time-driver). composer's HeartbeatSync sentant = CONDUCTOR-PLL (std tier); mine = leaderless PCO (MCU) —
    flagged the mixed-TG model-alignment Q.
  - **LCD status surface RESTORED** (`988f0ac`) — ST7735S in the async render loop (GPIO48 active-low,
    offset 26,1, Deg90, 20MHz), shows role/ip/TG/build/beats/dlv/`fleet: IN SYNC` from atomics io_task
    updates. WiFi + routed frames + PCO heartbeat + LCD all coexist, no panic.
  - **🎯 GOAL #2 — intra-TG TRUST DELIVER-GATE working on hardware** (`045048b`). Real HMAC-SHA256
    (r2-trust `GroupHmac`, which BUILDS for xtensa — 38s, no getrandom issue) gates delivery at the B1
    deliver branch ONLY; relay stays trust-agnostic. AP originates signed intra-TG Events alternating
    good/bad HMAC; STA: `DELIVERED msg_id=6 'in-TG' (tg+hmac ok)` / `DELIVER-BLOCKED msg_id=7 hmac_ok=false
    (relay unaffected)`, consistent. Canon (core 5f8798b): `target_group = FNV-1a-32(TG_UUID string)` via
    r2_fnv const; `sign_extended`/`verify_extended` (target_group+event_hash inside the MAC). Both boards
    share TG_UUID + hk (demo stand-in for the join). LCD shows dlv/blk.
  - **TONIGHT'S ARC (all on metal, 2 boards):** WiFi ✅ · routed R2-WIRE frame (deliver+dedup) ✅ · synced
    heartbeat ✅ · LCD ✅ · intra-TG trust deliver-gate ✅ · conductor-PLL heartbeat (TG-scoped + version
    telemetry) ✅. **Both headline goals — TN + trust groups — proven + canon-aligned on real hardware.**
  - **CONTINUED-SESSION metal wins (all committed):** N-board broadcast (fire/Event → subnet 192.168.4.255,
    verified) ✅ · **unique per-board STA IP** from low MAC byte (the real N-board fix; .2 would collide) ✅ ·
    **organic lub-DUB LED heartbeat** via LEDC PWM hardware duty-fades (Roy: "heartbeat not flash"; io_task
    FIRE_SIGNAL → main renders the envelope) ✅ · **OTA bootloader CONFIRMED (test a)**: my no_std app boots
    under the ESP-IDF BL (extract first 0x8000 of /tmp/dfr1195-merged.bin → espflash --bootloader; "Loaded app
    from 0x20000" + app runs) — the OTA BL blocker is closed ✅ · esp-storage builds for xtensa ✅. STA
    (ttyACM1) now runs the ESP-IDF BL. Conductor-PLL note: locks but ~0.1-period steady-state OFFSET (tighten
    with β freq term / higher gain — refinement).
  - **MORE continued-session metal wins:** **conductor-only beaconing (NO-FLOOD)** — only the conductor beacons
    the fire, followers PLL-listen silently ✅ · **2nd-order conductor-PLL (β/freq term)** — kills the ~200ms
    offset, e→±0.005–0.025 (<50ms), 5 LEDs as ONE ✅ · **5-board mesh** (my 2 + composer's 3, ESP-IDF BL) ✅ ·
    **real-TG persona reader (#20)** — read bundle raw @0x12000, r2_cbor-decode, run on PROVISIONED hk/tg/derived-
    hive; **TG=4b3df45d OFF DEMO** on both my boards (persona=true), cond=3e0d688f, synced=true, DELIVERED good /
    BLOCKED bad on the real hk ✅. Hand-rolled derive_hive_id (HKDF→v4-UUID-string→FNV; r2_trust::derive_hive_id
    not in pinned r2-trust). **KS1-CANONICAL derive_hive_id** — re-synced r2-trust to **abde165** (the no-v4-forcing
    fix; 256489b + my hand-roll BOTH v4-forced = matched each other but DIVERGED from KS1). ids now byte-exact to
    composer: **502698→480e900e, b79010→2cab5f69** (were the wrong v4-forced 3e0d688f/cce44b60). Conductor re-elects
    to lowest (STA 2cab5f69); AP follows+locks (STA→AP broadcast direction also confirmed). r2-trust pinned abde165 ✅. **OTA test (b) PASS** —
    wrote valid image to ota_1, firmware activate_next_partition() + reboot, ESP-IDF BL booted ota_1 @0x200000;
    both OTA prereqs CLOSED; converted to report-only (production-safe). Op-note: espflash flash does NOT reset
    otadata — erase 0xf000/0x2000 to recover a board to ota_0 ✅.
  - **EVEN MORE wins (this session):** **health #18** — r2.hb.health CBOR (13-key), every-5th-beat, followers
    DIRECT to the collector AP, AP logs `HEALTH <hex>` for composer's orchestrator serial-reader; verified e2e
    (AP collects own 480e900e + STA 2cab5f69) ✅ · **shared parse_persona** — adopted r2_trust::parse_persona
    (core 1b93108), dropped my decode glue; one codebase with workshop ✅ · **carrier-aware has_screen** — LCD
    init+render gated on board-profile byte @0x13000 (0x00=XIAO no-screen, else=DFR1195); ONE binary runs on
    screenless XIAO-S3 (9-board) ✅ · **perfect sync** — 2nd-order PLL now locks to e=-0.000 (zero offset) ✅.
    r2-trust pinned 1b93108. 9-board = 5 DFR1195 + 4 XIAO-S3 (all-S3, true PLL, GPIO21 LED); role-by-MAC →
    only 502698=AP, XIAO=STA; composer flashes my binary + provisions XIAO (persona@0x12000 + 0x00@0x13000).
  - **9-BOARD MESH CONFIRMED (metal) 🎉** — composer flashed all 4 XIAO + 3 DFR1195; ALL on tuxedo USB
    (my ACM0=AP/ACM1=STA, XIAO ACM2-5, DFR1195 ACM9-11). Verified synced=true + dlv climbing (trust delivering)
    across composer's DFR1195 (ACM9/10/11 dlv~1692) AND a XIAO (ACM2) = cross-arch (S3 DFR1195 + XIAO)
    beat-as-one on real TG 4b3df45d, conductor = lowest canon id 06ae082b. AP serial held by r2-compos
    (composer orchestrator) = the health #18 dashboard feed working by design; do NOT re-flash the live AP.
  - **OTA network receiver (#17)** — DE-RISK PASSED (flash-write-while-WiFi: 20ms/sector, heartbeat-safe, no
    quiesce). Receiver built (UDP 21043 START/DATA/COMMIT stream → sector-write → SHA-256 → activate+reboot) +
    otadata anchor (Factory→ota_0 so activate→ota_1 seq=2). PROVEN: 512KB stream+write+sha_ok+valid 0xE9 image+
    activate ok + test-b slot-switch. NOT yet cleanly e2e (board-to-board boot-INTO-ota_1 snagged on test-
    corrupted otadata + can't test on the live AP). Test sender gated OFF (OTA_SELFTEST=false). Next clean
    verify: a fresh-otadata board, NOT the live soft-AP. LESSON: never re-flash the live soft-AP mid-demo.
  - **LATEST (0621.1227):** **per-carrier LED polarity** — XIAO-S3 GPIO21 is ACTIVE-LOW (inverse of DFR1195);
    profile byte1 @0x13001 (0x01=active-low; erased→active-low iff no-screen, so XIAO byte0=0x00 already works);
    LEDC idle + lub-DUB envelope polarity-mapped ✅. **#23a conductor-timeout re-elect** — forget a SILENT
    conductor after 4 beats → re-elect next-lowest; healthy conductor = no churn (replaced the churny every-3
    forget) ✅. **AP-SPOF live (#23b):** the soft-AP (502698) went dark (my live re-flash wedged it) → STAs
    stranded (no network → no app-layer election can help; my STA came up alone/CONDUCTOR). FIX = revive 502698
    (Roy physical RST; port held by composer's health reader so no remote reset). **#23b AP-FAILOVER = the real
    fix, NOT YET built:** pre-designated backup (lowest AP-capable hive from the heartbeat roster) detects
    esp-radio disassociation + promotes STA→AP at runtime @192.168.4.1; others re-scan/associate. Substantial +
    risky (runtime WiFi mode switch) — implement on a test pairing, not the live mesh.
  - **CONVERGENCE BUG FOUND + FIXED (serial-verified, 0621.1227):** the 9-board "not converged" root was a
    VERSION MISMATCH — 3 DFR1195 (ACM9/10/11) were on a STALE pre-KS1 build (0621.0858) computing WRONG hive_ids
    (a0dce700/63f798ea/b658276e) → SPLIT-BRAIN conductor election (boards disagreed on the lowest id). XIAO were
    on 0621.1148 (pre-LED-polarity → dark). FIX: re-flashed all 7 accessible boards to 0621.1227 (KS1 ids + LED
    polarity + conductor-timeout). RESULT (direct serial): 8/9 lock to cond=06ae082b (=529928/ACM10), e≈0.000,
    synced=true, cross-arch (DFR1195 + XIAO). 9th = AP 502698/ACM0 still dark on old build (port held by
    composer's health reader) → revive via Roy RST (beats+follows) or composer port-release + re-flash to canon.
    LESSON: a mixed-build fleet WILL split — keep ALL nodes on one build; verify by SERIAL not telemetry.
  - **9/9 CONVERGED + UNIFIED + AP REVIVED (0621.1244, serial-verified) 🎉** — all 9 on ONE build/span;
    single conductor = ACM10 (529928→06ae082b); all 8 others (incl the AP) lock cond=6ae082b synced=true
    e≈0.000 cross-arch (5 DFR1195 + 4 XIAO). AP 502698 revived via composer port-release re-flash → canon id
    480e900e, role=AP, beats as follower. **AP later re-wedged → composer un-wedged it (espflash-reset,
    firmware intact) → all 9 back to sync_state=1; composer fixed the dashboard feed (their plugin poll bug,
    NOT my HEALTH format — parsed all 9 byte-exact). Health dashboard LIVE.**
  - **XIAO LED FIXED + ROBUST (Roy confirmed correct).** The XIAO GPIO21 LEDs are EXTERNAL active-HIGH (not
    the built-in active-low user LED). The byte-toggle (0x13001) was FRAGILE (composer's 1-byte re-provisioning
    leaves byte1 erased → the old !has_screen inference re-inverted on every re-flash). FIX (committed, 0621.1314,
    re-flashed the 4 XIAO): read_board_profile DEFAULTS active-high — led_active_low only on byte1==0x01 explicit
    override; NEVER infer from has_screen (polarity is hardware/wiring-specific, not SoC-derivable). Robust across
    re-flash + re-provisioning. **R2-WIRE v0.6**
    (msg_id-in-HMAC-span) = deferred: SEPARATE all-9-coordinated update; current bench all on the same span.
  - **#24 BLE↔WiFi TWO-PLANE — STARTED (Roy: now the focus; AP wedged again = the motivating need).**
    Architecture settled (workshop+core, r2-route pattern): pure no_std S0–S4 negotiation ENGINE in
    **r2-discovery** (core lands it from my interface) behind a **NegotiationRadio trait**; radio glue
    per-platform (hive=esp-radio, workshop=esp-idf); protocol primitives reused (r2-wire/trust/beacon);
    reuse `lowest_live_id` (conductor election). DELIVERED: the engine interface (S0–S4 table + trait
    surface) → core, who **LANDED THE ENGINE** (r2-discovery::negotiation, 03648fb — pure no_std heap-free
    S0–S4, 4 tests green, conforms my §4A table). core's answers: engine carries its own thin roster
    (NegotiationEngine<16>); `lowest_live_id` exported; trait = poll_scan→NegObservation{hive_id,caps} /
    send_control+poll_control(HiveId) / bring_up_provider+join_provider(DataPlaneParams fixed-buf) /
    data_plane_state→TransportState / now_ms; drive eng.poll(&mut radio) each tick + request_data_plane()
    + set_power_state(); new(my_hive,my_caps,5000,10000). Eligibility source: R2-BEACON §7.2 flags — power_state
    bits 1-0 readable NOW, provider_capable bit 2 PENDING Roy's authorization (I model both). **MY NEXT = the
    esp-radio NegotiationRadio impl** (THE focus): control plane (ble HCI + trouble-host: advertise RBID+flags
    / scan / L2CAP CoC) + data plane (existing SoftAP/UDP → Available/Failed). BLE foundation scouted
    (esp-radio `ble` HCI + trouble-host/bt-hci). Big lift: deps+coex → HCI↔trouble wiring → advertise → scan
    → L2CAP, on a TEST PAIRING first. Subsumes #23/#23b (wedged AP → auto-renegotiate over BLE). §4A Profile-A.
    (AP-WEDGE cause diagnosed: esptool-flash on the LIVE AP wedges it — NOT the read-only health-reader; use
    `systemctl --user stop/start r2-orchestrator` around any AP re-flash; the durable fix is this BLE-failover.)
  - **NAMED REQUIREMENTS (roadmap, careful test-pairing — NOT on the live mesh):** #23b **AP-FAILOVER** (Roy:
    "TN should renegotiate the hotspot if it goes away") — pre-designated backup (lowest AP-capable hive from
    the roster) detects disassociation → promotes STA→AP (same SSID/IP) → others re-associate; conductor-timeout
    app-half DONE, WiFi-layer half remains open. **BLE-BEACON discovery** (R2-DISCOVERY) = the out-of-band substrate
    that solves the no-network-to-elect chicken-and-egg (beacon presence/hive_id/TG/AP-capability/roster over
    BLE, independent of the WiFi-AP) — #23 negotiation rides it. **IDENTIFY** cmd (LED solid on /r2 identify).
    **PER-CARRIER PLATFORM BUILDS — REQUIRED (Roy, reverses the earlier deprioritization).** Next firmware
    deliverable = SEPARATE DFR1195 (4MB/no-PSRAM) + XIAO (8MB/octal-PSRAM) binaries running the SAME ENSEMBLE
    (identical logic; only the platform layer differs) = unified-hive proof (logical=portable, platform=
    per-carrier). Architecture in docs/r2-per-carrier-builds.md: ONE crate, features carrier-dfr1195(default)/
    carrier-xiao; ensemble shared (no cfg) — io_task heartbeat+route+trust+persona+health+IDENTIFY+#24 engine;
    platform #[cfg]-gated — PSRAM init (xiao), LCD init (dfr1195), LED/screen. Partition flash-time (4MB/8MB
    CSVs both pushed). hive builds the 2 binaries (esp toolchain) from composer's ONE ensemble + 2 board.tomls;
    composer flashes per MAC-reservation. **The has_screen/LED bytes become #[cfg] carrier CONSTS → RETIRES
    the fragile profile-byte.** Carrier-detection boot-guard (MAC-OUI + PSRAM-probe → reject wrong-build) =
    hive's. composer leads composition (CARRIER-COMPOSITION.md, sdkconfig=Path-A/std only; my Path-B uses Cargo
    features). FOLD into the SAME next deliverable as the #24 BLE stack. (composer driving both S3 targets now.)
  - **IDENTIFY (Roy locate-a-board) — DONE + VALIDATED.** Device-side: r2.hb.identify Directed frame →
    target LED SOLID ~5s override (polarity-aware), refresh/clear. INJECT-BRIDGE (uart_rx_task): reads
    "IDENTIFY <wire_hex> <1|0>" off the USB-Serial-JTAG RX half + broadcasts the frame; runs on every board,
    composer points --identify-port at b79010. VALIDATED on b79010: RX-sharing OK (esp-println TX intact)
    + inject works. composer flipping --identify-port now (composer-side done, 7ec3706). NOTE: the device-
    side override needs the IDENTIFY build on each TARGET board (only b79010 has it now → rides the next
    fleet re-flash). sync_state→0/1/2 (composer dashboard now treats 1=locked; resolved). LED byte DROPPED
    by composer (byte1 reserved; polarity = my active-high default + a Cargo feature) — fragility gone for good.
  - **#24 BLE→WiFi — ACTIVE, 3 METAL MILESTONES HIT (Roy: push now, not parked).** Off-by-default `ble`
    Cargo feature (live fleet still builds). On b79010 (--features ble), all metal-verified:
    (1) **deps resolve+compile** — esp-radio ble+coex + bt-hci 0.8.1 + trouble-host 0.6.0;
    (2) **BLE controller inits + WiFi+BLE COEX holds** (BleConnector + WiFi mesh stays synced);
    (3) **trouble-host ADVERTISE up + EXTERNALLY SCAN-CONFIRMED** — bluetoothctl on tuxedo sees
    `Device C0:52:2C:AB:5F:69` (= my random addr, hive 2cab5f69), while the board stays WiFi-synced.
    (4) **REAL R2-BEACON codec wired + advertising** — `ble_task` uses `r2_discovery::beacon::{compute_rbid,
    encode_advert, LegacyBeacon, BeaconFlags, PowerState}` (core, byte-exact) → 24-byte canonical payload in
    the 0xFF manufacturer AD; metal: `BLE advertising R2-BEACON rbid=471a93a8.. (24 B)`; external scan
    confirms `ManufacturerData 0x01b2` (the encode_advert output, vs the old 0x3252 placeholder).
    **VERSION-COMPAT (the #1 risk) SOLVED: trouble 0.6.0 = bt-hci 0.8** (esp-radio 0.18; 0.2=bt-hci0.3 /
    0.7=bt-hci0.9 both mismatch). Built against core's **r2-discovery @9996fa3** (beacon+negotiation;
    default + --features ble both build clean). **Advertise CANON-CORRECT**: `my_key =
    derive_beacon_session_key(&hk, my_hive)` (PER-MEMBER, HKDF(hk, salt=r2-beacon-rbid-v1, info=hive_be32)[..16]
    — core fb5b189; a TG-wide key would make all RBIDs identical) → compute_rbid; metal-verified rbid changed
    per-member key, Expand-only construction @9996fa3, metal rbid=baf64d9d. epoch=0 still placeholder until a shared coarse-time base.
    (5) **SCAN + RESOLVE on metal — S0 DISCOVER COMPLETE.** ble_task ADVERTISES + SCANS concurrently
    (join3: run_with_handler + advertise + scan). R2ScanHandler.on_adv_reports → ble_find_mfg_ad →
    decode_advert → resolve_rbid_windowed(rbid, registry, epoch, 1) → hive_id. 2-board metal: ACM11
    (0dcadbf8) scans → `BLE scan -> peer hive=2cab5f69 (rbid baf6..)` resolving ACM1, both advertising +
    WiFi-synced. Full cross-board crypto chain proven. (BUG fixed: ScanSession must be HELD — its Drop
    cancels the scan.) registry=KNOWN_HIVE_IDS bring-up roster (real roster from peers.rs/persona later).
    (6) **M7 L2CAP CoC CONNECTIVITY on metal** — provider (lowest test hive 0dcadbf8) connectable-advertises →
    Advertiser::accept (ACL) → L2capChannel::accept(PSM 0x00D2); joiner (2cab5f69) central.connect →
    L2capChannel::create → send. METAL: provider `CoC RECV 7 B: [05,00,52,32,2d,4d,37]` = `[len_lo=5,len_hi=0,
    "R2-M7"]` — the LE len-prefix frame (R2-BLE §6.4) crossed BYTE-EXACT, matching workshop's esp-idf l2cap.rs
    (interop-ready). Repeatable. **So the two-plane is REAL on metal: S0 DISCOVER + control-plane data path both proven.**
    **NEXT: M8 NegotiationRadio** (re-integrate non-conn beacon + scan + HiveId↔addr map + HiveId↔Connection map +
    shared r2_discovery::ControlMsg codec [core landing]) → **M9 run S0–S4 engine** → **M10 network-forming + fallback/reform + telemetry**.
    Full plan: docs/r2-24-l2cap-implementation-plan.md.
    (7) **M8a — NEGOTIATION ENGINE LIVE on metal.** EspNegRadio (sync NegotiationRadio façade) over static
    bridge queues (SCAN_OBS/CTRL_OUT/CTRL_IN/DATA_PLANE) + engine_task running NegotiationEngine::<16>. METAL
    (ACM1): `NEG state -> Negotiate provider=Some(0x2cab5f69)` -> `Data` — the §4A S0→S1→S2 state machine RUNS,
    elected itself provider (alone, provider_capable), bring_up_provider→Available→Data (formed). Sync↔async
    bridge + engine integration PROVEN on metal. NEXT M8b: rewire ble_task to FEED the bridge — scan→SCAN_OBS
    (real peers) + conn-mgr (CTRL_OUT↔CoC↔CTRL_IN, the M7 CoC) → multi-board discover→negotiate→form; then
    M8c real WiFi bring_up/join (currently stubbed Available) + M10 fallback/reform + telemetry.
    (8) **M9 NETWORK-FORMING on metal — discover→negotiate→form, 2 boards.** Both elect 0dcadbf8 (lowest
    provider_capable, leaderless §4A.3); joiner sends WifiReq [0x01] over the L2CAP CoC → provider RECV →
    WifiOffer (7B) → joiner RECV → both reach DATA. serve_coc bridges CTRL_OUT/IN↔CoC; engine drives via the
    sync façade; shared ControlMsg codec byte-exact cross-board. Election-race fixes: continuous peer-obs
    refresh + ~3s discover-delay. **HONEST:** bring_up/join_provider STUB the WiFi (DATA_PLANE_AVAIL=true) →
    "Data" = forming-logic reaching S2, not a real SoftAP. So **discover→negotiate→FORM negotiation PROVEN on
    metal**; data-plane bring-up is M8c. NEXT: **M8c** real SoftAP/STA (runtime WiFi reconfig) → **M10**
    fallback/reform (lose-AP→S3→S4→reform) + composer telemetry (key13/14/15).
    (FIX noted: the crates index was stale → `cargo search` refreshes it before resolving trouble.)
    (9) **M8c — REAL two-board WiFi FORM on metal (BLE→WiFi network-forming COMPLETE).** Provider serves its
    own SoftAP "r2-tn-form" from boot; joiner is a STA configured for it but connects ONLY on the engine's
    join_provider (after the BLE WifiOffer) via DATA_PLANE_JOIN→wifi_task connect_async. METAL: joiner
    `data plane UP — joined r2-tn-form (REAL WiFi formed, B->W)` + provider `[ap] station joined` = a REAL WiFi
    association formed by BLE negotiation. Full chain on hardware: discover→elect lowest (0dcadbf8)→negotiate
    WifiReq/WifiOffer over the BLE L2CAP CoC→FORM real WiFi. **cfg-gated: default (mesh) build UNTOUCHED**
    (serve_ap=is_ap/r2-fieldlab/wait_config_up); ble = M8c (serve_ap=elected/r2-tn-form/form-on-negotiation).
    **THE WHOLE TN ON HARDWARE: S0 discovery + M7 CoC + M8 engine-bridge + M9 forming-negotiation + M8c REAL
    WiFi form** — it discovers, negotiates, and forms a real infra-less WiFi network. NEXT: **M10** = lose-AP →
    S3→S4→reform (self-HEALING) + composer telemetry (key13/14/15); the M8c boards form their own net
    (r2-tn-form) separate from the mesh — coordinate proof-surface wiring w/ composer at M10.
    (10) **FORM→SYNC VERIFIED ON METAL — acceptance criterion #1 COMPLETE (infra-mode).** 2 boards: discover →
    negotiate over BLE → form real WiFi → **lub-dub-SYNC together**. Joiner (2cab5f69): `HB<-192.168.4.1 cond=dcadbf8
    e=-0.000 (lock)` `synced=true dlv=5`; provider (0dcadbf8): `synced=true role=AP` `FIRE seq=27/28 (CONDUCTOR)`.
    Two fixes verified: (a) conductor-send TIMEOUT-guard (was stalling at beat 8 on SoftAP-no-STA) → fires
    continuously; (b) role-align is_ap=serve_ap → provider correctly role=AP. So discover→negotiate→form→SYNC
    works on hardware. **STRATEGIC PIVOT (Roy/supervisor): reality2-mesh ARC greenlit** (specs→core→hive) — the
    GENERAL case = ESP-NOW/WiFi/LoRa TRUE-MESH (no AP; mobile wearables, continual reform); this infra-mode
    (SoftAP-star) is KEPT as mode-1b (fixed/workshop). ESP-NOW verdict: docs/r2-espnow-mesh-verdict.md (feasible
    + favored; esp-radio has esp-now; reuses S0-M9+route+heartbeat; kills AP-role/two-IP bug). QUEUED for hive
    (after specs+core): platform Transport impls (ESP-NOW hive_id↔MAC + UDP) + mesh-mode + M10 runtime-elected-
    single-AP (infra). Rig: use /dev/serial/by-id MAC paths (provider F4:12:FA:50:23:E4, joiner F4:12:FA:B7:90:10).
  - **Per-carrier Cargo features** (composer board.toml mapping): `display` (DFR1195 LCD) + `psram` (XIAO
    octal-PSRAM@80MHz baked via PsramConfig in code — esp-hal has no psram Cargo feature); next deliverable.
  - **PRECISE NEXT STEPS:** (1) composer re-flashes its 3 with the persona-reader (personas survive app-flash)
    → all 5 OFF DEMO on the real TG; I verify 5-board real-TG sync. (2) **OTA network receiver (#17)** — the
    slot-switch is PROVEN (test b); remaining = UDP image transfer + write ota_1 with esp-radio QUIESCED
    (esp-storage#31) + sha256 + activate-on-commit; flash-touching = careful. (3) **health #18** — r2.hb.health
    CBOR, UNICAST to collector (NOT broadcast, per af4ebcb), every-5th-beat+on-change, ota_status from slot
    report. (4) dedup v0.4 (origin=route_stack[0]; future
    r2-route bump). (5) 4-board entanglement (cross-TG gate: GroupHmac first, then trial PeeringHmac; §7.5.4).
    (6) **LoRa rung** — core landed LoRaTransport (fb13b17, r2-transport/src/lora_transport.rs); impl LoRaRadio
    for Sx1262 (wrap lora-phy) → LoRaTransport::new → single-owner lora.service() in the radio task; send()=
    broadcast-on-air so RouteEngine+dedup+trust+conductor-PLL transfer UNCHANGED from WiFi. Swap the ref's
    RefCell<VecDeque> TX queue for an embassy/heapless channel (separate async radio task). Open before TX:
    region/duty-cycle gate, LBT/CAD, RXEN switch (SX1262-LORA-DESIGN.md). Ping core when starting.
  - **QUEUE (post-headline):**
    1. **OTA receiver (#17)** — plan ready (`docs/dfr1195-ota-receiver-plan.md`: OtaUpdater + esp-storage +
       UDP :21043 transfer + sha256 + software_reset). **2 go/no-go prereqs FLAGGED:** (a) espflash's default
       bootloader may not honor otadata for slot-switch → may need a custom OTA bootloader (BLOCKER candidate,
       coordinate core/workshop); (b) flash-write-while-WiFi can hang on dual-core S3 → quiesce radio around
       writes. Run the bootloader test (write ota_1 + flip otadata + reboot) before the full receiver.
    2. **Heartbeat → leaderless CONCAVE-M&S PRC** f(φ)=(1/b)ln(1+(e^b-1)φ) b=3 once specs pins v0.2 (NO rush;
       conductor-PLL holds; drop-in swap of the phase-update, keep the broadcast+jitter). (Canon flip-flopped
       v0.1 conductor-PLL → v0.2 leaderless-concave; supervisor's latest = leaderless-concave for no-SPOF.)
    3. **Real-TG provisioning** — consume composer's keystore (R2-PROVISION): replace hardcoded TG_UUID+hk +
       MAC-low3 hive_id with provisioned device_master_secret + TG persona → derive canonical hive_id
       (FNV(HKDF(secret,tg_id))) + group hk. Asked composer for the NVS layout/read API. Crypto path unchanged.
    4. **N-board scaling (#19)** — fire BROADCAST to all co-members (not 2-board unicast) + multi-peer table;
       converges with the leaderless-concave swap. Then 5-board mesh (my 2 + workshop's 3).
    5. **Health telemetry (#18)** — r2.hb.health CBOR companion (composer's HEALTH-TELEMETRY-CONTRACT), after
       OTA (needs ota_status). 6. **Entanglement** (2 TGs/4 boards, PeeringHmac, lexicographic pubkey order).
    Canon follow-ups: dedup origin = route_stack[0] self-stamp for multi-hop (3rd relay). Hardware → SPECS FIRST.
  - **⚡⚡ PROOF SURFACE WORKING on BOTH boards** (`876bb98`, `docs/dfr1195-proof-surface-learnings.md`).
    LCD + LED running on ttyACM0 (rev v0.1) AND ttyACM1 (rev v0.2). **LCD (ST7735S):** status line on top +
    event log scrolling up; 20MHz SPI, mipidsi 0.9, offset(26,1)/Deg90/inverted. **KEY find: GPIO48
    controller power is ACTIVE-LOW** (HIGH = backlit-but-dead; cost a debug cycle — in the board profile).
    **LED (mono GPIO21):** gentle heartbeat "lub-dub" = all-well (visible even when screen off). Pins:
    MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48(active-low); LED21; btn18/btn0. **PUSHED to composer via
    supervisor** to create TWO general device-SPANNING capabilities + StatusDisplay sentant: display plugin
    (ST7735S driver, contracted ed50505) + **LED indicator plugin (NEW** — mono/rgb/canvas per-board, pattern
    vocab all-well/ota/joining/error/identify; Roy: LED signals status when screen down). hive owns device
    drivers (display+LED heartbeat done; pattern-set + plugin-ization next); composer the sentant+catalogue;
    specs/core the general capability traits.
  - **r2.hw.led capability DRAFTED for specs/core** (`4a9f0dd`, `docs/r2-hw-led-capability-proposal.md`) —
    semantic CMD_SET_STATUS{status} vocab (ok/joining/ota/error/identify/idle — meanings not blink-codes);
    descriptor kind:mono|rgb + statuses + dimmable + (rgb) colour slots; device driver maps status→rendering.
    **CRITICAL (Roy): LED INDEPENDENT of display** — firmware-direct base statuses (boot/ota/error) signal
    when the screen is down → don't route LED via the render plugin. **Firmware follow-up:** init the LED
    before/around the display + a panic→error pattern, so a display fault never silences the LED. Sent specs.
  - **PROJECT: LoRa heartbeat-SYNC ("fireflies")** (`33eac83`, `docs/lora-heartbeat-sync-design.md`) — Roy's
    next showcase: synchronise the LED heartbeats via sentants exchanging r2.sync.fire events over LoRa
    (pulse-coupled oscillators). **PREREQUISITE (Roy): both nodes on the SAME TG** (events are TG-scoped) →
    needs identity (workshop hive_id/NVS) + **r2-trust no_std verify** (group-HMAC on MCU, currently std) +
    R2-PROVISION join on MCU. Deployment-reality catch (refuter): synced firing = simultaneous half-duplex
    TX = collisions → TX jitter/desync so LEDs sync tight while radio announces spread. Gated on LoRa + TG
    tiers (both downstream). **Algorithm is host-prototypable NOW** (offered to supervisor: r2-harness-style
    convergence sim + tune ε/jitter/T + partition/heal; + a TN-sync conjecture for specs). composer owns the
    HeartbeatSync sentant.
  - **FIRST-LIGHT PASS DONE (board live!)** (`db33289`, `docs/dfr1195-first-light-findings.md`). Board on
    **tuxedo-os /dev/ttyACM0**; hive on **Alfred** (esp/Xtensa toolchain); passwordless SSH = build-on-Alfred
    /flash-on-tuxedo. **SILICON-confirmed esp32s3 rev v0.1 / 4MB** (espflash board-info — settles SoC for
    good). core's skeleton **BUILDS for xtensa-esp32s3** with 3 hive fixes (patch `docs/dfr1195-s3-validation.patch`):
    C6→S3 re-target; wifi.rs:139 embassy-net SocketAddrV4→IpEndpoint; source export-esp.sh
    (`~/Development/homelab/export-esp.sh`) for the Xtensa linker. esp-hal/esp-wifi/embassy matrix compiles
    clean (no footgun). **FLASH BLOCKED:** espflash 4.4.0 requires the ESP-IDF app descriptor; esp-hal 0.23
    doesn't emit it (no bypass). **FIX = core bumps skeleton to esp-hal 1.0 + esp-bootloader-esp-idf matrix**
    (API migration; core's call — flagged + patch handed). I re-validate on metal the moment core pushes.
    Coexistence on tuxedo OK (only /dev/ttyACM0, no service restarts; workshop's :21042 untouched).
    **MATRIX DISCOVERED (cargo search):** esp-hal **1.1.1**, esp-hal-embassy **0.9.1**, esp-wifi **0.15.1**
    (restructured around NEW **esp-rtos 0.3** scheduler), esp-bootloader-esp-idf **0.5.0**, esp-alloc 0.10,
    esp-backtrace 0.19, esp-println 0.17, + embassy-* bumps. esp-wifi 0.12→0.15 = near-rewrite of the
    controller/init bringup = **core's authored domain** → handed core the migration + matrix; **hive =
    fast metal-validator** (isolated git worktree `~/Development/R2/dfr1195-fw-wt` + board + esp toolchain
    ready; core pushes → I build+flash+report in minutes). core is ACTIVELY on the skeleton (4d15812 S3
    re-target + c4927bb LoRaRadio) — do NOT touch its live working tree; validate via the worktree.
  - DONE (unblocked prep): **2-slot OTA partition table** (`3ad44e1`, `docs/dfr1195-ota-partitions.md`) —
    critical-path gap #5, hive-owned. 4MB S3: ota_0/ota_1 @ 0x1E0000 (1.875MB) + nvs/otadata/phy, fits +
    128KB headroom. FirmwareSink::slot_capacity()=0x1E0000 → OtaReceiver TOO_BIG bound. Handed to core for
    integration into platforms/dfr1195 once S3-re-targeted.
  - **Part D4: LCD display PLUGIN** (Roy directive; post-first-light, NOT blocking). DFR1195 LCD =
    **0.96in color 160×80 = ST7735S** (DFRobot wiki); pins MOSI11/SCK12/CS17/DC14/RST15/BL16/PWR48.
    Roy's split: **hive = device-specific no_std ST7735S output plugin** implementing a **GENERAL display
    capability** (render trait + descriptor: res/color-format/has-backlight/has-power-cut) that **specs
    defines + core implements** (LoRaRadio-pattern); **composer = display SENTANT + view-model** (the WHAT,
    calm-tech glanceable). General/reusable for composer's catalogue, not test-specific. Contract Qs
    answered to composer (now the GENERAL `b32d47d` DISPLAY-PLUGIN-CONTRACT-PROPOSAL, supersedes LCD-only):
    one general 'display' capability + per-board driver selected by board.toml (LoRa-carrier pattern).
    **LOCKED contract (composer `ed50505`, confirmed — final):** MANDATORY device-agnostic core = **CMD_RENDER
    (r2_cbor int-keyed view-model) + CMD_CLEAR**. OPTIONAL + descriptor-gated **CMD_BACKLIGHT(level u8 0..255,
    0=off → GPIO16 PWM)** — sentant sends it only when descriptor.backlight != 0; my ST7735S driver implements
    it; driver MAY self-manage a calm-tech default (idle-dim/wake) when none sent. **power_cut (GPIO48) =
    driver-local via descriptor flag, no command.** DFR1195 descriptor: **ST7735S / 160×80 / RGB565 /
    backlight=dimmable / power_cut=yes**. General capability TRAIT + descriptor = specs/core to define +
    ratify (LoRaRadio pattern; converged ask from composer + me); composer view-model rides on top.
    **Driver impl sequences after esp-hal-1.1 first-light.**
- **PAUSED (Roy, pending UX feedback): storing-backend / BOS-on-R2.** Branch `storing-backend` —
  RecordStore seam skeleton landed + shelved-ready (`docs/storing-backend-hive-scoping.md`). Do NOT
  build further until Roy resumes. Resume point: SQLite-behind-the-seam + persistence ensemble.
- ~~TN refutation re-run~~ DONE (`2642263`) — core `da89050` wired the knobs; re-ran both vs r2-harness:
  TN-L2-XT-BL-001 (OOM guard, set_scf_buffer_cap+tail-drop) and TN-L2-XT-AB-001 (entanglement epoch) now
  DECIDABLE → CONFIRMED. Filed to specs+core with 2 deployment-lens refinements (tail-drop vs TTL-aware
  eviction; epoch/buffer RAM-volatility). Resolution addendum in docs/phase3-tn-refutation-batch3.md.
  Standing refuter duty otherwise idle (remaining L0/L1/L3 functional cells sweepable on request).
- ~~CONVERGENCE BLOCKER: R2-WEB v0.6 CSP drift~~ **RESOLVED** (`827295b`) — Roy ratified R2-WEB v0.6 csp;
  synced hive web.rs to `WebPluginManifest.csp = Option<CspPolicy>`: `MountedBundle.csp` → `CspPolicy`,
  `build_csp`→`render_csp` (renders the directive BTreeMap), `restrictive_default` defensive fallback,
  `DEFAULT_CSP` removed, tests + integration manifests updated. BIN builds vs core's current tree; full
  workspace green (17 blocks). SECURITY FLAG to specs: §3.4.1 restrictive_default dropped
  `frame-ancestors 'none'` (+base-uri/form-action) vs the pre-v0.6 hive default → unframed web UIs now
  clickjackable unless they author csp; suggested specs re-add it. **→ RATIFIED as R2-WEB v0.7**
  (specs 5553f80): restrictive_default restores frame-ancestors 'none'+base-uri 'self'+form-action 'self'
  + adds script-src 'wasm-unsafe-eval'. `restrictive_default()` is **r2-def's (core)** — hive web.rs only
  CALLS it, so hive INHERITS the fix automatically once core updates r2-def (flagged core; no hive code
  change for the default). **hive v0.7 follow-ups (low pri, behind firmware lead):** (a) re-add the
  `frame-ancestors 'none'` assertion to web_plugin_integration test once core's restrictive_default emits
  it; (b) connect-src `+ws` serve-time append (render_csp adds hive's live WS origin when serving).

## Done + green
- **v0.2 migration + relay handshake + 4 vector fixtures** — full r2-hive suite GREEN; on
  `v0.2-relay-handshake` (pushed). Fixtures all specs-verified + landing: host-api (28),
  usb (specs), usb-pair (12 → canonical home **R2-PROVISION §5.3.4**), plugin-web (11, Ed25519).
  Generators: `crates/r2-hive-bin/examples/gen_{host_api,usb_pair,plugin_web}_vectors.rs`.
- **core D3a synced + relay driver CONFIRMED** (`3c5ba9c`) — core's WebSocketTransport §4.4.1 fan-out +
  UDP-LAN are now REAL (core `52b0e4e`). hive's relay driver (`compat/handshake.rs`: v0.1/v0.2 Ed25519
  handshake → `peers().connect()`→OutboundRx, `push_inbound` on recv, drain `outbound_rx.next()`→ws.send,
  `remove_peer` on cleanup) builds + runs GREEN against the real machinery (was scaffold). One core
  API-drift fix: `WebPluginManifest.subscriptions` added to 3 test manifest builders. Full suite green.
- **Transport + router integration tests** (`11443cf`,`828b419`) — filled a zero-coverage gap now that
  core D3a transports are real. `tests/transport_integration.rs` (3): HiveState send path round-trips
  over REAL loopback UDP-LAN sockets (set_udp_transport + send_to_hive_via → Wifi slot), no-transport→None,
  Wifi-hint routing. `tests/router_integration.rs` (5): route_frame NotR2Wire rejection, the 32-byte
  HMAC-tag trim fallback, valid-frame routing, and engine dedup (seeded neighbour → flood then dup-drop).
  Transport layer now VERIFIED working against core's real machinery, not just compile-green.
- **USB spec citations resolved** (`4c70d2c`,`8f31231`) — usb_pair/usb/main/usb_serial/usb_hotplug/api.rs
  all R2-HIVE §6.4.x → R2-PROVISION §5.3.4 (specs ruled it the canonical pairing home); R2-USB v2→v0.1.
  Type-byte divergence: specs RULED **ratify** as R2-USB §3.2.1 (don't drop; collision-free). Both
  wire extracts (type-byte table + CAPS + legacy detection; PAIR_* msg vocab + CBOR layout) committed
  `docs/r2-usb-wire-extract-for-specs.md` (`5232e61`) + sent to specs. Spec authoring is Roy-gated.

## In flight — Platform-trait extraction (north-star convergence step 1)
Split today's std hive → `r2-hive-core` (no_std+alloc host loop) behind a `Platform` trait +
thin platform layers (linux first). Verifiable on Linux now; foundation for esp32/wasm/unoq.
- DONE seams: 1 = clock (`69ab8fb`), 2 = RNG (`04d19cc`), 3 = **transports** (`1e24da8`):
  `src/platform.rs` (`Platform` trait + `LinuxPlatform`); `HiveState.platform` (default,
  no `new()` sig change); `src/transport_seam.rs` (`HiveTransports` trait = outbound
  multi-transport contract, `HiveState` impls it, `&dyn` proven). 100 lib tests + full suite green.
- DONE: **sync host-loop seam** (`sync_host.rs`, `683241f`) — `SyncTransport` trait
  (`kind`/`send`/`poll_recv`) + `TransportAddr`/`InboundFrame` + `provisional_hive_id` +
  `poll_inbound` tick primitive; Linux-verified via sync-stub. **TRANSITIONAL local mirror** of
  the seam core+hive AGREED (R2-DISCOVERY §5 sync). Core will EXTEND r2-transport
  (`Transport::poll_recv` default-None + TransportAddr/InboundFrame) → then delete the mirror,
  import `r2_transport::`. Host resolves source_addr→hive_id; driver-owned RX buffer.
- DONE: **RouteEngine wired into the sync host loop** (`route_inbound_sync`, `3ebdb61`) — parse
  R2-WIRE → ingest neighbour → `plan_forward` → execute Drop/DeliverOnly/Directed/Flood over
  `SyncTransport`; routing-only (no ensemble/TG/WS host bits); host-centralised resolution
  (specs-confirmed conformant, R2-DISCOVERY §5). Linux-verified end-to-end (real RouteEngine +
  sync-stub relay). 106 lib tests, full suite green.
- DONE: **`r2-hive-core` crate split started** (`a05b108`) — new `#![no_std]`+alloc crate (deps
  r2-wire/route/fnv only, no tokio/axum/std-net); **`sync_host` moved into it and compiles no_std**
  = PROOF the routing host-loop is MCU-portable. bin depends on it + re-exports `sync_host`
  (zero churn). Full workspace green (r2-hive-core 6 tests + bin suite).
- DONE: **Platform + transport seams migrated into r2-hive-core** (`234fd60`) — `Platform` trait
  (clock+RNG) → `core/src/platform.rs` (no_std), `LinuxPlatform` impl stays in bin + re-exports trait;
  `HiveTransports` outbound seam → `core/src/transport_seam.rs` (async-trait, no_std+alloc, needs
  `alloc::boxed::Box`), `HiveState` impl + `&dyn` trait-object test stay in bin (`hive.rs`).
  r2-hive-core builds no_std; full workspace green (100 bin lib + 6 core tests). Pushed.
- DONE: **storage seam migrated into r2-hive-core** (`b42658c`) — `core/src/identity.rs` (no_std+alloc):
  `MasterSecret` derivation (HKDF-SHA256 → hive_id/DEV_PK/DEV_SK), `DerivedIdentity`, fingerprint, UUIDv4,
  web-auth-key + the seam itself (`IdentityStore` trait, `StoreBackend`, platform-neutral `StoreError`
  replacing `io::Error` at the trait boundary). bin keeps std stores (`FileStore`/`KeyringStore`/
  `auto_store` + permissions/XDG/getuid), impls the core trait (io→StoreError), re-exports core types
  (mgmt::identity::* unchanged). RNG stays platform-side (getrandom→`from_bytes`); `bytes()` →
  documented storage-only `expose_secret_bytes()`. ed25519-dalek/hkdf/sha2/zeroize added to core
  default-features=false. r2-hive-core no_std; full workspace green (94 bin lib + 13 core tests).
- DONE: **OTA-receiver seam in r2-hive-core** (`354f395`) — `core/src/ota.rs` (no_std), the portable
  half of the firmware receiver: constants (OTA_PORT 21043/CMD_*/STATUS_*/PREAMBLE_LEN),
  `OtaPreamble::parse` (image_len u32 LE + sha256[32]), `OtaError` CODEs (PREAMBLE/TOO_BIG/BAD_MAGIC/
  SHA_MISMATCH/WRITE_FAIL/NO_SLOT/SHORT) + alloc-free `encode_reply/ok/error`, `FirmwareSink` trait
  (storage seam = flash I/O), `OtaReceiver` state machine (TOO_BIG bound-check BEFORE begin, streaming
  SHA-256, verify→finalize, abort-on-error). NOT a migration (no OTA code existed in bin) — built from
  core's `platforms/esp32/src/ota_tcp.rs` reference + composer's OTA-REPLY-STATUS-CONTRACT. 11 tests.
  Heads-up sent to composer to confirm CODE set / push-side framing. **Platform supplies:** embassy-net
  byte reads + esp-storage `FirmwareSink` impl (device); host uses a RAM mock. CMD_QUERY handled by
  platform layer (build info), not core.
- NEXT: with routing/identity/OTA cores all no_std + **5 seams** in place (sync_host, platform,
  transports, identity, ota), the convergence's host-side factoring is largely done. Remaining is
  firmware-tier (gated): swap `sync_host` seam mirror → `r2_transport::` when core EXTENDs r2-transport
  (poll_recv default-None + TransportAddr/InboundFrame); esp-hal/embassy board crate (P0) + esp-storage
  FirmwareSink + embassy-net OTA host loop (needs xtensa toolchain + hardware + core D3b).

## Next major phase — D2: DFR1195 (ESP32-S3) firmware, Path B pure no_std (esp-hal/embassy)
Gated on the convergence above + core's D3b. Sketch: `docs/esp32-hive-firmware-architecture.md`.
- Firmware = core's no_std stack + core's **D3b** no_std SYNC radio bindings, wrapped in an
  esp-hal/embassy host loop. Consume **R2-TRANSPORT SYNC** (R2-DISCOVERY §5), not async §4.
- hive owns: board layer (SX1262 LoRa / LCD / IO18 button), on-device host loop, **no_std OTA
  receiver** (embassy-net; std `ota_tcp.rs` is reference only). **Validation handoff:** core
  authors D3b but can't flash — **hive validates on real DFR1195**, feeds defects back.
- **Identity:** my firmware CONSUMES the shared `r2-esp/hive_id` module (workshop-owned, one impl per
  north-star) — incl. the agreed `usb_link_id = HKDF(master_secret,"r2-usb-link-v1")` (stable USB-link
  id) / `mesh_hive_id = HKDF(master_secret,info=tg_id)` split. Do NOT fork a parallel derivation. Gated
  on specs ratifying R2-USB §3.6 (workshop holds the change until then).
- Near-term scope flag: r2-def/ensemble/dispatch are std-tier → initial MCU hive is
  ROUTING+TRANSPORT only (no on-device ensembles) until those are re-tiered no_std.
- References (std, patterns not code): core `platforms/esp32`, workshop `firmware/esp32-s3`.

## Pending Roy / cross-repo
- **OPEN — CAPS device-identity gap: CONFIRMED REAL, fix agreed, spec-first** (awaiting specs §3.6
  authoring, Roy-gated). ROOT CAUSE (workshop firmware answer): ESP32 derives `hive_id_bytes =
  HKDF(master_secret, info=tg_id)` = TG-SCOPED, and the SAME 16 bytes feed CAPS §3.6 + my link-key store
  key + reconnect HMAC + mesh hive_id (§6.2.1). Cross-TG provisioning → different value → my LinkKeyStore
  (keyed solely on CAPS hive_id_bytes) misses → silent forced re-pair. AGREED FIX (workshop owns,
  r2-esp/hive_id.rs): split — `usb_link_id = HKDF(master_secret,"r2-usb-link-v1")` STABLE/TG-indep → CAPS
  + link-key store; `mesh_hive_id = HKDF(master_secret,info=tg_id)` → mesh. **My host needs ZERO change**
  (store keys on whatever stable CAPS id arrives). PROPOSED NORMATIVE RULE relayed to specs: CAPS
  hive_id_bytes MUST be stable for device life + TG-independent; mesh hive_id (§6.2.1) is separate →
  R2-USB §3.6 + R2-WIRE §6.2.1 cross-ref; composer also a consumer (provisioning/OTA). workshop HOLDS
  firmware change until specs ratifies §3.6 wording. Minor: dev devices paired pre-fix do a 1-time
  re-pair (harmless pre-launch). eFuse-MAC comment already marked impl-defined-pending-spec (`b33547f`).
- ~~Roy: greenlight R2-PROVISION §5.3.4~~ DONE — specs confirms COMMITTED (`4b74b20`, v0.6, Roy
  green-lit) on `spec-conformance-v0.2`. Cite by paragraph name (no §5.3.4.y sub-numbers).
- ~~hive usb_pair.rs citation fix~~ DONE (`4c70d2c`) — usb_pair.rs §6.4.x → R2-PROVISION
  §5.3.4 (SAS verification/Link key/Reconnect/Key agreement); main.rs+usb_serial.rs "R2-USB v2" →
  "R2-USB v0.1", SYNC frame → §3.3. Doc-only; builds clean.
- ~~OPEN: type-byte divergence + usb.rs frame-vocab mapping~~ **CLOSED — RATIFIED + VERIFIED.** specs
  authored all three (`71ee053` spec-conformance-v0.2, Roy-authorized): **R2-USB v0.2** §3.3 version
  negotiation / §3.5 type byte / §3.6 CAPS / §3.7 control + Appendix A transport kinds; **R2-PROVISION
  v0.7 §5.3.4** message vocabulary (PAIR_* 4-11). I VERIFIED both against usb.rs — all bytes match (CAPS
  keys, msg fields, nonce_rc/tag b16, abort vocab exact 8-match). **Both normative tightenings specs
  added were ALREADY honoured by the impl:** (a) failed reconnect does NOT fall back to first-attach
  (`usb.rs:846-848` → fail_pairing→Closed); (b) AutoPairUnsafe NOT default (Strict default; dev-only
  ctor used only in tests; prod watcher `usb_hotplug.rs:590` = Strict). usb.rs cites finalized
  (`12c6a43`): 'pending ratification' dropped, framing→§3.5-3.7, pairing→§5.3.4. Impl is now CANON.
- **Deps:** core **D3b** (no_std sync BLE/WiFi/LoRa) = hard blocker for radios; composer = OTA
  push + carrier + ensemble; specs = hw test defs.
- Phase-3 adversarial-refuter role (deployment reality): FILED first batch to specs (the 5
  high-value TN conjectures). Two systemic findings — (A) must_text bounds by TTL/time, never
  MEMORY (MCU RAM = fixed tables+eviction; fixed-size dedup evicts before window W); (B) hop-TTL
  ≠ wall-clock (a carried frame's hop-TTL never decrements while carried). Verdicts:
  TN-L2-IT-BL-001 + TN-L2-IT-AB-001 FALSIFIED-as-stated; BL-002/XT-BL-001/L1-IT-BL-004 REFINE.
  + sim-tier-decidability flag (sim needs bounded-mem + carry-time model, else mark tier=hardware).
  Awaiting specs adjudication; more conjectures can be reviewed on request.
  DYN-family batch (v0.3, 13 conjectures) ALSO filed: grounded vs real r2-route (f32 + libm::expf,
  multiplicative c+0.2*(1-c), mobility is an engine INPUT not RSSI-classified). Findings: (A)
  TN-L0-IT-BL-100 spec-vs-impl — must_text additive +0.1 vs impl multiplicative +0.2*(1-c) [core
  reconcile]; (B) TN-L2-IT-BL-100 RSSI-sigma classifier UNREALIZED + fragile under real RSSI noise
  → tier=hardware [strongest]; (C) soft-float expf cost on no-FPU (ESP32-C6); (D) fixed-point future
  → 0.05*(1-c) underflow (TN-L2-IT-BL-101). DYN batch ADJUDICATED by specs (`a9c28b1`): 3 new
  R2-ROUTE issues (8→11) — additive-vs-multiplicative BLOCKED+Roy-gated, RSSI-sigma re-tiered
  HARDWARE, expf/fixed-point forward-flagged.
  **BATCH 3 FILED** (`d161054`, docs/phase3-tn-refutation-batch3.md) — un-refuted SCF + XT/entanglement
  cells, grounded in real r2-route + r2-harness code. Key: RouteEngine has NO buffer/queue/entanglement
  (ForwardAction lacks a Queue variant; no-path → Drop(NoViableNeighbour) = silent drop); entanglement
  is SIM-ONLY (r2-harness live:bool, honesty #6; r2-trust §7 = no keep-alive/@entangled routing).
  Verdicts: TN-L2-IT-BL-002 FALSIFIED (no queue); TN-L2-IT-AB-000 FALSIFIED for carry>60s dedup;
  TN-L2-XT-BL-001 OOM-guard not sim-decidable (re-tier hw); all XT-AB cells test sim gate not
  authenticated crossing (passes-while-violating-spirit); BL-101 CONFIRM / BL-100 FALSIFY (no
  heartbeat → entangled-but-unreachable on duty-cycled links); XT-AB-001 undecidable (no instance id);
  XT-BL-100 'kept' conflicts w/ 30min route eviction.
  **BATCH 3 ADJUDICATED** (supervisor, verdict-of-record; catalogue write pending perm): IT-BL-002
  ACCEPT-FALSIFIED → R2-ROUTE #7 (MUST → named SCF layer, DUAL bound RAM×TTL; engine silent-Drop OK at
  routing layer); IT-AB-000 ACCEPT-FALSIFIED → operative rule = IT-AB-001 (idempotency at dispatch);
  IT-BL-000/XT-BL-000 = PRODUCTION-UNREALIZED (sim tests logic only, lifts no impl signal); XT-BL-001
  ACCEPT not-decisive → experiment revised (inject buffer cap; true OOM=hardware); XT-AB cells honesty-#6
  (authenticated-crossing MUSTs deferred to r2-trust §7 production); **XT-BL-100 entangled-but-unreachable
  = HEADLINE** → BLOCKED impl-missing (§7.3 keep-alive DEFINED-unimplemented); 3 Roy options, supervisor
  recommends implement §7.3 minimal keep-alive (decay-exemption REJECTED-leaning — contradicts BL-101);
  XT-AB-001 ACCEPT sim-undecidable → instance/epoch id (harness + R2-TRUST §7.6, Roy-gated); XT-BL-100
  NOT-falsified CLARIFIED (record-retention §7.3 vs route-eviction R2-ROUTE 2.5 both defined, no conflict).
  Remaining open cells: IT/XT main-path L0/L1/L3 functional cells (lower deployment-lens value) on request.

## Resume hygiene
Keep this current. WIP-checkpoint + push `platform-trait` periodically. Safe git only:
named `git add` / `git add -u` — never `git add -A`/`.`; never stage secrets.
