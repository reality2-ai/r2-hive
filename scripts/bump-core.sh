#!/usr/bin/env bash
# bump-core.sh — move r2-hive's pinned r2-core rev (the deliberate-uptake gate).
#
# WHY THIS EXISTS: r2-hive consumes r2-core as git deps pinned to ONE rev in
# the root Cargo.toml's [workspace.dependencies] (Roy-ratified model, 2026-07-06;
# retires the live path-dep instant-breakage failure mode). This script is the
# only sanctioned way to move that pin: it refuses un-pushed or CI-red revs,
# moves every rev line atomically, and commits only if hive's full suite and
# the public-hygiene gate pass.
#
# USAGE:  ./scripts/bump-core.sh <core-sha> [--force-ci]
#   <core-sha>   full or short sha on the r2-core remote (must be pushed)
#   --force-ci   skip the hosted-CI-green check (use ONLY when core's CI has
#                no run for the sha AND you have local verification instead;
#                say so in the commit you make)
#
# INTERLINKS: Cargo.toml [workspace.dependencies] (the pin lines this edits);
# .cargo/config.toml (git-fetch-with-cli auth); the commented [patch] escape
# hatch in Cargo.toml (local-loop alternative during migrations).
set -euo pipefail

REPO="reality2-ai/r2-core"
MANIFEST="$(cd "$(dirname "$0")/.." && pwd)/Cargo.toml"
SHA_IN="${1:?usage: bump-core.sh <core-sha> [--force-ci]}"
FORCE_CI="${2:-}"

# Resolve to the full sha on the REMOTE (structural guard: pin only pushed revs).
FULL=$(git ls-remote "https://github.com/${REPO}.git" | awk '{print $1}' | grep -i "^${SHA_IN}" | head -1 || true)
if [ -z "$FULL" ]; then
  # Not a branch/tag tip — ask the API whether the commit exists remotely.
  FULL=$(gh api "repos/${REPO}/commits/${SHA_IN}" --jq .sha 2>/dev/null || true)
fi
[ -n "$FULL" ] || { echo "ERROR: ${SHA_IN} not found on ${REPO} remote (un-pushed?)"; exit 1; }
echo "target rev: $FULL"

# CI-green precondition (never pin a red / un-CI'd rev).
if [ "$FORCE_CI" != "--force-ci" ]; then
  CONCLUSION=$(gh run list -R "$REPO" --commit "$FULL" --json conclusion --jq '.[0].conclusion' 2>/dev/null || echo "")
  if [ "$CONCLUSION" != "success" ]; then
    echo "ERROR: no successful CI run for $FULL on $REPO (got: '${CONCLUSION:-none}')."
    echo "       Wait for green, pick a green sha, or re-run with --force-ci + local verification."
    exit 1
  fi
  echo "CI: green at $FULL"
else
  echo "WARNING: --force-ci — hosted-CI check SKIPPED; record local verification in the commit."
fi

OLD=$(grep -oE 'rev = "[0-9a-f]{40}"' "$MANIFEST" | head -1 | grep -oE '[0-9a-f]{40}')
[ -n "$OLD" ] || { echo "ERROR: no rev pin found in $MANIFEST"; exit 1; }
N_OLD=$(grep -c "rev = \"$OLD\"" "$MANIFEST")
echo "moving pin: $OLD -> $FULL ($N_OLD lines)"
sed -i "s/rev = \"$OLD\"/rev = \"$FULL\"/g" "$MANIFEST"

# Consistency guard: every rev line must now carry the new sha.
STRAGGLERS=$(grep -cE 'rev = "[0-9a-f]{40}"' "$MANIFEST")
MOVED=$(grep -c "rev = \"$FULL\"" "$MANIFEST")
[ "$STRAGGLERS" = "$MOVED" ] || { echo "ERROR: mixed revs after sed — aborting"; git checkout -- "$MANIFEST"; exit 1; }

cd "$(dirname "$MANIFEST")"
echo "building + testing against the new pin..."
cargo test --workspace 2>&1 | tail -3
./ci/public-hygiene.sh

git add Cargo.toml Cargo.lock 2>/dev/null || git add Cargo.toml
git commit -m "chore: bump r2-core pin ${OLD:0:7} -> ${FULL:0:7}

Deliberate uptake via scripts/bump-core.sh (CI-green gated$( [ "$FORCE_CI" = "--force-ci" ] && echo ' — FORCED, see local verification note' )).
Full workspace suite + hygiene green against the new pin."
echo "DONE — pin moved and committed. Push when ready."
