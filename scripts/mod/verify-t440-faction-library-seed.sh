#!/usr/bin/env bash
# T-440 / T-478 — Class-R: `make seed` must apply `seeds/faction_library.sql` via a
# live shell redirect to psql, and the seed file must carry a live
# `INSERT INTO user_factions` with starter BLUFOR name `'US Army 1980s'` (T-256).
#
# Wave 10 / residual adversarial: cold/schema gates validate
# faction-library.sample.json but never pin that `make seed` applies
# apps/website/api/seeds/faction_library.sql. Deleting that Makefile seed
# line still greens the cold gate.
#
# T-478 (Wave 29 THIS-WAVE BLOCKER): prior Class-R was false-green —
#   (1) raw `grep 'US Army 1980s'` PASS'd `-- US Army 1980s` + `SELECT 1;`;
#   (2) path substring on any non-# recipe line PASS'd
#       `echo seeds/faction_library.sql >/dev/null` and psql `-c` comment smuggles;
#   (3) script never pinned wave.sh cold (`cmd_gate`) + slice (`gate_slice`) wiring.
# Cure: strip SQL `--` / `/* */` before name pin; require live INSERT INTO
# user_factions that includes `'US Army 1980s'` as a string literal; require a
# recipe line with shell redirect `< seeds/faction_library.sql` (reject echo);
# pin wave.sh both gate paths invoke this script. RED→GREEN on perturbations.
#
# Gate: bash scripts/mod/verify-t440-faction-library-seed.sh
# (Wired into scripts/platform/wave.sh gate / gate --slice as "T-440 faction library seed".)
# make verify-t440 → this script.
#
# OWNS WIDEN: wave_plan T-440/T-478 lists Makefile + wave.sh + faction_library.sql;
# this script is the Class-R perturbation guard (same spirit as T-437/T-444/T-472).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MAKEFILE="$ROOT/Makefile"
SEED="$ROOT/apps/website/api/seeds/faction_library.sql"
WAVE="$ROOT/scripts/platform/wave.sh"
VERIFY_REL='scripts/mod/verify-t440-faction-library-seed.sh'
STARTER_NAME='US Army 1980s'

if [[ ! -f "$MAKEFILE" ]]; then
	echo "FAIL: missing $MAKEFILE"
	echo "      restore Makefile so the seed recipe can be pinned."
	exit 1
fi

if [[ ! -f "$SEED" ]]; then
	echo "FAIL: missing $SEED"
	echo "      T-440 requires apps/website/api/seeds/faction_library.sql for make seed."
	exit 1
fi

if [[ ! -s "$SEED" ]]; then
	echo "FAIL: $SEED is empty"
	echo "      seed file must contain starter faction library rows (BLUFOR + OPFOR)."
	exit 1
fi

if [[ ! -f "$WAVE" ]]; then
	echo "FAIL: missing $WAVE"
	echo "      T-478 requires wave.sh cold + slice wiring for this verify script."
	exit 1
fi

# Assert seed SQL + Makefile recipe + wave.sh dual-path pins.
# Args: seed_path makefile_path wave_path label
# Exit 0 = OK; 1 = pin fail (messages on stdout).
assert_t440_pins() {
	local seed="$1" makefile="$2" wave="$3" label="$4"
	SEED="$seed" MAKEFILE="$makefile" WAVE="$wave" LABEL="$label" \
		STARTER_NAME="$STARTER_NAME" VERIFY_REL="$VERIFY_REL" python3 - <<'PY'
import os, re, sys

seed = os.environ["SEED"]
makefile = os.environ["MAKEFILE"]
wave = os.environ["WAVE"]
label = os.environ["LABEL"]
starter = os.environ["STARTER_NAME"]
verify_rel = os.environ["VERIFY_REL"]
fail = 0

def fail_msg(msg: str) -> None:
    global fail
    print(f"FAIL ({label}): {msg}")
    fail = 1

def strip_sql_comments(src: str) -> str:
    """Strip SQL -- line and /* */ block comments; preserve string literals + newlines."""
    out = []
    i = 0
    n = len(src)
    in_squote = False
    in_dquote = False
    while i < n:
        c = src[i]
        if in_squote:
            out.append(c)
            # SQL '' escape inside single-quoted string
            if c == "'" and i + 1 < n and src[i + 1] == "'":
                out.append(src[i + 1])
                i += 2
                continue
            if c == "'":
                in_squote = False
            i += 1
            continue
        if in_dquote:
            out.append(c)
            if c == '"' and i + 1 < n and src[i + 1] == '"':
                out.append(src[i + 1])
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
        # -- line comment
        if c == "-" and i + 1 < n and src[i + 1] == "-":
            i += 2
            while i < n and src[i] != "\n":
                i += 1
            continue
        # /* block comment */
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            i += 2
            while i + 1 < n and not (src[i] == "*" and src[i + 1] == "/"):
                if src[i] == "\n":
                    out.append("\n")
                i += 1
            i = min(i + 2, n)
            continue
        out.append(c)
        i += 1
    return "".join(out)

def strip_hash_comments(text: str) -> str:
    """Strip # comments outside quotes (Makefile / bash). Preserves newlines."""
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

# ── 1. Seed: live INSERT INTO user_factions + starter name string literal ──────
raw_seed = open(seed, encoding="utf-8").read()
stripped_seed = strip_sql_comments(raw_seed)
lit = f"'{starter}'"
# Require INSERT INTO user_factions that includes the name literal (same statement-ish span).
insert_re = re.compile(
    r"(?is)INSERT\s+INTO\s+user_factions\b(?:(?!;).)*?" + re.escape(lit)
)
if not insert_re.search(stripped_seed):
    fail_msg(
        f"seed must contain live `INSERT INTO user_factions` including {lit} "
        f"(non-comment). Comment-only name + SELECT 1 is not enough (T-478)."
    )
elif lit not in stripped_seed:
    fail_msg(f"missing live string literal {lit} after SQL comment strip")

# ── 2. Makefile seed: recipe must redirect-apply the file (not echo/path smuggle) ─
mf = open(makefile, encoding="utf-8").read()
recipe_lines = []
in_seed = False
for line in mf.splitlines():
    if re.match(r"^seed:", line):
        in_seed = True
        continue
    if not in_seed:
        continue
    if re.match(r"^[^\s#]", line) and not line.startswith("\t"):
        break
    if line.startswith("\t"):
        recipe_lines.append(line)

if not recipe_lines:
    fail_msg("Makefile has no tab-indented body under the seed: target")
else:
    live = []
    for ln in recipe_lines:
        cleaned = strip_hash_comments(ln).rstrip()
        if re.match(r"^\t\s*$", cleaned) or cleaned == "\t":
            continue
        live.append(cleaned)

    # Live contract (Makefile seed:): shell redirect into psql —
    #   ... < seeds/faction_library.sql
    # Rejects: echo path, path in -c comment, path-only substring without `<`.
    redirect_re = re.compile(r"<\s*seeds/faction_library\.sql\b")
    echo_smuggle = re.compile(r"\becho\b.*seeds/faction_library\.sql")
    has_redirect = any(redirect_re.search(ln) for ln in live)
    if any(echo_smuggle.search(ln) for ln in live) and not has_redirect:
        fail_msg(
            "Makefile seed: recipe echoes seeds/faction_library.sql but does not "
            "redirect-apply it (`< seeds/faction_library.sql`)"
        )
    elif not has_redirect:
        fail_msg(
            "Makefile seed: recipe must apply seeds/faction_library.sql via shell "
            "redirect (`< seeds/faction_library.sql`), not a bare path / echo / "
            "psql -c comment smuggle (T-478)."
        )
        print("      found live recipe lines:")
        for ln in live:
            print(f"        {ln!r}")

# ── 3. wave.sh: both gate_slice (slice) and cmd_gate (cold) must invoke us ─────
wave_src = open(wave, encoding="utf-8").read()
wave_stripped = strip_hash_comments(wave_src)

def extract_fn_body(src: str, fn_name: str) -> str | None:
    m = re.search(rf"(?m)^{re.escape(fn_name)}\(\)\s*\{{", src)
    if not m:
        return None
    start = m.end() - 1  # '{'
    depth = 0
    for i in range(start, len(src)):
        ch = src[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return src[start : i + 1]
    return None

invoke_re = re.compile(
    r"run\s+\"T-440[^\"]*\"\s+bash\s+\"\$ROOT/"
    + re.escape(verify_rel)
    + r"\"",
)
# Also accept unquoted / slight spacing variants that still invoke the script.
invoke_loose = re.compile(re.escape(verify_rel))

for fn, role in (("gate_slice", "slice gate"), ("cmd_gate", "cold gate")):
    body = extract_fn_body(wave_stripped, fn)
    if body is None:
        fail_msg(f"wave.sh missing `{fn}()` ({role}) after comment strip")
        continue
    if not invoke_loose.search(body):
        fail_msg(
            f"wave.sh `{fn}()` ({role}) does not invoke {verify_rel} "
            f"(T-478 dual-path pin)"
        )
    elif not invoke_re.search(body) and verify_rel not in body:
        fail_msg(f"wave.sh `{fn}()` missing verify script path {verify_rel}")

sys.exit(fail)
PY
}

FAIL=0

if ! assert_t440_pins "$SEED" "$MAKEFILE" "$WAVE" "live"; then
	FAIL=1
fi

# ── RED→GREEN perturbation proofs (TMP only; live files untouched) ─────────────
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT
TMP_SEED="$TMPDIR/faction_library.sql"
TMP_MAKE="$TMPDIR/Makefile"
TMP_WAVE="$TMPDIR/wave.sh"

# RED 1: starter name only in SQL -- comment (+ SELECT 1)
cat > "$TMP_SEED" <<EOF
-- ${STARTER_NAME}
SELECT 1;
EOF
cp "$MAKEFILE" "$TMP_MAKE"
cp "$WAVE" "$TMP_WAVE"
if assert_t440_pins "$TMP_SEED" "$TMP_MAKE" "$TMP_WAVE" "RED-comment-name" 2>/dev/null; then
	echo "FAIL: RED comment-only '$STARTER_NAME' still passed — SQL comment strip weak"
	FAIL=1
else
	echo "RED proof: comment-only '$STARTER_NAME' + SELECT 1 → FAIL (expected)"
fi

# RED 2: recipe path via echo (no shell redirect)
cp "$SEED" "$TMP_SEED"
cp "$WAVE" "$TMP_WAVE"
python3 - "$MAKEFILE" "$TMP_MAKE" <<'PY'
import sys
src = open(sys.argv[1], encoding="utf-8").read()
old = "\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/faction_library.sql"
new = "\techo seeds/faction_library.sql >/dev/null"
if old not in src:
    sys.stderr.write("RED2 setup failed: live redirect recipe line not found\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(src.replace(old, new, 1))
PY
if assert_t440_pins "$TMP_SEED" "$TMP_MAKE" "$TMP_WAVE" "RED-echo-path" 2>/dev/null; then
	echo "FAIL: RED echo-path recipe still passed — redirect pin weak"
	FAIL=1
else
	echo "RED proof: echo seeds/faction_library.sql >/dev/null → FAIL (expected)"
fi

# RED 2b: path only inside a non-redirect psql -c comment (no `< file`)
python3 - "$MAKEFILE" "$TMP_MAKE" <<'PY'
import sys
src = open(sys.argv[1], encoding="utf-8").read()
old = "\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/faction_library.sql"
new = "\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger -c \"SELECT 1 -- seeds/faction_library.sql\""
if old not in src:
    sys.stderr.write("RED2b setup failed: live redirect recipe line not found\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(src.replace(old, new, 1))
PY
if assert_t440_pins "$TMP_SEED" "$TMP_MAKE" "$TMP_WAVE" "RED-psql-c-comment" 2>/dev/null; then
	echo "FAIL: RED psql -c path-in-comment still passed — redirect pin weak"
	FAIL=1
else
	echo "RED proof: psql -c with path in comment (no redirect) → FAIL (expected)"
fi

# RED 3: delete one wave.sh run (gate_slice only) — dual-path must FAIL
cp "$SEED" "$TMP_SEED"
cp "$MAKEFILE" "$TMP_MAKE"
python3 - "$WAVE" "$TMP_WAVE" <<'PY'
import sys
src = open(sys.argv[1], encoding="utf-8").read()
needle = '  run "T-440 faction library seed" bash "$ROOT/scripts/mod/verify-t440-faction-library-seed.sh"\n'
# Remove only the first occurrence (inside gate_slice).
idx = src.find(needle)
if idx < 0:
    sys.stderr.write("RED3 setup failed: wave.sh T-440 run line not found\n")
    sys.exit(2)
out = src[:idx] + src[idx + len(needle) :]
if out.count(needle) != 1:
    sys.stderr.write(
        f"RED3 setup failed: expected exactly 1 remaining run, got {out.count(needle)}\n"
    )
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if assert_t440_pins "$TMP_SEED" "$TMP_MAKE" "$TMP_WAVE" "RED-delete-one-wave-run" 2>/dev/null; then
	echo "FAIL: RED delete-one-wave.sh-run still passed — dual-path pin weak"
	FAIL=1
else
	echo "RED proof: delete one wave.sh T-440 run (gate_slice) → FAIL (expected)"
fi

# GREEN: live trio must still PASS after all RED proofs
if ! assert_t440_pins "$SEED" "$MAKEFILE" "$WAVE" "live-restore"; then
	echo "FAIL: live pins no longer pass after RED proofs (files should be untouched)"
	FAIL=1
else
	echo "GREEN proof: live INSERT + redirect recipe + wave dual-path → PASS"
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t440-faction-library-seed: FAIL"
	exit 1
fi

echo "PASS: T-440/T-478 faction library seed — live INSERT INTO user_factions '$STARTER_NAME'; Makefile \`< seeds/faction_library.sql\`; wave.sh gate_slice + cmd_gate wired"
exit 0
