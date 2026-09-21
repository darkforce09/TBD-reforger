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
- **Map assets:** the API serves `GET /map-assets/*` from `MAP_ASSETS_DIR` (default `../../../assets_v2/terrains` from the `api_v2/` working directory). Trunk proxies same-origin. Pull LFS: `cargo xtask ci lfs-dem` / `cargo xtask ci lfs-sat` — see [`DEV_RUNBOOK.md`](../../docs/website/DEV_RUNBOOK.md) §Map assets.
- **Runtime storage:** uploads and staged missions go to `UPLOAD_DIR` / `MISSION_STAGE_DIR` (development default `assets_v2/scratch/website-api/`, never inside the crate).
- **Prod SPA flip:** set `SPA_DIST_DIR=../frontend/dist` (relative to `api_v2/`) so Axum can serve the Trunk release build.
- **Seeds:** `api_v2/seeds/` — `cargo xtask db seed` applies `discord_roles.sql` + `registry_dev.sql`; `mock_data.sql` is manual `psql` only.

Full commands: [`DEV_RUNBOOK.md`](../../docs/website/DEV_RUNBOOK.md) · conventions: [`WHERE_DOES_X_GO.md`](../../docs/platform/WHERE_DOES_X_GO.md)

## Documentation

**All docs:** [`docs/website/README.md`](../../docs/website/README.md)

| Role | Start here |
|------|------------|
| **Ticket backlog** | [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) · [`docs/TICKET_REGISTRY.md`](../../docs/TICKET_REGISTRY.md) |
| Frontend | [`docs/website/frontend/ROADMAP.md`](../../docs/website/frontend/ROADMAP.md) |
| Backend | [`docs/website/backend/ROADMAP.md`](../../docs/website/backend/ROADMAP.md) · crate atlas [`api_v2/README.md`](api_v2/README.md) |
| Mission Creator | [`docs/specs/Mission_Creator_Architecture/ROADMAP.md`](../../docs/specs/Mission_Creator_Architecture/ROADMAP.md) |
| AI agents | [`CLAUDE.md`](CLAUDE.md) → root [`CLAUDE.md`](../../CLAUDE.md) |
| Archive / mockups | [`docs/website/archive/README.md`](../../docs/website/archive/README.md) |

## Stack

- **Backend:** Rust (Axum + sqlx), PostgreSQL — `api_v2/src/`, migrations embedded via sqlx
- **Frontend:** Leptos 0.8 CSR (Rust→wasm, Trunk) — `frontend/`
