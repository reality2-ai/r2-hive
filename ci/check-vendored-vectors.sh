#!/usr/bin/env bash
# check-vendored-vectors.sh — drift alert for the vendored conformance vectors.
#
# WHY (specs-requested, R2-BUILDMODE vendoring ruling 2026-07-06): r2-hive's
# tests/vectors/ are READ-ONLY pinned copies of r2-specifications' canonical
# test-vectors (see crates/r2-hive-bin/tests/vectors/_SYNC.md). The pin is
# DELIBERATE — reproducible CI requires it; the suite must NOT auto-follow canon
# HEAD. But a pinned copy can silently fall behind. This script is the ALERT
# specs blessed: it compares each vendored file against the canonical sibling and
# shouts if they diverge. It never edits, never auto-syncs — re-vendoring stays a
# deliberate human/agent step (copy + bump _SYNC.md + FLEET_SKIP_SECRET_SCAN=1).
#
# HERMETIC-SAFE: if the r2-specifications sibling is absent (a clean clone, a CI
# runner — exactly where the hermetic build must still work), this exits 0 with an
# informational note. The pin is authoritative there; drift can only be checked
# where canon is on disk (dev boxes, a scheduled fleet job with the sibling).
#
# USAGE:  ./ci/check-vendored-vectors.sh          # alert-only (exit 0 always where checkable-clean)
#         ./ci/check-vendored-vectors.sh --strict # exit 1 on drift (for a gated context)
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENDORED="$ROOT/crates/r2-hive-bin/tests/vectors"
# Canonical source: r2-specifications as a sibling of the r2-hive checkout.
CANON="$ROOT/../r2-specifications/testing/test-vectors"
STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1

if [ ! -d "$CANON" ]; then
  echo "check-vendored-vectors: canon sibling not present ($CANON) — cannot check drift here."
  echo "  (the pin is authoritative on this host; run where r2-specifications is checked out.)"
  exit 0
fi

drift=0
missing=0
for f in "$VENDORED"/*.json; do
  name="$(basename "$f")"
  src="$CANON/$name"
  if [ ! -f "$src" ]; then
    echo "⚠ check-vendored-vectors: $name has NO canonical source at $src (renamed/removed upstream?)"
    missing=1
    continue
  fi
  # Version-gap check (specs-requested 2026-07-18): compare the `version`
  # field mechanically, not just bytes. A byte-identity check is a
  # point-in-time snapshot a later canon push can RACE (a vendored copy can
  # be byte-identical to canon at the instant you check, then canon moves);
  # the version field surfaces the gap as a plain "vX vs vY" a race can't
  # hide. Complements the byte-diff below (which catches same-version drift).
  ver_re='"version"[[:space:]]*:[[:space:]]*"[^"]*"'
  vv="$(grep -m1 -oE "$ver_re" "$f" 2>/dev/null | grep -oE '"[^"]*"$' | tr -d '"')"
  cv="$(grep -m1 -oE "$ver_re" "$src" 2>/dev/null | grep -oE '"[^"]*"$' | tr -d '"')"
  if [ -n "$vv" ] && [ -n "$cv" ] && [ "$vv" != "$cv" ]; then
    echo "⚠ VERSION GAP: $name vendored v$vv vs canon v$cv — re-vendor + bump _SYNC.md @ specs sha."
    drift=1
  fi
  if ! diff -q "$f" "$src" >/dev/null 2>&1; then
    echo "⚠ DRIFT: $name differs from canon — re-vendor (copy $src → tests/vectors/, bump _SYNC.md @ specs sha)."
    drift=1
  fi
done

if [ "$drift" -eq 0 ] && [ "$missing" -eq 0 ]; then
  echo "check-vendored-vectors: all $(ls "$VENDORED"/*.json 2>/dev/null | wc -l | tr -d ' ') vendored vectors match canon — no drift."
  exit 0
fi

echo ""
echo "Vendored vectors have drifted from r2-specifications canon (or lost their source)."
echo "Fix: re-vendor per crates/r2-hive-bin/tests/vectors/_SYNC.md, then re-run the suite both modes."
[ "$STRICT" -eq 1 ] && exit 1
exit 0
