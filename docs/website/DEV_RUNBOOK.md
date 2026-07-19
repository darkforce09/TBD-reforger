# Dev Runbook — spin up the stack

Quick steps to bring up DB + Axum API + Leptos (Trunk) locally. Canonical context: root [`CLAUDE.md`](../../CLAUDE.md).
Backend planning (partially archive): [`docs/website/backend/ROADMAP.md`](backend/ROADMAP.md).
Conventions: [`WHERE_DOES_X_GO.md`](../platform/WHERE_DOES_X_GO.md).

## Start everything

**Toolchain:** Rust stable (API + SPA + tooling). Postgres **18** (`postgres:18-alpine` in `apps/website/api/docker-compose.yml`). Node exists only for `enfusion-mcp` under `scripts/mod` (T-165).

**CI replay:** Primary gate [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml). Local mirror:

```bash
make db-up          # Postgres on host :5434
make ci-local       # editorconfig + website-api + coding-standards + leptos + schema/citations
```

**Formatting:** `make verify-editorconfig` · `cargo fmt --check` in `apps/website/api` + `-p website-frontend`. Coding-standards: `make verify-coding-standards`.

```bash
# 1. Postgres (port 5434)
make db-up

# 2. Axum API on :8080 (CWD = apps/website/api; migrates on boot)
make api

# 3. Leptos Trunk SPA on :3000 (proxies /api + /map-assets → :8080)
#    T-173: make leptos = trunk serve --release (day-to-day / perf-honest).
#    Fast rebuilds only: make leptos-debug (unoptimized wasm — do not judge FPS).
#    T-174: satellite = preview→full progressive by default (sharp TBDS).
#           ?sat=preview = Range-only (gates / fast local); ?sat=full is a no-op.
make leptos
```

Config: `apps/website/api/.env` (`FRONTEND_URL=http://127.0.0.1:3000`). Prod SPA flip: `SPA_DIST_DIR=../frontend/dist`.

## Confirm it's up

```bash
curl -sf http://localhost:8080/api/v1/health
```

- API: http://localhost:8080
- Web: http://127.0.0.1:3000

## Contract codegen, validation & CI (T-123)

```bash
make schema-codegen    # → apps/website/api/src/contract/generated/ (DO NOT hand-edit)
make schema-validate   # packages/tbd-schema goldens
make verify-citations
```

CI jobs: `website-api` + `website-frontend` (renamed from `rust-backend` / `website-leptos` at T-171). Path-filtered supplements: [`contracts.yml`](../../.github/workflows/contracts.yml), [`schema.yml`](../../.github/workflows/schema.yml).

## Log in (no Discord needed)

```
http://localhost:8080/api/v1/auth/dev-login?role=admin
```

Roles: `admin | mission_maker | leader | enlisted`.

## Stop

```bash
make db-down      # stops Postgres, keeps volume
# API + trunk: kill the background processes
```

## Postgres 18 upgrade (T-124)

If `make api` fails migrations after pulling T-124, the local volume may still be Postgres **16** data. Re-init:

```bash
make db-down
# podman volume rm tbd-reforger_db_data   # or docker — inspect compose project name
make db-up && make seed
```

Dev data is reseedable; mock missions are optional (see below).

## Registry catalog (T-068 / T-150 / T-068.9)

**Dev seed** (`make seed` → `apps/website/api/seeds/registry_dev.sql`) is the thin 21-row smoke set.

**Full catalog** (Workbench universal export): **1,880 items** + **4,012 compat edges**.

```bash
# From repo root — upserts both committed envelopes into the dev DB (idempotent)
make registry-import

# Or explicit paths / prune:
# cargo run --bin import-registry --manifest-path apps/website/api/Cargo.toml -- \
#   --items packages/tbd-schema/registry/registry-items.workbench.json \
#   --compat packages/tbd-schema/registry/registry-compat.workbench.json \
#   [--modpack <uuid>] [--prune]
```

Restart `make api` after handler changes — `cargo run` does not hot-reload.

| Route | Auth | Notes |
|-------|------|--------|
| `GET /api/v1/registry` | mission_maker+ JWT | Items; weak ETag / 304 |
| `GET /api/v1/registry/compat` | mission_maker+ JWT | Edges; `?edge_type=` filter; ETag |

**Mod compiled mission (T-092.2):**

```bash
# Requires SERVICE_TOKEN in apps/website/api/.env
curl -sS -H "X-Service-Token: $SERVICE_TOKEN" \
  http://localhost:8080/api/v1/missions/{mission_id}/compiled | jq .schemaVersion
```

## Map assets (T-090 / T-091 / T-171)

Corpus: `packages/map-assets/` — Everon ~1.3 GB on disk; **tracked in LFS = exactly 2 objects**:

| Object | Size | Purpose |
|--------|------|---------|
| `everon/dem/everon-dem-16bit.png` | ~72 MB | DEM / hillshade / map-engine tests |
| `everon/satellite/everon-sat.tbd-sat` | ~153 MB | Unified satellite basemap |

`**/staging/` + `**/tiles/` are gitignored (rebuildable via `make map-*`). `.gitattributes` LFS patterns: `packages/map-assets/**/*.{png,r16,tbd-sat}`.

| Consumer | Needs | Mechanism |
|----------|-------|-----------|
| CI `map-engine` job | DEM only | `git lfs pull --include …/everon-dem-16bit.png` |
| CI other jobs | none | sat deliberately never dragged |
| Local dev editor | DEM + sat | Axum `ServeDir` `/map-assets` (`MAP_ASSETS_DIR`, default `../../../packages/map-assets` from `api/` CWD) ← Trunk proxy ← SPA `fetch("/map-assets/…")` |
| Gate harness | dist + optional map-assets | `gate serve --map-assets` |
| Clone without LFS | degraded | manifest/JSON/chunks plain-git; DEM/sat 404 → no sat/hillshade |

**Convenience targets:**

```bash
make lfs-dem   # ~72 MB — enough for map-engine tests + hillshade
make lfs-sat   # ~153 MB — full satellite bundle
# or: git lfs install && git lfs pull
```

Each terrain has a `manifest.json` validated against [`terrain-manifest.schema.json`](../../packages/tbd-schema/schema/terrain-manifest.schema.json).

**Tile pyramid (optional):** not in git. Rebuild:

```bash
make map-water-everon
make map-cartographic-everon
make map-cartographic-verify
```

**Mission Settings → Map basemap (T-173):** the Satellite/Map radio is live. **Map** view needs the cartographic tile pyramid from `make map-cartographic-everon`; when those tiles are absent the host **falls back to satellite** (not a broken toggle).

**Satellite load (T-174):** day-to-day `make leptos` upgrades preview→full TBDS automatically (no `?sat=full`). Use `?sat=preview` only for Range-only / fast iteration (same as CI gates). Density-heatmap green glow is removed.

**Forest canopy (T-176):** island forest highlight is **8 m TBDD canopy mass** (not the old 32 m Path B landcover forest wash). Clearings stay open. Retune tightness: `CANOPY_KERNEL_RADIUS_CELLS` / `CANOPY_MASS_ISO`, then `cargo run -p tbd-tools --bin world -- redensify --terrain everon` (committed-chunk path; no Workbench).

See [`packages/map-assets/README.md`](../../packages/map-assets/README.md). **Ops:** ImageMagick spill → `/var/tmp`.

**Verify:**

```bash
make verify-terrain
make verify-terrain-strict
```

**Frontend/engine tests:** `cargo test -p website-frontend` + `cargo test -p map-engine-core --all-features` (DEM peaks need `make lfs-dem` or `git lfs pull`).

## Notes

- A fresh DB only has Discord role mappings + registry smoke rows (`make seed` → `apps/website/api/seeds/`).
- Frontend: `make ci-local-leptos`; full editor gates: `make leptos-gates` (see [`EDITOR_GATE_RUNBOOK.md`](EDITOR_GATE_RUNBOOK.md) — `gate doctor` preflight, full Chrome `--headless=new`, toolchain **1.95.0**).
- Integration tests: `make test-it` (needs `make db-up`).

## Mock data (optional, not run by `make seed`)

`apps/website/api/seeds/mock_data.sql` (Operation Red Dawn etc.) is **manual psql only** — the Go `cmd/seed` applier was deleted at T-145. Example:

```bash
podman exec -i tbd_reforger_db psql -U tbd -d tbd_reforger < \
  apps/website/api/seeds/mock_data.sql
```

To purge those four fixed-UUID missions (children first; no ON DELETE CASCADE):

```bash
docker compose -f apps/website/api/docker-compose.yml exec -T db psql -U tbd -d tbd_reforger <<'SQL'
DELETE FROM mission_versions  WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM mission_armories  WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM mission_bookmarks WHERE mission_id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
UPDATE missions SET current_version_id = NULL WHERE id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
DELETE FROM missions WHERE id IN ('00000000-0000-4000-c000-000000000001','00000000-0000-4000-c000-000000000002','00000000-0000-4000-c000-000000000003','00000000-0000-4000-c000-000000000004');
SQL
```
