#!/usr/bin/env bash
# T-438 / T-461 — deploy-staging.sh must point docker compose at
# apps/website/docker-compose.staging.yml (T-251), never the api/ sibling.
#
# T-461 (Wave 23 adversarial): prior Class-R was false-green —
#   (1) a `//` / `#` comment containing the good path counted as presence;
#   (2) only one exact `cd '$TBD_REMOTE_DIR/apps/website/api'` string was banned,
#       so live could use api/compose while dry-run stayed good (or good path
#       lived only in a comment).
# This gate strips // and # comments, requires the good -f path on BOTH the
# dry-run echo and the live ssh_cmd compose lines (paths must match), and
# explicitly rejects apps/website/api/docker-compose.staging.yml on those lines.
#
# Gate: make verify-t438
#   (or: bash scripts/mod/verify-t438-deploy-staging-compose-path.sh)
# Wired into ci-local + ci.yml mod-gates-hosted via `make verify-t438` (T-467).
#
# OWNS WIDEN: wave_plan T-438 lists only scripts/mod/deploy-staging.sh; this
# script is the Class-R perturbation guard for that path contract. T-461 owns
# the script hardening.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/scripts/mod/deploy-staging.sh"
COMPOSE="$ROOT/apps/website/docker-compose.staging.yml"
STALE="$ROOT/apps/website/api/docker-compose.staging.yml"

GOOD_PATH='apps/website/docker-compose.staging.yml'
BAD_PATH='apps/website/api/docker-compose.staging.yml'

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

FAIL=0

# Python owns comment-strip + executable-line path pins (quote-safe).
# Exit codes: 0 = pins OK; 1 = pin fail (messages on stdout); 2 = internal error.
pin_out="$(
	GOOD_PATH="$GOOD_PATH" BAD_PATH="$BAD_PATH" FILE="$FILE" python3 - <<'PY'
import os, re, sys

good = os.environ["GOOD_PATH"]
bad = os.environ["BAD_PATH"]
path = os.environ["FILE"]
src = open(path, encoding="utf-8").read()

def strip_shell_comments(text: str) -> str:
    out = []
    i = 0
    n = len(text)
    in_squote = False
    in_dquote = False
    while i < n:
        c = text[i]
        if in_squote:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if c == "'":
                in_squote = False
            i += 1
            continue
        if in_dquote:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if c == '"':
                in_dquote = False
            i += 1
            continue
        if c == "'":
            in_squote = True
            out.append(c)
            i += 1
            continue
        if c == '"':
            in_dquote = True
            out.append(c)
            i += 1
            continue
        if c == "#":
            while i < n and text[i] != "\n":
                i += 1
            continue
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            i += 2
            while i < n and text[i] != "\n":
                i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)

stripped = strip_shell_comments(src)
fail = 0

compose_re = re.compile(r"docker\s+compose\s+-f")
f_re = re.compile(r"""-f\s+(?:'([^']+)'|"([^"]+)"|(\S+))""")

dry_line = None
live_line = None
for raw in stripped.splitlines():
    line = raw.strip()
    if not compose_re.search(line):
        continue
    if "[dry-run]" in line:
        dry_line = line
    elif "ssh_cmd" in line:
        live_line = line

if dry_line is None:
    print("FAIL: no dry-run docker compose -f line after comment strip")
    fail = 1
if live_line is None:
    print("FAIL: no live ssh_cmd docker compose -f line after comment strip")
    fail = 1

def f_path(line: str):
    m = f_re.search(line)
    if not m:
        return None
    return next(g for g in m.groups() if g is not None)

dry_path = f_path(dry_line) if dry_line else None
live_path = f_path(live_line) if live_line else None

if dry_line is not None and dry_path is None:
    print("FAIL: dry-run compose line has no parseable -f path:")
    print(f"      {dry_line}")
    fail = 1
if live_line is not None and live_path is None:
    print("FAIL: live compose line has no parseable -f path:")
    print(f"      {live_line}")
    fail = 1

if dry_path is not None and dry_path != good:
    print(f"FAIL: dry-run -f path must be {good} (got: {dry_path})")
    fail = 1
if live_path is not None and live_path != good:
    print(f"FAIL: live ssh_cmd -f path must be {good} (got: {live_path})")
    fail = 1

if dry_path is not None and live_path is not None and dry_path != live_path:
    print("FAIL: dry-run and live compose -f paths diverge:")
    print(f"      dry-run: {dry_path}")
    print(f"      live:    {live_path}")
    fail = 1

for label, line in (("dry-run", dry_line), ("live", live_line)):
    if line and bad in line:
        print(f"FAIL: {label} compose line still references {bad}")
        print(f"      {line}")
        fail = 1

# Forbidden: cd into api for the compose step (stripped source).
if "cd '$TBD_REMOTE_DIR/apps/website/api'" in stripped:
    print("FAIL: deploy-staging.sh still cds into apps/website/api (compose must not)")
    fail = 1
if 'cd "$TBD_REMOTE_DIR/apps/website/api"' in stripped:
    print("FAIL: deploy-staging.sh still cds into apps/website/api (double-quoted form)")
    fail = 1

sys.exit(fail)
PY
)" || pin_rc=$?
pin_rc=${pin_rc:-0}

if [[ -n "$pin_out" ]]; then
	printf '%s\n' "$pin_out"
fi
if [[ "$pin_rc" -ne 0 ]]; then
	FAIL=1
fi

# File on disk must exist at the live path (T-251); must not exist under api/.
if [[ ! -f "$COMPOSE" ]]; then
	echo "FAIL: missing apps/website/docker-compose.staging.yml"
	FAIL=1
fi
if [[ -e "$STALE" ]]; then
	echo "FAIL: unexpected apps/website/api/docker-compose.staging.yml (stale path)"
	FAIL=1
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t438-deploy-staging-compose-path: FAIL"
	exit 1
fi

echo "verify-t438-deploy-staging-compose-path: PASS"
exit 0
