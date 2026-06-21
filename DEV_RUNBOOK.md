# Dev Runbook — spin up the stack

Quick steps to bring up DB + API + Vite locally. See [`CLAUDE.md`](CLAUDE.md) for full context.
Backend docs: [`docs/backend/README.md`](docs/backend/README.md).

## Start everything

```bash
# 1. Postgres (port 5434) — quick, run in foreground
make db-up

# 2. Go API on :8080 — IMPORTANT: go is NOT on PATH, prepend it.
#    Run in background; it compiles + migrates on boot.
PATH="/var/home/Samuel/.local/go/bin:$PATH" make api

# 3. Vite dev server on :5173 (proxies /api -> :8080) — run in background
make web
```

> **Gotcha:** plain `make api` fails with `make: go: No such file or directory`.
> Go lives at `/var/home/Samuel/.local/go/bin` and isn't on PATH — always prepend it.

## Confirm it's up

```bash
# API responds (200) once migrations finish (~few seconds)
curl -sf http://localhost:8080/api/v1/health
```

- API: http://localhost:8080
- Web: http://localhost:5173

## Log in (no Discord needed)

Open in browser (mints a real session, redirects to the SPA):

```
http://localhost:8080/api/v1/auth/dev-login?role=admin
```

Roles: `admin | mission_maker | leader | enlisted`.

## Stop

```bash
make db-down      # stops Postgres, keeps volume
# API + Vite: kill the background processes
```

## Notes

- A fresh DB only has the Discord role→permission mappings (`make seed`).
  Events/missions must be seeded via the API or `psql`.
- Frontend checks: `cd frontend && npm run build` (tsc+vite), `npm run lint`.
- Integration tests: `make test-it` (needs `make db-up`).

## Mock data (optional, not run by `make seed`)

`internal/db/seeds/mock_data.sql` (Operation Red Dawn etc.) is **only** applied by the
explicit `go run ./cmd/seed` command — `make seed` does not touch it. The Mission Library
renders live API data, so these four fixed-UUID missions show up as real entries. To purge
them (children first; there are no ON DELETE CASCADE FKs):

```bash
docker compose exec -T db psql -U tbd -d tbd_reforger <<'SQL'
DELETE FROM mission_versions  WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM mission_armories  WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM mission_bookmarks WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
UPDATE missions SET current_version_id = NULL WHERE id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM missions WHERE id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
SQL
```
