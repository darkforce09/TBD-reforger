#!/usr/bin/env bash
# T-296 — ResultsReporter must not claim `#tbd link` / link-confirm is unimplemented.
# T-181.35 shipped `TBD_IdentityLink` (`#tbd link <code>`, Arm()'d from MissionLoader).
# A banner/log that says otherwise is a false-green for the next reader.
#
# OWNS WIDEN: wave_plan T-296 lists only TBD_ResultsReporter.c; this script is the
# perturbation guard for that comment contract (no existing mod-comment verify path).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

FAIL=0

# Forbidden: the pre-T-296 lies that T-181.35 never landed.
if rg -n --fixed-strings 'There is no `#tbd link` command' "$FILE"; then
	echo "FAIL: ResultsReporter still claims there is no #tbd link command"
	FAIL=1
fi
if rg -n 'this mod does not implement' "$FILE"; then
	echo "FAIL: ResultsReporter still claims the mod does not implement link-confirm"
	FAIL=1
fi
if rg -n 'ATTENDANCE IS INERT UNTIL T-181\.35|until T-181\.35 (lands|ships)' "$FILE"; then
	echo "FAIL: ResultsReporter still frames attendance as inert until T-181.35"
	FAIL=1
fi

# Required: truth pins so a rewrite cannot drop the shipped surface quietly.
for pin in 'TBD_IdentityLink' '#tbd link <code>' 'IDENTITY LINKING (T-181.35 SHIPPED)'; do
	if ! rg -q --fixed-strings "$pin" "$FILE"; then
		echo "FAIL: missing truth pin: $pin"
		FAIL=1
	fi
done

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t296-results-reporter-identity-comments: FAIL"
	exit 1
fi

echo "verify-t296-results-reporter-identity-comments: PASS"
exit 0
