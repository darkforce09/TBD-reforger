#!/usr/bin/env bash
# T-438 — deploy-staging.sh must point docker compose at
# apps/website/docker-compose.staging.yml (T-251), never cd into apps/website/api
# for that step.
#
# OWNS WIDEN: wave_plan T-438 lists only scripts/mod/deploy-staging.sh; this
# script is the Class-R perturbation guard for that path contract.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FILE="$ROOT/scripts/mod/deploy-staging.sh"
COMPOSE="$ROOT/apps/website/docker-compose.staging.yml"
STALE="$ROOT/apps/website/api/docker-compose.staging.yml"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

FAIL=0

# Required: compose file path at apps/website/ (not api/).
if ! rg -q --fixed-strings 'apps/website/docker-compose.staging.yml' "$FILE"; then
	echo "FAIL: deploy-staging.sh must reference apps/website/docker-compose.staging.yml"
	FAIL=1
fi

# Forbidden: the pre-T-438 lie (cd into api/ then compose).
if rg -q --fixed-strings "cd '\$TBD_REMOTE_DIR/apps/website/api'" "$FILE"; then
	echo "FAIL: deploy-staging.sh still cds into apps/website/api (compose must not)"
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
