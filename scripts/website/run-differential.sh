#!/usr/bin/env bash
# T-145 gate G5 — boot Go + Rust each against its OWN Postgres, seeded identically
# (deterministic fixed data), and diff every response under the `≡` relation.
# Separate DBs avoid a dual-migration clash; a per-server .env dir avoids the
# .env-vs-export precedence fight (each server loads its own .env from CWD).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WEB="$ROOT/apps/website"
SEED="$ROOT/scripts/website/differential_seed.sql"
LOG="${TMPDIR:-/tmp}/t145-diff"
rm -rf "$LOG"; mkdir -p "$LOG/go" "$LOG/rust"
export PATH="$HOME/.local/go/bin:$HOME/.cargo/bin:$PATH"
# Ensure no stray DATABASE_URL leaks into the servers.
unset DATABASE_URL PORT APP_ENV JWT_SECRET SERVICE_TOKEN FRONTEND_URL ALLOWED_ORIGINS SKIP_MIGRATE
# Kill stale harness servers from a prior run (do NOT touch the dev :8080).
pkill -f "t145-diff/(go|rust)-api" 2>/dev/null || true; sleep 1

psql_db() { podman exec -i tbd_reforger_db psql -U tbd -d "$1" -qv ON_ERROR_STOP=1; }
admin_db() { podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc "$1" >/dev/null; }

GO_PID=""; RUST_PID=""
cleanup() {
  [ -n "$RUST_PID" ] && kill "$RUST_PID" 2>/dev/null || true
  [ -n "$GO_PID" ] && kill "$GO_PID" 2>/dev/null || true
  wait 2>/dev/null || true
  admin_db "DROP DATABASE IF EXISTS diff_db_go WITH (FORCE);" || true
  admin_db "DROP DATABASE IF EXISTS diff_db_rust WITH (FORCE);" || true
}
trap cleanup EXIT

mkenv() {  # $1 = dir, $2 = db, $3 = port
  cat > "$1/.env" <<EOF
APP_ENV=development
DATABASE_URL=postgres://tbd:tbd@localhost:5434/$2?sslmode=disable
JWT_SECRET=diffsecret
SERVICE_TOKEN=diff-service-token
FRONTEND_URL=http://localhost:5173
ALLOWED_ORIGINS=http://localhost:5173
PORT=$3
TZ=UTC
PGTZ=UTC
EOF
}

wait_health() {
  for _ in $(seq 1 60); do
    (exec 3<>/dev/tcp/localhost/"$1") 2>/dev/null && { exec 3>&-; return 0; }
    sleep 1
  done
  echo "server on :$1 never came up"; return 1
}

echo "== reset DBs =="
for d in diff_db_go diff_db_rust; do
  admin_db "DROP DATABASE IF EXISTS $d WITH (FORCE);"
  admin_db "CREATE DATABASE $d;"
done

echo "== build =="
( cd "$WEB" && go build -o "$LOG/go-api" ./cmd/api )
( cd "$WEB" && cargo build --quiet --bin api )
cp "$WEB/target/debug/api" "$LOG/rust-api"

echo "== boot Go :8080 (migrates diff_db_go) =="
mkenv "$LOG/go" diff_db_go 8090
( cd "$LOG/go" && exec "$LOG/go-api" ) >"$LOG/go.log" 2>&1 &
GO_PID=$!
wait_health 8090

echo "== boot Rust :8081 (migrates diff_db_rust) =="
mkenv "$LOG/rust" diff_db_rust 8091
( cd "$LOG/rust" && exec "$LOG/rust-api" ) >"$LOG/rust.log" 2>&1 &
RUST_PID=$!
wait_health 8091

echo "== seed both identically =="
psql_db diff_db_go   < "$SEED" >"$LOG/seed-go.log" 2>&1   || { echo "go seed failed"; tail -5 "$LOG/seed-go.log"; exit 1; }
psql_db diff_db_rust < "$SEED" >"$LOG/seed-rust.log" 2>&1 || { echo "rust seed failed"; tail -5 "$LOG/seed-rust.log"; exit 1; }

echo "== differential =="
GO_URL="http://localhost:8090" RUST_URL="http://localhost:8091" SERVICE_TOKEN="diff-service-token" \
  node "$ROOT/scripts/website/differential.mjs"
