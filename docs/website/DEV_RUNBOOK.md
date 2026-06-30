# Dev Runbook — spin up the stack

Quick steps to bring up DB + API + Vite locally. See [`CLAUDE.md`](../../apps/website/CLAUDE.md) for full context.
Backend docs: [`docs/backend/README.md`](backend/README.md).

## Start everything

**Toolchain (T-124):** Go **1.26**, Node **26** (repo root [`.nvmrc`](../../.nvmrc) — `nvm use` before frontend work), Postgres **18** (`postgres:18-alpine` in compose).

**CI replay (T-125 — full program):** Primary gate [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) on every push/PR to `main`. Local mirror:

```bash
make db-up          # Postgres on host :5434
nvm use             # Node 26
make ci-local       # verify-editorconfig + backend + verify-coding-standards + FE format:check/lint/build/test + schema/citations
```

**Formatting (T-125.5):** `make verify-editorconfig` (FMT-2, repo root; needs `editorconfig-checker` in `~/go/bin`) · `cd apps/website/frontend && npm run format:check` (FMT-3). Coding-standards bundle: `make verify-coding-standards` (includes `verify-doc-layout` per DOCUMENTATION_STANDARDS §8.2) — see [`CODING_STANDARDS.md`](../platform/CODING_STANDARDS.md) §11.

```bash
# 1. Postgres (port 5434) — quick, run in foreground
make db-up

# 2. Go API on :8080 — run in background; compiles + migrates on boot.
make api

# 3. Vite dev server on :5173 (proxies /api -> :8080) — run in background
make web
```

> **Note:** root `Makefile` prepends `~/.local/go/bin` to PATH for Go targets. If you run `go` directly outside `make`, use `export PATH="$HOME/.local/go/bin:$PATH"`.

## Confirm it's up

```bash
# API responds (200) once migrations finish (~few seconds)
curl -sf http://localhost:8080/api/v1/health
```

- API: http://localhost:8080
- Web: http://localhost:5173

## Contract codegen, validation & CI (T-123)

```bash
# Regenerate Go + TS contract types from packages/tbd-schema/schema/*.json (DO NOT hand-edit outputs)
make schema-codegen

# Validate the shared schemas + golden fixtures (Ajv)
cd packages/tbd-schema && npm run validate

# Verify every @contract tag in the repo resolves to a schema file + JSON pointer
make verify-citations
```

These run in CI via [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) (primary full-repo gate: backend integration tests + golangci + frontend build/lint/test + schema validate + citations) and path-filtered supplements in [`contracts.yml`](../../.github/workflows/contracts.yml) (codegen-drift, supplemental golangci, ESLint TSDoc) and [`schema.yml`](../../.github/workflows/schema.yml). `CreateVersion` validates the mission version payload against `mission-editor-payload.schema.json` before persist.

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

## Postgres 18 upgrade (T-124)

If `make api` fails migrations after pulling T-124, the local volume may still be Postgres **16** data. Re-init:

```bash
make db-down
# podman volume rm tbd-reforger_db_data   # or docker — inspect compose project name
make db-up && make seed
```

Dev data is reseedable; mock missions are optional (see below).

## Registry catalog (T-068)

`make seed` applies `internal/db/seeds/registry_dev.sql` (21 vanilla rows, modpack
`00000000-0000-4000-a000-000000000001`). After a Workbench export, upsert the flat JSON:

```bash
cd apps/website
export PATH="$HOME/.local/go/bin:$PATH"
go run ./cmd/import-registry-items \
  --file ../../packages/tbd-schema/registry/registry-items.workbench.json
```

Run from `apps/website` (module root). Paths to `packages/` are relative to that cwd.
Restart `make api` after handler changes — `go run` does not hot-reload.

`GET /api/v1/registry` requires mission_maker+ JWT; supports weak ETag + `If-None-Match` → 304.

## Map assets (T-090 / T-091)

Static terrain binaries live under `packages/map-assets/{everon,arland}/`. Large PNG/WebP files are tracked via **Git LFS** (see root `.gitattributes`).

```bash
git lfs install   # once per clone
git lfs pull      # after checkout if tiles/DEM missing
```

Each terrain has a `manifest.json` validated against [`packages/tbd-schema/schema/terrain-manifest.schema.json`](../tbd-schema/schema/terrain-manifest.schema.json). **Everon** has real DEM dims (6400×6400) @ T-091.0; **Arland** still stub (`widthPx/heightPx: 0`).

**DEM re-export runbook:** [`t091_0_dem_tile_export.md`](../specs/Mission_Creator_Architecture/t091_0_dem_tile_export.md) §Re-export runbook (GetSurfaceY plugin — manual WE export dead on packed Eden).

**Verify alignment:**

```bash
make verify-terrain           # manifest ↔ terrains.ts + anchor schema
make verify-terrain-strict    # T-091.0 gate — real DEM + ≥10 anchors ±1 m (PASS @ 6d96339)
```

**Local dev serving:** run `make map-assets-link` once per clone (symlinks `packages/map-assets` → `frontend/public/map-assets`). `make web` runs this automatically. Required for Everon DEM fetch in the Mission Creator (T-091.1+). Basemap tiles still T-090.1.

**Frontend tests:** `cd apps/website/frontend && npm test` — vitest `sampleElevation.test.ts` (11 anchor cases vs committed PNG; requires LFS pull).

## Notes

- A fresh DB only has the Discord role→permission mappings (`make seed`).
  Events/missions must be seeded via the API or `psql`.
- **Node 26** required for frontend (`nvm use` at repo root). **Go 1.26** for API (`make build` / `make api`).
- Frontend checks: `cd apps/website/frontend && npm run build` (tsc+vite), `npm run lint`, `npm test` (vitest **21/21**).
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
