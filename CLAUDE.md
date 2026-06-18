# CLAUDE.md — TBD Reforger Platform

Working context for AI sessions. Read this first; it is the source of truth for
**current state and how to run things**. The design specs (`Claude_Context_Handoff.md`,
`BACKEND_ARCHITECTURE.md`) describe the *intended* product and remain the reference
for unbuilt features.

## What this is
A web suite for the "TBD" Arma Reforger milsim community: Discord auth, event /
ORBAT scheduling, a mission library (2D editor payloads), server telemetry +
leaderboards, doctrine wiki, CMS, and admin tooling.

- **Backend:** Go (Gin + GORM), PostgreSQL. Module `github.com/tbd-milsim/reforger-backend`, Go 1.25.
- **Frontend:** React 19 + TypeScript + Vite, TanStack Query, Zustand, Tailwind. Node 20. In `frontend/`.
- **Auth:** Discord OAuth2 → JWT access token + rotating single-use refresh token.

## Repo layout
- `cmd/api/` — API entrypoint (loads `.env`, runs migrations on boot, serves `/api/v1`).
- `internal/handlers/` — HTTP handlers, one file per resource (auth, missions, events, telemetry, admin, …).
- `internal/models/` — GORM models; **JSON field names (snake_case) here are the API contract**.
- `internal/db/migrations/` — SQL run before AutoMigrate (extensions, enums, indexes, leaderboard MV).
- `internal/services/`, `internal/middleware/`, `internal/realtime/` (SSE hub).
- `frontend/src/` — `api/` (axios client + single-flight refresh), `hooks/` (queries.ts, mutations.ts, useAuthBootstrap), `pages/`, `components/`, `store/useAuthStore.ts`, `types/` (hand-written API types).

## Run it locally
Everything is configured in `.env` (`APP_ENV=development`, DB on port 5434, `FRONTEND_URL=http://localhost:5173`). Go lives at `/var/home/Samuel/.local/go/bin` (not on PATH).

```bash
make db-up        # start local Postgres (podman/docker compose), port 5434
make api          # run Go API on :8080 (migrates on boot)
make web          # run Vite dev server on :5173 (proxies /api -> :8080)
make test-it      # Go integration tests (needs db-up; sets TEST_DATABASE_URL)
make db-down      # stop Postgres (keeps volume)
```

Frontend checks: `cd frontend && npm run build` (tsc + vite), `npm run lint`.

### Dev login (no Discord needed)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=admin|mission_maker|enlisted`.
It mints a real session and 302-redirects to the SPA callback exactly like Discord —
open it in the browser to log in, or curl it and read `access_token` from the
`Location` fragment for API testing.

## Conventions
- API JSON is **snake_case** (from GORM struct tags). Frontend `types/` are hand-written
  to match — when changing a model, update the matching TS type. The mission **export**
  JSON (`/missions/:id/export`) is the one camelCase exception.
- List endpoints return `{data, total, limit, offset}` (audit logs use a `next_cursor`).
- Auth tiers: public, `RequireAuth` (JWT), `RequireMinRole(admin|mission_maker)`,
  `RequireServiceToken` (`X-Service-Token`, for game-server ingest).
- Refresh tokens are **single-use** (rotated + revoked each call). All refreshes go
  through one single-flight helper (`frontend/src/api/refresh.ts`) so the token is
  never double-spent.
- Git: **commit directly to `main`; never create a branch.** End commit messages with
  the `Co-Authored-By` trailer. Commits are tagged `T-00x`.

## Status (latest feature work: T-004, commit fad698b — 2026-06-18)
Commits after T-004 (T-005+) are documentation only; the feature status below is current.

**Done:**
- T-001 initial backend (full schema + all handlers) + frontend scaffold.
- T-002 Discord OAuth2 callback end-to-end.
- T-003 dev-login shortcut (`internal/handlers/dev.go`).
- T-004 frontend wired to backend (typed query/mutation hooks, auth bootstrap +
  AuthGate/AdminGate, all pages on live data). Verified end-to-end against a running
  stack (full API contract smoke + headless browser E2E of every route). Fixed during
  verification: refresh-token rotation/persistence + single-flight refresh, several
  TS↔Go contract mismatches (pending_code, armory quantity, next_cursor), leaderboards
  empty `[]`, external avatar fallback, lint.

**Not yet built / next:**
- The 2D mission editor UI (backend stores/serves `json_payload`; the visual editor
  page is the big remaining frontend piece).
- Real Discord OAuth credentials are blank in `.env` (dev uses dev-login).
- Telemetry is ingested via service-token endpoints; no live game-server bridge wired.
- A fresh DB is empty of content (events, missions, etc.) — seed those via the API
  or `psql`. The one committed seed is the Discord role→permission mappings
  (`internal/db/seeds/discord_roles.sql`, applied with `make seed`).

## Verifying changes
Source of truth for the API contract is the Go handlers + `internal/models` tags;
frontend types yield to Go on conflict. To check a wire change for real, run the stack,
`dev-login`, hit the endpoint, and confirm the JSON matches the TS type — `tsc` alone
only proves the frontend is self-consistent, not that it matches the backend.
