# T-092.2 — Verify log (mod compile flatten + /compiled API)

**Branch:** `ticket/T-092` (worktree `.ai/artifacts/worktrees/TBD-T-092`) → merged to `main` @ `a73224f2`
**Date:** 2026-07-03 · **post-merge verify 2026-07-04 on `main`**
**Status:** **ALL GATES PASS** — automated + live-API re-run on `main`, and the Workbench REST E2E
(S2/S3/S6/M3/M4) ran in the 2026-07-04 coordinated window via wb_play (see §Workbench below;
M4 carries one out-of-scope harness caveat, OBS-1).

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

**Post-merge re-run on `main` @ `a73224f2` (2026-07-04):** S1 goldens **PASS**; `make test-it`
**PASS** (handlers 5.9 s incl. `TestGetCompiledMission`); FE build + lint **PASS**; FE tests **53/53**
(main superset of the worktree's 49). Live smoke repeated against `make api` @ :8080: mission
`ea60cf5b-…` + version 0.1.1 → **201**; `/compiled` **200** with token, **401** without, empty-version
mission → **409** `no placed slots`; body `schemaVersion "1.2"`, 4 slots, ids/kits/y/heading/zones as
below; **passes `scripts/validate-file.mjs`** against `mission.schema.json`.

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

## Workbench REST E2E (PASS — 2026-07-04 post-merge window, `main` @ `a73224f2`)

Ran per the recipe: `TBD_BackendConfig.json` → `http://127.0.0.1:8080`, `serverToken` =
`.env SERVICE_TOKEN` (the profile config had a stale 31-char placeholder token — would 401),
`missionId` = live smoke mission `ea60cf5b-31c4-465c-a04f-dea7a6935b34` (created via
dev-login → POST /missions → POST versions 0.1.1 with the 4-slot fixture payload). Fresh
Workbench start per config change (loader statics — see T-092.1 log ops note). WB log
`logs_2026-07-04_00-06-19`.

| ID | Step | Result |
|----|------|--------|
| S2 | Mod loads compiled JSON (REST or profile cache); spawn points built | **PASS** — `[TBD] Fetching mission ea60cf5b-… from http://127.0.0.1:8080/api/v1/missions/ea60cf5b-…/compiled` → `[TBD] Mission loaded from backend: T-092 post-merge smoke` → 4× `built slot spawn`; cache written to `$profile:missions/ea60cf5b-….json` (removed after verify) |
| S3 | headingDeg ±5° @ 3 anchors | **PASS** — 4 anchors: SL:0 `heading=270`, TL:0 `heading=90` (**450 → 90 normalization through the full pipeline**), TL:1 `heading=0`, RFL:0 `heading=90`; deployed at the heading-90 point: `yaw=-89.9984` (engine CCW ≡ compass 090°, ±0.002°) |
| S6 | Editor save → /compiled → mod spawn @ bridgehead ±2 m horizontal | **PASS** — spawn points at **exact** editor x/z: `<4839.2, 121.787, 6620.8>`, `<4836.9, 142.5, 6626.5>` (**explicit y=142.5 passthrough**), `<4831.2, 123.6, 6628.8>`, `<6010, 215.453, 7211.5>`; `MAX_Y_DELTA_M` warn fired as designed on the synthetic 142.5 (19.18 m vs surfaceY 123.323) |
| M3 | Dedicated/wb_play server load log | **PASS** — fetch → build → `[TBD] Stage → LOBBY` (wb_play; dedicated-server run not required for this gate) |
| M4 | Deploy test player within 2 m of editor x/z | **PASS w/ caveat (OBS-1)** — deployed `pos=<6009.13, 215.718, 7211>` = **1.0 m** horizontal from the opfor slot's editor x/z (6010, 7211.5); `groundDelta=-0.0029 m`, `yaw=-89.9984` matches that point's heading 90. Caveat: player had been **assigned** `blufor:Alpha:SL:0` (`assigned slot … to player 1 at (4839.2,6620.8)` + `spawn requested … kit kit:us_sl`) but the respawn system deployed at the **opfor** spawn point — see OBS-1 |

### Observations (out of T-092 scope, for follow-up)

- **OBS-1 — assigned-slot ↔ deploy-point binding:** the dev roster harness is round-robin only
  (`RosterLoader: fetch failed — round-robin slots only`); slot assignment + spawn request carry the
  right slot/kit, but the SCR respawn point selection deployed the test player at the opfor point
  instead of the assigned blufor slot's point. Spawn-point **transforms** (T-092 surface) are proven
  correct at all 4 anchors; slot→player binding is the LOBBY/ORBAT program's surface (**T-068.13 /
  T-071**), where this belongs.
- **OBS-2 — `TBD_MissionList` still on the legacy path:** `[TBD] MissionList: fetching
  http://127.0.0.1:8080/api/missions` → 404 (`MissionList: fetch failed`). T-092.2 fixed the mission
  **loader** path only; the mission-browser list component still uses pre-v1 `/api/missions` and
  needs the same `/api/v1` + `X-Service-Token` treatment in a future slice.

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
