#!/usr/bin/env bash
# T-456 / T-460 — OnBackendFetchSuccess must refuse oversized REST bodies before
# ParseMissionJson, using the same MISSION_FILE_MAX_BYTES ceiling as LoadFromProfileFile.
#
# T-460 (Wave 22 adversarial): prior Class-R was false-green —
#   (1) a `//` comment containing MISSION_FILE_MAX_BYTES counted as the size check
#       before ParseMissionJson;
#   (2) only the IsMissionBodyWithinCap signature was required — `return true;` greens.
# This gate strips comments before the order assert, requires a live
# IsMissionBodyWithinCap( call before ParseMissionJson(, and pins the helper body to
# Length() <= MISSION_FILE_MAX_BYTES.
#
# Gate: bash scripts/mod/verify-t456-mission-rest-size-gate.sh
# (No Makefile sibling hook — T-452/T-296 verify scripts are the same shape.)
#
# OWNS WIDEN: wave_plan T-456 lists TBD_MissionLoader.c; this script is the Class-R
# perturbation guard for the REST size gate. T-460 owns the script hardening.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

# Strip Enforce `//` line comments and `/* */` block comments so comment text cannot
# satisfy the before-parse order pin (T-460 adversarial).
strip_c_comments() {
	python3 -c '
import sys
src = sys.stdin.read()
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
sys.stdout.write("".join(out))
'
}

# Extract OnBackendFetchSuccess body (from its signature through the next method).
extract_success() {
	local src="$1"
	awk '
		/^[[:space:]]*protected static void OnBackendFetchSuccess\(RestCallback cb\)/ { grab=1 }
		grab { print }
		grab && /^[[:space:]]*protected static void OnBackendFetchError\(RestCallback cb\)/ { exit }
	' "$src"
}

# Extract IsMissionBodyWithinCap method body (signature through closing brace of method).
extract_helper() {
	local src="$1"
	awk '
		/^[[:space:]]*protected static bool IsMissionBodyWithinCap\(string data\)/ { grab=1 }
		grab { print }
		grab && /^[[:space:]]*protected static bool ParseMissionJson\(string data\)/ { exit }
	' "$src"
}

assert_rest_size_gate() {
	local src="$1"
	local label="$2"
	local body raw_body helper stripped
	raw_body="$(extract_success "$src")"

	if [[ -z "$raw_body" ]]; then
		echo "FAIL ($label): could not extract OnBackendFetchSuccess"
		return 1
	fi

	# Cap constant must appear in the success handler (not only in the profile path).
	# Comment-stripped so a lone comment mentioning the constant cannot satisfy this.
	stripped="$(strip_c_comments <<<"$raw_body")"
	if ! grep -q 'MISSION_FILE_MAX_BYTES' <<<"$stripped"; then
		echo "FAIL ($label): OnBackendFetchSuccess has no non-comment MISSION_FILE_MAX_BYTES reference"
		return 1
	fi

	# T-460: size check must be a live IsMissionBodyWithinCap( call before ParseMissionJson(
	# inside OnBackendFetchSuccess — comments are stripped first so // … MISSION_FILE_MAX_BYTES
	# cannot win the order race.
	body="$stripped"
	local check_line parse_line
	check_line="$(grep -n 'IsMissionBodyWithinCap(' <<<"$body" | head -1 | cut -d: -f1 || true)"
	parse_line="$(grep -n 'ParseMissionJson(' <<<"$body" | head -1 | cut -d: -f1 || true)"
	if [[ -z "$check_line" || -z "$parse_line" ]]; then
		echo "FAIL ($label): missing IsMissionBodyWithinCap( and/or ParseMissionJson( in OnBackendFetchSuccess (non-comment)"
		return 1
	fi
	if (( check_line >= parse_line )); then
		echo "FAIL ($label): IsMissionBodyWithinCap( (line $check_line) is not before ParseMissionJson( (line $parse_line)"
		return 1
	fi

	# Helper must exist and its body must actually compare Length() to the cap
	# (signature-only + `return true;` is a false-green — T-460).
	if ! rg -q 'protected static bool IsMissionBodyWithinCap\(string data\)' "$src"; then
		echo "FAIL ($label): missing IsMissionBodyWithinCap(string) helper"
		return 1
	fi
	helper="$(extract_helper "$src")"
	helper="$(strip_c_comments <<<"$helper")"
	if ! grep -Eq 'Length\(\)[[:space:]]*<=[[:space:]]*MISSION_FILE_MAX_BYTES' <<<"$helper"; then
		echo "FAIL ($label): IsMissionBodyWithinCap body does not compare Length() <= MISSION_FILE_MAX_BYTES"
		return 1
	fi
	# Reject an always-true stub that also happens to mention the compare in a dead branch.
	if grep -Eq 'return[[:space:]]+true[[:space:]]*;' <<<"$helper" \
		&& ! grep -Eq 'return[[:space:]]+data\.Length\(\)[[:space:]]*<=[[:space:]]*MISSION_FILE_MAX_BYTES[[:space:]]*;' <<<"$helper"; then
		echo "FAIL ($label): IsMissionBodyWithinCap returns true without the Length() <= MISSION_FILE_MAX_BYTES return"
		return 1
	fi

	if ! rg -q 'MISSION_FILE_MAX_BYTES = 8 \* 1024 \* 1024' "$src"; then
		echo "FAIL ($label): MISSION_FILE_MAX_BYTES is not the pinned 8*1024*1024"
		return 1
	fi

	# Profile path must still gate on the same constant (no drift).
	if ! rg -q 'fileSize > MISSION_FILE_MAX_BYTES' "$src"; then
		echo "FAIL ($label): LoadFromProfileFile no longer compares fileSize to MISSION_FILE_MAX_BYTES"
		return 1
	fi

	return 0
}

FAIL=0

if ! assert_rest_size_gate "$FILE" "live"; then
	FAIL=1
fi

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

# ── RED 1: comment-only "size check" (comment mentions the constant; live call removed) ──
python3 - "$FILE" "$TMP" <<'PY'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
# Drop the live IsMissionBodyWithinCap if-block; leave the T-456 comments that mention the cap.
pat = re.compile(
    r"\n\t\tif \(!IsMissionBodyWithinCap\(data\)\)\n\t\t\{.*?\n\t\t\}\n",
    re.S,
)
out, n = pat.subn("\n", src, count=1)
if n != 1:
    sys.stderr.write(f"RED1 setup failed: could not strip IsMissionBodyWithinCap call (n={n})\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if assert_rest_size_gate "$TMP" "RED-comment-only"; then
	echo "FAIL: RED comment-only still passed — order pin ignores comments? or call not required"
	FAIL=1
else
	echo "RED proof: comment-only MISSION_FILE_MAX_BYTES (no live IsMissionBodyWithinCap) → FAIL (expected)"
fi

# ── RED 2: post-parse check (call moved after ParseMissionJson) ──
python3 - "$FILE" "$TMP" <<'PY'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
block_re = re.compile(
    r"\n\t\t// T-456 — REST path must honour the same MISSION_FILE_MAX_BYTES ceiling as profile load\."
    r".*?\n\t\t\}\n",
    re.S,
)
m = block_re.search(src)
if not m:
    sys.stderr.write("RED2 setup failed: could not find T-456 REST size-gate block\n")
    sys.exit(2)
gate_block = m.group(0)
src_wo = src[: m.start()] + "\n" + src[m.end() :]
# Insert the gate block immediately after the ParseMissionJson if-block inside OnBackendFetchSuccess.
parse_re = re.compile(
    r"(if \(!ParseMissionJson\(data\)\)\n\t\t\{.*?\n\t\t\}\n)",
    re.S,
)
out, n = parse_re.subn(r"\1" + gate_block, src_wo, count=1)
if n != 1:
    sys.stderr.write(f"RED2 setup failed: could not relocate gate after ParseMissionJson (n={n})\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if assert_rest_size_gate "$TMP" "RED-post-parse"; then
	echo "FAIL: RED post-parse still passed — order pin is not discriminating"
	FAIL=1
else
	echo "RED proof: IsMissionBodyWithinCap after ParseMissionJson → FAIL (expected)"
fi

# ── RED 3: helper stubbed to `return true;` ──
python3 - "$FILE" "$TMP" <<'PY'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
out, n = re.subn(
    r"(protected static bool IsMissionBodyWithinCap\(string data\)\n\t\{\n\t\t)return data\.Length\(\) <= MISSION_FILE_MAX_BYTES;",
    r"\1return true;",
    src,
    count=1,
)
if n != 1:
    sys.stderr.write(f"RED3 setup failed: could not stub IsMissionBodyWithinCap (n={n})\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY
if assert_rest_size_gate "$TMP" "RED-return-true"; then
	echo "FAIL: RED return-true helper still passed — body pin is not discriminating"
	FAIL=1
else
	echo "RED proof: IsMissionBodyWithinCap return true → FAIL (expected)"
fi

# Live file must still PASS after all RED perturbations (TMP only; FILE untouched).
if ! assert_rest_size_gate "$FILE" "live-restore"; then
	echo "FAIL: live file no longer passes after RED proofs (FILE should be untouched)"
	FAIL=1
else
	echo "GREEN proof: live IsMissionBodyWithinCap before ParseMissionJson + Length() compare → PASS"
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t456-mission-rest-size-gate: FAIL"
	exit 1
fi

echo "verify-t456-mission-rest-size-gate: PASS"
exit 0
