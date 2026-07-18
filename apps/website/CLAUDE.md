# CLAUDE.md — apps/website

**Canonical context:** [`../../CLAUDE.md`](../../CLAUDE.md) at the monorepo root. Read that first.

## Layout (T-171)

| Path | Role |
|------|------|
| [`api/`](api/) | Axum + sqlx API — Cargo package `website-api` |
| [`frontend/`](frontend/) | Leptos 0.8 CSR SPA (Trunk) — Cargo package `website-frontend` |

App-level docs stay here; runtime config, migrations, seeds, compose, and `.env` live under `api/`.

## Run from repo root

```bash
make db-up    # Postgres :5434
make api      # Axum API :8080 (CWD = apps/website/api)
make leptos   # Trunk SPA :3000 (proxies /api + /map-assets → :8080)
```

Do **not** use deleted targets (`make web`, Go, Vite/React). Full conventions: [`docs/platform/WHERE_DOES_X_GO.md`](../../docs/platform/WHERE_DOES_X_GO.md).
