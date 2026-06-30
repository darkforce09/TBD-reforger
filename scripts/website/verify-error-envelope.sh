#!/usr/bin/env bash
set -euo pipefail
# ERR-4 (CODING_STANDARDS.md §4): error responses (4xx/5xx) use ONLY the keys
# {error, details}. Success (2xx) bodies are out of scope. Heuristic: error bodies
# are flat gin.H literals (no nested braces), so a gin.H{...} starting on the c.JSON
# line is captured up to its first `}`; multi-line literals accumulate until `}`.
# Portable POSIX awk (mawk/gawk): 2-arg match + substr only.
cd "$(dirname "$0")/../.."
DIR=apps/website/internal/handlers
files=$(find "$DIR" -maxdepth 1 -name '*.go' ! -name '*_test.go' | sort)

# shellcheck disable=SC2086
awk '
  function checkbuf(   s, tok, key) {
    s = buf
    while (match(s, /"[A-Za-z_]+"[ \t]*:/)) {
      tok = substr(s, RSTART, RLENGTH)
      key = tok; sub(/^"/, "", key); sub(/".*/, "", key)
      if (key != "error" && key != "details") {
        printf("ERR-4: %s:%d %s error body uses key \"%s\" (allowed: error, details)\n", file, line, status, key) > "/dev/stderr"
        fail = 1
      }
      s = substr(s, RSTART + RLENGTH)
    }
    buf = ""
  }
  BEGIN { fail = 0 }
  collect {
    buf = buf " " $0
    if (index($0, "}") > 0) { checkbuf(); collect = 0 }
    next
  }
  {
    if (match($0, /c\.JSON\(http\.Status[A-Za-z]+, gin\.H\{/)) {
      pre = substr($0, RSTART, RLENGTH)
      sub(/c\.JSON\(http\.Status/, "", pre); sub(/, gin\.H\{.*/, "", pre)
      if (pre ~ /^(BadRequest|Unauthorized|Forbidden|NotFound|MethodNotAllowed|Conflict|Gone|RequestEntityTooLarge|UnprocessableEntity|TooManyRequests|PreconditionFailed|Locked|InternalServerError|NotImplemented|BadGateway|ServiceUnavailable|GatewayTimeout)$/) {
        status = pre; file = FILENAME; line = FNR
        rest = substr($0, RSTART); sub(/.*gin\.H\{/, "", rest)
        buf = rest
        if (index(buf, "}") > 0) { checkbuf() } else { collect = 1 }
      }
    }
  }
  END { exit fail }
' $files
echo "ERR-4: all error responses use only {error, details}."
