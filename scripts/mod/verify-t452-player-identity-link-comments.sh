#!/usr/bin/env bash
# T-452 — PlayerIdentity must not claim `#tbd link` / link-confirm is unimplemented.
# T-181.35 shipped `TBD_IdentityLink` (`#tbd link <code>`, Arm()'d from MissionLoader).
# T-296 fixed the same lie in ResultsReporter; this file's header still said T-181.35.
#
# OWNS WIDEN: wave_plan T-452 lists only TBD_PlayerIdentity.c; this script is the
# perturbation guard for that comment contract (same shape as verify-t296-*).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_PlayerIdentity.c"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

FAIL=0

# Forbidden: the pre-T-452 lies that T-181.35 never landed / link-confirm is unimplemented.
if rg -n --fixed-strings 'The mod does not implement it yet' "$FILE"; then
	echo "FAIL: PlayerIdentity still claims the mod does not implement link-confirm"
	FAIL=1
fi
if rg -n 'does not implement it yet[[:space:]]*—[[:space:]]*that is T-181\.35|that is T-181\.35' "$FILE"; then
	echo "FAIL: PlayerIdentity still frames link-confirm as future T-181.35 work"
	FAIL=1
fi
if rg -n 'T-181\.35 must not resolve' "$FILE"; then
	echo "FAIL: PlayerIdentity still speaks of T-181.35 in the future tense for GetArmaId"
	FAIL=1
fi

# Required: truth pins so a rewrite cannot drop the shipped surface quietly.
for pin in 'TBD_IdentityLink' '#tbd link <code>' 'T-181.35 shipped' 'ENGINE-resolved identity is still not a LINKED one'; do
	if ! rg -q --fixed-strings "$pin" "$FILE"; then
		echo "FAIL: missing truth pin: $pin"
		FAIL=1
	fi
done

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t452-player-identity-link-comments: FAIL"
	exit 1
fi

echo "verify-t452-player-identity-link-comments: PASS"
exit 0
