#!/usr/bin/env bash
set -euo pipefail
# GO-1/GO-9 (CODING_STANDARDS.md §2): the handlers package must not import
# internal/{auth,realtime,db}. Allowed internal deps: services, models, middleware,
# contract, config (plus stdlib, gin, gorm, uuid, ...). Structural exceptions are
# allowlisted in .coding-standards-allowlist.yaml under `rule: GO-9` (path: <file>).
# Portable: POSIX grep/awk only (runs on CI runners without ripgrep).
cd "$(dirname "$0")/../.."
ALLOW=.coding-standards-allowlist.yaml
DIR=apps/website/internal/handlers
FORBIDDEN='reforger-backend/internal/(auth|realtime|db)"'

# Allowlisted file paths under a `rule: GO-9` block.
exempt=$(awk '
  /rule:[[:space:]]*GO-9/ { go9=1; next }
  /^[[:space:]]*-?[[:space:]]*rule:/ { go9=0 }
  go9 && /path:/ { sub(/.*path:[[:space:]]*/, ""); print }
' "$ALLOW")

fail=0
for f in "$DIR"/*.go; do
  case "$f" in *_test.go) continue ;; esac
  skip=0
  for p in $exempt; do [ "$f" = "$p" ] && skip=1; done
  [ "$skip" = 1 ] && continue
  if hits=$(grep -nE "$FORBIDDEN" "$f"); then
    echo "GO-9: $f imports a forbidden internal package (auth/realtime/db):" >&2
    echo "$hits" >&2
    fail=1
  fi
done
if [ "$fail" = 0 ]; then echo "GO-9: handlers import only allowed internal packages."; fi
exit "$fail"
