#!/usr/bin/env bash
# Smoke-test Phase 1 game-server API: link codes + roster.
# Usage: bash scripts/mod/test-phase1-api.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
WEB="$WEB"
API="${API_BASE:-http://127.0.0.1:8080}"
TOKEN="${GAME_SERVER_TOKEN:-dev-server-token-change-in-prod}"
EVENT_ID="${EVENT_ID:-b0000000-0000-4000-8000-000000000001}"

auth=(-H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json")

echo "== POST /api/link =="
curl -sS -X POST "$API/api/link" "${auth[@]}" \
  -d '{"code":"123456","identityId":"test-identity-abc","platform":"pc"}' | tee /tmp/tbd-link.json
echo

echo "== GET /api/game/events/$EVENT_ID/roster (empty) =="
curl -sS "${auth[@]}" "$API/api/game/events/$EVENT_ID/roster" | tee /tmp/tbd-roster.json
echo

echo "== GET /api/missions/msn_8f3a2c/compiled =="
code=$(curl -sS -o /tmp/tbd-mission.json -w '%{http_code}' "${auth[@]}" "$API/api/missions/msn_8f3a2c/compiled")
echo "HTTP $code"
head -c 120 /tmp/tbd-mission.json; echo "..."

echo
echo "Done. Link without login requires POST /api/me/link (needs Discord session)."
