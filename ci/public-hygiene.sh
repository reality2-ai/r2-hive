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

# (3) Private gateway-product naming (Roy-gated private spec, authorized by supervisor). NARROW: the
# resident-premises gateway product + its private spec name MUST NOT appear in the public tree (provenance
# lives in gitignored .r2-local/). The broad historical Mariko/Earthgrid ID scrub is a SEPARATE Roy decision
# and is deliberately NOT guarded here (some are functional/commit identifiers with real rename cost).
gwhits=$(git grep -inE 'mk-?homehub|home-?hub' -- . "${PATHSPEC[@]}" || true)
if [ -n "$gwhits" ]; then
  echo "::error::private gateway-product term(s) found (Home-Hub / MK-HOMEHUB). This naming is Publish:Private —"
  echo "::error::keep it out of the public tree (provenance belongs in gitignored .r2-local/). Offending lines:"
  echo "$gwhits"
  fail=1
fi

# Placeholder/example MAC allowlist — legit example MACs a doc/test may show. Add specific legit examples here.
# (Roy-canon; extend only via Roy.)
MAC_ALLOW='00:00:00:00:00:00|([fF]{2}:){5}[fF]{2}|[dD][eE]:[aA][dD]:[bB][eE]:[eE][fF]|00:11:22:33:44:55|[aA]{2}:[bB]{2}:[cC]{2}:[dD]{2}:[eE]{2}:[fF]{2}|12:34:56:78:9[aA]:[bB][cC]'

# (4) Real MAC addresses in CONTENT (HARD-FAIL, default-on) — device/infra fingerprints must not reach the
# public tree (r2-composer 2026-07-15: full board-MAC inventory leaked to public main). General
# xx:xx:xx:xx:xx:xx catch minus the placeholder allowlist.
machits=$(git grep -inE '\b[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}\b' -- . "${PATHSPEC[@]}" | grep -viE "$MAC_ALLOW" || true)
if [ -n "$machits" ]; then
  echo "::error::real MAC address(es) in the tree — bench/board MACs are Publish:Private (r2-composer MAC-leak)."
  echo "::error::Redact, move to gitignored .r2-local/, or use an allowlisted placeholder; add legit examples to MAC_ALLOW."
  echo "$machits"
  fail=1
fi

# (5) Bench/infra hostnames — private dev-box names shouldn't identify the bench in public docs.
# SEVERITY is Roy-canon and PENDING his Alfred/Tuxedo ruling: set HOSTNAME_SEVERITY=hardfail to enforce,
# or 'advisory' (default) to warn-only without failing the build. BENCH_HOSTS = the Roy-canon list.
HOSTNAME_SEVERITY='advisory'   # ⏳ PENDING ROY: 'hardfail' | 'advisory'
BENCH_HOSTS='Alfred'           # ⏳ PENDING ROY: add Tuxedo?
HOST_ALLOW=''                  # per-line legit exceptions (e.g. Alfred in a non-bench prose context)
hosthits=$(git grep -inwE "$BENCH_HOSTS" -- . "${PATHSPEC[@]}" | { [ -n "$HOST_ALLOW" ] && grep -viE "$HOST_ALLOW" || cat; } || true)
if [ -n "$hosthits" ]; then
  if [ "$HOSTNAME_SEVERITY" = "hardfail" ]; then
    echo "::error::bench/infra hostname(s) ($BENCH_HOSTS) — keep private bench box names out of the public tree:"
    echo "$hosthits"
    fail=1
  else
    echo "::warning::[ADVISORY — severity pending Roy] bench hostname(s) ($BENCH_HOSTS) present in $(echo "$hosthits" | wc -l) line(s); not failing the build yet."
  fi
fi

# (6) MAC fragments in tracked FILENAMES (git grep checks CONTENT only) — heuristic: 3+ hex pairs
# separated by ':' or '-' in a path (e.g. a leaked bench log named by its board's OUI tail). HARD-FAIL:
# rename the file (device fingerprint). Tune if a legit path trips it.
macfiles=$(git ls-files | grep -iE '([0-9a-fA-F]{2}[:-]){2,}[0-9a-fA-F]{2}' | grep -viE "$MAC_ALLOW" || true)
if [ -n "$macfiles" ]; then
  echo "::error::tracked FILENAME(s) embed a MAC fragment — rename them (device fingerprints, Publish:Private):"
  echo "$macfiles"
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "public-content-hygiene: FAIL"
  exit 1
fi
echo "public-content-hygiene: OK (no scrubbed terms outside the allowlist; no macrons; no private gateway naming; no real MACs; no MAC-in-filenames)"
