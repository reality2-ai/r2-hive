#!/usr/bin/env bash
# setup-hooks.sh — wire the tracked pre-push vendored-vector drift guard into git,
# WITHOUT disabling the fleet secret-scan hook.
#
# The fleet installs a secret-scan hook at .git/hooks/pre-push that, after its scan
# passes, `exec`s .git/hooks/pre-push.local as an extension point. So we install our
# guard THERE (a symlink to the tracked .githooks/pre-push), preserving the
# secret-scan. We deliberately do NOT set core.hooksPath — that would make git
# ignore .git/hooks/ entirely and silently disable the secret-scan failsafe.
#
# If no fleet pre-push hook is present (a clone without the fleet install), we
# install our guard directly as .git/hooks/pre-push so it still runs. We NEVER
# overwrite an existing pre-push that we did not create.
#
# Idempotent. Run once per clone:  ./scripts/setup-hooks.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS="$(git -C "$ROOT" rev-parse --git-path hooks)"
SRC="$ROOT/.githooks/pre-push"          # tracked source of truth
LOCAL="$HOOKS/pre-push.local"           # fleet-hook extension point
MAIN="$HOOKS/pre-push"                  # fleet secret-scan hook (if installed)

chmod +x "$SRC"
mkdir -p "$HOOKS"

install_symlink() {  # $1 = target path
  ln -sf "$SRC" "$1"
  echo "  installed: $1 -> .githooks/pre-push"
}

# Always install at the extension point so the fleet hook chains to us.
install_symlink "$LOCAL"

if [ -e "$MAIN" ] && [ ! -L "$MAIN" ]; then
  # A real (non-symlink) pre-push exists — assume it's the fleet secret-scan hook,
  # which chains to pre-push.local. Confirm the chain, warn if it's absent.
  if grep -q 'pre-push.local' "$MAIN" 2>/dev/null; then
    echo "  fleet secret-scan hook detected at $MAIN — it chains to pre-push.local (our guard). Both run."
  else
    echo "  ⚠ a pre-push hook exists at $MAIN but does NOT chain to pre-push.local —"
    echo "    our guard will NOT run via it. Leaving it untouched (not overwriting a hook we didn't create)."
    echo "    Add \`[ -x \"\$(git rev-parse --git-path hooks)/pre-push.local\" ] && exec … pre-push.local\` to it,"
    echo "    or run our guard manually: ci/check-vendored-vectors.sh --strict"
  fi
elif [ ! -e "$MAIN" ]; then
  # No fleet hook — install our guard directly so it runs on push.
  install_symlink "$MAIN"
  echo "  (no fleet secret-scan hook present; installed our guard directly as pre-push)"
fi

echo "hooks: done. pre-push now runs ci/check-vendored-vectors.sh --strict (bypass: git push --no-verify)."
