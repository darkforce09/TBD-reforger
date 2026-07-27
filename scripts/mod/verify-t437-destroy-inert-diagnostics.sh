#!/usr/bin/env bash
# T-437 — Destroy-target inert diagnostics must not claim entities[] never spawn.
#
# After T-254, TBD_MissionDocumentStruct models entities[] and SpawnMissionEntities
# places resolvable rows. Operator-facing strings / schema prose that still blame a
# build that "does not spawn/model entities[]" are lies (wave 9 adversarial MAJOR M1).
#
# Gate: bash scripts/mod/verify-t437-destroy-inert-diagnostics.sh
# OWNS WIDEN: wave_plan T-437 lists Objectives/* + mission.schema.json; this script is
# the Class-R perturbation guard. Also covers TBD_MissionValidator.c (same lie class).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

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

FAIL=0

# Forbidden: the pre-T-437 lies that the build never spawns/models entities[].
FORBIDDEN=(
	'This build does not spawn the mission document'
	'does not spawn the mission document'
	'nothing spawns the mission document'
	'nothing spawns mission `entities[]`'
	'on today'\''s build nothing spawns mission'
	'TBD_MissionDocumentStruct does not model them'
	'TBD_MissionDocumentStruct ignore `entities[]`'
	'does not spawn mission entities'
	'this build cannot create it'
)

scan_forbidden() {
	local file="$1"
	local hit
	for needle in "${FORBIDDEN[@]}"; do
		if hit=$(rg -n --fixed-strings "$needle" "$file" 2>/dev/null); then
			echo "FAIL: forbidden lie in ${file#"$ROOT"/}:"
			echo "$hit" | sed 's/^/  /'
			FAIL=1
		fi
	done
}

scan_forbidden "$REG"
scan_forbidden "$COMP"
scan_forbidden "$RULES"
scan_forbidden "$SCHEMA"
scan_forbidden "$VALIDATOR"

# Required truth pins — rewrite must keep the accurate diagnostic surface.
require_pin() {
	local file="$1"
	local pin="$2"
	if ! rg -q --fixed-strings "$pin" "$file"; then
		echo "FAIL: missing truth pin in ${file#"$ROOT"/}: $pin"
		FAIL=1
	fi
}

require_pin "$REG" 'DiagnoseEmptyDestroyTargets'
require_pin "$REG" 'SpawnMissionEntities'
require_pin "$REG" 'out-of-zone placement'
require_pin "$REG" 'not in the registry, so there is no prefab to look for'
require_pin "$COMP" 'SpawnMissionEntities'
require_pin "$RULES" 'TBD_MissionDocumentStruct` models `entities[]`'
require_pin "$SCHEMA" 'SpawnMissionEntities'
require_pin "$SCHEMA" 'out-of-zone authorship'
require_pin "$VALIDATOR" 'entities[] is modeled + SpawnMissionEntities'

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t437-destroy-inert-diagnostics: FAIL"
	exit 1
fi

echo "verify-t437-destroy-inert-diagnostics: PASS"
exit 0
