#!/usr/bin/env bash
# T-468 — CI schema job must stay on `make ci-local-schema` (full gate set).
#
# T-434 aligned `.github/workflows/ci.yml` to `make ci-local-schema` so CI runs
# the full schema-validate set (incl. map-object-enums) + citations. Without a
# tripwire, someone can revert the job to `cargo run -p xtask -- schema validate`
# + citations only and the map-object-enums hole returns while CI stays green.
#
# Gate: bash scripts/mod/verify-t468-ci-schema-parity.sh
# (No Makefile sibling — same shape as verify-t438 / verify-t456.)
#
# OWNS WIDEN: wave_plan T-468 lists `.github/workflows/ci.yml` +
# `scripts/platform/wave.sh`; this script is the Class-R perturbation guard
# (plus wiring into wave.sh gate_slice / cmd_gate).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/.github/workflows/ci.yml"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

# Python owns YAML-ish comment strip + schema-job run pins (quote-safe).
# Exit codes: 0 = pins OK; 1 = pin fail (messages on stdout); 2 = internal error.
pin_out="$(
	FILE="$FILE" python3 - <<'PY'
import os, re, sys

path = os.environ["FILE"]
src = open(path, encoding="utf-8").read()

def strip_yaml_hash_comments(text: str) -> str:
    """Strip # comments outside quotes. Preserves newlines for line structure."""
    out = []
    i = 0
    n = len(text)
    in_squote = False
    in_dquote = False
    while i < n:
        c = text[i]
        if in_squote:
            out.append(c)
            if c == "'" and not (i + 1 < n and text[i + 1] == "'"):
                in_squote = False
            elif c == "'" and i + 1 < n and text[i + 1] == "'":
                out.append(text[i + 1])
                i += 2
                continue
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
        out.append(c)
        i += 1
    return "".join(out)

stripped = strip_yaml_hash_comments(src)
lines = stripped.splitlines()

# Find top-level `schema:` job (2-space indent under jobs:).
schema_start = None
for idx, line in enumerate(lines):
    if re.match(r"^  schema:\s*$", line):
        schema_start = idx
        break

if schema_start is None:
    print("FAIL: no top-level `schema:` job in .github/workflows/ci.yml")
    sys.exit(1)

# Job body = lines after schema: until next top-level job key (2 spaces + word + :).
schema_lines = []
for line in lines[schema_start + 1 :]:
    if re.match(r"^  [A-Za-z0-9_-]+:\s*$", line):
        break
    schema_lines.append(line)

if not schema_lines:
    print("FAIL: schema job has empty body")
    sys.exit(1)

# Collect executable `run:` values (single-line only — CI uses single-line today).
run_re = re.compile(r"^\s+-\s+run:\s*(.+?)\s*$|^\s+run:\s*(.+?)\s*$")
runs = []
for line in schema_lines:
    m = run_re.match(line)
    if not m:
        continue
    cmd = next(g for g in m.groups() if g is not None).strip()
    # Drop surrounding quotes if present.
    if (cmd.startswith("'") and cmd.endswith("'")) or (cmd.startswith('"') and cmd.endswith('"')):
        cmd = cmd[1:-1]
    runs.append(cmd)

fail = 0

if not runs:
    print("FAIL: schema job has no `run:` steps after comment strip")
    fail = 1

GOOD = "make ci-local-schema"
# Exact match preferred; also accept whitespace variants of the same make target.
has_good = any(
    r == GOOD or re.fullmatch(r"make\s+ci-local-schema", r) is not None for r in runs
)

if not has_good:
    print("FAIL: schema job must run `make ci-local-schema` (full gate set)")
    print("      found run steps:")
    for r in runs:
        print(f"        - {r}")
    print("      Pre-T-434 hole: validate + citations alone misses map-object-enums.")
    fail = 1

# Ban the narrow xtask-only pattern that reopens the enums hole when ci-local-schema is absent.
narrow = [
    r
    for r in runs
    if re.search(r"schema\s+validate\b", r)
    and "ci-local-schema" not in r
    and "schema-validate" not in r
]
if narrow and not has_good:
    print("FAIL: schema job uses narrow `schema validate` without ci-local-schema:")
    for r in narrow:
        print(f"      {r}")
    fail = 1

# Soft ban: if both good and a lone validate step exist, still require good (already checked).
# Extra: ensure Makefile still defines the target we pin (repo contract).
# path = <root>/.github/workflows/ci.yml → three dirname hops to repo root.
root = path
for _ in range(3):
    root = os.path.dirname(root)
makefile = os.path.join(root, "Makefile")
if not os.path.isfile(makefile):
    print(f"FAIL: missing Makefile at {makefile}")
    fail = 1
else:
    mf = open(makefile, encoding="utf-8").read()
    if not re.search(r"(?m)^ci-local-schema:", mf):
        print("FAIL: Makefile missing `ci-local-schema:` target (CI pin would be vacuous)")
        fail = 1

sys.exit(fail)
PY
)" || pin_rc=$?
pin_rc=${pin_rc:-0}

if [[ -n "$pin_out" ]]; then
	printf '%s\n' "$pin_out"
fi
if [[ "$pin_rc" -ne 0 ]]; then
	echo "verify-t468-ci-schema-parity: FAIL"
	exit 1
fi

echo "verify-t468-ci-schema-parity: PASS"
exit 0
