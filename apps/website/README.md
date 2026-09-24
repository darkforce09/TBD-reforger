# TBD Reforger — website app

Web suite for the TBD Arma Reforger milsim community: Discord auth, event/ORBAT scheduling, mission library, server telemetry, doctrine wiki, CMS, and admin tooling.

## Layout

| Path | Contents |
|------|----------|
| [`api_v2/`](api_v2/) | Rust Axum + sqlx API (`website-api`): `src/` (core, background workers, eight domains), `migrations/`, `seeds/`, `tests/`, `docker-compose.yml`, `.env` |
| [`frontend/`](frontend/) | Leptos 0.8 CSR SPA (`website-frontend`): Trunk → wasm, proxies `/api` + `/map-assets` |
| [`map-engine/`](map-engine/) | World, spatial computation, formats, and the mission domain (`website-map-engine`) |
| [`graphics-engine/`](graphics-engine/) | Pure GPU rendering primitives (`website-graphics-engine`) |

## Quick start (repo root)

```bash
cargo xtask db up            # Postgres on :5434
cargo xtask mk rust-api      # Axum API on :8080
cargo xtask mk leptos        # Trunk SPA on :3000 (release build)
cargo xtask mk leptos-debug  # debug wasm only (not for judging FPS)
```

- **FRONTEND_URL** (dev callback): `http://127.0.0.1:3000`
- **Map assets:** the API serves `GET /map-assets/*` from `MAP_ASSETS_DIR` (default `../../../assets_v2/terrains` from the `api_v2/` working directory). Trunk proxies same-origin. Pull LFS: `cargo xtask ci lfs-dem` / `cargo xtask ci lfs-sat` — see [`DEV_RUNBOOK.md`](/documentation_v2/runbooks/local_development.md) §Map assets.
- **Runtime storage:** uploads go to `UPLOAD_DIR` (development default `assets_v2/scratch/website-api/uploads`, never inside the crate).
- **Prod SPA flip:** set `SPA_DIST_DIR=../frontend/dist` (relative to `api_v2/`) so Axum can serve the Trunk release build.
- **Seeds:** `api_v2/seeds/` — `cargo xtask db seed` applies `discord_roles.sql` + `registry_dev.sql`; `mock_data.sql` is manual `psql` only.

Full commands: [`DEV_RUNBOOK.md`](/documentation_v2/runbooks/local_development.md) · conventions: [`WHERE_DOES_X_GO.md`](/documentation_v2/standards/where_does_x_go.md)

## Documentation

**All docs:** [`documentation_v2/README.md`](/documentation_v2/README.md)

| Role | Start here |
|------|------------|
| **Ticket backlog** | [ticketboard](/apps/ticketboard/README.md), the desktop viewer of the ticket registry |
| Frontend | [`documentation_v2/website/frontend/README.md`](/documentation_v2/website/frontend/README.md) |
| Backend | [`documentation_v2/website/api_v2/api_overview.md`](/documentation_v2/website/api_v2/api_overview.md) · crate atlas [`api_v2/README.md`](api_v2/README.md) |
| Mission Creator | [`documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md`](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md) |
| AI agents | [`CLAUDE.md`](CLAUDE.md) → root [`CLAUDE.md`](../../CLAUDE.md) |
| Archive / mockups | [`documentation_v2/archive/monorepo_migration/docs_website_archive_readme.md`](/documentation_v2/archive/monorepo_migration/docs_website_archive_readme.md) |

## Stack

- **Backend:** Rust (Axum + sqlx), PostgreSQL — `api_v2/src/`, migrations embedded via sqlx
- **Frontend:** Leptos 0.8 CSR (Rust→wasm, Trunk) — `frontend/`
