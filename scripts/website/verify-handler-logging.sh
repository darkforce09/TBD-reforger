#!/usr/bin/env bash
set -euo pipefail
# LOG-3 (CODING_STANDARDS.md §9): consequential handler failures are logged.
# Two bands enforced:
#   Band 1 — every 5xx (InternalServerError/BadGateway/ServiceUnavailable/GatewayTimeout).
#   Band 2 — 400/409/413 returns inside MUTATING handlers (POST/PUT/PATCH/DELETE).
# Read-path (GET) 4xx and all 2xx are exempt; no SSE-stream 5xx exist today. A log is
# satisfied by a `log.` or `logHandlerErr(` call within the 3 non-blank lines above the
# c.JSON. The mutator set is derived from handlers.go Register(). Portable POSIX awk.
cd "$(dirname "$0")/../.."
DIR=apps/website/internal/handlers
REG="$DIR/handlers.go"

# Handler names wired with a mutating verb (last arg of the route line is h.<Name>).
mutators=$(grep -oE '\.(POST|PUT|PATCH|DELETE)\("[^"]*",.*h\.[A-Za-z_]+\)$' "$REG" \
  | grep -oE 'h\.[A-Za-z_]+\)$' | sed -E 's/^h\.//; s/\)$//' | sort -u | tr '\n' ' ')

files=$(find "$DIR" -maxdepth 1 -name '*.go' ! -name '*_test.go' | sort)

# shellcheck disable=SC2086
awk -v mut="$mutators" '
  BEGIN {
    n = split(mut, a, /[ \t\n]+/); for (i = 1; i <= n; i++) if (a[i] != "") ism[a[i]] = 1
    split("InternalServerError BadGateway ServiceUnavailable GatewayTimeout", f5, " "); for (i in f5) five[f5[i]] = 1
    split("BadRequest Conflict RequestEntityTooLarge", f4, " "); for (i in f4) four[f4[i]] = 1
    fail = 0
  }
  {
    if (match($0, /^func \(h \*Handler\) [A-Za-z_]+\(/)) {
      cur = $0; sub(/^func \(h \*Handler\) /, "", cur); sub(/\(.*/, "", cur)
    } else if (match($0, /^func [A-Za-z_]+\(/)) {
      cur = $0; sub(/^func /, "", cur); sub(/\(.*/, "", cur)
    }
    if (match($0, /c\.JSON\(http\.Status[A-Za-z]+,/)) {
      code = $0; sub(/.*c\.JSON\(http\.Status/, "", code); sub(/,.*/, "", code)
      need = 0
      if (code in five) need = 1
      else if ((code in four) && (cur in ism)) need = 1
      if (need) {
        logged = 0; seen = 0
        for (k = NR - 1; k >= 1 && seen < 3; k--) {
          l = lines[k]
          if (l ~ /^[ \t]*$/) continue
          seen++
          if (l ~ /log\.|logHandlerErr\(/) { logged = 1; break }
        }
        if (!logged) {
          printf("LOG-3: %s:%d %s in %s() not logged within 3 lines\n", FILENAME, FNR, code, cur) > "/dev/stderr"
          fail = 1
        }
      }
    }
    lines[NR] = $0
  }
  END { exit fail }
' $files
echo "LOG-3: all consequential 5xx + mutator 4xx returns are logged."
