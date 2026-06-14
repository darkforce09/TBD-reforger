# TBD Event Website

Website for organising the TBD PvP event in Arma Reforger. A single Go binary serves the React frontend and REST API, with content stored in PostgreSQL.

**Platform repo:** part of [tbd-reforger-platform](https://github.com/darkforce09/tbd-reforger-platform)

## Stack

- **Backend**: Go, chi, pgx, goose migrations, Discord OAuth, scs sessions
- **Frontend**: React, Vite, TypeScript, TanStack Query
- **Database**: PostgreSQL

## Quick start

### 1. Start PostgreSQL

```bash
docker compose up -d
# or: podman start tbdevent-postgres  (dev .env uses port 5433)
```

### 2. Configure environment

```bash
cp .env.example .env
```

Edit `.env` and set at minimum:

- `DATABASE_URL` (e.g. `postgres://...@127.0.0.1:5433/tbdevent?sslmode=disable` for podman)
- `SESSION_SECRET`
- Discord OAuth values (`DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URI`)
- Admin access via `ADMIN_DISCORD_IDS` and/or `ADMIN_DISCORD_ROLE_ID` + `DISCORD_GUILD_ID`
- `GAME_SERVER_TOKENS` — bearer token(s) for dedicated servers
- `SCHEMA_DIR=../tbd-schema` — mission validation on publish

### 3. Development

Terminal 1 — API server (runs migrations on start):

```bash
make dev-api
```

Terminal 2 — React dev server with API proxy:

```bash
cd web && npm install && npm run dev
```

Open http://localhost:5173 (API on http://127.0.0.1:8080)

### 4. Production build

```bash
make build
./bin/tbdevent
```

Open http://localhost:8080

### 5. Game-server API smoke test

From repo root:

```bash
bash scripts/test-phase1-api.sh
bash scripts/manual-test.sh    # full suite (needs DB + migrations)
```

## Docker image

```bash
docker build -t tbdevent-website .
docker run --env-file .env -p 8080:8080 tbdevent-website
```

### Staging stack (192.168.0.140)

[`docker-compose.staging.yml`](docker-compose.staging.yml) runs **Postgres + API** together. Create `.env` on the server only (never rsync from dev). Volume-mounts `missions/` and `../tbd-schema`. Deploy via [`scripts/deploy-staging.sh`](../scripts/deploy-staging.sh) — see [`docs/STAGING-SERVER.md`](../docs/STAGING-SERVER.md).

Game servers authenticate with `GAME_SERVER_TOKENS` (same value as `TBD_GAME_SERVER_TOKEN` in `scripts/deploy.env`).

## Pages

| Route | Content slug |
|-------|----------------|
| `/rules` | `rules` |
| `/compliance` | `compliance` |
| `/server` | `server-info` |
| `/mods` | `mods` |
| `/admin` | CMS (Discord admin only) |

## API (public)

- `GET /api/pages`, `GET /api/pages/{slug}` — CMS pages
- `GET /api/events`, `GET /api/events/next`, `GET /api/events/{slug}` — events
- `GET /api/events/{slug}/roster` — public roster view
- `GET /api/announcements` — announcements
- `GET /auth/discord`, `GET /auth/discord/callback` — OAuth
- `GET /api/auth/me` — current user + admin flag
- `POST /api/events/{slug}/register`, `DELETE .../register` — event sign-up

## API (authenticated user)

- `GET /api/me/registrations`
- `GET /api/me/game-identity`
- `POST /api/me/link` — consume 6-digit game link code

## API (admin)

- CMS: `PUT /api/admin/pages/{slug}/sections`, etc.
- Events, announcements, registrations CRUD
- `POST /api/missions` — publish mission JSON (schema-validated)
- `GET /api/admin/events/{id}/slots`
- `PUT /api/admin/events/{id}/slots/{slotId}` — manual ORBAT assignment

## API (game server — bearer `GAME_SERVER_TOKENS`)

- `GET /api/missions/{id}/compiled` — mission JSON for Enfusion loader
- `GET /api/game/events/{id}/roster` — identityId → slotId for ORBAT enforcement
- `POST /api/link` — register 6-digit link code from in-game player
- `POST /api/results` — match results (log-only until Phase 1 persistence)
- `POST /api/telemetry` — gameplay events (log-only)

## Migrations

| File | Purpose |
|---|---|
| `00001_init.sql` | Core schema |
| `00002_seed_content.sql` | CMS seed |
| `00003_events_announcements.sql` | Events hub |
| `00004_missions_orbat_identity.sql` | Missions, link codes, game identities, slot assignments |

## Compiled missions

Static copies for the game API live in [`missions/`](missions/README.md).  
Golden source: [`../tbd-schema/golden-missions/`](../tbd-schema/golden-missions/).

## Environment variables

See [`.env.example`](.env.example) for the full list.
