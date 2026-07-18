# Vendored conformance vectors — SYNC POINTER

These four JSON files are **read-only vendored copies** of canonical test
vectors that **r2-specifications owns**. They live here so r2-hive's test
suite builds and runs **hermetically** — without requiring an
`r2-specifications` sibling checkout on disk (which the CI runner, a fresh
clone, and any future open-source consumer do not have).

## Canonical source (specs owns these — never edit here)

```
r2-specifications/testing/test-vectors/
    r2-host-api-vectors.json      → host_api_conformance.rs (replayed) + vector_coverage.rs
    r2-usb-vectors.json           → vector_coverage.rs (min_referenced=9, cited in src/usb.rs)
    r2-usb-pair-vectors.json      → vector_coverage.rs (pinned, USB-2 phase)
    r2-plugin-web-vectors.json    → vector_coverage.rs (web plugin conformance)
```

Vendored 2026-07-18 from r2-specifications @ 7b13594 (R2-USB v0.26; superseded e03df92).
Re-sync scope: r2-usb-vectors.json v0.7→v0.26 (§2.2 kind-unify TV21/22, TV8
behavioural correction reset→preserve, new TV27 observation + TV28-30 tier
vectors, duplicate-ID fix, TV31 region+properties CAPS vector, TV21
usb_frame_hex kind 1→2 completing the v0.8 unification) and r2-usb-pair-vectors.json →v0.16 (SIMPLE SECURE
pairing: UP1-8/13/14/18 active, UP9-12/15-17 retired to active:false). Only
r2-usb-vectors.json + r2-usb-pair-vectors.json moved; the other two vendored
files were already at canon. specs-confirmed pin (specs owns these vectors).

## Why vendored, not read live (the finding this fixes)

`host_api_conformance.rs` previously did a **compile-time** `include_str!`
of `../../../../r2-specifications/…`, and `vector_coverage.rs` read the same
sibling path at runtime. That made the ENTIRE `cargo test --workspace` build
non-hermetic: it compiled only on a machine with r2-specifications checked
out adjacent to r2-hive. It passed locally (dev boxes have the sibling) and
**failed the whole workspace test build in hosted CI** (run 28778301868) —
the exact false-green the modernized ci.yml exists to catch. Vendoring +
this pointer matches the fleet's established norm (core vendors its own
`crates/*/vectors/` with a `_SYNC.md`); it is uptake of canon, not
divergence from it.

## Re-vendor discipline (the drift guard)

specs OWNS these vectors. When specs changes a vector file:
1. copy the updated file(s) here verbatim (no local edits — ever);
2. update the `@ <sha>` line above;
3. re-run `cargo test --workspace` both modes — `vector_coverage.rs`'s
   `min_upstream` floors guard against an accidental shrink at vendor time.

The fleet norm is a heads-up from specs to hive when a consumed vector
moves (the same cross-repo sync courtesy as the vendored core crates —
specs has RECORDED this consumer-notify obligation, 2026-07-06).

## Drift alert (specs-blessed backup to the heads-up)

`ci/check-vendored-vectors.sh` compares these copies against the canonical
sibling and ALERTS on divergence (it never auto-syncs — the pin is
deliberate; reproducible CI must not follow canon HEAD). It is
hermetic-safe: where the r2-specifications sibling is absent (CI, a clean
clone) it exits 0 with a note. Run it where canon is on disk:

```
./ci/check-vendored-vectors.sh            # alert-only
./ci/check-vendored-vectors.sh --strict   # exit 1 on drift
```

## Secret-scanner note

`r2-usb-pair-vectors.json` contains synthetic `secret`/`shared_secret`
fields (deterministic TEST-ONLY constants + their crypto outputs — see the
file's own `description`). `.gitleaks.toml` allowlists this path (specs'
ratified fleet pattern). The fleet's LOCAL pre-push bash hook is a separate
failsafe that does NOT read that allowlist, so a re-vendor push needs
`FLEET_SKIP_SECRET_SCAN=1 git push` until the shared hook learns allowlists.
