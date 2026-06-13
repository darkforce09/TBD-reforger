#!/usr/bin/env bash
# Phase 0.1 REST spike client.
#
# Emulates what a dedicated Reforger server does: fetch the compiled mission with
# a bearer token, then POST a final result and a telemetry batch. Point it at the
# restspike harness (cmd/restspike) or the full server.
#
# Usage:
#   BASE_URL=http://localhost:8099 GAME_SERVER_TOKEN=spike-token MISSION_ID=msn_8f3a2c ./scripts/rest-spike.sh
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8099}"
TOKEN="${GAME_SERVER_TOKEN:?set GAME_SERVER_TOKEN}"
MISSION_ID="${MISSION_ID:-msn_8f3a2c}"
AUTH=(-H "Authorization: Bearer ${TOKEN}")

echo "==> GET mission ${MISSION_ID}"
status=$(curl -sS -o /tmp/restspike-mission.json -w '%{http_code}' "${AUTH[@]}" \
  "${BASE_URL}/api/missions/${MISSION_ID}/compiled")
echo "    HTTP ${status}"
[ "${status}" = "200" ] || { echo "FAIL: expected 200"; exit 1; }
name=$(grep -o '"name":[^,]*' /tmp/restspike-mission.json | head -n1)
echo "    mission ${name}"

echo "==> POST results"
status=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "${AUTH[@]}" \
  -H 'Content-Type: application/json' \
  -d "{\"missionId\":\"${MISSION_ID}\",\"winner\":\"blufor\",\"endReason\":\"all_objectives_captured\"}" \
  "${BASE_URL}/api/results")
echo "    HTTP ${status}"
[ "${status}" = "202" ] || { echo "FAIL: expected 202"; exit 1; }

echo "==> POST telemetry"
status=$(curl -sS -o /dev/null -w '%{http_code}' -X POST "${AUTH[@]}" \
  -H 'Content-Type: application/json' \
  -d "{\"missionId\":\"${MISSION_ID}\",\"events\":[{\"t\":1,\"type\":\"capture\",\"zone\":\"z3\"}]}" \
  "${BASE_URL}/api/telemetry")
echo "    HTTP ${status}"
[ "${status}" = "202" ] || { echo "FAIL: expected 202"; exit 1; }

echo "==> unauthenticated request should be rejected"
status=$(curl -sS -o /dev/null -w '%{http_code}' "${BASE_URL}/api/missions/${MISSION_ID}/compiled")
echo "    HTTP ${status}"
[ "${status}" = "401" ] || { echo "FAIL: expected 401"; exit 1; }

echo "PASS: REST loop (GET mission, POST results, POST telemetry, auth gate) verified."
