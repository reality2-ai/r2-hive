#!/usr/bin/env bash
# Public-content hygiene guard (Roy's ruling as a green check, not memory).
#
# r2-hive is a PUBLIC repo. The pilot-site location name + te-reo terms are scrubbed at HEAD pending
# the Māori-reviewer gate (see commit 56a9458; provenance is in gitignored .r2-local/). Restoration is
# a deliberate, reviewed act — this guard fails the build on ACCIDENTAL reintroduction via a re-vendor
# or a careless commit.
#
# WHY THIS FILE IS SHAPED THE WAY IT IS — five fail-open bugs, each found by refutation, not review:
#   1. '\b' boundaries fail open: '_' is a word char, so usb_dev_<MAC>-if00 never matched.
#   2. A line-wise allowlist drops the WHOLE line, letting a real MAC ride beside a placeholder.
#   3. A SUBSTRING allowlist ('DE:AD:BE:EF') suppresses any MAC containing it (02:DE:AD:BE:EF:03).
#   4. Two code paths — a production prefilter and a KAT path that skipped it — meant the KATs proved
#      nothing about production. Known tails sat in hyphen and compact forms while the gate stayed GREEN.
#   5. A denylist of unkeyed SHA-256 tail hashes claimed to protect the values it listed. A 3-byte tail
#      is a 24-bit space: all 17 preimages were recovered in ~30s. It was a plaintext denylist with
#      latency. It is GONE — this guard now contains no device fingerprints at all.
#
# The structural answer to (4) is the rule this file must keep: there is exactly ONE scanner entry
# point, hygiene_scan(). Production pipes the whole tree through it; the KATs pipe fixtures through
# the SAME function, prefilter included. If a fixture passes, production sees it that way too.
set -euo pipefail

# ── VALUE-based denylist (terms / allow / gateway / host) — loaded from a GITIGNORED, OUT-OF-REPO file.
# WHY out-of-repo: a value-based forbid-pattern IS a list of the values; keeping it inline made this gate
# the last, complete, public copy of exactly what to look for ([[enforcer-can-become-the-last-copy]]). SHAPE
# classes (MAC/tail/IP/UUID/key/macron) name no value and stay inline; VALUE classes ride this denylist.
# OWNER = hive (single-source canonical); composer VERIFIES via content-sha. FAIL-CLOSED on absent/unreadable/
# no-schema-version/zero-records/git-tracked — a scan against an empty or missing list passes clean, so its
# absence MUST fail, never pass.
# load_denylist: RETURNS non-zero (never exit) so the --selftest can prove fail-closed by calling it in a
# subshell with a bad path. Sets TERM_RE/ALLOW_RE/GATEWAY_RE/HOST_RE. DENYLIST from $R2_HYGIENE_DENYLIST or
# the per-host fallback (per-host, NOT per-clone: a fresh clone on an existing host finds it).
load_denylist() {
  DENYLIST="${R2_HYGIENE_DENYLIST:-$HOME/.config/r2-fleet/hygiene-denylist}"
  if [ ! -r "$DENYLIST" ]; then
    echo "::error::hygiene denylist NOT FOUND / not readable at: $DENYLIST" >&2
    echo "::error::ONE-TIME PER-HOST PLACEMENT (not per-clone): get the canonical value list from the HIVE lane" >&2
    echo "::error::(owner) and write it to that path, or set \$R2_HYGIENE_DENYLIST. Gitignored/out-of-repo by" >&2
    echo "::error::design. A fail-closed loader with no file is INTENTIONAL — an absent list would pass clean." >&2
    return 1
  fi
  if git ls-files --error-unmatch "$DENYLIST" >/dev/null 2>&1; then
    echo "::error::denylist is git-TRACKED ($DENYLIST) — it is the raw value list and MUST be out-of-repo/gitignored." >&2; return 1
  fi
  grep -q '^#schema-version:' "$DENYLIST" || { echo "::error::denylist $DENYLIST missing '#schema-version:' (schema drift/wrong file) — fail-closed." >&2; return 1; }
  local n; n=$(grep -cvE '^[[:space:]]*#|^[[:space:]]*$' "$DENYLIST")
  [ "$n" -gt 0 ] || { echo "::error::denylist $DENYLIST has ZERO records — an empty-list scan is a false-green; fail-closed." >&2; return 1; }
  dl_class() { awk -F'\t' -v c="$1" '$1==c{print $2}' "$DENYLIST" | paste -sd'|' -; }
  TERM_RE=$(dl_class term); ALLOW_RE=$(dl_class allow); GATEWAY_RE=$(dl_class gateway); HOST_RE=$(dl_class host)
  { [ -n "$TERM_RE" ] && [ -n "$GATEWAY_RE" ] && [ -n "$HOST_RE" ]; } || { echo "::error::denylist $DENYLIST missing a required class (term/gateway/host) — fail-closed." >&2; return 1; }
}
# ── MODE (supervisor ruling 2026-07-27, option b: shapes-only CI backstop) ───────────────────────
# DEFAULT (local + pre-push): FULL sweep — VALUE-classes + SHAPE-classes — REQUIRES the per-host
# out-of-repo denylist and FAILS CLOSED without it. --ci-shapes-only: the hosted-CI backstop — SHAPE/
# signal classes only, no denylist needed. The flag is EXPLICIT + CI-declared; it is NEVER an absent-
# denylist fallback (DEFAULT mode with no denylist still fails closed, right below). ★ WHY THIS IS SOUND,
# not merely cheaper: load_denylist FAILS CLOSED, so a host WITHOUT the denylist CANNOT PUSH AT ALL — the
# value-class invariant is ENFORCED-OR-REFUSED, with no third outcome where a push lands value-unchecked.
# Removing value-classes from CI therefore does NOT weaken coverage; the pre-push fail-closed does. And the
# mode is unreachable-by-accident: the only way to a partial pass is to pass the flag on purpose; a plain
# local run either fully passes (denylist present) or fails closed (absent), never a silent partial.
SHAPES_ONLY=""
if [ "${1:-}" = "--ci-shapes-only" ]; then
  SHAPES_ONLY=1
  echo "::notice::MODE=CI-SHAPES-ONLY — a PARTIAL backstop, NOT a full hygiene pass (run without the flag, denylist present, for full)."
  echo "::notice::NOT CHECKED here (VALUE-classes, enforced PRE-PUSH via the per-host out-of-repo denylist): scrubbed location/cultural TERMS, gateway-product NAME, bench/personal HOSTNAMES."
  echo "::notice::load_denylist FAILS CLOSED ⇒ a host without the denylist cannot push ⇒ value coverage is enforced-or-refused, never silently dropped."
  echo "::notice::CHECKED here (SHAPE/signal classes, no denylist needed): device MAC/tail, RFC1918+CGNAT IP, dashed-UUID, API-key, te-reo macron."
else
  load_denylist || exit 1
fi
# Exclude the guard's own files from the sweep. NOTE: this is a deliberate, bounded fail-open — the KAT
# fixtures below are synthetic MACs that would (correctly) flag themselves. Keep this file small and
# reviewed; do not park real values here on the strength of the exclusion.
PATHSPEC=(':!ci/public-hygiene.sh' ':!.github/workflows/public-content-hygiene.yml' ':!ci/shape-scan-vectors.tsv')
# NB ci/shape-scan-vectors.tsv is the shared CONTRACT vector file — deliberately FULL of synthetic shape
# fixtures (MAC/IP/UUID/key/macron), so it MUST be excluded from the sweep or it flags itself. Every fleet
# repo carrying the vectors excludes them the same way; that exclusion is part of the contract.

fail=0

# Any gate can report a line that happens to contain a fingerprint unrelated to that gate. Public CI
# must not echo it as collateral damage. This final output scrub is intentionally broader than the
# classifier: false redaction is harmless; publishing a private value is not.
redact_stream() {
  perl -pe '
    s{(?i)(?<![0-9a-f])(?:[0-9a-f]{2}[:-]){2,}[0-9a-f]{2}(?![0-9a-f])}{<redacted-hex-run>}g;
    s{(?i)(?<![0-9a-z_])0x[0-9a-f]{7,8}(?![0-9a-z_])}{0x<redacted-hex>}g;
    s{(?i)(?<![0-9a-z_])[0-9a-f]{7,8}(?![0-9a-z_])}{<redacted-hex>}g;
    s{(?i)(?<![0-9a-z_])0x[0-9a-f]{6}(?![0-9a-z_])}{0xXXXXXX}g;
    s{(?i)(?<![0-9a-z_])[0-9a-f]{6}(?![0-9a-z_])}{<redacted-hex>}g;
  '
}

# rc-aware git grep and line filter for the hard-gate term checks below. Same discipline as the docs
# gate (hive-codex round-6): git grep / grep rc 0 (match) and rc 1 (no match) are BOTH success and emit
# output; rc >1 (a real failure — invalid pattern, bad pathspec, or -P with no PCRE support) PROPAGATES
# so `set -e` aborts = fail CLOSED. The prior `… || true` / bare `if assignment` mapped a scanner TOOL
# failure to a clean result, so a broken term/macron/gateway pattern would silently pass the hard gate.
# ★ CAPTURE THE RC IN AN `|| rc=$?` GUARD, NOT a bare `out=$(...); rc=$?` (supervisor 2026-07-27): rc 1 =
# CLEAN NO-MATCH = the SUCCESS case for a hygiene grep, but a FAILURE case for `set -e`; a bare assignment
# can errexit the whole gate on the CLEAN path under some bash/set-e configs — and that path is invisible to
# every negative/error control (they plant a match, so the clean branch never runs). The `|| rc=$?` puts the
# assignment in an ||-list, which `set -e` never errexits, and captures rc explicitly. rc>1 = engine error =
# fail-closed (return propagates under pipefail). Bash-e clean-path is exercised by a --selftest control.
gg()        { local out rc=0; out=$(git grep "$@" 2>/dev/null) || rc=$?; [ "$rc" -le 1 ] || return "$rc"; printf '%s' "$out"; }
gg_filter() { local out rc=0; out=$(grep "$@")               || rc=$?; [ "$rc" -le 1 ] || return "$rc"; printf '%s' "$out"; }

# (1) Scrubbed location + cultural terms (case-insensitive), minus the allowlisted identifiers.
# rc>1 in EITHER the term grep or the allowlist filter now fails closed (pipefail propagates gg's rc).
if [ -z "$SHAPES_ONLY" ]; then   # VALUE-class: needs the denylist; skipped in CI-shapes-only (enforced pre-push)
# TOKEN-CONTAINMENT, not whole-line exemption (fixed 2026-07-28, supervisor-directed, in lane).
# WAS: `gg -inE "$TERM_RE" | gg_filter -viE "$ALLOW_RE"` — `grep -v` drops the ENTIRE LINE, so ANY line
# carrying an allowlisted identifier was exempt from the whole class. Measured on the live tree at the
# time: 18 lines matched TERM_RE and 0 survived the filter — an EFFECTIVE DENOMINATOR OF ZERO. The gate
# printed "terms clean" having evaluated nothing. Worse, both allow records EMBED a scrubbed term, so the
# exempting shape was the NORMAL shape, not a rare one — and the allowlisted lines are precisely the ones
# nobody re-checks, so their silent pass was the only signal the matcher was wrong.
# NOW: a term occurrence is exempt ONLY IF THAT OCCURRENCE lies inside an allowlisted-identifier match on
# the same line. A bare term sharing a line with an allowlisted identifier now FAILS, which is the case the
# old shape could never see. Same token-level principle as the UUID class at (5).
hits=$(gg -inE "$TERM_RE" -- . "${PATHSPEC[@]}" | TERM_RE="$TERM_RE" ALLOW_RE="$ALLOW_RE" perl -ne '
  my $t = $ENV{TERM_RE}; my $a = $ENV{ALLOW_RE};
  my @allow;
  if (defined $a && length $a) { while (/$a/gi) { push @allow, [ $-[0], $+[0] ]; } }
  my $flag = 0;
  while (/$t/gi) {
    my ($s, $e) = ($-[0], $+[0]);
    my $covered = 0;
    for my $r (@allow) { if ($s >= $r->[0] && $e <= $r->[1]) { $covered = 1; last; } }
    if (!$covered) { $flag = 1; last; }
  }
  print if $flag;')
if [ -n "$hits" ]; then
  echo "::error::a scrubbed location/cultural term (see the denylist term-class) reintroduced. Restoration is"
  echo "::error::gated on the Māori-reviewer review — do not re-add in a normal commit. Offending lines:"
  printf '%s\n' "$hits" | redact_stream
  fail=1
fi
fi

# (2) Te-reo macron signal (ā ē ī ō ū, upper/lower) — a strong indicator of unreviewed te-reo prose.
# The old `if macrons=$(git grep -P …)` treated rc>1 (e.g. git built without PCRE) as "no macrons" =
# fail-open. Capture the status: rc 0/1 proceed, rc>1 fails the gate CLOSED.
macrons=$(gg -inP '[\x{0101}\x{0113}\x{012B}\x{014D}\x{016B}\x{0100}\x{0112}\x{012A}\x{014C}\x{016A}]' -- . "${PATHSPEC[@]}") && mrc=0 || mrc=$?
if [ "$mrc" -gt 1 ]; then
  echo "::error::macron scan (git grep -P) failed rc=$mrc — failing closed (fix the PCRE/tooling, do not ignore)."
  exit "$mrc"
fi
if [ -n "$macrons" ]; then
  echo "::error::macron character(s) found (te-reo signal). Route te-reo content through the reviewer gate:"
  printf '%s\n' "$macrons" | redact_stream
  fail=1
fi

# (3) Private gateway-product naming (Roy-gated private spec, authorized by supervisor). NARROW: the
# resident-premises gateway product + its private spec name MUST NOT appear in the public tree (provenance
# lives in gitignored .r2-local/). Values are in the denylist gateway-class. A broader historical ID scrub is
# a SEPARATE Roy decision, deliberately NOT guarded here (some are functional/commit ids with real rename cost).
if [ -z "$SHAPES_ONLY" ]; then   # VALUE-class: needs the denylist; skipped in CI-shapes-only (enforced pre-push)
gwhits=$(gg -inE "$GATEWAY_RE" -- . "${PATHSPEC[@]}")
if [ -n "$gwhits" ]; then
  echo "::error::private gateway-product naming found (see denylist gateway-class). This naming is Publish:Private —"
  echo "::error::keep it out of the public tree (provenance belongs in gitignored .r2-local/). Offending lines:"
  printf '%s\n' "$gwhits" | redact_stream
  fail=1
fi
fi

# ── Shape-based value classes (SAFE-TO-PUBLISH INLINE, parity UNCONDITIONAL — supervisor 2026-07-27) ──
# A pattern for these describes the SHAPE and names no real value, so it is safe inline and every fleet
# tree gets it (cross-repo coverage parity). False-positive management IS the work: the IP pattern is
# RANGE-RESTRICTED (only RFC1918 + CGNAT 100.64/10 — public/loopback/RFC5737-example/4-part-semver are
# out of range → pass); the UUID pattern is the dashed 8-4-4-4-12 shape (a 40/8-hex COMMIT SHA is not that
# shape → passes); the key pattern needs a prefix AND >=2 digits (prose slugs are digitless/short → pass).
# VALUE-based classes (terms/macron/gateway/hostnames, above) name their values and MUST NOT be duplicated
# cross-repo — they ride a SHARED gitignored denylist (fleet artefact; coordinate, do not fork).
# >>> SHAPE-PATTERN-SET v1 — hive's IMPLEMENTATION of the shape classes. The FLEET CONTRACT is NOT this
# text (superseded 2026-07-27): it is ci/shape-scan-vectors.tsv, co-pinned by CONTENT-sha; each repo
# implements however suits it (hive perl+bash, composer shell/mjs) and both must PASS the vectors. These
# markers just delimit the shape patterns; they are NOT blob-sha-co-pinned. UUID_ALLOW_EXACT is per-repo. >>>
IP_RE='(^|[^0-9.])(10\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}|172\.(1[6-9]|2[0-9]|3[01])\.[0-9]{1,3}\.[0-9]{1,3}|192\.168\.[0-9]{1,3}\.[0-9]{1,3}|100\.(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.[0-9]{1,3}\.[0-9]{1,3})([^0-9.]|$)'
UUID_RE='[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}'
KEY_PFX="$(printf '%s%s' 's' 'k-')"   # assembled from parts so this block never spells the prefix contiguously
KEY_RE="(^|[^A-Za-z0-9])${KEY_PFX}(ant-)?[A-Za-z0-9_-]{40,}"
KEY_DIGITS='[0-9].*[0-9]'
# <<< SHAPE-PATTERN-SET v1 <<<
# EXACT whole-token UUID allowlist (lowercased), PER-REPO, PUBLIC-BY-DEFINITION ONLY: the WS-mesh protocol
# GUID (compared by both mesh ends — scrubbing it breaks matching) + the nil UUID. Token-level, never
# substring. NEVER add a real TG/custody UUID here. (Outside the co-pin markers: allowlist values are local.)
UUID_ALLOW_EXACT='258eafa5-e914-47da-95ca-c5ab0dc85b11
00000000-0000-0000-0000-000000000000'

# (4) Private-range + CGNAT IP shapes (range-restricted = the FP filter; no real value named).
iphits=$(gg -inE "$IP_RE" -- . "${PATHSPEC[@]}")
if [ -n "$iphits" ]; then
  echo "::error::private-range / CGNAT IP address(es) found (SHAPE class, Publish:Private). Use a role token"
  echo "::error::(<build-host> …) or gitignored .r2-local/. Offending lines:"
  printf '%s\n' "$iphits" | redact_stream
  fail=1
fi

# (5) Dashed UUID shape (TG/custody ids) minus the EXACT public-constant allowlist (TOKEN-level, not line).
uuidhits=$(gg -inoE "$UUID_RE" -- . "${PATHSPEC[@]}" | UUID_ALLOW="$UUID_ALLOW_EXACT" perl -ne '
  BEGIN{ %a=map{chomp; ($_=>1)} split /\n/, ($ENV{UUID_ALLOW}||""); }
  if(/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/){ print unless $a{lc $1}; }')
if [ -n "$uuidhits" ]; then
  echo "::error::UUID(s) found (TG/custody-id shape, Publish:Private). Real ids live in gitignored custody;"
  echo "::error::a public constant goes in UUID_ALLOW_EXACT (exact token). Offending lines:"
  printf '%s\n' "$uuidhits" | redact_stream
  fail=1
fi

# (6) API-key-shaped secrets (prefix + >=2 digits; digitless/short prose slugs pass). TOKEN-level digit test.
# (6b) SoC SECURITY POSTURE from the census emission (D-20260728-94, supervisor: Publish:Private).
# WHY: `ARB-CENSUS-SOC` emits secure_boot_en / aggressive_revoke / flash_crypt_cnt from BLOCK0 eFuse.
# Each is harmless on a live console. The hazard is a COMPOSITION nobody chose: durable per-board captures
# (D-20260727-77) retired the ephemerality that was silently doing the security work, so a pasted census
# line would persist AND pass this gate — an inventory of which boards have secure boot / flash encryption
# OFF. The sensitive direction is `=0`, which on a dev bench is the COMMON case, not the rare one.
# DIRECTION IS ASYMMETRY, NOT SEVERITY: publishing is irreversible, withholding is reversible, and the
# values stay readable locally where every consumer of them already is. Re-examine ON DEPLOYMENT; do not
# re-litigate before it.
# SHAPE class, deliberately inline: these are FIELD NAMES, not secret values, so there is nothing here for
# an out-of-repo denylist to hold — and a shape pattern cannot become the last authoritative copy of a
# value it describes.
# TOKEN-level (-inoE), matching the UUID/key classes at (5)/(6) — NOT the whole-line `grep -v` shape,
# which exempts an entire line when any part of it is allowlisted.
# FOUR fields, enumerated from the FORMAT STRING at main.rs:8513 — not from a summary. `flash_enc_derived`
# is the vendor-rule derivation of flash_crypt_cnt and leaks the same fact; it was missing from the first
# version of this class because I took a three-field description instead of reading the emitter.
POSTURE_RE='(ARB-CENSUS-SOC|secure_boot_en|aggressive_revoke|flash_crypt_cnt|flash_enc_derived)'
posturehits=$(gg -inoE "$POSTURE_RE" -- . "${PATHSPEC[@]}")
if [ -n "$posturehits" ]; then
  echo "::error::SoC security-posture token(s) from the census emission found (Publish:Private, D-20260728-94)."
  echo "::error::secure_boot_en/aggressive_revoke/flash_crypt_cnt enumerate which boards are soft targets."
  echo "::error::Keep census captures in the gitignored per-board capture dir; do not paste them into a"
  echo "::error::tracked file. Offending lines:"
  printf '%s\n' "$posturehits" | redact_stream
  fail=1
fi

keyhits=$(gg -inoE "$KEY_RE" -- . "${PATHSPEC[@]}" | gg_filter -E "$KEY_DIGITS")
if [ -n "$keyhits" ]; then
  echo "::error::API-key-shaped token(s) found. A key is Publish:Private — ROTATE it and move to gitignored"
  echo "::error::custody; it is not fixed by deletion. Offending lines:"
  printf '%s\n' "$keyhits" | redact_stream
  fail=1
fi

# ── Device-fingerprint hygiene (Roy-approved 2026-07-15; MAC=hard-fail, hostname=advisory) ──────────

# Placeholder MACs a doc/test may legitimately show. EXACT WHOLE-TOKEN, normalised (lowercase, colons):
# a 6-byte token is compared with '=', never as a substring — that is what stops a real MAC from hiding
# inside an allowlisted fragment. Extend only via Roy, and only with full 6-byte values.
MAC_ALLOW_EXACT='00:00:00:00:00:00
ff:ff:ff:ff:ff:ff
aa:bb:cc:dd:ee:ff
00:11:22:33:44:55
12:34:56:78:9a:bc
de:ad:be:ef:ca:fe'
# 3-byte tails that are NOT fingerprints. An IEEE OUI is a public vendor prefix shared by every device
# that vendor ever shipped — it identifies Espressif, not a board. Only ever add PUBLIC-BY-DEFINITION
# values here; a per-device mac_low3 tail must never be allowlisted, it must be redacted.
TAIL_ALLOW_EXACT='d8:3b:da'
# Compact placeholders (bare or 0x 3-byte tail form). Same exactness rule.
COMPACT_ALLOW_EXACT='ffffff
000000
c0ffee
deadbe
cafeba
abcdef'
# A 3-byte tail is only a fingerprint IN CONTEXT. Without this, ISO dates and six-digit colours become
# false positives. \b matters — 'mac' must not fire on 'macro'/'machine'. Bare DEV/AP/STA and hive
# contexts are intentional: the stale exact audit found compact boot-log and role-note forms with no
# `mac_low3` label. `0x` compact forms remain context-free because one historical leak had no such label.
# Bare 'serial' is DELIBERATELY absent: every espflash log header reads "Serial port: '/dev/ttyACM3'",
# which turned the timestamp '12:34:56' into a tail hit in 18 files. 'serial' describes a wire, not a
# MAC. The 'raw_serial' JSON key IS kept — its value is a list of logs named after device tails.
TAIL_CTX='(?i:mac_low3|mac[ _-]?low|mac[ _-]?tail|\bmacs?\b|\bbssid\b|\beui\b|\boui\b|fingerprint|\bdevice\b|\bboards?\b|\bhive(?:[ _-]?id)?\b|\bsoft[ _-]?ap\b|raw_serial)|\b(?:DEV|AP|STA)\b'
# Bare compact tokens need a stricter vocabulary plus adjacency checks in the classifier. In particular,
# prose such as `-b 115200 -> Device programmed` is not a device ID merely because both tokens share a line.
COMPACT_CTX='(?i:mac_low3|mac[ _-]?low|mac[ _-]?tail|\bmacs?\b|\bbssid\b|\beui\b|\boui\b|fingerprint|device[ _-]?id|board[ _-]?id|hive[ _-]?id|raw_serial)'
# An 8-hex token is normally an opaque public ID. Two structural derivations are not: `00` + mac_low3
# under explicit fallback context, and a JSON `hive` value mapped directly to a `mac` field. Keep the
# recognition structural: treating every public 32-bit hive_id near the word "fallback" as a tail
# destroys useful field evidence and produces a large false-positive surface.
ZERO_PAD_CTX='(?i:mac_low3|mac[ _-]?low|mac[ _-]?tail|fallback|clobber)'
# Exact public flash/partition bounds seen in this tree. Unlike the old blanket `000$` exception, these
# pass only without nearby device context: `DEV 0x...` still fails even if its digits equal an offset.
OFFSET_ALLOW_EXACT='1e0000
100000
110000
200000
210000
3e0000
400000'

# ── THE single scanner entry point ─────────────────────────────────────────────────────────────────
# stdin:  `path\0lineno\0content\n` records (`git grep -z -n` in production and every KAT)
# stdout: 'path:lineno: [REASON] <redacted>' per violation. Clean and findings both exit 0; callers test
# output. A malformed record stream is a scanner error (non-zero), never an empty/green result.
hygiene_scan() {
  perl -e '
    use strict;
    use warnings;
    my ($mac_text, $tail_text, $compact_text, $offset_text, $tail_ctx_text, $compact_ctx_text, $zero_pad_ctx_text) = splice @ARGV, 0, 7;
    my %mac_allow = map { lc($_) => 1 } grep { length } split /\n/, $mac_text;
    my %tail_allow = map { lc($_) => 1 } grep { length } split /\n/, $tail_text;
    my %compact_allow = map { lc($_) => 1 } grep { length } split /\n/, $compact_text;
    my %offset_allow = map { lc($_) => 1 } grep { length } split /\n/, $offset_text;
    my $tail_ctx = qr/(?:$tail_ctx_text)/;
    my $compact_ctx = qr/(?:$compact_ctx_text)/;
    my $zero_pad_ctx = qr/(?:$zero_pad_ctx_text)/;
    my %reported;

    sub redact {
      my ($text) = @_;
      $text =~ s{(?i)(?<![0-9a-f])(?:[0-9a-f]{2}[:-]){2,}[0-9a-f]{2}(?![0-9a-f])}{<redacted-hex-run>}g;
      $text =~ s{(?i)(?<![0-9a-z_])0x[0-9a-f]{7,8}(?![0-9a-z_])}{0x<redacted-hex>}g;
      $text =~ s{(?i)(?<![0-9a-z_])[0-9a-f]{7,8}(?![0-9a-z_])}{<redacted-hex>}g;
      $text =~ s{(?i)(?<![0-9a-z_])0x[0-9a-f]{6}(?![0-9a-z_])}{0xXXXXXX}g;
      $text =~ s{(?i)(?<![0-9a-z_])[0-9a-f]{6}(?![0-9a-z_])}{<redacted-hex>}g;
      return $text;
    }
    sub nearby {
      my ($content, $start, $end, $radius) = @_;
      my $from = $start > $radius ? $start - $radius : 0;
      my $to = $end + $radius < length($content) ? $end + $radius : length($content);
      return substr($content, $from, $to - $from);
    }
    sub has_compact_context {
      my ($content, $start, $end) = @_;
      return 1 if nearby($content, $start, $end, 32) =~ $compact_ctx;
      my $before = substr($content, $start > 40 ? $start - 40 : 0, $start > 40 ? 40 : $start);
      my $after = substr($content, $end, 40);
      return 1 if $before =~ /\b(?:DEV|device|board|hive)\s*(?:[=:]\s*)?\z/i;
      return 1 if $after =~ /\A\s*(?:[=:]\s*)?(?:AP|STA)\b/;
      return 0;
    }
    sub report {
      my ($path, $line, $reason, $token) = @_;
      my $key = join "\0", $path, $line, $reason, lc($token);
      return if $reported{$key}++;
      $token =~ s/[0-9a-f]/x/ig;
      print redact("$path:$line"), ": [$reason] $token\n";
    }

    local $/;
    my $data = <STDIN> // "";
    my $consumed = 0;
    pos($data) = 0;
    while ($data =~ /\G([^\0]*)\0([0-9]+)\0([^\n]*)(?:\n|\z)/g) {
      my ($path, $line, $content) = ($1, $2, $3);
      $consumed = pos($data);
      # One internal prefilter, shared by production and KATs. It is a strict superset: separated
      # three-byte runs, 0x compact tails, and context-bearing bare compact tails all reach verdicts.
      next unless $content =~ /(?:[0-9a-f]{2}[:-]){2}[0-9a-f]{2}|(?<![0-9a-z_])(?:0x)?[0-9a-f]{6,8}(?![0-9a-z_])/i;

      # Extract the maximal pair run and NORMALISE both separators to ':' before classifying. Boundary
      # checks exclude only hex, not the separator: x-02-...-y and usb_02:...-if00 must both reach here.
      # MIXED separators are NOT skipped (hive-codex round-3, fail-open): a real MAC written 02:11-22:33-
      # 44:55 (sloppy edit or deliberate obfuscation) must still flag. The old `next if mixed` was the
      # fail-OPEN direction — a leak gate errs toward flagging. Colon, hyphen, and mixed now all normalise.
      while ($content =~ /(?<![0-9a-f])([0-9a-f]{2}(?:[:-][0-9a-f]{2}){2,})(?![0-9a-f])/ig) {
        my $token = $1;
        my $has_ctx = nearby($content, $-[1], $+[1], 96) =~ $tail_ctx;
        (my $normal = lc $token) =~ tr/-/:/;
        my @groups = split /:/, $normal;
        if (@groups >= 6) {
          for (my $i = 0; $i + 6 <= @groups; $i++) {
            my $mac = join ":", @groups[$i .. $i + 5];
            report($path, $line, "MAC", $mac) unless $mac_allow{$mac};
          }
          next; # full-MAC runs are not reclassified as independent three-byte tails
        }
        if ($has_ctx) {
          for (my $i = 0; $i + 3 <= @groups; $i++) {
            my $tail = join ":", @groups[$i .. $i + 2];
            report($path, $line, "TAIL", $tail) unless $tail_allow{$tail};
          }
        }
      }

      # Bare compact tails require device context. 0x tails do not. An offset-shaped value is excused
      # only by offset context and only when device context is absent; DEV 0x...000 therefore fails.
      while ($content =~ /(?<![0-9a-z_])(0x)?([0-9a-f]{6})(?![0-9a-z_])/ig) {
        my ($prefix, $hex) = (defined($1) ? $1 : "", lc $2);
        # Compact tokens are common short numbers/words, so context must be local. The historical
        # forms were adjacent (`DEV <tail>`, `hive <tail>`, `<tail>=AP`); a line-wide match turns a
        # baud or protocol constant elsewhere on a long RESUME line into a false device identifier.
        my $has_ctx = has_compact_context($content, $-[0], $+[0]);
        next if $compact_allow{$hex};
        my $tail_form = join ":", $hex =~ /(..)/g;
        next if $tail_allow{$tail_form};
        next if !$has_ctx && $offset_allow{$hex};
        next if $prefix eq "" && !$has_ctx;
        report($path, $line, $prefix eq "" ? "TAIL-COMPACT" : "TAIL-0x", $prefix . $hex);
      }

      # Eight-hex values are scanned only under the two structural derivations described above. Report
      # the whole token redacted; the guard deliberately carries no historical tail values or hashes.
      while ($content =~ /(?<![0-9a-z_])(0x)?([0-9a-f]{8})(?![0-9a-z_])/ig) {
        my ($prefix, $hex) = (defined($1) ? $1 : "", lc $2);
        my $before = substr($content, $-[0] > 48 ? $-[0] - 48 : 0, $-[0] > 48 ? 48 : $-[0]);
        my $after = substr($content, $+[0], 48);
        my $zero_padded = $hex =~ /\A00[0-9a-f]{6}\z/ && nearby($content, $-[0], $+[0], 96) =~ $zero_pad_ctx;
        my $json_mapping = $before =~ /"hive"\s*:\s*"\s*\z/i && $after =~ /\A\s*"\s*,\s*"mac"\s*:/i;
        next unless $zero_padded || $json_mapping;
        report($path, $line, "TAIL-EMBEDDED", $prefix . $hex);
      }
    }
    die "malformed hygiene record stream\n" if $consumed != length($data);
  ' "$MAC_ALLOW_EXACT" "$TAIL_ALLOW_EXACT" "$COMPACT_ALLOW_EXACT" "$OFFSET_ALLOW_EXACT" "$TAIL_CTX" "$COMPACT_CTX" "$ZERO_PAD_CTX"
}

# The production extraction path is also a single helper so the end-to-end KAT can exercise it in a
# temporary Git tree. `-z` makes filenames unambiguous; no path:line string parsing is involved. Tracked
# path segments become synthetic `raw_serial` records in the SAME stream, so content and filenames cannot
# drift into separate classifiers (and a compact tail in a filename cannot bypass the content-only grep).
hygiene_scan_tree() {
  local root=$1
  local grep_status
  shift
  (
    if git -C "$root" grep -z -I -n -e '' -- . "$@" 2>/dev/null; then
      :
    else
      grep_status=$?
      # Status 1 means there are no grep-able content lines. That is not permission to skip tracked
      # filenames: feed those through the same classifier below. Any actual Git error still fails closed.
      [ "$grep_status" -eq 1 ] || exit "$grep_status"
    fi
    git -C "$root" ls-files -z | perl -0ne '
      $seen = 1;
      s/\0\z//;
      die "newline in tracked path is unsupported by hygiene records\n" if /\n/;
      my $path = $_;
      my $content = join " ", map { "raw_serial $_" } split m{/}, $path;
      print $path, "\0", "0\0", $content, "\n";
      END { die "no tracked paths for hygiene scan\n" unless $seen; }
    ' || exit
  ) | hygiene_scan
}

# ── KAT self-test: `ci/public-hygiene.sh --selftest` ───────────────────────────────────────────────
# Every fixture goes through hygiene_scan() — the SAME function production uses, prefilter included.
# That is the whole point: the previous suite tested a helper that production reached via a different
# path, so it scored 7/7 while real tails sat exposed. Fixtures are synthetic only (02:… is
# locally-administered and non-allowlisted); no real device value appears in this public file.
if [ "${1:-}" = "--selftest" ]; then
  k=0; p=0
  kat() { # name, line, want(1=must flag, 0=must pass), [reason], [exact-count]
    # A bare flag/no-flag assertion lets a WRONG-CLASSIFIER regression stay green (a MAC misread as a
    # TAIL still "flags"; a 4-group run emits 2 overlapping TAILs so the count is load-bearing). When a
    # reason ([MAC]/[TAIL]/[TAIL-0x]/[TAIL-COMPACT]) and/or an exact count is given, assert them too.
    k=$((k+1))
    # Capture the scanner's EXIT STATUS, don't `|| true` it: a `|| true` maps a scanner CRASH on VALID
    # input to empty output, so a `want=0` (must-pass) fixture would false-pass on a broken scanner
    # (hive-codex round-4). A crash on a normal fixture is always a KAT failure; only the dedicated
    # malformed-stream KAT expects a non-zero scanner status, and it checks that directly, not via kat().
    out=$(printf 'f\0001\000%s\n' "$2" | hygiene_scan) && src=0 || src=$?
    if [ "$src" -ne 0 ]; then echo "  FAIL $1 (scanner crashed on valid input, rc=$src)"; return; fi
    got=$(printf '%s' "$out" | grep -c '\[' || true)
    ok=1
    if [ "$3" = 1 ]; then [ "$got" -gt 0 ] || ok=0; else [ "$got" -eq 0 ] || ok=0; fi
    if [ -n "${4:-}" ]; then printf '%s' "$out" | grep -q "\[$4\]" || ok=0; fi
    if [ -n "${5:-}" ]; then [ "$got" -eq "$5" ] || ok=0; fi
    if [ "$ok" = 1 ]; then
      p=$((p+1)); echo "  ok   $1"
    else echo "  FAIL $1 (matched=$got want=$3 reason=${4:-any} count=${5:-any})"; fi
  }
  # --- full-MAC positives: the boundary fail-opens (assert reason=MAC so a mis-class can't pass) ---
  kat "underscore-adjacent colon MAC flags"                  'usb_dev_02:11:22:33:44:55-if00'            1 MAC
  kat "underscore+hyphen MAC flags (prefilter fail-open)"    'usb_dev_02-11-22-33-44-55-if00'            1 MAC
  kat "hyphen MAC, hyphen on BOTH sides, flags"              'x-02-11-22-33-44-55-y'                     1 MAC
  kat "bare hyphen MAC flags"                                'mac 02-11-22-33-44-55 seen'                1 MAC
  kat "real + allowlisted on ONE line flags (line-wise bypass)" 'ex 00:00:00:00:00:00 real 02:11:22:33:44:55' 1 MAC
  kat "SUBSTRING allowlist hole closed (02:de:ad:be:ef:03)"  'mac 02:DE:AD:BE:EF:03 here'                1 MAC
  kat "MAC embedded in a longer hex run flags"               'run aa-02-11-22-33-44-55-bb'               1 MAC
  kat "uppercase hyphen MAC flags"                           'MAC 02-AB-CD-EF-11-22'                     1 MAC
  # --- full-MAC negatives ---
  kat "allowlisted-only line passes, even in MAC context"     'mac 00:00:00:00:00:00 only'                0
  kat "allowlisted uppercase/hyphen placeholder passes"      'mac AA-BB-CC-DD-EE-FF only'                0
  kat "redacted placeholder passes"                          'mac xx:xx:xx:xx:xx:xx redacted'            0
  kat "MIXED-separator MAC now flags (was fail-open skip)"   'frag 02:11-22:33-44:55 here'               1 MAC
  kat "MIXED-separator MAC, other order, flags"              'id 02-11:22-33:44-55 seen'                 1 MAC
  # --- 3-byte tail: contextual (assert reason=TAIL; the 4-group run emits exactly 2 windows) ---
  kat "tail flags WITH mac context"                          'mac_low3 = 02:34:5a'                       1 TAIL 1
  kat "hyphen tail after hex node label flags"               'raw_serial d3-02-34-5a.log'                1 TAIL
  kat "tail inside a 4-group run flags with context (2 windows)" 'bssid 11:02:34:5a'                     1 TAIL 2
  kat "bare compact boot-log tail flags"                     'DEV 02345A role=STA'                       1 TAIL-COMPACT
  kat "bare compact hive-adjacent tail flags"                'hive 02345A originates'                    1 TAIL-COMPACT
  kat "bare compact tail passes without device context"      'revision 1a2b3c landed'                   0
  kat "six-digit baud remote from board context passes"      'board boot command with padding padding -b 115200' 0
  kat "protocol prefix before hive field passes"             'HEALTH matches a7011a<hive8>'              0
  kat "non-tail 3-pair passes (no time/version false-positive)" 'at 12:34:56 today'                      0
  kat "ISO date passes (524 in this tree)"                   'entry 26-07-04 shipped'                    0
  kat "date passes even near the word macro"                 'macro rework 26-07-04 landed'              0
  kat "public OUI passes in MAC context"                     'oui D8-3B-DA vendor prefix'                0
  kat "compact public OUI passes in context"                 'oui D83BDA vendor prefix'                  0
  # --- compact 0x tail (assert reason=TAIL-0x) ---
  kat "0x compact tail flags without context"                'value 0x02345A'                            1 TAIL-0x
  kat "device-context 0x tail ending 000 still flags"         'DEV 0x02A000'                              1 TAIL-0x
  kat "device-context exact offset collision still flags"    'DEV 0x1E0000'                              1 TAIL-0x
  kat "0x flash offset passes"                               'persona @0x12000 and 0x1E0000'             0
  kat "0x placeholder passes"                                'magic 0xC0FFEE and 0xFFFFFF'               0
  kat "0x 8-digit word is not a 3-byte tail"                 'reg 0x02345A99 write'                      0
  # --- embedded compact tail derivations ---
  kat "zero-prefixed mac_low3 derivation flags"              'fallback 0x0002345A from mac_low3'         1
  kat "suffixed JSON hive-to-MAC mapping flags"              '{"hive":"02345Aaa","mac":"xx:xx:xx:xx:xx:xx"}' 1
  kat "opaque 32-bit hive id remains public"                'hive_id 02345A99'                         0

  # Output itself is a security surface: findings may identify locations/shapes, never the value.
  k=$((k+1))
  redacted=$(printf 'f\0001\000DEV 02345A mac 02:11:22:33:44:55 fallback 0x0002345A from mac_low3\n' | hygiene_scan)
  if printf '%s' "$redacted" | grep -qiE '02345a|02:11:22:33:44:55|0002345a'; then
    echo "  FAIL scanner output exposed a fixture value"
  else p=$((p+1)); echo "  ok   scanner output redacts every fingerprint"; fi

  # End-to-end negative control: the actual git-grep extraction + scanner must reject a mixed line;
  # after removing only that line, the same production path must accept the allowlisted control.
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  git -C "$tmp" init -q
  printf '%s\n' 'example 00:00:00:00:00:00 real 02:11:22:33:44:55' > "$tmp/reject.txt"
  printf '%s\n' 'mac 00:00:00:00:00:00 only' > "$tmp/allow.txt"
  git -C "$tmp" add reject.txt allow.txt
  # e2e MODE NAME: run the REAL extraction and capture BOTH output and exit status. A bare
  # `[ -n "$(hygiene_scan_tree ...)" ]` DISCARDS the status, so a scanner CRASH that happens to emit a
  # finding false-passes a positive case, and a crash that emits nothing false-passes a negative case
  # (hive-codex round-5). flag = findings AND rc0; clean = empty AND rc0; failclosed = rc != 0.
  e2e() {
    local out st ok=0; out=$(hygiene_scan_tree "$tmp") && st=0 || st=$?
    k=$((k+1))
    case "$1" in
      flag)       { [ "$st" -eq 0 ] && [ -n "$out" ]; } && ok=1 || : ;;
      clean)      { [ "$st" -eq 0 ] && [ -z "$out" ]; } && ok=1 || : ;;
      failclosed) [ "$st" -ne 0 ] && ok=1 || : ;;
    esac
    if [ "$ok" -eq 1 ]; then p=$((p+1)); echo "  ok   $2"
    else echo "  FAIL $2 (mode=$1 rc=$st out=$([ -n "$out" ] && echo nonempty || echo empty))"; fi
  }
  e2e flag "production extraction rejects mixed allowed/private line"
  rm "$tmp/reject.txt"; git -C "$tmp" add -u
  printf '%s\n' 'safe content' > "$tmp/board-02345A.log"; git -C "$tmp" add board-02345A.log
  e2e flag "production extraction rejects compact tail in filename"
  rm "$tmp/board-02345A.log"; git -C "$tmp" add -u
  # A MIXED-separator MAC must flag through the real git-grep extraction, not just the direct
  # classifier (hive-codex round-3: the old skip made this a production fail-open).
  printf '%s\n' 'note 02:11-22:33-44:55 seen' > "$tmp/mixed.txt"; git -C "$tmp" add mixed.txt
  e2e flag "production extraction rejects mixed-separator MAC"
  rm "$tmp/mixed.txt"; git -C "$tmp" add -u
  e2e clean "production extraction accepts allowlisted negative control"
  rm "$tmp/allow.txt"; git -C "$tmp" add -u
  touch "$tmp/board-02345A.log"; git -C "$tmp" add board-02345A.log
  e2e flag "production extraction scans filename-only tree"
  rm "$tmp/board-02345A.log"; git -C "$tmp" add -u
  e2e failclosed "empty tracked tree fails closed"
  rm -rf "$tmp"
  trap - EXIT

  # ── shape-class KATs (IP / UUID / API-key) — the gg-checks; assert the SHARED pattern on stdin ──
  # Fixtures are SYNTHETIC ONLY (generic private ranges, a synthetic UUID, a parts-built key) — never a
  # real scrubbed value, which would re-disclose it in this tracked file even though PATHSPEC self-excludes.
  skat() { k=$((k+1)); local d=$1 want=$2 got=$3
    if { [ "$want" = 1 ] && [ -n "$got" ]; } || { [ "$want" = 0 ] && [ -z "$got" ]; }; then p=$((p+1)); echo "  ok   $d"
    else echo "  FAIL $d (want=$want got='$got')"; fi; }
  # IP: range IS the discriminator — RFC1918/CGNAT flag; public/loopback/example/4-part-semver pass.
  skat "RFC1918 IP flags"          1 "$(printf 'host 10.0.0.1 up\n'    | grep -inoE "$IP_RE")"
  skat "192.168 IP flags"          1 "$(printf 'ap 192.168.0.1\n'      | grep -inoE "$IP_RE")"
  skat "CGNAT 100.64/10 flags"     1 "$(printf 'ts 100.64.0.1\n'       | grep -inoE "$IP_RE")"
  skat "public IP passes"          0 "$(printf 'dns 8.8.8.8\n'         | grep -inoE "$IP_RE")"
  skat "loopback passes"           0 "$(printf 'bind 127.0.0.1\n'      | grep -inoE "$IP_RE")"
  skat "RFC5737 example passes"    0 "$(printf 'doc 192.0.2.5\n'       | grep -inoE "$IP_RE")"
  skat "4-part semver passes"      0 "$(printf 'v 1.2.3.4 rel\n'       | grep -inoE "$IP_RE")"
  # UUID: dashed shape flags; commit-SHA / 8-hex hive-id / allowlisted-constant pass.
  uuid_check() { grep -inoE "$UUID_RE" | UUID_ALLOW="$UUID_ALLOW_EXACT" perl -ne 'BEGIN{%a=map{chomp;($_=>1)}split/\n/,($ENV{UUID_ALLOW}||"")} if(/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/){print unless $a{lc $1}}'; }
  skat "synthetic UUID flags"      1 "$(printf 'id 12345678-1234-1234-1234-1234567890ab\n' | uuid_check)"
  skat "allowlisted nil UUID passes" 0 "$(printf 'nil 00000000-0000-0000-0000-000000000000\n' | uuid_check)"
  skat "40-hex commit SHA passes"  0 "$(printf 'commit deadbeefcafe0123456789abcdef0123456789ab\n' | uuid_check)"
  skat "8-hex on-air hive id passes" 0 "$(printf 'hive deadbeef on air\n' | uuid_check)"
  # API-key: prefix + >=2 digits flags; digitless/short prose slug passes.
  key_check() { grep -inoE "$KEY_RE" | grep -E "$KEY_DIGITS"; }
  skat "key-shaped w/ digits flags" 1 "$(printf 'tok %sant-a1b2%s\n' "$KEY_PFX" "$(printf 'x%.0s' $(seq 1 60))" | key_check)"
  skat "digitless key-like slug passes" 0 "$(printf 'name %santhill-membership-token-fixture-value-longenough\n' "$KEY_PFX" | key_check)"

  # ── denylist LOADER KATs: prove fail-closed by deleting/breaking the file (supervisor requirement) ──
  dlt=$(mktemp -d)
  dlkat() { k=$((k+1)); local d=$1 path=$2 want=$3 r
    ( R2_HYGIENE_DENYLIST="$path" load_denylist ) >/dev/null 2>&1 && r=0 || r=1
    if [ "$r" = "$want" ]; then p=$((p+1)); echo "  ok   $d"; else echo "  FAIL $d (rc=$r want=$want)"; fi; }
  dlkat "loader fail-closed on MISSING file"            "$dlt/nope"      1
  : > "$dlt/empty";                                       dlkat "loader fail-closed on EMPTY (zero records)"    "$dlt/empty"    1
  printf 'term\tfoo\tbar\n' > "$dlt/noschema";            dlkat "loader fail-closed on NO schema-version"       "$dlt/noschema" 1
  printf '#schema-version: 1\nterm\tfoo\t<x>\n' > "$dlt/noclass"; dlkat "loader fail-closed on MISSING required class" "$dlt/noclass" 1
  printf '#schema-version: 1\nterm\tfoo\t<x>\ngateway\tg\t<y>\nhost\th\t<z>\n' > "$dlt/ok"; dlkat "loader loads a valid file" "$dlt/ok" 0
  rm -rf "$dlt"

  # A parser/instrument failure must fail closed rather than look like an empty clean scan.
  k=$((k+1))
  if printf 'malformed' | hygiene_scan >/dev/null 2>&1; then
    echo "  FAIL malformed scanner input passed open"
  else p=$((p+1)); echo "  ok   malformed scanner input fails closed"; fi
  # ── ERROR CONTROL for the gg() search wrapper (supervisor 2026-07-27: a ugrep regex-complexity limit on a
  # MACRON pattern produced no-matches and READ AS A PASS — a broken engine that looks clean). Our gg() returns
  # rc>1 on an engine error so `set -euo pipefail` / the macron `exit $mrc` fail CLOSED. Prove it EVERY run, not
  # just in review: a BROKEN pattern must yield rc>1 (not clean); a genuine no-match must yield rc0 (not error).
  k=$((k+1)); gg -inP '[\x{0101}' -- . >/dev/null 2>&1 && gecrc=0 || gecrc=$?
  if [ "$gecrc" -gt 1 ]; then p=$((p+1)); echo "  ok   gg error-control: broken PCRE -> rc=$gecrc (>1) = FAIL-CLOSED, not clean"
  else echo "  FAIL gg error-control: broken PCRE -> rc=$gecrc (want >1; error-read-as-clean = FAKE GATE)"; fi
  k=$((k+1)); gg -inE 'zzz_no_such_token_zzz_9137' -- . >/dev/null 2>&1 && gncrc=0 || gncrc=$?
  if [ "$gncrc" -eq 0 ]; then p=$((p+1)); echo "  ok   gg error-control: genuine no-match -> rc0 (clean is distinct from error)"
  else echo "  FAIL gg error-control: no-match -> rc=$gncrc (want 0)"; fi
  # ── CLEAN-PATH-UNDER-bash-e control (supervisor 2026-07-27): rc1 = clean no-match = SUCCESS for a hygiene
  # grep but FAILURE for set -e. The clean branch is the ONE the negative + error controls NEVER touch (they
  # plant a match), so a bare `out=$(git grep no-match); rc=$?` can errexit the whole gate on the clean path
  # and stay invisible. Run gg's clean no-match in a FRESH `bash -euo pipefail` subshell (exactly how CI runs
  # it) and assert it SURVIVES. This is the branch that reddened a peer's hosted CI while passing locally.
  k=$((k+1))
  if bash -euo pipefail -c '
      gg() { local out rc=0; out=$(git grep "$@" 2>/dev/null) || rc=$?; [ "$rc" -le 1 ] || return "$rc"; printf "%s" "$out"; }
      h=$(gg -inE "zzz_clean_path_probe_zzz" -- .); : "$h"' >/dev/null 2>&1
  then p=$((p+1)); echo "  ok   clean-path under bash -e: gg no-match SURVIVES (does not errexit the gate)"
  else echo "  FAIL clean-path under bash -e: gg no-match errexits — the invisible CLEAN-path trap"; fi
  # ── RUN THE CONTRACT FROM FILE (supervisor 2026-07-27): running inline COPIES + pinning the file's sha
  # proves the file is unchanged, NOT that the copies still equal it — editing an inline KAT leaves the sha
  # and the pin green while the contract is no longer run (the mirror-test, in the artefact meant to stop
  # drift). So execute every vector from ci/shape-scan-vectors.tsv through THIS gate's own matchers. The
  # inline KATs above are retained as targeted regression fixtures; the CONTRACT truth comes from here.
  uuid_hit() { grep -inoE "$UUID_RE" | UUID_ALLOW="$UUID_ALLOW_EXACT" perl -ne 'BEGIN{%a=map{chomp;($_=>1)}split/\n/,($ENV{UUID_ALLOW}||"")} if(/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/){print unless $a{lc $1}}'; }
  run_vectors() {
    local KP want class count input reason cnt ipm uidm keym macm hs hn ok
    KP=$(printf '%s%s' 's' 'k-')   # expand <KEYPFX> (vector-file header: fail-safe — a failed expansion makes
                                   # the KEY positive non-key-shaped ⇒ NOT caught ⇒ it FAILS loud, never silent-pass)
    while IFS=$'\t' read -r want class count input; do
      case "$want" in flag|pass) ;; *) continue ;; esac
      input=${input//<KEYPFX>/$KP}
      k=$((k+1)); ok=1; reason=''; cnt='-'
      # every matcher via $()=…|| true : a STANDALONE assignment with a failing substitution (grep no-match
      # rc1) EXITS under set -e (unlike an arg-position "$(…)"), so each is explicitly guarded to rc0.
      ipm=$(printf '%s\n' "$input" | grep -inoE "$IP_RE") || true
      uidm=$(printf '%s\n' "$input" | uuid_hit) || true
      keym=$(printf '%s\n' "$input" | grep -inoE "$KEY_RE" | grep -E "$KEY_DIGITS") || true
      macm=$(printf '%s\n' "$input" | grep -inP '[\x{0101}\x{0113}\x{012B}\x{014D}\x{016B}\x{0100}\x{0112}\x{012A}\x{014C}\x{016A}]') || true
      hs=$(printf 'f\0001\000%s\n' "$input" | hygiene_scan) || true
      hn=$(printf '%s' "$hs" | grep -c '\[') || true; hn=${hn:-0}
      if [ "$want" = pass ]; then
        if [ -n "$ipm" ] || [ -n "$uidm" ] || [ -n "$keym" ] || [ -n "$macm" ] || [ "$hn" -ne 0 ]; then ok=0; fi
      else
        case "$class" in
          IP)     if [ -z "$ipm" ]; then ok=0; fi ;;
          UUID)   if [ -z "$uidm" ]; then ok=0; fi ;;
          KEY)    if [ -z "$keym" ]; then ok=0; fi ;;
          macron) if [ -z "$macm" ]; then ok=0; fi ;;
          *)      if [ "$hn" -gt 0 ]; then cnt=$hn; reason=$(printf '%s' "$hs" | grep -oE '\[[A-Z0-9-]+\]' | head -1 | tr -d '[]') || true; else ok=0; fi
                  if [ "$class" != '-' ] && [ -n "$reason" ] && [ "$reason" != "$class" ]; then ok=0; fi
                  if [ "$count" != '-' ] && [ "$cnt" != "$count" ]; then ok=0; fi ;;
        esac
      fi
      if [ "$ok" = 1 ]; then p=$((p+1)); else echo "  FAIL contract-vector [$want $class $count] $input (reason=${reason:-none} cnt=$cnt)"; fi
    done < ci/shape-scan-vectors.tsv
    echo "  ran $(grep -cE '^(flag|pass)' ci/shape-scan-vectors.tsv) CONTRACT vectors from ci/shape-scan-vectors.tsv"
  }
  run_vectors

  # ── shared shape-vector CONTRACT drift alarm (supervisor 2026-07-27): the co-pinned vector file must
  # match its pinned content-sha. A vector added in ANY fleet repo bumps this; adopt + rebump here. The
  # inline shape/MAC/tail KATs above ARE these vectors — keep them in step (single-source run-from-file is
  # the planned follow-on). The vector file is SYNTHETIC + publishable, so the pin is on public content.
  k=$((k+1)); PINNED_VECTORS_SHA='79fdc80ee7f7e4fb6e6afbbf5fbe338c8cebc5858ce7dd07eebd4e22b39070a9'
  vsha=$(sha256sum ci/shape-scan-vectors.tsv 2>/dev/null | cut -d' ' -f1)
  if [ "$vsha" = "$PINNED_VECTORS_SHA" ]; then p=$((p+1)); echo "  ok   shape-vector contract sha pinned ($PINNED_VECTORS_SHA)"
  else echo "  FAIL shape-vector contract DRIFT (got ${vsha:-<none>} want $PINNED_VECTORS_SHA) — adopt the new vectors + rebump the pin in every repo"; fi

  # ── TERM-CLASS TOKEN-CONTAINMENT KAT (2026-07-28) ─────────────────────────────────────────────
  # The term class was a WHOLE-LINE `grep -v`, so any line carrying an allowlisted identifier was exempt
  # from the entire class (18 lines matched, 0 survived = effective denominator ZERO). This KAT is the
  # case the old shape could NEVER see, and it is why the class now tests CONTAINMENT of each occurrence.
  # Fixtures are SYNTHESISED AT RUNTIME from the gitignored denylist — no term or allow value is ever
  # written into this tracked file. If the denylist is absent the KAT is SKIPPED LOUDLY, never silently
  # passed: an unrunnable control must not read as a green one.
  term_filter() { TERM_RE="$1" ALLOW_RE="$2" perl -ne '
    my $t = $ENV{TERM_RE}; my $a = $ENV{ALLOW_RE}; my @al;
    if (defined $a && length $a) { while (/$a/gi) { push @al, [ $-[0], $+[0] ]; } }
    my $f = 0;
    while (/$t/gi) { my ($s,$e) = ($-[0],$+[0]); my $c = 0;
      for my $r (@al) { if ($s >= $r->[0] && $e <= $r->[1]) { $c = 1; last; } }
      if (!$c) { $f = 1; last; } }
    print if $f;'; }
  if load_denylist 2>/dev/null && [ -n "$TERM_RE" ] && [ -n "$ALLOW_RE" ]; then
    _t1=$(awk -F'\t' '$1=="term"{print $2; exit}'  "$DENYLIST")
    _a1=$(awk -F'\t' '$1=="allow"{print $2; exit}' "$DENYLIST")
    # (i) allowlisted identifier + a BARE term on ONE line -> MUST FLAG (the old shape scored 0 here)
    k=$((k+1))
    if [ "$(printf '%s\n' "ctx $_a1 and bare $_t1 here" | term_filter "$TERM_RE" "$ALLOW_RE" | wc -l)" -eq 1 ]
      then p=$((p+1)); else echo "  FAIL term-containment: bare term beside an allowlisted identifier NOT flagged"; fi
    # (ii) allowlisted identifier ALONE -> MUST PASS (the allowlist must still work)
    k=$((k+1))
    if [ "$(printf '%s\n' "ctx $_a1 alone" | term_filter "$TERM_RE" "$ALLOW_RE" | wc -l)" -eq 0 ]
      then p=$((p+1)); else echo "  FAIL term-containment: allowlisted identifier alone was flagged (false positive)"; fi
    # (iii) a bare term with NO allowlisted identifier -> MUST FLAG (guards a matcher that matches nothing)
    k=$((k+1))
    if [ "$(printf '%s\n' "ctx bare $_t1 here" | term_filter "$TERM_RE" "$ALLOW_RE" | wc -l)" -eq 1 ]
      then p=$((p+1)); else echo "  FAIL term-containment: bare term alone NOT flagged — class is INERT"; fi
    unset _t1 _a1
  else
    echo "  SKIP term-containment KAT: denylist unavailable (control NOT run — this is not a pass)"
  fi

  echo "selftest: $p/$k passed"
  [ "$p" -eq "$k" ] || exit 1
  exit 0
fi

# (4) Device fingerprints in CONTENT — HARD-FAIL, default-on (r2-composer 2026-07-15 board-MAC leak).
# git grep enumerates EVERY tracked line (-I skips binaries); hygiene_scan owns the whole verdict.
# No `|| true`: extraction/parser failure is a hard gate failure, never an empty success.
devhits=$(hygiene_scan_tree . "${PATHSPEC[@]}")
if [ -n "$devhits" ]; then
  echo "::error::device fingerprint(s) in the tree — board MACs / mac_low3 tails are Publish:Private."
  echo "::error::Values are REDACTED in all output; inspect only the reported source locations."
  echo "::error::Fix: redact to xx:xx:xx:xx:xx:xx, move to gitignored .r2-local/, or (placeholders only)"
  echo "::error::add the exact public placeholder to MAC_ALLOW_EXACT / COMPACT_ALLOW_EXACT."
  printf '%s\n' "$devhits" | redact_stream
  fail=1
fi

# (5) Bench/infra + personal hostnames — FORBIDDEN by Roy g23 ruling 2026-07-27 (SUPERSEDES the 2026-07-15
# advisory): real host names are scrubbed, NOT accepted; the earlier "accepted dev-box names" reading
# conflated this gate's allowlist with the g23 keep-set — different concerns. One host name is a HIGHER
# class (it carried the principal's name = a personal identifier). HARDFAIL so reintroduction is caught.
# VALUES now come from the denylist host-class (out-of-repo), so this gate no longer carries the list
# itself ([[enforcer-can-become-the-last-copy]]). PATHSPEC already excludes this file from the scan.
if [ -z "$SHAPES_ONLY" ]; then   # VALUE-class: needs the denylist host-class; skipped in CI-shapes-only (enforced pre-push)
HOSTNAME_SEVERITY='hardfail'
BENCH_HOSTS="$HOST_RE"
# rc-aware even though it is advisory today: the `|| true` here would become fail-OPEN the moment
# HOSTNAME_SEVERITY is flipped to hardfail (a scan tool failure would pass as "no hostnames"). Capture
# the status now so the flip is safe — advisory WARNS on a scan failure, hardfail EXITS closed
# (hive-codex round-7 latent note; the last rc-swallow in this gate).
hosthits=$(gg -inwE "$BENCH_HOSTS" -- . "${PATHSPEC[@]}" ':(exclude)ci/public-hygiene.sh') && hrc=0 || hrc=$?
if [ "$hrc" -gt 1 ]; then
  if [ "$HOSTNAME_SEVERITY" = "hardfail" ]; then
    echo "::error::hostname scan failed rc=$hrc — failing closed."; exit "$hrc"
  else
    echo "::warning::[ADVISORY] hostname scan failed rc=$hrc — the scan did NOT run; not failing (advisory)."
  fi
fi
if [ -n "$hosthits" ]; then
  if [ "$HOSTNAME_SEVERITY" = "hardfail" ]; then
    echo "::error::FORBIDDEN bench/personal hostname(s) (see denylist host-class) — scrub per Roy g23 2026-07-27 (use a role token e.g. <build-host>/<rig-host>):"
    printf '%s\n' "$hosthits" | redact_stream
    fail=1
  else
    echo "::warning::[ADVISORY] hostname hit(s) ($BENCH_HOSTS) $(printf '%s\n' "$hosthits" | wc -l) line(s); not failing (severity=advisory)."
  fi
fi
fi   # end VALUE-class host block (SHAPES_ONLY skips it)

if [ "$fail" -ne 0 ]; then
  echo "public-content-hygiene: FAIL"
  exit 1
fi
scanned=$(git ls-files -- . "${PATHSPEC[@]}" | wc -l | tr -d ' ')
if [ -n "$SHAPES_ONLY" ]; then
  echo "public-content-hygiene: OK (CI-SHAPES-ONLY, PARTIAL) — DENOMINATOR $scanned files scanned; SHAPE/signal classes clean (MAC/tail, RFC1918+CGNAT IP, dashed-UUID, API-key, te-reo macron). VALUE-classes (terms/gateway/hostnames) NOT checked here — enforced PRE-PUSH via the fail-closed per-host denylist. This is NOT a full hygiene pass."
else
  echo "public-content-hygiene: OK — DENOMINATOR $scanned files scanned (sweep scope); terms/macrons/gateway clean; no MACs; no device tails; no MAC-in-filenames; no SoC posture"
fi
