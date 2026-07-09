# CLAUDE.md — TBD Reforger Platform

Working context for AI sessions. Read this first; it is the source of truth for
**current state and how to run things**. Design specs live under [`docs/`](docs/website/README.md)
(`docs/website/platform/context_handoff.md`, `docs/backend/architecture.md`) — verify against
live code for post-T-008 behavior.

## What this is
A web suite for the "TBD" Arma Reforger milsim community: Discord auth, event /
ORBAT scheduling, a mission library (2D editor payloads), server telemetry +
leaderboards, doctrine wiki, CMS, and admin tooling.

- **Backend:** Go (Gin + GORM), PostgreSQL. Module `github.com/tbd-milsim/reforger-backend`, Go 1.26.
- **Frontend:** React 19 + TypeScript + Vite, TanStack Query, Zustand, Tailwind. Node 26. In `apps/website/frontend/`.
- **Mod:** Enfusion framework in `apps/mod/tbd-framework/`; shared mission schema in `packages/tbd-schema/`.
- **Auth:** Discord OAuth2 → JWT access token + rotating single-use refresh token.

## Monorepo layout
- `apps/website/` — Go API + React SPA (run via root `Makefile`)
- `apps/mod/` — Enfusion mod framework (`tbd-framework`, gitignored `crf_framework`/EnfusionMCP)
- `packages/tbd-schema/` — mission JSON schema + golden missions
- `docs/specs/` — design specs (Mission Creator, blueprints); `docs/mod/`, `docs/website/` — app docs (frontend surface specs: `docs/website/frontend/pages/`, not under `apps/`)
- `scripts/mod/`, `scripts/website/`, `scripts/deploy/` — ops scripts (dev/staging/deploy); **`scripts/mod/mcp-call.sh`** + warm daemon for Workbench MCP (see [`docs/mod/MCP_TOOLING.md`](docs/mod/MCP_TOOLING.md))
- `.ai/tickets/` + `scripts/ticket` — unified ticket registry at repo root; `.ai/artifacts/` pipeline output
- `apps/website/cmd/api/` — API entrypoint (loads `.env`, runs migrations on boot, serves `/api/v1`).
- `apps/website/internal/handlers/` — HTTP handlers, one file per resource (auth, missions, events, telemetry, admin, …).
- `apps/website/internal/models/` — GORM models; **JSON field names (snake_case) here are the API contract**.
- `apps/website/internal/db/migrations/` — SQL run before AutoMigrate (extensions, enums, indexes, leaderboard MV).
- `apps/website/internal/services/`, `apps/website/internal/middleware/`, `apps/website/internal/realtime/` (SSE hub).
- `apps/website/frontend/src/` — `api/` (axios client + single-flight refresh), `hooks/` (queries.ts, mutations.ts, useAuthBootstrap), `pages/`, `components/`, `store/useAuthStore.ts`, `types/` (hand-written API types).

## Run it locally
Everything is configured in `apps/website/.env` (`APP_ENV=development`, DB on port 5434, `FRONTEND_URL=http://localhost:5173`). Go lives at `~/.local/go/bin`; root `Makefile` prepends it for `make api` / `make build`.

```bash
make db-up        # start local Postgres (podman/docker compose), port 5434
make api          # run Go API on :8080 (migrates on boot)
make web          # run Vite dev server on :5173 (proxies /api -> :8080)
make test-it      # Go integration tests (needs db-up; sets TEST_DATABASE_URL)
make db-down      # stop Postgres (keeps volume)
```

Frontend checks: `cd apps/website/frontend && npm run build` (tsc + vite), `npm run lint`.

### Dev login (no Discord needed)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=admin|mission_maker|enlisted`.
It mints a real session and 302-redirects to the SPA callback exactly like Discord —
open it in the browser to log in, or curl it and read `access_token` from the
`Location` fragment for API testing.

## Conventions
- API JSON is **snake_case** (from GORM struct tags). Hand-written GORM models remain the
  snake_case DB/API source of truth, and hand-written frontend `types/` mirror them — when changing
  a model, update the matching TS type. Cross-boundary **contract** types are **generated** from
  `packages/tbd-schema/schema/*.json` via `make schema-codegen` into `apps/website/internal/contract/`
  + `apps/website/frontend/src/types/contract/` (DO NOT EDIT; T-123.4). The mission **export** JSON
  (`/missions/:id/export`) is the one camelCase exception.
- List endpoints return `{data, total, limit, offset}` (audit logs use a `next_cursor`).
- Auth tiers: public, `RequireAuth` (JWT), `RequireMinRole(admin|mission_maker)`,
  `RequireServiceToken` (`X-Service-Token`, for game-server ingest).
- Refresh tokens are **single-use** (rotated + revoked each call). All refreshes go
  through one single-flight helper (`apps/website/frontend/src/api/refresh.ts`) so the token is
  never double-spent.
- Git: **commit directly to `main`; never create a branch** (single-ticket mode). End commit messages with
  the `Co-Authored-By` trailer. Commits are tagged `T-00x`.
- **Ticket pipeline** ([`.ai/tickets/README.md`](.ai/tickets/README.md)): all work happens **directly on `main` — no branches** (supersedes the old `ticket/T-0xx` flow). Composer 2.5 owns doc writes/sync; Claude Code ships code + in-code comments; the registry is source of truth (`./scripts/ticket sync`).
- **Documentation standards:** [`docs/platform/DOCUMENTATION_STANDARDS.md`](docs/platform/DOCUMENTATION_STANDARDS.md) — cross-boundary `@contract` / `@route` / `@model`, Godoc/TSDoc/Enforce rules, codegen + validation + CI (**T-123**).
- Docs: see **§Documentation** — sync before commit. Ticket queue: [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md).

## Documentation

Keep docs in sync **in the same commit** as the code change (or immediately before — never merge stale docs).

**Agent split (2026-06):** **Cursor (Composer 2.5)** owns all documentation writes and sync. **Claude Code** reads specs and ships code only — return verify output to Cursor for doc updates. See [`agent_execution.md`](docs/specs/Mission_Creator_Architecture/agent_execution.md) §Agent roles and [`docs/website/AGENT_COMMIT_CHECKLIST.md`](docs/website/AGENT_COMMIT_CHECKLIST.md).

**CRITICAL — Executor gate:** Agents may **ONLY** execute ticket slices where `executor` is `claude-code` (Claude Code) or `cursor-docs` (Cursor documentation pass). If the active slice has `executor: workbench`, `human`, or `ci`, the agent **must stop** and wait for human completion. Do not edit `apps/mod/tbd-framework` Enfusion scripts unless the slice explicitly assigns `claude-code` to a mod script path. `./scripts/ticket run` skips non-`claude-code` rows automatically.

**Before every T-0xx commit, check what changed:**

| Change type | Update |
|-------------|--------|
| Shipped feature / milestone | **§Status** — new T-0xx bullet under **Done**; bump `latest shipped` line |
| **Active slice** (code in progress, not shipped) | **§Status — ACTIVE SLICE** block at top; keep `latest shipped` on last **git tag** only |
| New/changed route | Matching `docs/website/frontend/pages/*.md` + row in `docs/website/frontend/INDEX.md`; verify against `apps/website/frontend/src/router.tsx` |
| UI surface (no new route) | Relevant page doc + `Live source:` path to `apps/website/frontend/src/pages/` or `features/` |
| API / model change | `internal/models/` tags + matching `apps/website/frontend/src/types/`; note handler if behavior changed |
| Mission Creator | MC README, `agent_execution.md` Decisions log, and/or `feature_inventory.md` — only if editor contract or Eden parity changed |
| Deferred / queued work | [`.ai/tickets/registry.json`](.ai/tickets/registry.json) row `status: deferred` or `queued` — sync via `./scripts/ticket sync`; never mark shipped until verified |

**Doc hub:** [`docs/website/README.md`](docs/website/README.md) → [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md) → domain **`ROADMAP.md`** files. Tag contract: [`docs/website/TAGS.md`](docs/website/TAGS.md). **Commit checklist:** [`docs/website/AGENT_COMMIT_CHECKLIST.md`](docs/website/AGENT_COMMIT_CHECKLIST.md).

**Do not update** blueprint HTML, stitch exports, or mock-up HTML — archive tier only. Live UI = `apps/website/frontend/src/pages` + `features/`.

**Doc-only commits** (reorgs, typo fixes) get their own T-0xx tag and a §Status note if structure or authority changed.

## Ticket operations

**Source of truth:** [`.ai/tickets/registry.json`](.ai/tickets/registry.json). **Lead view:** [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md). **Full table:** [`docs/TICKET_REGISTRY.md`](docs/TICKET_REGISTRY.md).

| Step | Command / doc |
|------|----------------|
| Edit queue / status / spec | Edit `.ai/tickets/registry.json` |
| Regenerate views + CLAUDE status block | `./scripts/ticket sync` (or `make ticket-sync`) |
| Validate structure | `./scripts/ticket check` |
| Strict legacy-ID scan | `make ticket-check-strict` |
| Operator playbook | [`.ai/tickets/AI_PLAYBOOK.md`](.ai/tickets/AI_PLAYBOOK.md) |
| Claude Code brief | `./scripts/ticket brief T-0xx` |
| Batch implement | `./scripts/ticket run` on `ticket/T-0xx` branch (claude-code slices only) |
| Mod / Workbench queue | [`docs/TICKET_MOD_QUEUE.md`](docs/TICKET_MOD_QUEUE.md) |
| Advance slice | `./scripts/ticket advance-slice T-0xx` |

Do **not** hand-edit generated `docs/TICKET_*.md` or the `<!-- ticket-sync:status -->` markers — change the registry and sync.

## Status

<!-- ticket-sync:status:start -->
**Latest shipped:** **T-145**

**ACTIVE NOW:** **T-090** — T-090.6 (Map visualization program). Slice spec: `docs/specs/Mission_Creator_Architecture/t090_6_geometry_placement_audit.md`.

**Next (by order):**
- **T-071** — ORBAT Manager modal (`queued`)
- **T-072** — Ctrl multi-place (`queued`)
- **T-073** — Shift + map rotation (`queued`)
- **T-074** — Faction submode / catalog filter (`queued`)
- **T-075** — Spacebar flyTo vs widget (`queued`)
- **T-090** — Map visualization program (`ready`)
- **T-114** — Slot roster enforcement + production slot picker (`queued`)
- **T-115** — Capture win condition (`queued`)
- **T-116** — Results POST to backend (`queued`)
- **T-117** — Mission upload + validation UI (`queued`)
<!-- ticket-sync:status:end -->

T-005..T-007 between T-004 and T-008 are documentation/seed only; the status below is current.

**Workspace restructure (2026-06-26, `2a51d66`):** monorepo reorganized into the
`apps/` + `packages/` + unified `docs/` + `scripts/` + hidden `.ai/` layout (see
**§Monorepo layout**). Pure relocation + path-fix, no behavior change: `website`→`apps/website`,
`mod`→`apps/mod`, `shared/tbd-schema`→`packages/tbd-schema`, `tickets`→`.ai/tickets`,
`artifacts`→`.ai/artifacts`; ops scripts lifted to `scripts/{mod,website,deploy}`; app docs to
`docs/{mod,website}`. `paths.sh`, Makefile, CI, ticket libs, and `verify-monorepo-migration.sh`
updated; doc links repaired (incl. pre-existing `docs/specs/**` rot). Verified: go build,
frontend build+lint, schema, `ticket check --strict`, gate V1–V27.

### COMPLETE — Fable 5 audit program (T-126 → T-127 → T-128)

Hub: [`FABLE_5_AUDIT_PROGRAM.md`](docs/platform/FABLE_5_AUDIT_PROGRAM.md) · living tracker [`.ai/artifacts/fable_5_omni_audit_report.md`](.ai/artifacts/fable_5_omni_audit_report.md)

**T-126** security @ `4a47688e` → **T-127** MC UX @ `0515aabb` → **T-128** doc links (tag **T-128**) → **T-130** OPEN/PARTIAL remainder @ `90c9f261` (tag **T-130**, doc sync **T-130.7** @ `5e0c7754`). Fable program **complete** — tracker index authoritative; only **F5-10** spelling remains OPEN (trivial/deferred). Next work: [`docs/TICKET_LEAD.md`](docs/TICKET_LEAD.md) (**T-071** ORBAT Manager · **T-090.1.1.1** land-cover).

### Map Engine v2 — T-090 (implementation)

Plan @ `a222a146` · [implementation plan](.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md) · LOD v2 [`t090_render_lod_contract.md`](docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)

**Active:** **T-090.6** (geometry placement audit).

**Done (Map Engine v2 render lane — latest):**
- T-090.5.5 **Map Engine v2 tree/veg/prop glyphs** @ `2b1a0dda` (tag **T-090.5.5**). `IconLayer` for individual tree/veg/prop glyphs; `FpsCounter.tsx` HUD + `Ctrl+Alt+D` toggle; `getTreeStreamDebug()` surface; `loadManifest` deduplication. Verify: [`.ai/artifacts/t090_5_5_verify_log.md`](.ai/artifacts/t090_5_5_verify_log.md).
- T-090.5.4 **Map Engine v2 sea-band + DEM contours** @ `bd481cf1` (tag **T-090.5.4**). `world-sea` (slot 2) + `world-contours` (slot 5) from 6400² DEM via worker (`demGrid`/`seaBand`/`contours` pure modules + `demVectorStore`); vitest **223/223**; whole-island static geometry per interval band. Contour ladder implements §N3 (`20 m @ 0…+1`, `10 m @ +1…+3` — ticket prose reconciled). Manual M-shore/R5 = operator GPU pass. Verify: [`.ai/artifacts/t090_5_4_verify_log.md`](.ai/artifacts/t090_5_4_verify_log.md).

**Audit:** [`CODEBASE_AUDIT_2026.md`](docs/platform/CODEBASE_AUDIT_2026.md) · **T-122 shipped** @ `f131770` (tag **T-122**).

**T-091.2 shipped @ `dde589e` (tag T-091.2):** `terrainZ` in `ydoc`; CUR/SEL X/Y/Z @ 3 dp; Mission Settings hillshade + grid; `useDemLayer` + grid-over-hillshade; `useDemVersion` async CUR refresh. Vitest **21/21**. **T-091 program complete.** Spec: [`t091_2_z_axis_editor.md`](docs/specs/Mission_Creator_Architecture/t091_2_z_axis_editor.md).

**T-091.1 shipped @ `2c56c2e` (tag T-091.1):** `tactical-map/dem/*` — loader + `sampleElevation` API. Spec: [`t091_1_dem_loader.md`](docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md).

**T-091.0 shipped @ `6d96339` (tag T-091.0):** Everon 6400² DEM export + anchor verify. Spec: [`t091_0_dem_tile_export.md`](docs/specs/Mission_Creator_Architecture/t091_0_dem_tile_export.md).

### T-151 — wgpu Mission Creator engine program

Hub: [`t151_wgpu_engine_program.md`](docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md) · worktree
`tbd-reforger-wgpu-spike/` only (manual Claude prompts; no per-slice branches or `./scripts/ticket run`).

**Next slice:** **T-151.6** (W6 mission entities: slots, selection, drag, clusters) — `ready` ·
program hub [`t151_wgpu_engine_program.md`](docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md) §T-151.6

**Done (program slices):**
- T-151.5 **glyph atlas (W5)** @ `0b7621ed` (tag **T-151.5**). Atlas once + IconInstanced **20 B**;
  trees/props/badges on wgpu; vitest **372**; wasm **4,054,850 B**. Forest mass not retuned.
  Verify: [`.ai/artifacts/t151_5_verify_log.md`](.ai/artifacts/t151_5_verify_log.md). Spec:
  [`t151_5_glyph_atlas.md`](docs/specs/Mission_Creator_Architecture/t151_5_glyph_atlas.md).
- T-151.4.1 **building wipe + road joins** @ `552e68aa` (tag **T-151.4.1**). Empty mid-flight
  upload no longer clears building lane; inflight abort fixed; polyline miter joins + round caps.
  Forest overdraw deferred to post-glyph analysis. Verify:
  [`.ai/artifacts/t151_4_1_verify_log.md`](.ai/artifacts/t151_4_1_verify_log.md). Spec:
  [`t151_4_1_building_road_hotfix.md`](docs/specs/Mission_Creator_Architecture/t151_4_1_building_road_hotfix.md).
- T-151.4 **vector layers (W4)** @ `723490a0` (tag **T-151.4**). PolygonFill + sea/landcover/
  contours/roads/forest mass on wgpu; vitest **371**; wasm **4,005,415 B**. Verify:
  [`.ai/artifacts/t151_4_verify_log.md`](.ai/artifacts/t151_4_verify_log.md). Spec:
  [`t151_4_vector_layers.md`](docs/specs/Mission_Creator_Architecture/t151_4_vector_layers.md).
- T-151.3 **chunk residency + first world GPU instances (W3)** @ `32bf5ac5` (tag **T-151.3**).
  `WorldResidency` + chunk-keyed pick index; building OBB fill/outline on `WgpuTacticalMap`; vitest
  **371** (+28); merged wasm **3,946,734 B** (+88,143). P1–P14 proof ledger PASS; GPU-R building
  readback byte-exact; 10k pick + 22-step residency Class **S** parity. Deck worker untouched.
  Verify: [`.ai/artifacts/t151_3_verify_log.md`](.ai/artifacts/t151_3_verify_log.md). Spec:
  [`t151_3_world_residency.md`](docs/specs/Mission_Creator_Architecture/t151_3_world_residency.md).
- T-151.2 **world parser in Rust (W2 Piece 1)** @ `a51e9dcb` (tag **T-151.2**). `world/` module +
  wasm `WorldStore`; Class **R**/**S** on all **275** Everon chunks; census **391 / 508,291 / 888 /
  36 / 625** exact; vitest **343** (+9); merged wasm **3,858,591 B** (+135,399). Parse-only — no
  worker flip or GPU world draws. Verify:
  [`.ai/artifacts/t151_2_verify_log.md`](.ai/artifacts/t151_2_verify_log.md). Spec:
  [`t151_2_world_parser.md`](docs/specs/Mission_Creator_Architecture/t151_2_world_parser.md).
- T-151.1 **basemap lane (TBDS + hillshade + grid)** @ `3ab81587` (tag **T-151.1**).
  `TexturedQuad` + `Polyline` pipelines; `basemapResolve.ts` extracted (Deck-oracle); unified
  TBDS + pyramid/single/none fallback on wgpu; hillshade + procedural grid; vitest **334**
  (+17); merged wasm **3,723,192 B**. GPU gates byte-exact via headless CDP (`texture_self_check`,
  T-151.0 self_check regression, real-DEM hillshade). Verify:
  [`.ai/artifacts/t151_1_verify_log.md`](.ai/artifacts/t151_1_verify_log.md). Spec:
  [`t151_1_basemap_lane.md`](docs/specs/Mission_Creator_Architecture/t151_1_basemap_lane.md).
- T-151.0 **wasm merge + batch list + editor dual mount** @ `f019512d` (tag **T-151.0**). One wasm
  module (D1): `RenderEngine` + `MissionDoc` + `OrthoCameraJs` share one linear memory; merged
  `map_engine_wasm_bg.wasm` = **3,658,383 B**; batch list seam; `WgpuTacticalMap` lazy-loaded
  behind `?engine=wgpu`; L10 shared-memory HUD proof. Automated gates exit 0; browser S1–S3
  operator-pending. Verify: [`.ai/artifacts/t151_0_verify_log.md`](.ai/artifacts/t151_0_verify_log.md).
  Spec: [`t151_0_wasm_merge_dual_mount.md`](docs/specs/Mission_Creator_Architecture/t151_0_wasm_merge_dual_mount.md).

### T-068 — Virtual Arsenal (Phase 2 paused; not active)

See [`t068_virtual_arsenal_program.md`](docs/specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md) · **paused @ T-068.7** until **T-071.2** + **T-068.13** (T-092 spawn/compile gate **shipped** @ `a73224f2`) · dev queue [`docs/TICKET_DEV_QUEUE.md`](docs/TICKET_DEV_QUEUE.md).

**Phase 1 shipped @ 2026-06-27** (E2E **T-068.6 PASS**). **Boundary:** web loadout-export → profile JSON → mod dresses a **non-player test NPC** @ game-mode spawn — **not** the joining human player until **T-068.12** (compiler data @ **T-068.11**).

**T-068 program (Phase 1 — shipped slices):**
- **T-068.0.1** JSON schemas + golden fixtures @ `2487d59` — `registry-items` + `loadout-export` in `packages/tbd-schema/`. Spec: [`t068_0_1_registry_schemas.md`](docs/specs/Mission_Creator_Architecture/t068_0_1_registry_schemas.md).
- **T-068.1** Workbench/MCP flat export @ `ca4f2cd` — 21 vanilla rows @ `packages/tbd-schema/registry/registry-items.workbench.json`; plugin `TBD_RegistryItemsExportPlugin.c`. Spec: [`t068_1_workbench_flat_export.md`](docs/specs/Mission_Creator_Architecture/t068_1_workbench_flat_export.md).
- **T-068.2** Registry API @ `4c609fe` (tag **T-068.2**) — `GET /api/v1/registry` (weak ETag/304), `registry_items` model + migration `05_registry_items.sql`, `registry_dev.sql` seed (21 rows, all 5 kinds), `cmd/import-registry-items`, FE types only. Spec: [`t068_2_registry_api.md`](docs/specs/Mission_Creator_Architecture/t068_2_registry_api.md). Ops: [`DEV_RUNBOOK.md`](docs/website/DEV_RUNBOOK.md) §Registry catalog.
- **T-068.3** Factions palette wire @ `da78452` (tag **T-068.3**) — `useRegistry()` + `buildCatalogTree`; `assetCatalogMock.ts` deleted; DnD `assetId` = full `resource_name`; spinner-first loading. Closes **RIGHT-CAT-001**. Spec: [`t068_3_palette_wire.md`](docs/specs/Mission_Creator_Architecture/t068_3_palette_wire.md).
- **T-068.4** Arsenal dumb loadout UI @ `a85f16b` (tag **T-068.4**) — replace Attributes Arsenal stub; 4 gear dropdowns + `loadout-export.json` download; character slots only. Closes **ATTR-TAB-004**. Spec: [`t068_4_dumb_loadout_ui.md`](docs/specs/Mission_Creator_Architecture/t068_4_dumb_loadout_ui.md).
- **T-068.5** Mod equip scaffold @ `21ec91e` (tag **T-068.5**) — `TBD_LoadoutEquipComponent`; profile JSON → test NPC @ 6400 (log-only equip; superseded by .5.1). Spec: [`t068_5_mod_equip_loadout.md`](docs/specs/Mission_Creator_Architecture/t068_5_mod_equip_loadout.md).
- **T-068.5.1** Visual wear on test NPC @ `b233b11` (tag **T-068.5.1**) — `EquipCloth`/`EquipWeapon` + worn-verify; kit visible on **NPC**, not player. Spec: [`t068_5_1_visual_equip_fix.md`](docs/specs/Mission_Creator_Architecture/t068_5_1_visual_equip_fix.md).
- **T-068.6** Phase 1 E2E gate **PASS** @ 2026-06-27 — E1–E12; Phase 2 approved. Spec: [`t068_6_phase1_e2e_gate.md`](docs/specs/Mission_Creator_Architecture/t068_6_phase1_e2e_gate.md).

**Phase 2 next (after T-090 map program):** **T-071** ORBAT Manager (**deferred** — map-first lane) → **T-068.7** compat matrix → T-068.8–T-068.11 → **T-068.12** mod **player** loadout → **T-068.13** production LOBBY slot picker → **T-068.14** E2E. Do **not** `./scripts/ticket done T-068` until **T-068.14**. Hub: [`t071_orbat_manager_program.md`](docs/specs/Mission_Creator_Architecture/t071_orbat_manager_program.md).

**Done (shipped):**
- T-145 **Rust/Wasm doc core (Yjs replacement)** — backend Go→Rust (Axum + sqlx) + the mission document core moved into a Rust/wasm `yrs` doc. **Flip F1→F4 complete:** the `yrs` wasm doc behind `WasmMissionDoc` is the sole document core; **yjs + y-indexeddb removed** from the app. Commits F3 `a335cc23` · F3.1 `06fab65c` · F4 `a228ed98`. **Pivot:** the world-object zero-copy port (kickoff `.ai/artifacts/t145_world_zerocopy_kickoff.md`) is **superseded by the wgpu render-engine spike (T-151)** — Deck.gl `IconLayer` can't take binary buffers, so world objects can't reach zero-copy render through Deck; a pure wgpu/wasm engine replaces it.
- T-090.8.1 **Map Engine v2 forest mass render** @ `e28d073a` (tag **T-090.8.1**). `world-landcover` (36 region hulls) + `world-forest` / `world-forest-outline` (TBDD marching squares, worker-streamed); vitest **192/192**; P2b headroom (2.2 MB / 29 ms full island). No tree glyphs. Verify: [`.ai/artifacts/t090_8_1_verify_log.md`](.ai/artifacts/t090_8_1_verify_log.md).
- T-090.5.3 **Map Engine v2 worker chunk streaming** @ `155651b9` (tag **T-090.5.3**). `worldObjectsCore.ts` + thin Comlink worker; `chunkStore` LRU (≤4 ms/frame apply, worst chunk 0.65 ms); roads main-thread one-shot; trees indexed in worker, not rendered. Vitest **150/150**; build/lint clean. Verify: [`.ai/artifacts/t090_5_3_verify_log.md`](.ai/artifacts/t090_5_3_verify_log.md).
- T-090.5.2.2 **Map Engine v2 taxonomy render pass** @ `346a31c9` (tag **T-090.5.2.2**).
- T-090.5.2.1 **Road centerline + casing + solid buildings** @ `04b60857` (tag **T-090.5.2.1**). `extractRoadCenterline` from quad-soup; `world-roads-casing`; dark building fills. Operator visual pass.
- T-090.5.2 **Map Engine v2 roads + buildings live** @ `e410545e` (tag **T-090.5.2**). First world-object Deck layers: `world-roads`, `world-buildings`, `world-building-badges`; glyph atlas 19→28 glyphs; `worldData.ts` chunk loader. Vitest 102/102. Verify: [`.ai/artifacts/t090_5_2_verify_log.md`](.ai/artifacts/t090_5_2_verify_log.md).
- T-090.3.3 **Map Engine v2 taxonomy + highway network** @ `887a6ed1` (tag **T-090.3.3**). Full `.topo` road mapping (RIVER/STREAM → asphalt); **888** road segments; data-driven classify rebuild; measured prefab OBBs; **391** prefabs / **508,291** instances / **275** chunks (4,131 buildings + 2,299 piers + 501,861 trees). Enums + 28-glyph atlas; all export gates PASS. Verify: [`.ai/artifacts/t090_5_2_verify_log.md`](.ai/artifacts/t090_5_2_verify_log.md) §T-090.3.3.
- T-090.5.1 **Map Engine v2 render spine scaffold** @ `589ded9e` (tag **T-090.5.1**).
- T-090.3.2 **Map Engine v2 export P2 (density + trees + forest regions)** @ `a055df95` (tag **T-090.3.2**). Built from T-090.3.1 staged raw — **no Workbench re-export**. Cumulative **361** prefabs / **507,467** instances / **270** chunks (6.25 MB gz, plain commit); **501,861** trees (51 types) + unchanged **5,606** P1 buildings. **625** `objects/density/{cx}_{cy}.bin` TBDD grids (732.5 KB); **forest-regions.json.gz** 36 Path B regions (F2 exact: 496,693 + 5,168 = 501,861). Gates: P2 24/24, P1 re-verify 19/19, schema-validate (S13/S14), map-export-validate, map-census, verify-spike-all — ALL PASS. Decisions: Latin species classify rules; debris→prop; verify-phase phase-scope split; prefabId renumber on cumulative rebuild; mega-region `forest-everon-001` (479k trees — tunable in `lib/forest-regions.mjs` @ T-090.8). Verify: [`.ai/artifacts/t090_3_2_verify_log.md`](.ai/artifacts/t090_3_2_verify_log.md).
- T-090.3.1 **Map Engine v2 export core (P1 buildings + roads)** @ `e47f25fc` (tag **T-090.3.1**). Full-world export 1.41M entities / 8.9 s; **310** building prefabs, **5,606** instances, **219** chunks (84 KB gz); **roads.json.gz** 766 segments (decode-topo). Gates 19/19 PASS. Verify: [`.ai/artifacts/t090_3_1_verify_log.md`](.ai/artifacts/t090_3_1_verify_log.md). **Note:** Workbench new plugin classes need Script Editor compile (`wb_reload` insufficient).
- T-090.10.1 **Map Engine v2 implementation plan** @ `a222a146` (tag **T-090.10.1**).
- T-144.1 **Arma 3 map architecture study** @ `b1949182` (tag **T-144.1**). A3 Arcade editor 2D map — no basemap tiles; live `GLandscape`; vectors on top. Artifacts: [report](.ai/artifacts/t144_arma3_map_architecture_report.md). Spec: [`t144_arma3_map_architecture_study.md`](docs/specs/Mission_Creator_Architecture/t144_arma3_map_architecture_study.md).
- T-090.1.1.1 **Map program — cartographic land-cover compose** @ `018ea70d` (tag **T-090.1.1.1**). **L1** SAP-read-only forest/open masks (`build-landcover-mask.mjs` @ 3200² → soft masks; water excluded; ~25 m morphology) + pre-upscale tint pass in `build-map-cartographic.mjs` (open `#CDC6A3` @ 0.70, forest `#37502D` @ 0.80; TGA olive relief preserved). Spike: TGA provably monochrome — zero px on plugin `forestArea` ramp. **`make map-cartographic-everon`** ~2 min (tints @ 4096² before Lanczos — avoids 12800² magick OOM). M3 operator PASS @ (4870, 7760); M4 alignment ≪50 m. Satellite frozen. Verify: [`.ai/artifacts/t090_1_1_1_source_spike.json`](.ai/artifacts/t090_1_1_1_source_spike.json), [verify log](.ai/artifacts/t090_1_1_1_verify_log.md). Spec: [`t090_1_1_1_map_landcover_compose.md`](docs/specs/Mission_Creator_Architecture/t090_1_1_1_map_landcover_compose.md).
- T-092 **Spawn transform parity + mod mission compile** @ **`a73224f2`** (tags **T-092.1** `4eefc169`, **T-092.2** `a73224f2`; verify log commit **`452ce501`**). **T-092.1:** `mission.schema.json` optional slot `y` + **schemaVersion "1.2"**; `TBD_MissionSlotStruct`/`TBD_SpawnManager` jsonY→GetSurfaceY policy, `CAPSULE_GROUND_OFFSET_M=0.0` (measured ≈0), `headingDeg`, `[TBD][Spawn]` logs. **T-092.2:** `flattenEditorToModDocument` (TS) + `services.FlattenToModDocument` (Go), `kit-aliases.json` + codegen mirror, **`GET /api/v1/missions/:id/compiled`** (`X-Service-Token`, 409 no-slots), mod loader `/api/v1/...` + token header fix. wb_play + live REST E2E **PASS** 2026-07-04 (M4 roster caveat OBS-1 → T-068.13; OBS-2 `TBD_MissionList` legacy path). Unblocks **T-071**. Verify: [`.ai/artifacts/t092_1_verify_log.md`](.ai/artifacts/t092_1_verify_log.md), [`.ai/artifacts/t092_2_verify_log.md`](.ai/artifacts/t092_2_verify_log.md). Hub: [`t092_spawn_transform_program.md`](docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md).
- T-090.1.1 **Map program — Map cartographic view (pyramid + UI switch)** @ `6e06e679` (tag **T-090.1.1**). G1-A MapDataExporter TGA (4096² north-up) upscaled → composed ortho (inland-water tint from `.2.5.2` mask + `.topo` road strokes; **`despike()`** on geometry-baked width excursions). **`make map-cartographic-everon`** / **`map-cartographic-verify`** (`VIEW=map` z0–6, ~5461 WebP tiles local/gitignored). Manifest `tiles.map` → `workbench-cartographic` + `webp-lossy`. Frontend: T-127 `'map'` coercion removed; Map radio live; per-view `useTerrainBasemapLayer` (`basemap-map-*` ids; satellite unified texture survives switches); `basemapView.test.ts`. M1/M2/M3/M4/M5/M7/M8 PASS; M6/M9 operator browser. Ops: magick spill → `/var/tmp`. Artifacts: [`.ai/artifacts/t090_1_1_source_spike.json`](.ai/artifacts/t090_1_1_source_spike.json), [verify log](.ai/artifacts/t090_1_1_verify_log.md). Spec: [`t090_1_1_map_cartographic_view.md`](docs/specs/Mission_Creator_Architecture/t090_1_1_map_cartographic_view.md).
- T-090.1.2.5.2 **Map program — .topo road guard + one-button water** @ `1c07d97a` (tag **T-090.1.2.5.2**). `decode-topo.mjs` (G1-B: roads only, no hydro in `.topo`); `roadFrac ≤ 0.45` guard; relaxed wet-channel; **`make map-water-everon`**. Operator **good enough** 2026-07-03; perfect water → **T-143** (`idea`). Artifacts: [`.ai/artifacts/t090_1_2_5_2_source_spike.json`](.ai/artifacts/t090_1_2_5_2_source_spike.json), [verify log](.ai/artifacts/t090_1_2_5_2_verify_log.md). Spec: [`t090_1_2_5_2_water_topo_refine.md`](docs/specs/Mission_Creator_Architecture/t090_1_2_5_2_water_topo_refine.md).
- T-090.2 **Map program — map object taxonomy ship** @ `691d9b26` (tag **T-090.2**). +29 golden prefabs (S9 full enum coverage), +4 road segments, +1 `waterBody` region, instances/resolved samples; **`verify-map-object-golden.mjs`** S2–S9 wired into `make schema-validate`; +12 append-only `prefab-classify.json` rules; Everon manifest `objects` stub. Census stays `pending_export` until **T-090.3**. Verify: [`.ai/artifacts/t090_2_verify_log.md`](.ai/artifacts/t090_2_verify_log.md). Spec: [`t090_2_map_object_taxonomy.md`](docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md).
- T-090.1.2.5.1 **Map program — inland water mask refine** @ `82488c6f` (tag **T-090.1.2.5.1**). Two-tier inland mask: compact `FLAT_FRAC_MAX` 0.5→**0.12** (road FP rejection); linear grey-river + wet-channel classes with DEM valley carve (48 m boxBlur) — 114 accepted bodies, 85 new stream segments; original operator FP sites PASS; **operator post-ship: residual FP/FN** at full-map pan (~4617, 8711 viewport). R3: Eden.topo BE-float32 polylines confirmed, framing undecoded → **T-090.8** lead. Artifacts: [`.ai/artifacts/t090_1_2_5_1_refine_spike.json`](.ai/artifacts/t090_1_2_5_1_refine_spike.json), [verify log](.ai/artifacts/t090_1_2_5_1_verify_log.md). Spec: [`t090_1_2_5_1_water_mask_refine.md`](docs/specs/Mission_Creator_Architecture/t090_1_2_5_1_water_mask_refine.md).
- T-090.1.2.6 **Map program — hillshade blend strength slider** @ `b958e3b4` (tag **T-090.1.2.6**). Mission Settings slider `hillshadeOpacity` 0–100% @ **0.1%** steps; split memo in `useDemLayer.ts`. Spec: [`t090_1_2_6_hillshade_blend_control.md`](docs/specs/Mission_Creator_Architecture/t090_1_2_6_hillshade_blend_control.md).
- T-090.1.2.5 **Map program — satellite water composite** @ `6396960f` (tag **T-090.1.2.5**). Ocean mask A (DEM≤0) + inland mask E (SAP grey appearance ∩ DEM filters); composited ortho → unified bundle + lossless pyramid. Spec: [`t090_1_2_5_satellite_water_composite.md`](docs/specs/Mission_Creator_Architecture/t090_1_2_5_satellite_water_composite.md).
- T-127 **Fable audit — Mission Creator UX fixes** @ `0515aabb` (tag **T-127**). U1–U5: conflict IDB + warm marker, export toasts, basemap `'map'` coerce, folder delete confirm, ORBAT 409 messages. Vitest 26/26; FE build/lint clean. **Partial:** F4-03 new-tab conflict deferred. Verify: [`.ai/artifacts/t127_verify_log.md`](.ai/artifacts/t127_verify_log.md). Spec: [`t127_mc_ux_audit_fixes.md`](docs/platform/t127_mc_ux_audit_fixes.md).
- T-126 **Fable audit — security + auth follow-up** @ `4a47688e` (tag **T-126**).
- T-125 **Coding standards + 11/10 enforcement** @ `e21dac3` (tag **T-125.5**). [`CODING_STANDARDS.md`](docs/platform/CODING_STANDARDS.md) (38 rules, all gates live): golangci, strict TS, GO-7 `@route` route-match, verify-* scripts, ENF-4 ×10, `.editorconfig` + Prettier (FMT-2/3). `make ci-local` @ ~22.7s mirrors **`ci.yml`** (backend + frontend + schema + editorconfig). Spec: [`t125_coding_standards_enforcement.md`](docs/platform/t125_coding_standards_enforcement.md).
- T-124 **Dependency & toolchain upgrade** @ `cd11db0`. FE npm to latest (vitest **4.1.9**, deck.gl 9.3.5, vite 8); Go modules gin **1.12**, gorm **1.31.2**, pgx **5.10**; **Go 1.26**, **Node 26** (`.nvmrc` + CI), **Postgres 18** dev image; dropped unused `@tailwindcss/container-queries`. Verify: FE build/lint/**21/21** tests, `make build`, `make test-it`, `make schema-codegen` clean. Spec: [`t124_dependency_upgrade.md`](docs/platform/t124_dependency_upgrade.md).
- T-123 **Documentation standards rollout** @ `169e47d` (tag **T-123**). In-code `@contract`/`@route`/`@authority` tags (Go/TS/Enfusion); schema codegen → `apps/website/internal/contract/` + `apps/website/frontend/src/types/contract/` via `make schema-codegen`; `CreateVersion` validates `mission-editor-payload.schema.json` (400 on invalid; `internal/contract/validate.go`); `contracts.yml` CI (citation verifier, golangci revive, eslint TSDoc, codegen-drift). Resolves audit T1/T8. Spec: [`t123_documentation_standards_rollout.md`](docs/platform/t123_documentation_standards_rollout.md).
- T-122 **Codebase audit hotfix (single bundle)** @ `f131770` (tag **T-122**). 37/41 findings (C/R/T/M/D); deferred T1/T3/T8/T15 with rationale (T1/T8 since resolved by T-123). `make test-it` + FE build/lint clean. Spec: [`CODEBASE_AUDIT_2026.md`](docs/platform/CODEBASE_AUDIT_2026.md).
- T-091.2 **Mission Creator — Z-axis editor UX** @ `dde589e` (tag **T-091.2**).
- T-091.1 **Mission Creator — DEM loader + sampleElevation** @ `2c56c2e` (tag **T-091.1**). Spec: [`t091_1_dem_loader.md`](docs/specs/Mission_Creator_Architecture/t091_1_dem_loader.md).
- T-091.0 **Map program — Everon 16-bit DEM export + anchor verify** @ `6d96339` (tag **T-091.0**). **PATH 3:** `TBD_TerrainExportPlugin.c` resamples `WorldEditorAPI.GetTerrainSurfaceY` over 6400² grid → ASCII uint16 → `raw-u16-to-dem-png.mjs` → LFS PNG (`dem.source`: `mod-getsurfacey-resample`). Manual WE **Export Height Map** dead on packed Eden. **`make verify-terrain-strict` PASS** — 11 anchors, maxDeltaM **0.204 m** (threshold 1.0). Verify fix: pngjs `{ skipRescale: true }` + `.depth` not `.bitDepth`. Tiles deferred (T-090.1). Spec: [`t091_0_dem_tile_export.md`](docs/specs/Mission_Creator_Architecture/t091_0_dem_tile_export.md). Ops: [`.ai/artifacts/t091_0_ops_log.txt`](.ai/artifacts/t091_0_ops_log.txt).
- T-090.1.2.8 **Map program — unified satellite texture (tbd-sat v1)** @ `db9057ef` (tag **T-090.1.2.8**). One `everon-sat.tbd-sat` bundle (205.9 MB LFS, 14-level mip chain); `satelliteUnified.ts` → single trilinear GPU texture on one BitmapLayer — zero tile HTTP/layer churn on pan/zoom; pyramid fallback via manifest `delivery: "pyramid"`. **Operator U1–U4 PASS** (2026-07-02). Format spike: [`.ai/artifacts/t090_1_2_8_format_spike.json`](.ai/artifacts/t090_1_2_8_format_spike.json). Verify: [`.ai/artifacts/t090_1_2_8_verify_log.md`](.ai/artifacts/t090_1_2_8_verify_log.md). Spec: [`t090_1_2_8_unified_satellite_texture.md`](docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md).
- T-090.1.2.4 **Map program — engine render ortho spike (honest P0 FAIL)** @ `0d6fe485` (tag **T-090.1.2.4**). Exhaustive Workbench MCP api_search: no orthographic projection, no per-point terrain colour, no RenderTarget readback — **no grid-free sat-class 12800² source**. SAP + T-090.1.2.2 apron-bridge **locked as production source**. Pivot 110% to **T-090.1.2.8** (unified GPU texture + mips). Artifacts: [`.ai/artifacts/t090_1_2_4_engine_render_spike.json`](.ai/artifacts/t090_1_2_4_engine_render_spike.json), [verify log](.ai/artifacts/t090_1_2_4_verify_log.md). Spec: [`t090_1_2_4_engine_render_ortho_spike.md`](docs/specs/Mission_Creator_Architecture/t090_1_2_4_engine_render_ortho_spike.md).
- T-067 **Mission Creator — spatial chunks / bulk-paste scale**. **`slot-add-bulk`** incremental patch in `incPatchPlan` / `_patchAddSlotsBulk` — O(k) paste ≤10k. Dormant 512m chunk scaffolding. **T-067.0.1:** CPU viewport cull reverted — `getBaseIcons()` @ ~160 fps pan @ 367k. Follow-ons **T-111** (lazy RAM) + **T-112** (GPU cull) in registry `idea`. Spec: [`t067_spatial_chunks.md`](docs/specs/Mission_Creator_Architecture/t067_spatial_chunks.md) @ `d2128cf`.
- T-066 **Mission Creator — worker compile offload (T-066.1 `pickMapSnapshot`)**. Save Version + Export compile in `compiler.worker.ts` via Comlink; `pickMapSnapshot(useMapStore.getState())` strips Zustand actions before postMessage (fixes DataCloneError 25 on raw `getState()`). `terminateCompiler()` on mission unmount. Manual @ ~367k: Save **201**. Spec: [`t066_worker_compile.md`](docs/specs/Mission_Creator_Architecture/t066_worker_compile.md).
- T-065 **Mission Creator — cluster/LOD @ extreme zoom**. `supercluster` index (`slotClusterIndex.ts`); pan-stable `getClusterMarkers` full-terrain cache (T-065.2); `ZOOM_CLUSTER_MAX = -4` — default zoom `-2` stays detail @ ~160 fps @ 367k; cluster discs + drill-in only when zoomed out past -4 on missions >500 slots. Spec: [`t065_cluster_lod.md`](docs/specs/Mission_Creator_Architecture/t065_cluster_lod.md).
- T-064 **Mission Creator — virtualized outliner @ ~367k**. `@tanstack/react-virtual` + segment-index flatten (`flattenOutliner.ts`, `VirtualOutliner.tsx`, `TreeRow.tsx`); `virtualSlotIds` + `VIRTUAL_SLOT_THRESHOLD=50`; replaces T-059 `OUTLINER_LEAF_CAP`. **T-064.1:** callback-ref `scrollEl` fixes blank outliner until first map selection. Manual @ ~367k: outliner on first paint; scrollable 367k virtual rows; no tab freeze. Spec: [`t064_virtualized_outliner.md`](docs/specs/Mission_Creator_Architecture/t064_virtualized_outliner.md).
- T-063 **Mission Creator — spatial index for click/marquee pick @ ~367k**. rbush R-tree (`slotSpatialIndex.ts`) kept in sync via `slotIconCache` mutators; `pickNearest` / `pickRect` replace Deck GPU pick; `slot-icons` `pickable: false`; click-select moved to `useSelectTool` pending-left pointerUp. FE build/lint clean; manual @ ~367k: significantly faster click/marquee. Spec: [`t063_spatial_index.md`](docs/specs/Mission_Creator_Architecture/t063_spatial_index.md).
- T-062.1.1 **Mission Creator — Save orbat payload dedup**. Save Version omits duplicate `orbat[]`
  (editor-only POST); Go `services.ParseOrbatTemplate` derives ORBAT from `editor` for Event attach.
  Export keeps full superset. `make test-it` + FE build/lint clean. Spec:
  [`t062_1_1_batch_save.md`](docs/specs/Mission_Creator_Architecture/t062_1_1_batch_save.md).
- T-062.1 **Mission Creator — chunked IDB slot restore @ 360k**. v2 persistence (`tbd-mission-persist`):
  meta JSON + 5k slot chunks via `idb`; v2 boot skips y-indexeddb; one-time v1→v2 migration deletes
  legacy `tbd-mission-${id}`. Determinate restoring progress (no 0→300k jump on 2nd+ load). Debounced
  persist on `LOCAL_ORIGIN`; flush on tab hide/pagehide. Manual verify @ ~360k: good enough.
  Spec: [`t062_1_idb_streaming_load.md`](docs/specs/Mission_Creator_Architecture/t062_1_idb_streaming_load.md).
- T-062.2 **Mission Creator — editor session / background-tab resilience**. Dev: `viteReloadGuard`
  blocks Vite HMR full reload on `/missions/:id/edit` (alt-tab WS reconnect). Warm session:
  `editorSession.ts` + `sessionStorage` marker → skip multi-MB `GET /missions/:id` on same-tab
  return when IndexedDB has content. Background-safe `yieldToUi` + visibility-aware restore poll.
  Manual verify @ ~360k (Firefox dev): alt-tab extended period → no automatic load overlay.
  Spec: [`t062_2_editor_session_persistence.md`](docs/specs/Mission_Creator_Architecture/t062_2_editor_session_persistence.md).
- T-062 **Mission Creator — incremental bindings @ 360k**. T-062.0: `incPatchPlan.classifyTransaction`
  → O(k) Zustand patches (`slot-fields`, `slot-add`, `slot-remove`, `meta`, `editor-layers`) instead of full
  `docToSnapshot(n)` on everyday edits. T-062.0.1: batched `removeEntities('slots')` (pasteSlots-style detach),
  `slotCount`/`slotsRevision` (no O(n) `slotsById` spread on add/remove), `REMOVE_PATCH_CAP` 10_000. Manual verify
  @ ~360k: delete 4k, undo 6k, asset drop, drag OK. Spec:
  [`t062_incremental_bindings.md`](docs/specs/Mission_Creator_Architecture/t062_incremental_bindings.md).
- T-061 **Mission Creator — drag-move performance @ 360k (good enough)**. T-061.0: dual
  IconLayer + split `dragPreviewIds`/`dragPreviewDelta` + rAF-coalesced delta — sustained
  ~60 fps while dragging @ ~360k (was 5–10 fps). T-061.0.1: `slotIconCache` O(k) exclude/restore
  + bindings `fastSlotPatchIds` slot-position fast path — pickup/release materially improved
  (no ~10 fps release collapse). Build + lint clean. **Product call:** good enough for Eden-blocking
  work; mega optimizations (**T-094** typed-array, release repack collapse, T-066 worker, **T-110**
  terrain) deferred — see MC [`ROADMAP.md`](docs/specs/Mission_Creator_Architecture/ROADMAP.md)
  §Deferred mega optimizations. Spec: [`t061_drag_move_hotfix.md`](docs/specs/Mission_Creator_Architecture/t061_drag_move_hotfix.md).
- T-060 **Mission Creator — fast load + save at scale (API body limit + progress UX)**. Unblocks
  large-mission save/load in the T-059..T-067 scale program (spec:
  `docs/specs/Mission_Creator_Architecture/t060_fast_initial_load.md`). Three blockers fixed:
  **(A) API 1 MB body cap** — `cmd/api/main.go` wrapped **every** JSON route in
  `http.MaxBytesReader(1 MB)`, rejecting a 360k-slot payload (tens–hundreds of MB) before
  `CreateVersion` ran. The limiter moved to `internal/middleware/bodylimit.go`: `GlobalBodyLimit`
  (1 MB default, multipart bumped) now **skips** the versions POST, and that route is registered
  with its own `BodyLimit(cfg.MissionVersionBodyLimit())` — **256 MB** default, override
  `MISSION_VERSION_MAX_BODY_BYTES` (`Config.MissionVersionBodyLimit()` falls back to 256 MB so a
  manually-built config still works). `CreateVersion` maps an over-cap body to **413**
  `{"error":"payload too large (max … MB)"}` (via `*http.MaxBytesError`) instead of a generic 400.
  **(B) Load freeze** — a bulk-sync window in `tactical-map/state/bindings.ts`
  (`beginBulkSync`/`endBulkSync`; the prime + observer defer while open) coalesces the boot
  sequence (IndexedDB replay + seed) into **one** store snapshot; `useMissionDoc` opens it before
  binding/replay and exposes `docStatus: 'loading' | 'ready'`, holding `loading` until the local
  sync **and** the server hydrate settle (`onSynced` now returns its promise). `MissionCreatorPage`
  shows a full-bleed loading overlay (v2: determinate restoring `done/total` T-062.1; legacy v1: indeterminate) and
  **defers the LeftSidebar mount** until ready so the outliner tree isn't built mid-boot.
  **(C) Save hang + hidden errors** — `compiler/compile.ts` gains async `compileMissionWithProgress`
  (chunked, yields every ~5k slots, reports compile %); `useMissionEditor.saveVersion` runs
  Compiling→Uploading phases (axios `onUploadProgress`) and **surfaces real errors** (413 → "too
  large", 409 → semver, else backend `error`) instead of always "Could not save version";
  `TopCommandStrip` renders the phase + progress bar in the Save dialog. Backend + four frontend
  modules + one CSS keyframe. The initial T-060 manual verify @ ~300k failed (load 2–3 min on an
  **indeterminate** bar; save upload → "Could not reach the server"), so **T-060.1** completes
  acceptance (spec: `docs/specs/Mission_Creator_Architecture/t060_1_scale_load_save_completion.md`):
  **(1) determinate load** — `docToSnapshotWithProgress` (chunked snapshot) + `hydrateMissionDocWithProgress`
  (per-chunk INIT_ORIGIN transactions) + `api.get` `onDownloadProgress` feed a `loadProgress`
  {phase,value,label,done,total} threaded to a determinate overlay ("N / M objects"; download 0–0.2,
  apply 0.2–0.5, local 0.5–1.0); **(2) bulk-timing fix** — `endBulkSync` is now async and runs the
  single coalesced snapshot **after** the server hydrate (was firing before → double 300k flush);
  **(3) save upload** — version POST `timeout: 600_000` + `maxBody/maxContentLength: Infinity`, Vite
  `/api` proxy `timeout`/`proxyTimeout: 600_000`, chunked `editor.slots` assembly, and the `!resp`
  catch surfaces axios `code`/`message`. Verified: `make test-it` + FE build/lint clean. **T-060.1.1:** load partial pass @ ~360k. **T-060.1.2 (E1/E2/E3b):** `buildVersionBlob`, preparing, direct `:8080` bypass. **T-060.1.3:** @ 367k, `SaveDebugReport` captured **141,574,630 bytes** compiled, **`direct`** route, failure at **5,573,612 bytes** (~3.9%), `ERR_NETWORK` — failure fully diagnosed (not 256 MB cap, not proxy). **T-060.1.4 (mid-upload reset FIXED):** root cause was the **1 MB `GlobalBodyLimit` cap reaching the version route** (the skip not applying — most likely a **stale `go run` binary**; a clean build's `FullPath()` matches correctly). `http.MaxBytesReader` tripped at 1 MB and reset the socket mid-stream → browser `ERR_NETWORK` at ~5 MB buffered (TCP overshoot past the 1 MB read point = the locked `5,573,612`). Fixes: **`isMissionVersionPOST(c)`** in `bodylimit.go` (FullPath + URL-path fallback); **production-like integration test** (`setupITProd` mounts `GlobalBodyLimit` like `main.go`); `phaseAtFailure='uploading'` on first upload tick; repro `scripts/mission-version-upload-repro.sh`. **Shipped** in `b1fd25a` (2026-06-23): curl **140 MB → 201**; **browser Save @ ~367k / ~142 MB → 201** (semver 0.1.3/0.1.4, `direct → :8080`). **Ops:** restart `make api` after middleware changes — `go run` does not hot-reload.
- T-059 **Mission Creator — bulk paste/delete at scale**. First slice of the
  **T-059..T-067 scale program** toward the **1M–10M editable-entity** north star. Pasting ~10k
  slots hard-froze the tab; three confirmed causes fixed (spec:
  `docs/specs/Mission_Creator_Architecture/t059_bulk_paste_operations.md`): **(a)** `pasteSlots`
  (`tactical-map/state/ydoc.ts`) dropped its per-slot `[...spread, id]` appends to `slotIds` /
  `entityIds` (**O(n²)**) — the loop now accumulates into local arrays (a `Map<squadId, ID[]>`
  seeded once per squad so per-slot squad re-targeting still works, plus one array per target
  layer) and writes each Y.Map **once** at the end (O(n); `index` derived from the accumulating
  length). **(b)** post-paste **selection cap** in `MissionCreatorPage` Ctrl+V branch
  (`BULK_SELECT_CAP = 500`): ≤500 selects the paste (T-056 behavior), above it clears to `none`
  so `selection.ids` never holds 10k ids — OBJ still updates from `slotsById`. **(c)** **outliner
  leaf cap** (`OUTLINER_LEAF_CAP = 500`, T-059 band-aid) rendered count-only labels with no slot
  rows — **superseded by T-064** virtualization (`@tanstack/react-virtual`, `virtualSlotIds`,
  `VIRTUAL_SLOT_THRESHOLD=50`). The conditional **chunked paste + progress**
  and `bindings._bulkMode` (spec items d/e) were **not** needed after the batch fix — the single
  transact + existing one-flush-per-transaction coalescing meets the no-freeze bar; revisit only if
  a manual 10k paste still stalls. Slots only; no schema/compiler/backend change. Four real files
  (`ydoc.ts`, `MissionCreatorPage.tsx`, `EditorLayersSection.tsx`, `OrbatSection.tsx`). Verified:
  frontend build + lint clean (10k no-freeze + ≥55 fps pan after = manual in-browser check).
  **Live validated (2026-06):** repeat **6k-object paste** loops smooth; **360k objects @ 100+ fps**
  pan/zoom. Load/save at scale deferred to **T-060** / **T-060.1** (see §Status T-060 bullet).
- T-058 **Mission Creator — toolbelt entity count readout (OBJ total + SEL selected)**. Scale
  telemetry ahead of the T-059..T-067 scale program: the bottom toolbelt now shows **OBJ** =
  total placed slots and **SEL** = selected slot count, right of the X/Y/Z block (JetBrains Mono,
  `tabular-nums`, plain integers). **OBJ** reads a new memoized `selectSlotCount(slotsById)` in
  `tactical-map/state/selectors.ts` (re-exported from the barrel `index.ts`); **SEL** is
  `selection.ids.length` when `selection.kind==='slot'`, else 0. Both subscribe **inside the
  already-memoized `BottomToolbelt`** (a new `slotsById` selector + the existing selection slice),
  so they update on add/remove/paste/delete/selection but **never on a cursor move** — the T-057
  cursor channel (`useMapStore.cursor`, rAF-throttled) is untouched, as are Deck layers/picking,
  pan coalescing, and `MissionCreatorPage` state. Slots only (vehicles/markers join the count in a
  later **T-069**/**T-070** slice); no schema/compiler/backend change. Two real files + barrel export. Closes
  feature_inventory `BOTTOM-OBJCOUNT-001`. Verified: frontend build + lint clean.
- T-057 **Mission Creator — map performance hotfix (≥55 fps pan/zoom @ 200+ slots)**. Restores
  the engineering-plan perf contract after a regression dropped pan/zoom to **~9 fps at ~100–200
  slots**. Deck.gl already draws the icons on the GPU — the bottleneck was the **React layer**, fixed
  on three fronts (spec: `docs/specs/Mission_Creator_Architecture/t057_map_performance_hotfix.md`):
  **(1) Cursor off the render path** — the toolbelt X/Y/Z read-out is now a transient
  `useMapStore.cursor` slice (set rAF-throttled), so a pointer move no longer re-renders the whole
  `MissionCreatorPage` (both Outliner trees, palette); only `BottomToolbelt` (now reading
  `s.cursor`) re-renders. `MissionCreatorPage` drops its `cursor` `useState`/`cursorRef`; paste reads
  `useMapStore.getState().cursor`. **(2) No hover picking** — `TacticalMap` drops Deck's `onHover`
  (which ran a GPU pick over every icon each move just to feed cursor coords) and computes the
  cursor itself by **unprojecting the mouse** (`view.makeViewport(...).unproject`, same flipY:false
  math as `onDrop`) on the container `onPointerMove`, rAF-throttled, with `onPointerLeave`→`null` for
  the off-map `—`; `getCursor` is now constant (`'crosshair'`) so Deck stops computing `isHovering`.
  Picking is kept **only** for click / dbl-click / marquee / drag-start. **(3) Pan coalesced** —
  `useSelectTool` rAF-coalesces the pan branch so a high-rate mouse can't outrun the display
  (one `setViewState`/frame; flushed on pointer-up). **(4) `React.memo`** on `TacticalMap`,
  `LeftSidebar`, `AssetPalette`, `TopCommandStrip`, `BottomToolbelt`, `AttributesModal`
  (+ `onOpenChange` stabilized) so unrelated page renders can't cascade into the trees/map.
  **Only behavioral change:** the pointer no longer switches to a "pointer" glyph over an icon (no
  per-move hover pick) — ROADMAP-sanctioned. No schema/compiler/backend change; all interactions
  (select, Ctrl-toggle, marquee, drag-move+undo, dbl-click→Attributes, Ctrl+C/V, Space, Delete)
  unchanged. Verified: frontend build + lint clean (fps acceptance is a manual in-browser check
  with 200+ slots via the `FpsCounter`).
- T-056 **Mission Creator — Ctrl+C/V copy-paste (T-056)**. Placed slots can now be
  duplicated: **Ctrl/Cmd+C** snapshots the current slot selection to an in-editor clipboard and
  **Ctrl/Cmd+V** pastes it at the **map cursor**, preserving the group's relative layout
  (translate so the clip's centroid lands at the cursor; mouse off-map → fixed **+20m/+20m**
  nudge from originals). New batched `pasteSlots(md, clip, { anchorAt, layerId })` in
  `tactical-map/state/ydoc.ts` — one `transact()` (one undo step), mirrors `addSlot`: re-attaches
  each copy to its **source squad** (or `ensureDefaultSquad` if it was deleted), files into the
  **active Outliner layer** (or `ensureDefaultLayer`), clamps x/y to terrain bounds, and returns
  the new ids; the paste becomes the selection. New serializable `ClipboardSlot` type
  (`state/schema.ts`). The two keydown branches live in `MissionCreatorPage` next to undo/Space/
  Delete, behind the existing INPUT/SELECT/TEXTAREA/contentEditable guard (so Ctrl+C/V in an
  Attributes field stays **native** text copy/paste); the clipboard + cursor are read via refs
  (`cursorRef` mirrors the live cursor so the `window` keydown listener isn't re-bound on every
  mouse move). Both no-op without `preventDefault` when they can't act. **Scope:** copy+paste,
  slots only — Cut (Ctrl+X) and paste-at-original (Ctrl+Shift+V) stay out. Four files, no
  backend / `useSelectTool` / compiler change. Closes gap_analysis **T-056** (`ACTION-COPY-001` /
  `ACTION-PASTE-001`). Verified: frontend build + lint clean.
- T-055 **Mission Creator — asset browser search (T-055)**. The right palette's
  **Asset Browser** (Factions tab) gets a search field so finding a unit no longer means
  hand-expanding the Faction → Category → Class tree. `RightInspector/AssetBrowser.tsx` filters
  `ASSET_CATALOG` through a recursive `filterCatalog(nodes, q)` — **case-insensitive label
  substring**; a folder is kept on a self-match (→ its full subtree, so "nato" shows all of
  NATO) or on any descendant match (→ only matching children); retained folders are
  force-`defaultExpanded`. Because `TreeView` seeds its expanded set once at mount
  (`collectExpanded`), the tree is **keyed on the query** (`key={query.trim() || 'all'}`) so
  the expand pass re-runs and reveals matches; empty result → "No assets match"; an `X` button
  + **Esc** clear the box. Filtered leaves still drag-to-place (`ASSET_DND_MIME` unchanged).
  Search is scoped to `AssetBrowser` (the only live catalog) — the stub tabs and `TreeView` /
  `ASSET_CATALOG` are untouched; classname-prefix search stays **T-084**
  (`RIGHT-SEARCH-002`). One real file. Closes gap_analysis **T-055** / `RIGHT-SEARCH-001`.
  Verified: frontend build + lint clean.
- T-054 **Mission Creator — Attributes modal entry points (T-054)**. Unifies how the
  **Attributes** modal opens onto one native-`dblclick` contract. **Map (`SEL-MAP-004` harden):**
  `tactical-map/TacticalMap.tsx` drops the hand-rolled 350ms `lastClick` double-click timer in
  `onClick` for a native `onDoubleClick` on the gesture-host container `<div>` that picks the slot
  under the cursor via `deckRef.pickObject({ layerIds: ['slot-icons'] })` (the same pick
  `useSelectTool.onPointerDown` does) → `onEntityActivate`; `onClick` now only selects/toggles.
  **ORBAT (`SEL-ORBAT-DBL-001`):** `OrbatSection` gains an `onActivateSlot` prop (threaded through
  `LeftSidebar`, mirroring `EditorLayersSection`) and passes `onActivate` to its `TreeView`, whose
  existing native `onDoubleClick` on a slot row now opens Attributes. Three-file change — no
  `TreeView`/`MissionCreatorPage`/store change. `MissionCreatorPage.onEntityActivate` keeps its
  `selection.ids.length <= 1` guard, so the **T-053 Ctrl/Cmd toggle** is unchanged (a Ctrl-built
  multi still suppresses dbl-click→Attributes). Closes gap_analysis **T-054** / `SEL-ORBAT-DBL-001`.
  Verified: frontend build + lint clean.
- T-053 **Mission Creator — Ctrl/Cmd+LMB additive (toggle) select (T-053)**. Marquee
  box-select already did multi-select, but a single click on a unit always **replaced** the
  selection — so trimming/extending a multi-selection meant redrawing a marquee. This adds
  modifier-click additive select in the Deck `onClick` of `tactical-map/TacticalMap.tsx`
  (the gesture machine in `useSelectTool` owns only drags; sub-threshold clicks fall through to
  Deck, whose `onClick` 2nd arg is a `MjolnirGestureEvent` carrying `srcEvent.ctrlKey/metaKey`).
  **Ctrl or Cmd** + click a slot **toggles** it in/out of `selection.ids` (removing the last id →
  `none`); **Ctrl/Cmd + empty-click preserves** the selection (only a plain empty click
  deselects). **Shift stays unbound** (reserved for a future range-select); marquee still
  replaces; a Ctrl-built multi (>1) keeps dbl-click→Attributes suppressed. One-file change — no
  store/schema or `useSelectTool` change. Closes gap_analysis **T-053** / `SEL-MOD-001`. Verified:
  frontend build + lint clean.
- T-052 **Mission Creator — undo/redo keyboard shortcuts (T-052)**. The editor toolbar's
  Undo/Redo buttons already drove the `Y.UndoManager`; this adds the matching keyboard shortcuts
  to the host keydown handler in `MissionCreatorPage` (reusing the existing `UndoController` — no
  second stack): **Cmd/Ctrl+Z** undo, **Cmd/Ctrl+Shift+Z** or **Ctrl+Y** redo. Skipped while focus
  is in an `INPUT`/`SELECT`/`TEXTAREA`/contentEditable field (so Ctrl+Z in an Attributes number
  field edits the field, not the map); `preventDefault` on a match, but only drives the stack when
  `canUndo()`/`canRedo()`. Closes gap_analysis **T-052** / `KEY-UNDO-001`. Also fixed a `useMissionDoc`
  React 19 StrictMode lifecycle bug that left undo dead in dev: the setup→cleanup→setup double-invoke
  destroyed the memoized `Y.UndoManager` while `useMemo` returned the same dead instance (`canUndo()`
  always false) — a one-shot `instanceKey` bump on teardown now forces a fresh `md`+`UndoController` so
  dev undo tracks edits. Verified: frontend build + lint clean.
- T-050 **Mission Creator — cursor Z readout**. One-line follow-up to T-049: the bottom
  toolbelt's **CUR** (cursor) mode now shows **X/Y/Z** instead of X/Y with a dimmed `—` for Z.
  The engine `onCursorMove` payload (`tactical-map/types.ts`) gained `z`; `TacticalMap` `onHover`
  emits `z: info.coordinate[2] ?? 0` (Deck.gl unproject); `MissionCreatorPage` cursor state +
  `BottomToolbelt` show it. **Z = 0 on the flat map** (a real ground-plane value, not a placeholder)
  and will carry real elevation once **T-091** DEM feeds z; off-map hover still shows `—` on all
  axes; SEL mode unchanged. Verified: frontend build + lint clean.
- T-049 **Mission Creator — terrain, title, numeric position (T-049)**. Code-only
  editor slice (no **T-090** tiles / **T-091** DEM / **T-068** registry). **Terrain:** `MissionCreatorPage`
  reads `meta.terrain` and passes it to `<TacticalMap key={terrainId} terrain={terrainId}>`
  (the `key` remounts the viewport so the camera + base grid resize to Everon 12800 vs Arland
  4096). **Title hydrate:** new `applyMissionRowMeta` (INIT_ORIGIN) in `tactical-map`
  `state/ydoc.ts` sets `meta.title`/`terrain`/`environment` from the `GET /missions/:id`
  row; `useMissionEditor.onSynced` was rewritten so it **no longer early-returns when
  `json_payload` is `{}`** (the bug that left every freshly-created mission on "Untitled
  Mission"/Everon) — empty payload → apply row meta; non-empty → hydrate then re-apply row
  title; conflict "load server" re-applies the cached row meta. Hydrate-only (no `PATCH`;
  Save Version still compiles payload). **Numeric transform:** new `updateSlotPosition`
  (x/y clamped to terrain bounds, rotation normalized 0–360, one undo step per commit) +
  a mono `NumberField` (blur/Enter commit, no effects) make the Attributes **Transform** tab
  X/Y/Z/rotation editable (replacing the read-only fields + stale "coming later" copy). The
  **bottom toolbelt** is now selection-aware: single selected slot → `SEL` X/Y/Z, otherwise
  `CUR` cursor X/Y. `MissionDetail.current_version` type gained `json_payload?`. No backend
  changes. Verified: frontend build + lint clean.
- T-048 **Mission create from Library (macOS Dialog)** — the standalone `/missions/create`
  full-page wizard is replaced by a transient `CreateMissionDialog`
  (`apps/website/frontend/src/features/mission-creator/CreateMissionDialog.tsx`) launched from the Mission
  Library: a **New Mission** header button + a **My Missions** true-empty-state CTA + **Cmd/Ctrl+N**,
  all `mission_maker+` only (enlisted see nothing). Opening create closes the dossier Sheet first
  (one overlay at a time); the form resets on every close. The `/missions/create` route, the
  `MissionCreatorPage` wizard export in `pages/missions.tsx`, the sidebar nav item, and the
  stitch-map entry are removed. **Mission Creator** naming stays on the dossier CTA
  (`OPEN IN MISSION CREATOR`) and the `/missions/:id/edit` breadcrumb — only the wizard tab went
  away. `POST /missions` unchanged. Verified: frontend build + lint clean.
- T-047 **Doc authority alignment** — `agent_execution.md` Decisions log + agent rules now point agents at **`ROADMAP.md`** for open work and state the shell phases (PRE-3.5–9) are complete (T-033–T-040), replacing the old strict-phase-order / `00`–`09` numbered shorthand; `eden/wiki_manifest.yaml` deduped (`Eden_Editor:_Scenario_Attributes` was listed twice → 28 unique pages). (T-046 was the link-integrity pass: stale numbered cross-refs + relative link depths.)
- T-045 **Roadmap-centric naming** — each domain gets **`ROADMAP.md`** (FE, BE, Mission Creator); MC docs renamed to descriptive names (`engineering_plan.md`, `agent_execution.md`, …); stubs at old numbered paths.
- T-043 **Platform documentation reorg** — [`docs/website/README.md`](docs/website/README.md) hub with
  frontend/backend/archive master indexes; platform docs moved to `docs/platform/` and
  `docs/backend/architecture.md`; Mission Creator corpus reorg (`eden/`, `reference/`);
  FD vs T split retired in **T-043**; T-0xx-only contract in [`docs/website/TAGS.md`](docs/website/TAGS.md); frontend
  surface specs refreshed (SplitPane events, mission editor route, §Documentation rule here).
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
  `.bg-grid-overlay` utilities. New reusable primitives in `apps/website/frontend/src/components/ui/`
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
- T-018..T-025 **Global Aegis consistency refactor** (presentation-only; no API/query/
  contract changes). A platform-wide audit collapsed the remaining inconsistencies into
  four systemic defects, fixed across eight build/lint-verified commits:
  - **R3 mono telemetry** (T-018, T-022, T-023): player-count heroes (Dashboard, Server
    Intel, Server Control) and ORBAT slot counts now render in JetBrains Mono.
  - **R4 token sweep** (T-020, T-023, T-025): all off-palette vivid `blue/slate/red/amber`
    replaced with Aegis tokens — active/selection → `primary`, CTAs → `action`, body →
    `on-surface-variant`, markdown callouts → `error`/`tactical-yellow`/`primary`. The
    `white`/`black`-opacity utilities are kept (shared glass vocabulary, used on the
    reference-clean pages too); the leaderboard silver-podium tint is intentional.
  - **R5 shared primitives** (T-019, T-025): the mission dossier moved off a hand-rolled
    `DialogPrimitive` onto the shared `Sheet` (new `bleed` edge-to-edge mode +
    `SheetTitle`/`SheetDescription` exports).
  - **R1/R2 full-bleed + SplitPane** (T-021, T-022, T-023, T-025): Modpacks, Wiki, Vehicle
    Database, Server Control and Comms Broadcaster migrated to the shared `SplitPane` (via a
    `GlassSplit`/`SidebarSearch` helper pair in `doctrine.tsx`); Mortar Calculator and Event
    Hub converted to full-bleed (routes gained the `fullBleed` handle).
  - **R6 anti-pattern** (T-024): Event Manager's always-on form-beside-calendar replaced by
    a calendar + per-day op list, with create moved into a frosted `Dialog`.
  - Deliberately left as-is (not master/detail; `SplitPane` would degrade them): the
    embedded `OrbatSelector` card widget, the Deployments service-record dossier, and the
    wide-table Personnel roster (token-fixed only).
  - Verified: `npm run build` + `npm run lint` clean after every commit. Runtime layout of
    the migrated split-pane/full-bleed pages is worth an in-browser pass (`make web`).

- T-029..T-040 **2D Mission Creator — Deck.gl editor (Eden editor shipped; phases 2/5/6/8 blocked)**. New self-contained
  feature modules `apps/website/frontend/src/features/tactical-map/` (terrain-agnostic engine) +
  `apps/website/frontend/src/features/mission-creator/` (editor wrapper), code-split lazy route
  `/missions/:id/edit` (mission_maker+, `fullBleed`). Execution authority is
  `docs/specs/Mission_Creator_Architecture/ROADMAP.md` (T-0xx backlog) +
  `agent_execution.md` (Eden UX Decisions log); `engineering_plan.md` remains
  authoritative for the data model / workers / compiler / DEM. New deps:
  `deck.gl @deck.gl/core /layers /react @luma.gl/core yjs y-indexeddb comlink idb`.
  - **T-029 Phase 0/1 — core viewport:** `<TacticalMap>` = `<DeckGL>` `OrthographicView`
    + `COORDINATE_SYSTEM.CARTESIAN` (flat Arma meters, `flipY:false` → north-up, identity
    projection). Self-contained vector grid base map (`LineLayer`, no tiles), clamped
    pan/zoom, `FpsCounter` debug HUD. `coords/terrains.ts` per-terrain bounds (Everon
    12.8km²).
  - **T-030 Phase 4 — state foundation:** `state/` is the Y.Doc-backed normalized store
    (source of truth) mirrored into Zustand (`useMapStore`) via `bindings.ts` `observeDeep`;
    `ydoc.ts` actions wrap `transact(...LOCAL_ORIGIN)`; `undo.ts` = `Y.UndoManager`;
    **T-062.1 v2 persistence** (`tbd-mission-persist` via `idb`: meta JSON + 5k slot chunks;
    legacy v1 y-indexeddb migrate-once path only) in `hooks/useMissionDoc.ts` +
    `persistence/*`. Entities render through a GPU `IconLayer` (the 200-slot
    answer). `schema.ts` = the §2 entity model.
  - **T-031 Phase 3 — Aegis-glass shell:** full-bleed map (`z-0`) under a
    `pointer-events-none` overlay of `pointer-events-auto` frosted panels (shared
    `layout/overlay.ts` recipe — more transparent than `.glass`, Aegis tokens not slate).
    Top Command Strip (title, Undo/Redo, gear→`MissionSettingsDialog`, disabled Export),
    Bottom Toolbelt (Select/Ruler/LoS + mono X/Y/Z), Inspector (`SlotInspector`).
  - **T-032 Phase 3 UI overhaul — Eden Editor tree paradigm:** reusable recursive
    `layout/tree/TreeView.tsx`; Left Outliner + Right Asset Browser; `AttributesModal`
    on double-click. **Subsequently wired to Y.Doc in T-033–T-037** (mock data removed).
  - **T-033 PRE-3.5 — wire Outliner + asset drag-to-map to the Y.Doc:** new `editorLayers`
    entity map (the 10th) = workflow-only Outliner folders (`parentId` nesting, `entityIds`),
    threaded through `schema`/`ydoc`/`bindings`/`useMapStore` (`activeLayerId` = drop target).
    Left "Placed Entities" tree now reads the live Y.Doc (`buildTree`); select→`flyTo`,
    folder→active layer, "+"→`addEditorLayer` (`placedEntitiesMock` deleted). Asset Browser
    leaves are draggable; dropping one on `<TacticalMap>` unprojects the cursor and
    `addSlot`s under the active layer (`ASSET_DND_MIME`/`AssetDropPayload`,
    `onDragOver`/`onDrop`, `onAssetDrop`). **Still mock/deferred:** reparent DnD, `assetId`
    persistence, and the always-on Asset Palette (right panel still swaps to `InspectorPanel`).
  - **T-034 DOC-0 — doc alignment:** created `04_eden_ux_spec.md` (UX contract), tracked
    `05_agent_execution_plan.md` (execution authority), patched `03_engineering_ultra_plan.md` +
    `mission_creator_design.md` to the Eden docked-shell UX. Docs-only.
  - **T-035 Phase 3.5 — Eden docked shell:** fullscreen via an `AppLayout` `chromeless` route
    handle (no platform `Sidebar`/`TopNav` on `/missions/:id/edit`); left `w-64` + right `w-80`
    panels docked flush, map full-bleed behind (`overlayDocked` recipe). Top strip = menu stubs +
    Eden time scrubber/weather + undo/redo + settings + Export. Left = ORBAT + Editor Layers
    sections (`LeftSidebar`/`OrbatSection`/`EditorLayersSection`). Right = always-on `AssetPalette`
    (tabs; removed the `InspectorPanel`→`SlotInspector` swap). `AttributesModal` editable
    (Transform/Identity/States/Arsenal, role/tag/stance via `updateSlot`). Spacebar centers; one
    Deck.gl base grid over a flat `bg-background`.
  - **T-036 Phase 7b — map drag + multi-select:** `Selection` is now `{ kind, ids[] }`. New
    `tools/useSelectTool.ts` pointer state machine (Deck `dragPan` off): left-drag icon = move
    (transient preview → one `moveEntities` transact on release), left-drag empty = marquee
    (`layers/useSelectionLayer.ts` + `slotSpatialIndex.pickRect`), middle/right-drag = pan. `ydoc`
    `moveEntities`/`removeEntities` (atomic group ops). Removed click-to-teleport; Delete/Backspace
    removes selection; Spacebar centers on the selection centroid.
  - **T-037 Phase 7a — outliner tree ops:** `ydoc` `renameEditorLayer`/`reparentEditorLayer`
    (cycle-guarded)/`moveSlotToLayer` + **destructive** `removeEditorLayer` (deletes a folder's
    whole subtree, one transact, keeps ≥1 layer). `TreeView` gains opt-in DnD (data-driven
    `isFolder` so **empty** folders are drop targets), inline rename, hover row actions.
    `EditorLayersSection` wires reparent/refile/rename/delete + a "Move folder to root" dropzone.
    `Slot.assetId` persisted from the palette drop.
  - **T-038 Phase 9 — compiler + persistence:** `compiler/compile.ts` → `json_payload` superset
    (backend-compatible `orbat[]` + an editor-only `editor` block for lossless reload);
    `compiler/exportSchema.ts` camelCase mod envelope; `ydoc.hydrateMissionDoc`; `useMissionEditor`
    (load current version, conflict prompt, dirty tracking, **manual Save Version** → POST, Export
    download). Autosave stays **local** (v2 `idb` chunk store + legacy migration — T-062.1; server
    versions API is immutable (unique semver, no overwrite). Live-verified: POST 201, dup semver 409, ORBAT round-trips.
  - **T-039 / T-040 — wiring fixes:** Save Version surfaces the backend `response.data.error` +
    an invalid-mission-id banner (T-039); the `/missions/create` wizard now sends `max_players`,
    uses the real weather enums, and navigates to `/missions/:id/edit` (T-040).

**Next (Eden — after T-068 ship @ T-068.14):**
- **T-069** — markers on map
- **T-070** — vehicles placeable
- **T-110** — terrain base + sparse deltas for millions of map props ([`t110_terrain_base_mission_layers.md`](docs/specs/Mission_Creator_Architecture/t110_terrain_base_mission_layers.md))

**Active map program (blocks T-071 + T-068 Phase 2):**
- **T-090** / **T-091** — aligned map tiles + DEM / Z-axis — hub [`t090_091_map_terrain_program.md`](docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md). **T-091 shipped** @ `dde589e`; **T-090.3.0** Workbench spike active; **T-090.1** tiles **queued** until 3.0.
- **T-092** — mod compile + spawn Y/yaw — [`t092_spawn_transform_program.md`](docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md)
- **T-071** — ORBAT Manager modal (queued, after T-092)
- Ruler/LoS/viewshed — after **T-091** heightmap phase.
- Real Discord OAuth credentials are blank in `.env` (dev uses dev-login).
- Telemetry is ingested via service-token endpoints; no live game-server bridge wired.
- A fresh DB is empty of content (events, missions, etc.) — seed those via the API
  or `psql`. The one committed seed is the Discord role→permission mappings
  (`internal/db/seeds/discord_roles.sql`, applied with `make seed`).
  `internal/db/seeds/mock_data.sql` (Operation Red Dawn etc., four fixed UUIDs) is **not**
  run by `make seed` — only by the explicit `go run ./cmd/seed`. DEV_RUNBOOK.md has the
  DELETE SQL to purge those mock missions if they leak into the live library.

## Verifying changes
Source of truth for the API contract is the Go handlers + `internal/models` tags;
frontend types yield to Go on conflict. To check a wire change for real, run the stack,
`dev-login`, hit the endpoint, and confirm the JSON matches the TS type — `tsc` alone
only proves the frontend is self-consistent, not that it matches the backend.

**Platform CI replay (T-125):** `make db-up` → `nvm use` → **`make ci-local`** (mirrors
[`ci.yml`](.github/workflows/ci.yml): **verify-editorconfig**, gofmt, CI-1, golangci, build, test-it,
verify-coding-standards, **format:check**, FE lint/build/test, schema validate, citations). See
[`CODING_STANDARDS.md`](docs/platform/CODING_STANDARDS.md) §11.
