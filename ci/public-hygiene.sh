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

# ── Device-fingerprint hygiene (Roy-approved 2026-07-15; MAC=hard-fail, hostname=advisory) ──────────
# BOUNDARIES: '\b' FAILS OPEN — '_' is a word char, so \b never fires in usb_..._AA:BB:CC:DD:EE:FF-if00
# (a real leak this gate previously missed; hive-codex finding 1). Use explicit non-hex/non-separator
# boundaries, and keep colon- and hyphen-separated MACs as SEPARATE patterns: '-' is both a separator
# AND a common adjacent char (-if00), so one shared boundary class breaks one of the two forms.
MAC_COLON='(^|[^0-9a-fA-F:])[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}([^0-9a-fA-F:]|$)'
MAC_HYPH='(^|[^0-9a-fA-F-])[0-9a-fA-F]{2}(-[0-9a-fA-F]{2}){5}([^0-9a-fA-F-]|$)'
MAC_TOKEN='[0-9a-fA-F]{2}([:-][0-9a-fA-F]{2}){5}'
# Placeholder/example MACs a doc/test may legitimately show (Roy-canon; extend only via Roy).
MAC_ALLOW='00:00:00:00:00:00|([fF]{2}[:-]){5}[fF]{2}|[dD][eE][:-][aA][dD][:-][bB][eE][:-][eE][fF]|00:11:22:33:44:55|[aA]{2}:[bB]{2}:[cC]{2}:[dD]{2}:[eE]{2}:[fF]{2}|12:34:56:78:9[aA]:[bB][cC]'
# Known device MAC TAILS (mac_low3 fingerprint) as SHA-256 prefixes — NOT the values: a plaintext
# denylist in a PUBLIC guard would leak the very fingerprints it protects (hive-codex finding 3).
MAC_TAIL_HASHES='0d0bf834fc785321|446177abe3e8fc38|4fe57239facd4454|59513c20bf1fee5e|5fcc26a59b3ec2b5|650178f913071d92|6a441741ef83f98e|91dd915a81d31460|97d81e50d3f467fa|a0b5b055b26ec8d2|a7bea959a70050a8|ab9b11d9157b4bb2|bbe81775916a4613|c849917d62bb967e|e6446d074152f486|edadb4e01e4c2f39|f755c9eb83115ca7'

# TOKEN-WISE allowlist: a line-wise 'grep -v' drops the WHOLE line when it also carries an allowed
# placeholder, letting a real MAC ride along (hive-codex finding 2). Filter each TOKEN instead.
mac_bad_lines() {
  while IFS= read -r l; do
    toks=$(printf '%s\n' "$l" | grep -oE "$MAC_TOKEN" || true)
    if [ -z "$toks" ]; then continue; fi
    bad=$(printf '%s\n' "$toks" | grep -viE "$MAC_ALLOW" || true)
    if [ -n "$bad" ]; then printf '%s\n' "$l"; fi
  done
  return 0
}
tail_bad_lines() {
  while IFS= read -r l; do
    toks=$(printf '%s\n' "$l" | grep -oE '[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){2}' || true)
    if [ -z "$toks" ]; then continue; fi
    while IFS= read -r t; do
      if [ -z "$t" ]; then continue; fi
      h=$(printf '%s' "$t" | tr 'A-F' 'a-f' | sha256sum | cut -c1-16)
      if printf '%s' "$h" | grep -qE "^($MAC_TAIL_HASHES)$"; then printf '%s\n' "$l"; break; fi
    done <<< "$toks"
  done
  return 0
}

# ── KAT self-test: `ci/public-hygiene.sh --selftest` (hive-codex regression fixtures) ───────────────
# Locks the three fail-open bugs this gate previously had. Uses only SYNTHETIC MACs (02:… is locally-
# administered and non-allowlisted); the tail positive hashes a synthetic tail at RUNTIME so no real
# device fingerprint is embedded in this public guard.
if [ "${1:-}" = "--selftest" ]; then
  k=0; p=0
  kat() { # name, line, want(1=must flag, 0=must pass), fn
    k=$((k+1))
    got=$(printf '%s\n' "f:1:$2" | "$4" | wc -l)
    if { [ "$3" = 1 ] && [ "$got" -gt 0 ]; } || { [ "$3" = 0 ] && [ "$got" -eq 0 ]; }; then
      p=$((p+1)); echo "  ok   $1"
    else echo "  FAIL $1 (matched=$got, want=$3)"; fi
  }
  kat "underscore-adjacent full MAC flags (\\b fail-open regression)" 'usb_dev_02:11:22:33:44:55-if00' 1 mac_bad_lines
  kat "hyphen-separated full MAC flags"                              'mac 02-11-22-33-44-55 seen'      1 mac_bad_lines
  kat "real + allowlisted on ONE line flags (line-wise bypass)"      'ex 00:00:00:00:00:00 real 02:11:22:33:44:55' 1 mac_bad_lines
  kat "allowlisted-only line passes"                                 'example 00:00:00:00:00:00 only'  0 mac_bad_lines
  kat "redacted placeholder passes"                                  'mac xx:xx:xx:xx:xx:xx redacted'  0 mac_bad_lines
  kat "non-tail 3-pair passes (no time/version false-positive)"      'at 12:34:56 today'               0 tail_bad_lines
  # tail POSITIVE: synthesise a tail, hash it, prove the matcher fires — without embedding a real tail.
  _t='ab:cd:ef'; _h=$(printf '%s' "$_t" | sha256sum | cut -c1-16)
  k=$((k+1))
  if [ "$(MAC_TAIL_HASHES="$_h"; printf 'f:1:x %s y\n' "$_t" | tail_bad_lines | wc -l)" -gt 0 ]; then
    p=$((p+1)); echo "  ok   hashed tail denylist flags a listed tail"
  else echo "  FAIL hashed tail denylist did not flag a listed tail"; fi
  echo "selftest: $p/$k passed"
  [ "$p" -eq "$k" ] || exit 1
  exit 0
fi

# (4) Real MAC addresses in CONTENT — HARD-FAIL, default-on (r2-composer 2026-07-15 board-MAC leak).
machits=$( { git grep -inE "$MAC_COLON" -- . "${PATHSPEC[@]}" 2>/dev/null || true; \
              git grep -inE "$MAC_HYPH"  -- . "${PATHSPEC[@]}" 2>/dev/null || true; } | sort -u | mac_bad_lines )
if [ -n "$machits" ]; then
  echo "::error::real MAC address(es) in the tree — bench/board MACs are Publish:Private."
  echo "::error::Redact to xx:xx:xx:xx:xx:xx, move to gitignored .r2-local/, or add a legit example to MAC_ALLOW."
  echo "$machits"
  fail=1
fi

# (4b) Known device MAC TAILS (mac_low3) — HARD-FAIL. Exact (hashed) match: no false-positives on
# times/versions, and no value leak in this public guard.
tailhits=$(git grep -inE '[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){2}' -- . "${PATHSPEC[@]}" 2>/dev/null | tail_bad_lines || true)
if [ -n "$tailhits" ]; then
  echo "::error::known device MAC tail(s) (mac_low3 fingerprint) present — redact to xx:xx:xx:"
  echo "$tailhits"
  fail=1
fi

# (5) Bench/infra hostnames — ADVISORY by Roy ruling 2026-07-15: Alfred/Tuxedo are ACCEPTED dev-box
# names and are NOT scrubbed. Kept as a warn-only signal; set HOSTNAME_SEVERITY=hardfail only on a
# further Roy ruling. (Roy-canon: extend BENCH_HOSTS via Roy.)
HOSTNAME_SEVERITY='advisory'
BENCH_HOSTS='Alfred|Tuxedo'
hosthits=$(git grep -inwE "$BENCH_HOSTS" -- . "${PATHSPEC[@]}" 2>/dev/null || true)
if [ -n "$hosthits" ]; then
  if [ "$HOSTNAME_SEVERITY" = "hardfail" ]; then
    echo "::error::bench hostname(s) ($BENCH_HOSTS):"; echo "$hosthits"; fail=1
  else
    echo "::warning::[ADVISORY — Roy: Alfred/Tuxedo accepted as dev-box names] $(printf '%s\n' "$hosthits" | wc -l) line(s); not failing."
  fi
fi

# (6) MAC fragments in tracked FILENAMES (git grep is content-only) — HARD-FAIL; rename the file.
macfiles=$(git ls-files | grep -iE '([0-9a-fA-F]{2}[:-]){2,}[0-9a-fA-F]{2}' || true)
if [ -n "$macfiles" ]; then
  echo "::error::tracked FILENAME(s) embed a MAC fragment — rename them:"; echo "$macfiles"; fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "public-content-hygiene: FAIL"
  exit 1
fi
echo "public-content-hygiene: OK (terms/macrons/gateway clean; no real MACs; no device tails; no MAC-in-filenames)"
