# TBD Reforger — website app

Web suite for the TBD Arma Reforger milsim community: Discord auth, event/ORBAT scheduling, mission library, server telemetry, doctrine wiki, CMS, and admin tooling.

## Layout (T-171)

| Path | Contents |
|------|----------|
| [`api/`](api/) | Rust Axum + sqlx API (`website-api`): `src/`, `migrations/`, `seeds/`, `tests/`, `docker-compose.yml`, `.env` |
| [`frontend/`](frontend/) | Leptos 0.8 CSR SPA (`website-frontend`): Trunk → wasm, proxies `/api` + `/map-assets` |

## Quick start (repo root)

```bash
make db-up    # Postgres on :5434
make api      # Axum API on :8080
make leptos        # Trunk SPA on :3000 (release — T-173)
make leptos-debug  # debug wasm only (not for judging FPS)
```

- **FRONTEND_URL** (dev callback): `http://127.0.0.1:3000`
- **Map assets:** API serves `GET /map-assets/*` from `MAP_ASSETS_DIR` (default `../../../packages/map-assets` from `api/` CWD). Trunk proxies same-origin. Pull LFS: `make lfs-dem` / `make lfs-sat` — see [`DEV_RUNBOOK.md`](../../docs/website/DEV_RUNBOOK.md) §Map assets.
- **Prod SPA flip:** set `SPA_DIST_DIR=../frontend/dist` (relative to `api/`) so Axum can serve the Trunk release build.
- **Seeds:** `api/seeds/` — `make seed` applies `discord_roles.sql` + `registry_dev.sql`; `mock_data.sql` is manual `psql` only.

Full commands: [`DEV_RUNBOOK.md`](../../docs/website/DEV_RUNBOOK.md) · conventions: [`WHERE_DOES_X_GO.md`](../../docs/platform/WHERE_DOES_X_GO.md)

## Documentation

**All docs:** [`docs/website/README.md`](../../docs/website/README.md)

| Role | Start here |
|------|------------|
| **Ticket backlog** | [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) · [`docs/TICKET_REGISTRY.md`](../../docs/TICKET_REGISTRY.md) |
| Frontend | [`docs/website/frontend/ROADMAP.md`](../../docs/website/frontend/ROADMAP.md) |
| Backend | [`docs/website/backend/ROADMAP.md`](../../docs/website/backend/ROADMAP.md) |
| Mission Creator | [`docs/specs/Mission_Creator_Architecture/ROADMAP.md`](../../docs/specs/Mission_Creator_Architecture/ROADMAP.md) |
| AI agents | [`CLAUDE.md`](CLAUDE.md) → root [`CLAUDE.md`](../../CLAUDE.md) |
| Archive / mockups | [`docs/website/archive/README.md`](../../docs/website/archive/README.md) |

## Stack

- **Backend:** Rust (Axum + sqlx), PostgreSQL — `api/src/`, migrations embedded via sqlx
- **Frontend:** Leptos 0.8 CSR (Rust→wasm, Trunk) — `frontend/`
