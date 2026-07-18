#!/usr/bin/env bash
# T-145 guard: a bare `SELECT *` / `RETURNING *` on a table with nullable columns
# re-introduces the Go→Rust null-tolerance 500 hazard (Go/GORM read NULL as the zero
# value; sqlx rejects it — see the registry + discord fixes). Model reads must list
# columns explicitly and COALESCE the nullable non-Option ones. Only tables with ZERO
# nullable columns may use `*`.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../apps/website/api" && pwd)"
ALLOW='modpack_mods|orbat_reservations'   # verified: no nullable columns
bad=0

while IFS= read -r line; do
  tbl=$(printf '%s' "$line" | grep -oE 'SELECT \* FROM [a-z_]+' | awk '{print $NF}')
  if [ -n "$tbl" ] && ! printf '%s' "$tbl" | grep -qE "^($ALLOW)\$"; then
    echo "  SELECT-* on nullable-column table — list columns + COALESCE:"
    echo "    $line"
    bad=1
  fi
done < <(grep -rnE 'SELECT \* FROM [a-z_]+' "$ROOT/src/handlers" "$ROOT/src/services" 2>/dev/null || true)

while IFS= read -r line; do
  if ! printf '%s' "$line" | grep -qE "($ALLOW)"; then
    echo "  RETURNING-* — list columns + COALESCE nullable ones:"
    echo "    $line"
    bad=1
  fi
done < <(grep -rnE 'RETURNING \*' "$ROOT/src/handlers" "$ROOT/src/services" 2>/dev/null || true)

if [ "$bad" -eq 0 ]; then
  echo "no-select-star: clean"
else
  echo "no-select-star: FAIL"
  exit 1
fi
