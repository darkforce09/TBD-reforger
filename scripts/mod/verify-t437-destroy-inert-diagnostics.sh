#!/usr/bin/env bash
# T-437 / T-474 — Destroy-target inert diagnostics must not claim entities[] never spawn.
#
# After T-254, TBD_MissionDocumentStruct models entities[] and SpawnMissionEntities
# places resolvable rows. Operator-facing strings / schema prose that still blame a
# build that "does not spawn/model entities[]" are lies (wave 9 adversarial MAJOR M1).
#
# T-474 (Wave 27 THIS-WAVE BLOCKER): prior Class-R was false-green —
#   (1) paraphrased lies (`entities[] are never placed… (struct ignores them)`) PASS;
#   (2) collapsed DiagnoseEmpty returns with `out-of-zone placement` only in a comment PASS;
#   (3) renamed live fn with `DiagnoseEmptyDestroyTargets` only in a comment PASS;
#   (4) unresolved-alias string moved to a comment while pin still greened.
# Cure: strip // and /* */ before pin searches; require a live fn definition; require
# three distinct return-string arms inside DiagnoseEmptyDestroyTargets; broaden
# forbidden paraphrases; RED→GREEN on verifier perturbations.
#
# Gate: bash scripts/mod/verify-t437-destroy-inert-diagnostics.sh
# OWNS WIDEN: wave_plan T-437 lists Objectives/* + mission.schema.json; this script is
# the Class-R perturbation guard. Also covers TBD_MissionValidator.c (same lie class).
# T-474 owns this script + TBD_ObjectiveRegistry.c (live code only if pinability requires).
#
# T-556 DIAGNOSIS. This gate was RED on merged main, and its trace was misleading: every
# RED proof fired as designed and the GREEN proof printed PASS, yet it returned 1. The
# cause was ONE line — the `rg` in assert_other_pins (below) — hit four times, and those
# four failures print BEFORE the perturbation proofs, so anyone reading the tail of the
# output saw only healthy lines and an unexplained exit 1. There is no second bug: with a
# working search tool on PATH this script exits 0 unchanged. `rg` is installed nowhere;
# it resolves only inside an agent shell that injects a function of that name. Moved to
# `grep -F` via scripts/mod/lib/gate-grep.sh.
#
# Note which direction the old bug ran: `if ! rg …` is a PIN, so an absent tool (127)
# inverted to true and failed LOUDLY. That is why this gate went red instead of quietly
# green — the opposite of the fail-open bans in the sibling scripts. It was still wrong:
# the message named a missing truth pin when the truth pin was there and the tool was not.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=lib/gate-grep.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/gate-grep.sh"

REG="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c"
COMP="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c"
RULES="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRules.c"
SCHEMA="$ROOT/packages/tbd-schema/schema/mission.schema.json"
VALIDATOR="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c"

for f in "$REG" "$COMP" "$RULES" "$SCHEMA" "$VALIDATOR"; do
	if [[ ! -f "$f" ]]; then
		echo "FAIL: missing $f"
		exit 1
	fi
done

# Forbidden exact historical lies + T-474 paraphrases are enforced inside
# scan_forbidden_file (Python). Negation wrappers ("never a 'build does not spawn
# entities[]' claim/lie") are allowlisted so truth-telling docs stay green.

scan_forbidden_file() {
	local file="$1"
	python3 - "$file" <<'PY'
import re, sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
fail = 0

exact = [
	"This build does not spawn the mission document",
	"does not spawn the mission document",
	"nothing spawns the mission document",
	"nothing spawns mission `entities[]`",
	"on today's build nothing spawns mission",
	"TBD_MissionDocumentStruct does not model them",
	"TBD_MissionDocumentStruct ignore `entities[]`",
	"does not spawn mission entities",
	"this build cannot create it",
]
for needle in exact:
	idx = text.find(needle)
	if idx >= 0:
		line = text.count("\n", 0, idx) + 1
		print(f"FAIL: forbidden lie in {path}:")
		print(f"  {line}: {needle}")
		fail = 1

# Normalize whitespace so split //! lines still match contiguous paraphrases.
norm = re.sub(r"\s+", " ", text)
paraphrases = [
	r"are never placed",
	r"never placed",
	r"struct ignores",
	r"ignores them",
	r"ignores entities",
	r"does not spawn entities",
	r"does not model entities",
]
# Truth docs that quote the lie inside a "never a … claim/lie" wrapper.
allow = re.compile(
	r"never a ['\"]build does not spawn entities\[\]['\"] (?:claim|lie)",
	re.I,
)
allow_spans = [m.span() for m in allow.finditer(norm)]

def in_allow(a, b):
	for s, e in allow_spans:
		if a >= s and b <= e:
			return True
	return False

for pat in paraphrases:
	for m in re.finditer(pat, norm, flags=re.I):
		if in_allow(m.start(), m.end()):
			continue
		# Also allow when the match sits inside a slightly wider "never a …" window
		# (multiline quote joined by normalize may leave the closing word just after).
		window = norm[max(0, m.start() - 24) : m.end() + 24]
		if re.search(r"never a ['\"][^'\"]{0,40}" + re.escape(m.group(0)), window, re.I):
			continue
		if re.search(r"never a ['\"]build does not spawn", window, re.I) and "entities" in m.group(0).lower():
			continue
		print(f"FAIL: forbidden paraphrase in {path}:")
		print(f"  /{pat}/ matched: {m.group(0)!r}")
		fail = 1
		break
	if fail:
		break

sys.exit(fail)
PY
}

# Assert structural truth pins on a registry source path (live or TMP perturbation).
assert_registry_pins() {
	local src="$1"
	local label="$2"
	python3 - "$src" "$label" <<'PY'
import re, sys

path, label = sys.argv[1], sys.argv[2]
raw = open(path, encoding="utf-8").read()

def strip_c_comments(src: str) -> str:
    out = []
    i = 0
    n = len(src)
    while i < n:
        if src[i] == "/" and i + 1 < n and src[i + 1] == "/":
            i += 2
            while i < n and src[i] != "\n":
                i += 1
            continue
        if src[i] == "/" and i + 1 < n and src[i + 1] == "*":
            i += 2
            while i + 1 < n and not (src[i] == "*" and src[i + 1] == "/"):
                if src[i] == "\n":
                    out.append("\n")
                i += 1
            i = min(i + 2, n)
            continue
        out.append(src[i])
        i += 1
    return "".join(out)

stripped = strip_c_comments(raw)
fail = 0

def fail_msg(msg: str) -> None:
    global fail
    print(f"FAIL ({label}): {msg}")
    fail = 1

# Live fn definition (not comment-only).
defn_re = re.compile(
    r"protected\s+static\s+string\s+DiagnoseEmptyDestroyTargets\s*\(",
)
if not defn_re.search(stripped):
    fail_msg("missing live definition `protected static string DiagnoseEmptyDestroyTargets(` (non-comment)")

# Extract DiagnoseEmptyDestroyTargets method body (signature through closing brace at column tab).
m = re.search(
    r"protected\s+static\s+string\s+DiagnoseEmptyDestroyTargets\s*\(notnull TBD_Objective objective\)\s*\{",
    stripped,
)
if not m:
    fail_msg("could not locate DiagnoseEmptyDestroyTargets body after comment strip")
    sys.exit(fail)

# Brace-match from the opening `{` of the method.
start = m.end() - 1  # points at '{'
depth = 0
end = None
for i in range(start, len(stripped)):
    ch = stripped[i]
    if ch == "{":
        depth += 1
    elif ch == "}":
        depth -= 1
        if depth == 0:
            end = i
            break
if end is None:
    fail_msg("DiagnoseEmptyDestroyTargets body was not brace-closed")
    sys.exit(fail)

body = stripped[start : end + 1]

# Three distinct live return-string arms (executable, not comments).
arms = {
    "out-of-zone": "out-of-zone placement",
    "missing-row": "No `entities[]` row with that alias was authored",
    "spawn-miss": "spawn likely skipped or failed",
}
for name, needle in arms.items():
    # Must appear inside a return string.Format(...) in the body.
    if needle not in body:
        fail_msg(f"DiagnoseEmptyDestroyTargets body missing live {name} arm pin: {needle!r}")
        continue
    # Ensure it sits on a return path (string.Format return), not a stray identifier.
    if not re.search(
        r"return\s+string\.Format\([^;]*" + re.escape(needle),
        body,
        re.S,
    ):
        fail_msg(
            f"DiagnoseEmptyDestroyTargets {name} pin {needle!r} is not inside a "
            f"`return string.Format(...)` arm"
        )

# Unresolved-alias registry pin must remain live executable code (ArmDestroyTargets),
# not comment-only (T-474 attack 4).
reg_pin = "not in the registry, so there is no prefab to look for"
if reg_pin not in stripped:
    fail_msg(f"missing live registry pin (non-comment): {reg_pin!r}")
elif not re.search(
    r"string\.Format\([^;]*" + re.escape(reg_pin),
    stripped,
    re.S,
):
    fail_msg(f"registry pin {reg_pin!r} is not inside a live string.Format(...)")

# SpawnMissionEntities must appear in a live return arm (missing-row), not only //! docs.
if "SpawnMissionEntities" not in body:
    fail_msg("DiagnoseEmptyDestroyTargets body missing live SpawnMissionEntities mention")

sys.exit(fail)
PY
}

# Surface pins on COMP/RULES/SCHEMA/VALIDATOR remain raw presence checks (T-437): those
# truths live in //! doc comments by design. Comment-strip applies to REG structural pins
# only — that is the false-green class T-474 closes.
assert_other_pins() {
	local file="$1"
	shift
	local pin rc=0
	for pin in "$@"; do
		# -F is the old `--fixed-strings`: these pins carry `[]`, backticks and `+`, and
		# must stay literal. gate_require distinguishes "the pin is genuinely gone" from
		# "the file moved" from "the search tool is absent" — the last two used to arrive
		# here wearing the first one's message (T-556).
		gate_require "missing truth pin in ${file#"$ROOT"/}: $pin" -F "$pin" "$file" || rc=1
	done
	return "$rc"
}

FAIL=0

for f in "$REG" "$COMP" "$RULES" "$SCHEMA" "$VALIDATOR"; do
	if ! scan_forbidden_file "$f"; then
		FAIL=1
	fi
done

if ! assert_registry_pins "$REG" "live"; then
	FAIL=1
fi

if ! assert_other_pins "$COMP" 'SpawnMissionEntities'; then FAIL=1; fi
if ! assert_other_pins "$RULES" 'TBD_MissionDocumentStruct` models `entities[]`'; then FAIL=1; fi
if ! assert_other_pins "$SCHEMA" 'SpawnMissionEntities' 'out-of-zone authorship'; then FAIL=1; fi
if ! assert_other_pins "$VALIDATOR" 'entities[] is modeled + SpawnMissionEntities'; then FAIL=1; fi

# ── RED→GREEN perturbation proofs (TMP only; live files untouched) ─────────────
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

# RED 1: paraphrased lie (entities never placed / struct ignores them)
python3 - "$REG" "$TMP" <<'PY'
import sys
src = open(sys.argv[1], encoding="utf-8").read()
# Inject into ArmDestroyTargets doc block — comment-only lie that old verifier ignored.
lie = '\t//! entities[] are never placed on today\'s build (struct ignores them)\n'
if "ArmDestroyTargets" not in src:
    sys.stderr.write("RED1 setup failed: ArmDestroyTargets missing\n")
    sys.exit(2)
# Insert just before the ArmDestroyTargets signature.
out = src.replace(
    "\tstatic void ArmDestroyTargets(notnull TBD_Objective objective)",
    lie + "\tstatic void ArmDestroyTargets(notnull TBD_Objective objective)",
    1,
)
if out == src:
    sys.stderr.write("RED1 setup failed: could not inject paraphrase\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if scan_forbidden_file "$TMP" 2>/dev/null; then
	echo "FAIL: RED paraphrased lie still passed — forbidden paraphrases not discriminating"
	FAIL=1
else
	echo "RED proof: paraphrased 'never placed' / 'struct ignores them' lie → FAIL (expected)"
fi

# RED 2: collapse three DiagnoseEmpty returns → one generic; leave out-of-zone in a comment
python3 - "$REG" "$TMP" <<'PY'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
pat = re.compile(
    r"(protected static string DiagnoseEmptyDestroyTargets\(notnull TBD_Objective objective\)\n\t\{).*?(\n\t\})",
    re.S,
)
repl = r'''\1
		//! Distinguishes missing/skipped spawn vs out-of-zone placement (comment only — T-474 RED).
		return "destroy targets empty — no matches in zone";
	\2'''
out, n = pat.subn(repl, src, count=1)
if n != 1:
    sys.stderr.write(f"RED2 setup failed: could not collapse DiagnoseEmpty returns (n={n})\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if assert_registry_pins "$TMP" "RED-collapse-returns" 2>/dev/null; then
	echo "FAIL: RED collapsed DiagnoseEmpty returns still passed — return-arm pins ignore comments?"
	FAIL=1
else
	echo "RED proof: collapsed DiagnoseEmpty returns (out-of-zone only in comment) → FAIL (expected)"
fi

# RED 3: rename live fn; leave DiagnoseEmptyDestroyTargets only in a comment
python3 - "$REG" "$TMP" <<'PY'
import sys
src = open(sys.argv[1], encoding="utf-8").read()
# Comment-only leftover of the old name, then rename def + call.
src2 = src.replace(
    "\tprotected static string DiagnoseEmptyDestroyTargets(notnull TBD_Objective objective)",
    "\t//! DiagnoseEmptyDestroyTargets — renamed; name kept in comment only (T-474 RED).\n"
    "\tprotected static string DiagnoseEmptyTargets(notnull TBD_Objective objective)",
    1,
)
src2 = src2.replace(
    "DiagnoseEmptyDestroyTargets(objective)",
    "DiagnoseEmptyTargets(objective)",
    1,
)
if "DiagnoseEmptyDestroyTargets(notnull" in src2 or src2 == src:
    sys.stderr.write("RED3 setup failed: live definition still present or rename missed\n")
    sys.exit(2)
# Ensure the name survives only in comments / strings we intentionally left.
open(sys.argv[2], "w", encoding="utf-8").write(src2)
PY
if assert_registry_pins "$TMP" "RED-rename-fn" 2>/dev/null; then
	echo "FAIL: RED comment-only DiagnoseEmptyDestroyTargets still passed — definition pin weak"
	FAIL=1
else
	echo "RED proof: DiagnoseEmptyDestroyTargets definition renamed (name comment-only) → FAIL (expected)"
fi

# RED 4: collapse unresolved-alias into DiagnoseEmpty; leave registry pin in a comment
python3 - "$REG" "$TMP" <<'PY'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
# Replace the live unresolved-alias string.Format with a comment that still mentions the pin.
old = (
    '\t\t\tobjective.m_sInertReason = string.Format('
    '"rules.targetAlias \'%1\' is not in the registry, so there is no prefab to look for", '
    'objective.m_sTargetAlias);'
)
new = (
    '\t\t\t//! was: not in the registry, so there is no prefab to look for (T-474 RED comment-only)\n'
    '\t\t\tobjective.m_sInertReason = DiagnoseEmptyDestroyTargets(objective);'
)
if old not in src:
    sys.stderr.write("RED4 setup failed: unresolved-alias Format line not found\n")
    sys.exit(2)
out = src.replace(old, new, 1)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if assert_registry_pins "$TMP" "RED-registry-comment" 2>/dev/null; then
	echo "FAIL: RED comment-only registry pin still passed — pin search ignores comments?"
	FAIL=1
else
	echo "RED proof: unresolved-alias registry pin comment-only → FAIL (expected)"
fi

# RED 5: exact historical lie restore (keep — must still FAIL)
python3 - "$REG" "$TMP" <<'PY'
import sys
src = open(sys.argv[1], encoding="utf-8").read()
lie = (
    '\t//! This build does not spawn the mission document `entities[]` — '
    'TBD_MissionDocumentStruct does not model them\n'
)
out = src.replace(
    "\tstatic void ArmDestroyTargets(notnull TBD_Objective objective)",
    lie + "\tstatic void ArmDestroyTargets(notnull TBD_Objective objective)",
    1,
)
if out == src:
    sys.stderr.write("RED5 setup failed: could not inject historical lie\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if scan_forbidden_file "$TMP" 2>/dev/null; then
	echo "FAIL: RED exact historical lie restore still passed"
	FAIL=1
else
	echo "RED proof: exact historical lie restore → FAIL (expected)"
fi

# GREEN: live registry must still PASS after all RED proofs
if ! assert_registry_pins "$REG" "live-restore"; then
	echo "FAIL: live registry no longer passes after RED proofs (REG should be untouched)"
	FAIL=1
else
	echo "GREEN proof: live DiagnoseEmptyDestroyTargets arms + registry pin → PASS"
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t437-destroy-inert-diagnostics: FAIL"
	exit 1
fi

echo "verify-t437-destroy-inert-diagnostics: PASS"
exit 0
