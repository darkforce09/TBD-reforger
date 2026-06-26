#!/usr/bin/env bash
#
# mission-version-upload-repro.sh — isolate the mission-version upload path (T-060.1.4).
#
# Reproduces a large `POST /missions/:id/versions` directly against the dev API with curl,
# bypassing the browser and the Vite proxy. Use it to split a Save-Version failure into
# server-side vs browser-side:
#
#   * curl returns 201 at 140 MB  -> the server/middleware is fine; a browser ERR_NETWORK
#                                     is client/axios/proxy side (or a STALE `make api` —
#                                     `go run` does NOT hot-reload; restart it).
#   * curl resets / 400 / 413     -> server-side. Check the `make api` log for the
#                                     `CreateVersion: mission=… content_length=…` line and
#                                     whether GlobalBodyLimit's 1 MB cap reached the route.
#
# Root cause found in T-060.1.4: the global 1 MB MaxBytesReader wrapping the version route
# (when the GlobalBodyLimit skip didn't apply — e.g. a stale binary) trips at 1 MB and resets
# the connection mid-upload, which the browser surfaces as ERR_NETWORK at ~5 MB buffered.
# Fix: bodylimit.go isMissionVersionPOST() (FullPath + URL-path fallback) + a production-like
# integration test that mounts GlobalBodyLimit.
#
# Prereqs: `make db-up && make api` running on :8080 (dev env, dev-login enabled), python3, curl.
set -euo pipefail

API="${API:-http://localhost:8080/api/v1}"
ROLE="${ROLE:-mission_maker}"
SIZES_MB="${SIZES_MB:-2 10 140}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "==> dev-login ($ROLE)"
LOC=$(curl -s -o /dev/null -w '%{redirect_url}' "$API/auth/dev-login?role=$ROLE")
TOKEN=$(echo "$LOC" | sed -n 's/.*access_token=\([^&]*\).*/\1/p')
[ -n "$TOKEN" ] || { echo "no token (is APP_ENV=development and the API up?)"; exit 1; }

echo "==> create mission"
MID=$(curl -s -X POST "$API/missions" \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"title":"version-upload-repro","terrain":"everon","game_mode":"pve_coop","max_players":8}' \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "    mission=$MID"

i=0
for MB in $SIZES_MB; do
  i=$((i + 1))
  F="$TMP/body_${MB}mb.json"
  # Start at 1.x so we never collide with the mission's auto-created initial 0.1.0 version.
  python3 - "$F" "$MB" "1.$i.0" <<'PY'
import sys
fn, mb, ver = sys.argv[1], int(sys.argv[2]), sys.argv[3]
notes = "x" * (mb * 1024 * 1024)
open(fn, "w").write('{"semver":"%s","payload":{"spawns":[]},"editor_notes":"%s"}' % (ver, notes))
PY
  printf '==> POST %4s MB  ' "$MB"
  curl -s -o "$TMP/resp.txt" -w 'HTTP %{http_code}  uploaded=%{size_upload}B  time=%{time_total}s\n' \
    -X POST "$API/missions/$MID/versions" \
    -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
    --data-binary @"$F" || echo "curl exit $?  (connection reset = server-side)"
done

echo "==> done. Watch the \`make api\` terminal for: CreateVersion: mission=$MID content_length=… status=…"
