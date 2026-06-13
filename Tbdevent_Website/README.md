# TBD Event Website

Website for organising the TBD PvP event in Arma Reforger. A single Go binary serves the React frontend and REST API, with content stored in PostgreSQL.

## Stack

- **Backend**: Go, chi, pgx, goose migrations, Discord OAuth, scs sessions
- **Frontend**: React, Vite, TypeScript, TanStack Query
- **Database**: PostgreSQL

## Quick start

### 1. Start PostgreSQL

```bash
docker compose up -d
```

### 2. Configure environment

```bash
cp .env.example .env
```

Edit `.env` and set at minimum:

- `DATABASE_URL`
- `SESSION_SECRET`
- Discord OAuth values (`DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URI`)
- Admin access via `ADMIN_DISCORD_IDS` and/or `ADMIN_DISCORD_ROLE_ID` + `DISCORD_GUILD_ID`

### 3. Development

Terminal 1 — API server (runs migrations on start):

```bash
make dev-api
```

Terminal 2 — React dev server with API proxy:

```bash
cd web && npm install && npm run dev
```

Open http://localhost:5173

### 4. Production build

```bash
make build
./bin/tbdevent
```

Open http://localhost:8080

## Docker image

```bash
docker build -t tbdevent-website .
docker run --env-file .env -p 8080:8080 tbdevent-website
```

## Pages

| Route | Content slug |
|-------|----------------|
| `/rules` | `rules` |
| `/compliance` | `compliance` |
| `/server` | `server-info` |
| `/mods` | `mods` |
| `/admin` | CMS (Discord admin only) |

## API

- `GET /api/pages` — list published pages
- `GET /api/pages/{slug}` — page with sections
- `GET /auth/discord` — start Discord login
- `GET /api/auth/me` — current user + admin flag
- `PUT /api/admin/pages/{slug}/sections` — bulk save sections (admin)

## Environment variables

See [`.env.example`](.env.example) for the full list.
