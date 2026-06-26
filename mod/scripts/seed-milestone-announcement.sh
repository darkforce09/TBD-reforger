#!/usr/bin/env bash
# Insert pinned Milestone #1 announcement on the event website (not Discord).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB="$ROOT/Tbdevent_Website"
set -a && source "$WEB/.env" && set +a

if command -v psql >/dev/null 2>&1; then
  PSQL=(psql "$DATABASE_URL")
elif podman ps --format '{{.Names}}' 2>/dev/null | grep -qx tbdevent-postgres; then
  PSQL=(podman exec -i tbdevent-postgres psql -U tbdevent -d tbdevent)
else
  echo "No psql and tbdevent-postgres container not running." >&2
  exit 1
fi

"${PSQL[@]}" -v ON_ERROR_STOP=1 <<'SQL'
INSERT INTO announcements (title, body, pinned, published, published_at)
SELECT
  'Milestone #1 — Saturday 22 August 2026',
  E'Our first **manual TBD PvP event** target is **Saturday 21 August 2026** (internal test, 20–40 players).

Mission loads from the backend; ORBAT slots enforce roles; VOIP is optional.

Sign up under **Events**. Mission Wizard arrives in Phase 2 — Milestone #1 uses hand-written JSON.',
  TRUE,
  TRUE,
  NOW()
WHERE NOT EXISTS (
  SELECT 1 FROM announcements WHERE title LIKE 'Milestone #1%'
);
SQL

echo "Website announcement seeded (if not already present)."
