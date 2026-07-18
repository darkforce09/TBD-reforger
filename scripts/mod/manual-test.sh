#!/usr/bin/env bash
# Manual test suite — everything except Discord OAuth and VOIP.
# Run from repo root: bash scripts/mod/manual-test.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
PASS=0
FAIL=0
SKIP=0

pass() { echo "  PASS  $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL  $1"; FAIL=$((FAIL + 1)); }
skip() { echo "  SKIP  $1"; SKIP=$((SKIP + 1)); }

section() { echo; echo "== $1 =="; }

# --- 1. tbd-schema ---
section "packages/tbd-schema validation"
if (cd "$SCHEMA" && npm run validate >/dev/null 2>&1); then
  pass "npm run validate (9 artifacts)"
else
  fail "npm run validate"
fi

if [ -f "$SCHEMA/schema/mission.schema.json" ] && \
   [ -f "$SCHEMA/bridge/bridge-contract.md" ] && \
   [ -f "$SCHEMA/golden-missions/bridgehead-at-levie.json" ]; then
  pass "schema + bridge + golden mission files exist"
else
  fail "missing packages/tbd-schema artifacts"
fi

# --- 2. Backend build/tests: gated at the repo root (`make ci-local` — Rust workspace) ---

# --- 3. Config / env ---
section "Config"
if grep -q 'GAME_SERVER_TOKENS' "$WEB/.env.example"; then
  pass ".env.example documents GAME_SERVER_TOKENS"
else
  fail ".env.example missing GAME_SERVER_TOKENS"
fi

if [ -f "$MONO_ROOT/.cursor/mcp.json" ] && { ! command -v jq >/dev/null || jq -e . "$MONO_ROOT/.cursor/mcp.json" >/dev/null 2>&1; }; then
  pass ".cursor/mcp.json is valid JSON"
else
  fail ".cursor/mcp.json invalid or missing"
fi

# --- 4. Mission files on disk ---
section "Compiled missions"
for id in msn_8f3a2c msn_2d91be; do
  if [ -f "$WEB/missions/${id}.json" ]; then
    if node -e "
      const m = JSON.parse(require('fs').readFileSync('$WEB/missions/${id}.json','utf8'));
      if (m.schemaVersion !== '1.0') process.exit(1);
      if (!m.meta?.name) process.exit(1);
      if (!m.radioPlan?.nets?.length) process.exit(1);
    " 2>/dev/null; then
      pass "missions/${id}.json parses (schemaVersion, meta, radioPlan)"
    else
      fail "missions/${id}.json missing required fields"
    fi
  else
    fail "missions/${id}.json missing"
  fi
done

# --- 5. Live REST spike (restspike harness) ---
section "Live game-server REST API"
(cd "$WEB" && go build -o /tmp/tbd-restspike ./cmd/restspike >/dev/null 2>&1) || { fail "build restspike"; exit 1; }

GAME_SERVER_TOKENS=test-manual-token MISSIONS_DIR="$WEB/missions" PORT=8199 \
  /tmp/tbd-restspike >/tmp/tbd-restspike.log 2>&1 &
SRV=$!
sleep 1.2
BASE=http://127.0.0.1:8199
AUTH="Authorization: Bearer test-manual-token"

cleanup() { kill $SRV 2>/dev/null || true; }
trap cleanup EXIT

# GET mission 1
code=$(curl -sS -o /tmp/m1.json -w '%{http_code}' -H "$AUTH" "$BASE/api/missions/msn_8f3a2c/compiled")
if [ "$code" = "200" ] && grep -q '"Bridgehead at Levie"' /tmp/m1.json; then
  pass "GET /api/missions/msn_8f3a2c/compiled -> 200 + correct name"
else
  fail "GET mission msn_8f3a2c (code=$code)"
fi

# GET mission 2
code=$(curl -sS -o /tmp/m2.json -w '%{http_code}' -H "$AUTH" "$BASE/api/missions/msn_2d91be/compiled")
if [ "$code" = "200" ] && grep -q '"Last Stand at Montfort"' /tmp/m2.json; then
  pass "GET /api/missions/msn_2d91be/compiled -> 200 + correct name"
else
  fail "GET mission msn_2d91be (code=$code)"
fi

# GET missing
code=$(curl -sS -o /dev/null -w '%{http_code}' -H "$AUTH" "$BASE/api/missions/msn_nope/compiled")
[ "$code" = "404" ] && pass "GET missing mission -> 404" || fail "GET missing mission (code=$code)"

# No auth
code=$(curl -sS -o /dev/null -w '%{http_code}' "$BASE/api/missions/msn_8f3a2c/compiled")
[ "$code" = "401" ] && pass "GET without token -> 401" || fail "GET without token (code=$code)"

# Bad token
code=$(curl -sS -o /dev/null -w '%{http_code}' -H "Authorization: Bearer wrong" "$BASE/api/missions/msn_8f3a2c/compiled")
[ "$code" = "401" ] && pass "GET bad token -> 401" || fail "GET bad token (code=$code)"

# POST results
code=$(curl -sS -o /tmp/res.json -w '%{http_code}' -X POST -H "$AUTH" -H 'Content-Type: application/json' \
  -d '{"missionId":"msn_8f3a2c","winner":"blufor"}' "$BASE/api/results")
if [ "$code" = "202" ] && grep -q '"accepted"' /tmp/res.json; then
  pass "POST /api/results -> 202 accepted"
else
  fail "POST /api/results (code=$code)"
fi

# POST telemetry
code=$(curl -sS -o /tmp/tel.json -w '%{http_code}' -X POST -H "$AUTH" -H 'Content-Type: application/json' \
  -d '{"missionId":"msn_8f3a2c","events":[{"t":1,"type":"capture"}]}' "$BASE/api/telemetry")
if [ "$code" = "202" ] && grep -q '"accepted"' /tmp/tel.json; then
  pass "POST /api/telemetry -> 202 accepted"
else
  fail "POST /api/telemetry (code=$code)"
fi

# POST bad JSON
code=$(curl -sS -o /dev/null -w '%{http_code}' -X POST -H "$AUTH" -H 'Content-Type: application/json' \
  -d 'not-json' "$BASE/api/telemetry")
[ "$code" = "400" ] && pass "POST bad JSON -> 400" || fail "POST bad JSON (code=$code)"

# Invalid mission id (path traversal attempt)
code=$(curl -sS -o /dev/null -w '%{http_code}' -H "$AUTH" "$BASE/api/missions/..%2Fsecret/compiled")
if [ "$code" = "400" ] || [ "$code" = "404" ]; then
  pass "GET traversal-like id rejected ($code)"
else
  fail "GET traversal-like id (code=$code)"
fi

# --- 6. Full server (needs Postgres) ---
section "Full website API (public, no Discord)"
if command -v podman >/dev/null 2>&1 && podman ps -a --format '{{.Names}}' 2>/dev/null | grep -q '^tbdevent-postgres$'; then
  podman start tbdevent-postgres >/dev/null 2>&1 || true
  sleep 1
fi

if [ -f "$WEB/.env" ]; then
  set -a && source "$WEB/.env" && set +a
fi

# Quick TCP check on common ports from .env (5433) or default 5432
HAS_PG=0
for pgport in 5433 5432; do
  if (echo >/dev/tcp/127.0.0.1/$pgport) 2>/dev/null; then
    HAS_PG=1
    break
  fi
done

if [ "$HAS_PG" = "1" ] && [ -n "${DATABASE_URL:-}" ]; then
  SESSION_SECRET=manual-test-secret \
  GAME_SERVER_TOKENS=test-manual-token \
  MISSIONS_DIR="$WEB/missions" \
  PORT=8198 \
  ENV=development \
    sh -c "cd '$WEB' && go run ./cmd/server" >/tmp/tbd-server.log 2>&1 &
  FULL=$!
  sleep 3
  FB=http://127.0.0.1:8198

  code=$(curl -sS -o /tmp/pages.json -w '%{http_code}' "$FB/api/pages")
  [ "$code" = "200" ] && pass "GET /api/pages -> 200" || fail "GET /api/pages (code=$code)"

  code=$(curl -sS -o /tmp/rules.json -w '%{http_code}' "$FB/api/pages/rules")
  [ "$code" = "200" ] && pass "GET /api/pages/rules -> 200" || fail "GET /api/pages/rules (code=$code)"

  code=$(curl -sS -o /tmp/events.json -w '%{http_code}' "$FB/api/events?upcoming=true")
  [ "$code" = "200" ] && pass "GET /api/events?upcoming=true -> 200" || fail "GET /api/events (code=$code)"

  code=$(curl -sS -o /tmp/ev.json -w '%{http_code}' "$FB/api/events/tbd-pvp-1")
  [ "$code" = "200" ] && pass "GET /api/events/tbd-pvp-1 -> 200" || fail "GET event detail (code=$code)"

  code=$(curl -sS -o /tmp/roster.json -w '%{http_code}' "$FB/api/events/tbd-pvp-1/roster")
  [ "$code" = "200" ] && pass "GET /api/events/tbd-pvp-1/roster -> 200" || fail "GET roster (code=$code)"

  code=$(curl -sS -o /tmp/ann.json -w '%{http_code}' "$FB/api/announcements")
  [ "$code" = "200" ] && pass "GET /api/announcements -> 200" || fail "GET announcements (code=$code)"

  code=$(curl -sS -o /dev/null -w '%{http_code}' "$FB/api/auth/me")
  [ "$code" = "401" ] && pass "GET /api/auth/me without session -> 401" || fail "GET /api/auth/me (code=$code)"

  code=$(curl -sS -o /dev/null -w '%{http_code}' "$FB/api/admin/pages/rules")
  [ "$code" = "401" ] && pass "GET /api/admin/pages/rules without session -> 401" || fail "GET admin (code=$code)"

  code=$(curl -sS -o /dev/null -w '%{http_code}' -H "$AUTH" "$FB/api/missions/msn_8f3a2c/compiled")
  [ "$code" = "200" ] && pass "Full server: GET game mission -> 200" || fail "Full server game mission (code=$code)"

  code=$(curl -sS -o /tmp/res.json -w '%{http_code}' -X POST -H "$AUTH" -H 'Content-Type: application/json' \
    -d '{"missionId":"msn_8f3a2c","winner":"blufor"}' "$FB/api/results")
  [ "$code" = "202" ] && pass "Full server: POST /api/results -> 202" || fail "Full server POST results (code=$code)"

  code=$(curl -sS -o /tmp/tel.json -w '%{http_code}' -X POST -H "$AUTH" -H 'Content-Type: application/json' \
    -d '{"missionId":"msn_8f3a2c","events":[]}' "$FB/api/telemetry")
  [ "$code" = "202" ] && pass "Full server: POST /api/telemetry -> 202" || fail "Full server POST telemetry (code=$code)"

  code=$(curl -sS -o /dev/null -w '%{http_code}' "$FB/")
  [ "$code" = "200" ] && pass "GET / (embedded SPA) -> 200" || fail "GET / static (code=$code)"

  kill $FULL 2>/dev/null || true
else
  skip "Full website API — Postgres not available"
fi

# --- 7. Docs / milestones ---
section "Documentation"
for f in CLAUDE-CONTINUATION.md MILESTONES.md; do
  [ -f "$MOD_ROOT/$f" ] && pass "$f exists" || fail "$f missing"
done
[ -f "$SCHEMA/spikes/rest-spike-0.1.md" ] && pass "rest-spike doc exists" || fail "rest-spike doc missing"

# --- Summary ---
section "Summary"
echo "Passed: $PASS  Failed: $FAIL  Skipped: $SKIP"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "All runnable manual tests passed."
