# T-092.2 — Verify log (mod compile flatten + /compiled API)

**Branch:** `ticket/T-092` (worktree `.ai/artifacts/worktrees/TBD-T-092`)
**Date:** 2026-07-03
**Status:** automated + live-API gates PASS · Workbench/dedicated-server gates **PENDING** (same
coordinated window as T-092.1 — Workbench held by Stream A / T-090.1.1).

## Automated

| Gate | Command | Result |
|------|---------|--------|
| S1 goldens | `cd packages/tbd-schema && npm run validate` | **PASS** — "All contracts valid" |
| S1 compile output | `TestGetCompiledMission` validates the served body against the **embedded** `mission.schema.json` (`contract.ValidateMissionDocument`) | **PASS** |
| Go tests | `go test ./internal/...` with `TEST_DATABASE_URL` (= `make test-it` scope) | **PASS** (handlers 5.8s incl. new integration test) |
| FE build | `npm run build` (tsc + vite) | **PASS** |
| FE lint | `npm run lint` | **PASS** |
| FE tests | `npm test` (vitest) | **49/49** — incl. 6 new `flattenModDocument.test.ts` |
| Go build/vet | `go build ./...`, `go vet ./...` | **PASS** |

## Live API smoke (worktree API @ :8081, dev DB @ :5434)

| ID | Step | Result |
|----|------|--------|
| M1 | dev-login mission_maker → `POST /missions` → `POST /missions/:id/versions` (editor payload, 4 slots) | **201** (semver 0.1.1 — CreateMission auto-seeds an empty 0.1.0, so 0.1.0 correctly 409s) |
| M2 | `curl -H "X-Service-Token: …" /api/v1/missions/{id}/compiled` | **200**, `slots[]` length 4 == editor slot count |
| S4 | Body shape | canonical document (NOT buildMissionDoc wrapper); `schemaVersion "1.2"`; live body **passes** `validate-file.mjs` against mission.schema.json |
| — | No token | **401** |
| — | Empty `{}` version (fresh-mission state) | **409** `mission version has no placed slots to compile` (deliberate — never an invalid body) |

Smoke body highlights: ids `blufor:Alpha:SL:0 / TL:0 / TL:1 / opfor:Grom:RFL:0`; kits
`kit:us_sl` (mapped from `Character_US_GL.et`), fallback `kit:us_rifleman` (unmapped assetId),
`kit:sov_rifleman`; slot y `142.5` + heading `450 → 90`; synthesized
`z_spawn_blufor (4835.8, 6625.4, r150)` / `z_spawn_opfor (6010, 7211.5, r150)`;
orbat instances 4 == slots 4 (loader parity gate); `meta.id msn_a7531a73…`, `templateId editor_v1`,
`playerRange [1, 64]`.

## Kit-alias mapping table (spec deliverable)

Source of truth: `packages/tbd-schema/registry/kit-aliases.json` (inverse of the `kit:` rows in
`apps/mod/tbd-framework/Data/registry.json`; mirrored by `make schema-codegen` into
`internal/contract/registry/` for go:embed + `frontend/src/types/contract/` for the TS flatten).

| ResourceName (palette assetId) | kit alias |
|---|---|
| `{26A9756790131354}…Character_US_Rifleman.et` | `kit:us_rifleman` |
| `{84029128FA6F6BB9}…Character_US_GL.et` | `kit:us_sl` (POC stand-in) |
| `{0B3167BB0FB68110}…Character_US_PL.et` | `kit:us_tl` |
| `{5B1996C05B1E51A4}…Character_US_AR.et` | `kit:us_ar` |
| `{DCB41B3746FDD1BE}…Character_USSR_Rifleman.et` | `kit:sov_rifleman` |
| `{5436629450D8387A}…Character_USSR_SL.et` | `kit:sov_sl` |
| `{23ADBBC31B6A3DC6}…Character_USSR_AR.et` | `kit:sov_ar` |

Fallbacks: unmapped/absent assetId → faction default (`blufor→kit:us_rifleman`,
`opfor→kit:sov_rifleman`); presets `blufor→preset:us_army_82nd`, `opfor→preset:sov_vdv`.
Unmapped palette characters today: US Medic / MG / LAT / Engineer (fall back with a console warn).

## PENDING — Workbench / dedicated server (coordinated window, with T-092.1 gates)

| ID | Step | Result |
|----|------|--------|
| S2 | Mod loads compiled JSON (REST or profile cache); spawn points built (`[TBD] SpawnManager: built slot spawn` × N) | **PENDING** |
| S3 | headingDeg ±5° @ 3 anchors | **PENDING** |
| S6 | Editor save → /compiled → mod spawn @ bridgehead ±2 m horizontal | **PENDING** |
| M3 | Dedicated/wb_play server load log | **PENDING** |
| M4 | Deploy test player within 2 m of editor x/z | **PENDING** |

Recipe: point `TBD_BackendConfig.json` at a running worktree API (`http://127.0.0.1:8080`,
serverToken = `SERVICE_TOKEN`) — the loader now fetches `/api/v1/missions/{id}/compiled` with
`X-Service-Token` (both fixed this slice) and caches to `$profile:missions/{id}.json`; then run
`bash scripts/mod/tbd-spawn-verify.sh` and read the `[TBD][Spawn]` + deployed-transform lines
(instrumentation shipped in T-092.1).

## Shipped code (this slice)

- `packages/tbd-schema/registry/kit-aliases.json` (new) + codegen mirror step (`codegen.mjs`).
- `flattenModDocument.ts` (new) — client-side flatten + 6 vitest specs; `vitest.config.ts` gains
  the tsconfig-paths resolver.
- `internal/contract/mission.go` (new) — embedded `mission.schema.json` validator +
  kit-aliases accessor; codegen `embedSchemas` += mission.schema.json.
- `internal/services/mission_compile.go` (new) — `FlattenToModDocument` (Go twin, same rules).
- `internal/handlers/missions_compiled.go` (new) — `GetCompiledMission`; route registered on a
  `RequireServiceToken` group in `handlers.go`; curl example in the handler Godoc (DEV_RUNBOOK
  sync = Cursor).
- `TBD_MissionLoader.c` — path `/api/missions/…` → **`/api/v1/missions/…/compiled`** and header
  `Authorization: Bearer` → **`X-Service-Token`** (backend never read the bearer on this tier —
  the REST fetch could never have authed); `backend.example.json` placeholder now names
  `SERVICE_TOKEN`.
