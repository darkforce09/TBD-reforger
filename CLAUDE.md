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

## Status (latest feature work: T-013 — 2026-06-19)
T-005..T-007 between T-004 and T-008 are documentation/seed only; the status below is current.

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
- T-008 **Event → Campaign refactor** (multi-mission events + ORBAT selection):
  - An `Event` is now a container; missions attach via the new `event_missions` table
    (`internal/models/event.go`). `orbat_slots`/`event_registrations` key on
    `event_mission_id` (was `event_id`); `events.mission_id` dropped, `briefing` +
    `banner_image_url` added. Migration `internal/db/migrations/02_campaign_refactor.sql`
    (clean cutover, idempotent, `to_regclass`-guarded) runs pre-AutoMigrate.
  - **Automated ORBAT:** `POST /events/:id/missions` parses the mission version's
    `json_payload.orbat` (`{faction,callsign,squad,role,count}[]`) and materializes slots
    — no manual squad creation. Reuses `parseOrbatTemplate`/`materializeSlots`.
  - Slot/registration actions moved to top-level `/event-missions/:emid/...`
    (orbat, register, slots/:slotId/assign). `GET /events/:id` returns the hub with
    nested mission dossiers (factions, armory-by-faction, fill counts, caller's state).
    Registration is per-mission; capacity = ORBAT slot count.
  - Frontend: `pages/events.tsx` = **EventHubPage** + macOS split-pane
    **OrbatSelectionPage**; Event Manager rebuilt as create-container + attach-mission;
    schedule/dashboard now route to the hub. Date formatters in `lib/format.ts` are
    invalid-date-safe.
  - Verified: `make test-it`, frontend build+lint, and a live dev-login API smoke
    (create event → attach mission → auto-ORBAT → claim slot → withdraw).
- T-009 inline ORBAT on the Event Hub: each mission dossier renders the
  faction/squad/slot selector + Register button inline (no "Open ORBAT" step). The
  split-pane is a reusable `OrbatSelector` in `pages/events.tsx`; the standalone
  `/events/:id/missions/:emid/orbat` route reuses it for deep-links.
- T-010 rich ORBAT slots + squad reservation:
  - Per-slot ORBAT schema in `json_payload`: `orbat[].slots[]` with `role`,
    `loadout`, optional `tag` (parsed in `events.go`; `OrbatSlot` gained
    `loadout`/`tag`). Rendered as a numbered list ("1: Squad Leader (L85A3 + GL) | MED").
  - New `leader` role (`enlisted<leader<mission_maker<admin` in `authz.go`; enum
    `ALTER TYPE` in `01_enums.sql`; dev-login + role-sync seed updated).
  - One-click squad **reservation/hold**: `OrbatReservation` model + `POST
    /event-missions/:emid/squads/{reserve,release}` (leader+). A held squad blocks
    others' claims; the reserver/admin fill it via `AssignSlot` + a `GET /members`
    directory search. Slot/assign routes moved to the leader tier.
- T-011 **macOS "Aegis" design-system foundation** (frontend, presentation-only):
  `index.css` adopts the full Aegis palette (desaturated `#adc6ff` primary, off-white
  `on-surface`, `tertiary`/`tactical-yellow`/`error-alert`/`surface-glass`) plus the
  many Aegis tokens pages already referenced but were undefined, the semantic type
  scale (`text-headline-lg`..`text-code-md`), and `.glass`/`.bg-topo-map`/
  `.bg-grid-overlay` utilities. New reusable primitives in `frontend/src/components/ui/`
  built on `@base-ui/react` (no new deps): `SplitPane`, `Dialog`, `Sheet`, `Switch`,
  `Badge`, `GlassPanel`/`HudBar`, `ListDetailItem`; `OpsCard` gained a `glass` variant.
  Shell: `AppLayout` honors a `fullBleed` route handle (split-pane pages run full-height);
  `TopNav` is a frosted glass bar; `Sidebar` uses the Aegis left-bar active state.
- T-012 **macOS page redesigns — split-pane master/detail** (presentation-only; no API/
  query changes). Announcements → Apple-Mail split-view; Event Schedule → split-pane with
  op cards + embedded `EventHubView` (no full-page replace; ORBAT selector logic unchanged);
  My Deployments → service-record split dossier; admin Personnel/Approvals/Audit →
  table/queue + slide-over dossier / review HUD; Event & Content Manager forms moved into
  frosted `Dialog`s (kills the form-over-list anti-pattern); new **Vehicle Database** page
  (`/vehicles`, split-pane dossier) + nav entry. Verified: tsc/build/lint clean + live
  dev-login API contract smoke.
- T-013 **macOS dashboards & grids restyle** (Phase 3; presentation-only). Dashboard →
  glass bento (hero countdown + status cards + intel feed); Server Intel & Settings →
  glass cards + `Badge` chips; Leaderboards podium → glass; Mission Library → featured
  card grid with thumbnails that opens the mission dossier in a `Sheet` slide-over
  (shared `MissionDossierBody` reused by the `/missions/:id` deep-link page); Modpacks &
  Mortar → glass. Verified: tsc/build/lint clean + live dev-login API contract smoke.
  All blueprint pages now on the Aegis design language; the **2D Mission Creator** remains
  the one unbuilt piece (separate effort).

**Not yet built / next:**
- The 2D mission editor UI (backend stores/serves `json_payload`; the visual editor
  page is the big remaining frontend piece). The Event Hub "Open in Mission Planner"
  button is a deliberate disabled stub until this lands.
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
