#!/usr/bin/env bash
# T-162 hard gate: no .py files and no Python interpreter invocations in scripts/ or Makefile.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FAIL=0

echo "==> find *.py (excl .git / node_modules / target / worktrees)"
mapfile -t PY < <(find . -name '*.py' -type f \
  ! -path './.git/*' \
  ! -path '*/node_modules/*' \
  ! -path '*/target/*' \
  ! -path './.ai/artifacts/worktrees/*' \
  | sort)
if [ "${#PY[@]}" -gt 0 ]; then
  printf 'FAIL: leftover .py files:\n'
  printf '  %s\n' "${PY[@]}"
  FAIL=1
else
  echo "  OK (none)"
fi

echo "==> rg for Python interpreter invocations in scripts/ Makefile"
# Exclude this gate script (documents the ban). Match shebangs + python3 binary calls.
HITS="$(rg -n 'python3|#!/usr/bin/env python' scripts/ Makefile \
  --glob '!**/node_modules/**' \
  --glob '!**/verify-no-python.sh' \
  || true)"
HITS="$(printf '%s\n' "$HITS" | grep -v 'verify-no-python' || true)"
# Drop empty lines
HITS="$(printf '%s\n' "$HITS" | sed '/^$/d' || true)"
if [ -n "$HITS" ]; then
  printf 'FAIL: Python interpreter references remain:\n%s\n' "$HITS"
  FAIL=1
else
  echo "  OK (none)"
fi

if [ "$FAIL" -ne 0 ]; then
  echo "verify-no-python: FAIL" >&2
  exit 1
fi
echo "verify-no-python: PASS"
exit 0
