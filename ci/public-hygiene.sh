#!/usr/bin/env bash
# Public-content hygiene guard (Roy's ruling as a green check, not memory).
#
# r2-hive is a PUBLIC repo. The pilot-site location name + te-reo terms are scrubbed at HEAD pending
# the Māori-reviewer gate (see commit 56a9458; provenance is in gitignored .r2-local/). Restoration is
# a deliberate, reviewed act — this guard fails the build on ACCIDENTAL reintroduction via a re-vendor
# or a careless commit.
set -euo pipefail

# Allowlist: the two preserved wire/code identifiers (documented in the scrub commit) that legitimately
# still contain the token and are pending a separate coordinated code/wire rename, NOT a doc-scrub.
ALLOW='wairoa_as923_nz|wairoa\.reading'
# Exclude the guard's own files (they necessarily contain the patterns) from the sweep.
PATHSPEC=(':!ci/public-hygiene.sh' ':!.github/workflows/public-content-hygiene.yml')

fail=0

# (1) Scrubbed location + cultural terms (case-insensitive), minus the allowlisted identifiers.
hits=$(git grep -inE 'wairoa|kaitiaki|marae' -- . "${PATHSPEC[@]}" | grep -viE "$ALLOW" || true)
if [ -n "$hits" ]; then
  echo "::error::scrubbed term(s) reintroduced (Wairoa / kaitiaki / marae). Restoration is gated on the"
  echo "::error::Māori-reviewer review — do not re-add in a normal commit. Offending lines:"
  echo "$hits"
  fail=1
fi

# (2) Te-reo macron signal (ā ē ī ō ū, upper/lower) — a strong indicator of unreviewed te-reo prose.
if macrons=$(git grep -inP '[\x{0101}\x{0113}\x{012B}\x{014D}\x{016B}\x{0100}\x{0112}\x{012A}\x{014C}\x{016A}]' -- . "${PATHSPEC[@]}" 2>/dev/null); then
  if [ -n "$macrons" ]; then
    echo "::error::macron character(s) found (te-reo signal). Route te-reo content through the reviewer gate:"
    echo "$macrons"
    fail=1
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "public-content-hygiene: FAIL"
  exit 1
fi
echo "public-content-hygiene: OK (no scrubbed terms outside the allowlist; no macrons)"
