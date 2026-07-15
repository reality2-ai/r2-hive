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

# Allowlist: the two preserved wire/code identifiers (documented in the scrub commit) that legitimately
# still contain the token and are pending a separate coordinated code/wire rename, NOT a doc-scrub.
ALLOW='wairoa_as923_nz|wairoa\.reading'
# Exclude the guard's own files from the sweep. NOTE: this is a deliberate, bounded fail-open — the KAT
# fixtures below are synthetic MACs that would (correctly) flag themselves. Keep this file small and
# reviewed; do not park real values here on the strength of the exclusion.
PATHSPEC=(':!ci/public-hygiene.sh' ':!.github/workflows/public-content-hygiene.yml')

fail=0

# Any gate can report a line that happens to contain a fingerprint unrelated to that gate. Public CI
# must not echo it as collateral damage. This final output scrub is intentionally broader than the
# classifier: false redaction is harmless; publishing a private value is not.
redact_stream() {
  perl -pe '
    s{(?i)(?<![0-9a-f])(?:[0-9a-f]{2}[:-]){2,}[0-9a-f]{2}(?![0-9a-f])}{<redacted-hex-run>}g;
    s{(?i)(?<![0-9a-z_])0x[0-9a-f]{6}(?![0-9a-z_])}{0xXXXXXX}g;
    s{(?i)(?<![0-9a-z_])[0-9a-f]{6}(?![0-9a-z_])}{<redacted-hex>}g;
  '
}

# (1) Scrubbed location + cultural terms (case-insensitive), minus the allowlisted identifiers.
hits=$(git grep -inE 'wairoa|kaitiaki|marae' -- . "${PATHSPEC[@]}" | grep -viE "$ALLOW" || true)
if [ -n "$hits" ]; then
  echo "::error::scrubbed term(s) reintroduced (Wairoa / kaitiaki / marae). Restoration is gated on the"
  echo "::error::Māori-reviewer review — do not re-add in a normal commit. Offending lines:"
  printf '%s\n' "$hits" | redact_stream
  fail=1
fi

# (2) Te-reo macron signal (ā ē ī ō ū, upper/lower) — a strong indicator of unreviewed te-reo prose.
if macrons=$(git grep -inP '[\x{0101}\x{0113}\x{012B}\x{014D}\x{016B}\x{0100}\x{0112}\x{012A}\x{014C}\x{016A}]' -- . "${PATHSPEC[@]}" 2>/dev/null); then
  if [ -n "$macrons" ]; then
    echo "::error::macron character(s) found (te-reo signal). Route te-reo content through the reviewer gate:"
    printf '%s\n' "$macrons" | redact_stream
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
  printf '%s\n' "$gwhits" | redact_stream
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
    my ($mac_text, $tail_text, $compact_text, $offset_text, $tail_ctx_text, $compact_ctx_text) = splice @ARGV, 0, 6;
    my %mac_allow = map { lc($_) => 1 } grep { length } split /\n/, $mac_text;
    my %tail_allow = map { lc($_) => 1 } grep { length } split /\n/, $tail_text;
    my %compact_allow = map { lc($_) => 1 } grep { length } split /\n/, $compact_text;
    my %offset_allow = map { lc($_) => 1 } grep { length } split /\n/, $offset_text;
    my $tail_ctx = qr/(?:$tail_ctx_text)/;
    my $compact_ctx = qr/(?:$compact_ctx_text)/;
    my %reported;

    sub redact {
      my ($text) = @_;
      $text =~ s{(?i)(?<![0-9a-f])(?:[0-9a-f]{2}[:-]){2,}[0-9a-f]{2}(?![0-9a-f])}{<redacted-hex-run>}g;
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
      next unless $content =~ /(?:[0-9a-f]{2}[:-]){2}[0-9a-f]{2}|(?<![0-9a-z_])(?:0x)?[0-9a-f]{6}(?![0-9a-z_])/i;

      # Extract the maximal pair run, then reject mixed separators as one malformed fragment. Boundary
      # checks exclude only hex, not the separator: x-02-...-y and usb_02:...-if00 must both reach here.
      while ($content =~ /(?<![0-9a-f])([0-9a-f]{2}(?:[:-][0-9a-f]{2}){2,})(?![0-9a-f])/ig) {
        my $token = $1;
        my $has_ctx = nearby($content, $-[1], $+[1], 96) =~ $tail_ctx;
        next if $token =~ /:/ && $token =~ /-/;
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
    }
    die "malformed hygiene record stream\n" if $consumed != length($data);
  ' "$MAC_ALLOW_EXACT" "$TAIL_ALLOW_EXACT" "$COMPACT_ALLOW_EXACT" "$OFFSET_ALLOW_EXACT" "$TAIL_CTX" "$COMPACT_CTX"
}

# The production extraction path is also a single helper so the end-to-end KAT can exercise it in a
# temporary Git tree. `-z` makes filenames unambiguous; no path:line string parsing is involved. Tracked
# path segments become synthetic `raw_serial` records in the SAME stream, so content and filenames cannot
# drift into separate classifiers (and a compact tail in a filename cannot bypass the content-only grep).
hygiene_scan_tree() {
  local root=$1
  shift
  (
    git -C "$root" grep -z -I -n -e '' -- . "$@" 2>/dev/null || exit
    git -C "$root" ls-files -z | perl -0ne '
      s/\0\z//;
      die "newline in tracked path is unsupported by hygiene records\n" if /\n/;
      my $path = $_;
      my $content = join " ", map { "raw_serial $_" } split m{/}, $path;
      print $path, "\0", "0\0", $content, "\n";
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
  kat() { # name, line, want(1=must flag, 0=must pass)
    k=$((k+1))
    got=$(printf 'f\0001\000%s\n' "$2" | hygiene_scan | grep -c '^' || true)
    if { [ "$3" = 1 ] && [ "$got" -gt 0 ]; } || { [ "$3" = 0 ] && [ "$got" -eq 0 ]; }; then
      p=$((p+1)); echo "  ok   $1"
    else echo "  FAIL $1 (matched=$got, want=$3)"; fi
  }
  # --- full-MAC positives: the boundary fail-opens ---
  kat "underscore-adjacent colon MAC flags"                  'usb_dev_02:11:22:33:44:55-if00'            1
  kat "underscore+hyphen MAC flags (prefilter fail-open)"    'usb_dev_02-11-22-33-44-55-if00'            1
  kat "hyphen MAC, hyphen on BOTH sides, flags"              'x-02-11-22-33-44-55-y'                     1
  kat "bare hyphen MAC flags"                                'mac 02-11-22-33-44-55 seen'                1
  kat "real + allowlisted on ONE line flags (line-wise bypass)" 'ex 00:00:00:00:00:00 real 02:11:22:33:44:55' 1
  kat "SUBSTRING allowlist hole closed (02:de:ad:be:ef:03)"  'mac 02:DE:AD:BE:EF:03 here'                1
  kat "MAC embedded in a longer hex run flags"               'run aa-02-11-22-33-44-55-bb'               1
  kat "uppercase hyphen MAC flags"                           'MAC 02-AB-CD-EF-11-22'                     1
  # --- full-MAC negatives ---
  kat "allowlisted-only line passes, even in MAC context"     'mac 00:00:00:00:00:00 only'                0
  kat "allowlisted uppercase/hyphen placeholder passes"      'mac AA-BB-CC-DD-EE-FF only'                0
  kat "redacted placeholder passes"                          'mac xx:xx:xx:xx:xx:xx redacted'            0
  kat "mixed separators are not a MAC"                       'frag 02:11-22:33-44:55 here'               0
  # --- 3-byte tail: contextual ---
  kat "tail flags WITH mac context"                          'mac_low3 = 02:34:5a'                       1
  kat "hyphen tail after hex node label flags"               'raw_serial d3-02-34-5a.log'                1
  kat "tail inside a 4-group run flags with context"         'bssid 11:02:34:5a'                         1
  kat "bare compact boot-log tail flags"                     'DEV 02345A role=STA'                       1
  kat "bare compact hive-adjacent tail flags"                'hive 02345A originates'                    1
  kat "bare compact tail passes without device context"      'revision 1a2b3c landed'                   0
  kat "six-digit baud remote from board context passes"      'board boot command with padding padding -b 115200' 0
  kat "protocol prefix before hive field passes"             'HEALTH matches a7011a<hive8>'              0
  kat "non-tail 3-pair passes (no time/version false-positive)" 'at 12:34:56 today'                      0
  kat "ISO date passes (524 in this tree)"                   'entry 26-07-04 shipped'                    0
  kat "date passes even near the word macro"                 'macro rework 26-07-04 landed'              0
  kat "public OUI passes in MAC context"                     'oui D8-3B-DA vendor prefix'                0
  kat "compact public OUI passes in context"                 'oui D83BDA vendor prefix'                  0
  # --- compact 0x tail ---
  kat "0x compact tail flags without context"                'value 0x02345A'                            1
  kat "device-context 0x tail ending 000 still flags"         'DEV 0x02A000'                              1
  kat "device-context exact offset collision still flags"    'DEV 0x1E0000'                              1
  kat "0x flash offset passes"                               'persona @0x12000 and 0x1E0000'             0
  kat "0x placeholder passes"                                'magic 0xC0FFEE and 0xFFFFFF'               0
  kat "0x 8-digit word is not a 3-byte tail"                 'reg 0x02345A99 write'                      0

  # Output itself is a security surface: findings may identify locations/shapes, never the value.
  k=$((k+1))
  redacted=$(printf 'f\0001\000DEV 02345A mac 02:11:22:33:44:55\n' | hygiene_scan)
  if printf '%s' "$redacted" | grep -qiE '02345a|02:11:22:33:44:55'; then
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
  k=$((k+1))
  if [ -n "$(hygiene_scan_tree "$tmp")" ]; then
    p=$((p+1)); echo "  ok   production extraction rejects mixed allowed/private line"
  else echo "  FAIL production extraction bypassed mixed allowed/private line"; fi
  rm "$tmp/reject.txt"
  git -C "$tmp" add -u
  printf '%s\n' 'safe content' > "$tmp/board-02345A.log"
  git -C "$tmp" add board-02345A.log
  k=$((k+1))
  if [ -n "$(hygiene_scan_tree "$tmp")" ]; then
    p=$((p+1)); echo "  ok   production extraction rejects compact tail in filename"
  else echo "  FAIL production extraction bypassed compact tail in filename"; fi
  rm "$tmp/board-02345A.log"
  git -C "$tmp" add -u
  k=$((k+1))
  if [ -z "$(hygiene_scan_tree "$tmp")" ]; then
    p=$((p+1)); echo "  ok   production extraction accepts allowlisted negative control"
  else echo "  FAIL production extraction rejected allowlisted negative control"; fi
  rm -rf "$tmp"
  trap - EXIT

  # A parser/instrument failure must fail closed rather than look like an empty clean scan.
  k=$((k+1))
  if printf 'malformed' | hygiene_scan >/dev/null 2>&1; then
    echo "  FAIL malformed scanner input passed open"
  else p=$((p+1)); echo "  ok   malformed scanner input fails closed"; fi
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

# (5) Bench/infra hostnames — ADVISORY by Roy ruling 2026-07-15: Alfred/Tuxedo are ACCEPTED dev-box
# names and are NOT scrubbed. Kept as a warn-only signal; set HOSTNAME_SEVERITY=hardfail only on a
# further Roy ruling. (Roy-canon: extend BENCH_HOSTS via Roy.)
HOSTNAME_SEVERITY='advisory'
BENCH_HOSTS='Alfred|Tuxedo'
hosthits=$(git grep -inwE "$BENCH_HOSTS" -- . "${PATHSPEC[@]}" 2>/dev/null || true)
if [ -n "$hosthits" ]; then
  if [ "$HOSTNAME_SEVERITY" = "hardfail" ]; then
    echo "::error::bench hostname(s) ($BENCH_HOSTS):"
    printf '%s\n' "$hosthits" | redact_stream
    fail=1
  else
    echo "::warning::[ADVISORY — Roy: Alfred/Tuxedo accepted as dev-box names] $(printf '%s\n' "$hosthits" | wc -l) line(s); not failing."
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "public-content-hygiene: FAIL"
  exit 1
fi
echo "public-content-hygiene: OK (terms/macrons/gateway clean; no MACs; no device tails; no MAC-in-filenames)"
