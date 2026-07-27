#!/usr/bin/env bash
# T-456 — OnBackendFetchSuccess must refuse oversized REST bodies before ParseMissionJson,
# using the same MISSION_FILE_MAX_BYTES ceiling as LoadFromProfileFile.
#
# Gate: bash scripts/mod/verify-t456-mission-rest-size-gate.sh
# (No Makefile sibling hook — T-452/T-296 verify scripts are the same shape.)
#
# OWNS WIDEN: wave_plan T-456 lists TBD_MissionLoader.c; this script is the Class-R
# perturbation guard for the REST size gate.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

# Extract OnBackendFetchSuccess body (from its signature through the next method).
extract_success() {
	local src="$1"
	awk '
		/^[[:space:]]*protected static void OnBackendFetchSuccess\(RestCallback cb\)/ { grab=1 }
		grab { print }
		grab && /^[[:space:]]*protected static void OnBackendFetchError\(RestCallback cb\)/ { exit }
	' "$src"
}

assert_rest_size_gate() {
	local src="$1"
	local label="$2"
	local body
	body="$(extract_success "$src")"

	if [[ -z "$body" ]]; then
		echo "FAIL ($label): could not extract OnBackendFetchSuccess"
		return 1
	fi

	# Cap constant must appear in the success handler (not only in the profile path).
	if ! grep -q 'MISSION_FILE_MAX_BYTES' <<<"$body"; then
		echo "FAIL ($label): OnBackendFetchSuccess has no MISSION_FILE_MAX_BYTES reference"
		return 1
	fi

	# Size check must precede ParseMissionJson inside OnBackendFetchSuccess.
	local check_line parse_line
	check_line="$(grep -n 'MISSION_FILE_MAX_BYTES\|IsMissionBodyWithinCap' <<<"$body" | head -1 | cut -d: -f1)"
	parse_line="$(grep -n 'ParseMissionJson' <<<"$body" | head -1 | cut -d: -f1)"
	if [[ -z "$check_line" || -z "$parse_line" ]]; then
		echo "FAIL ($label): missing size check and/or ParseMissionJson in OnBackendFetchSuccess"
		return 1
	fi
	if (( check_line >= parse_line )); then
		echo "FAIL ($label): size check (line $check_line) is not before ParseMissionJson (line $parse_line)"
		return 1
	fi

	# Helper (or inline compare) must exist in the class; constant must stay the 8MiB pin.
	if ! rg -q 'protected static bool IsMissionBodyWithinCap\(string data\)' "$src"; then
		echo "FAIL ($label): missing IsMissionBodyWithinCap(string) helper"
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

# RED proof: strip the REST size-gate block → asserts must FAIL; live file unchanged.
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT
python3 - "$FILE" "$TMP" <<'PY'
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
# Remove the T-456 REST size-gate if-block inside OnBackendFetchSuccess.
pat = re.compile(
    r"\n\t\t// T-456 — REST path must honour the same MISSION_FILE_MAX_BYTES ceiling as profile load\."
    r".*?\n\t\t\}\n",
    re.S,
)
out, n = pat.subn("\n", src, count=1)
if n != 1:
    # Fallback: drop any IsMissionBodyWithinCap call site in OnBackendFetchSuccess region.
    out, n = re.subn(
        r"\n\t\tif \(!IsMissionBodyWithinCap\(data\)\)\n\t\t\{.*?\n\t\t\}\n",
        "\n",
        src,
        count=1,
        flags=re.S,
    )
if n != 1:
    sys.stderr.write(f"RED setup failed: could not strip REST size gate (n={n})\n")
    sys.exit(2)
open(sys.argv[2], "w", encoding="utf-8").write(out)
PY

if assert_rest_size_gate "$TMP" "RED-perturbation"; then
	echo "FAIL: RED perturbation still passed — gate is not discriminating"
	FAIL=1
else
	echo "RED proof: stripped REST size check → assert FAIL (expected)"
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t456-mission-rest-size-gate: FAIL"
	exit 1
fi

echo "verify-t456-mission-rest-size-gate: PASS"
exit 0
