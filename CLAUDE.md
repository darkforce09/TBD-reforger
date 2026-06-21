# CLAUDE.md — TBD Reforger Platform

Working context for AI sessions. Read this first; it is the source of truth for
**current state and how to run things**. Design specs live under [`docs/`](docs/README.md)
(`docs/platform/context_handoff.md`, `docs/backend/architecture.md`) — verify against
live code for post-T-008 behavior.

## What this is
A web suite for the "TBD" Arma Reforger milsim community: Discord auth, event /
ORBAT scheduling, a mission library (2D editor payloads), server telemetry +
leaderboards, doctrine wiki, CMS, and admin tooling.

- **Backend:** Go (Gin + GORM), PostgreSQL. Module `github.com/tbd-milsim/reforger-backend`, Go 1.25.
- **Frontend:** React 19 + TypeScript + Vite, TanStack Query, Zustand, Tailwind. Node 20. In `frontend/`.
- **Auth:** Discord OAuth2 → JWT access token + rotating single-use refresh token.

## Repo layout
- `cmd/api/` — API entrypoint (loads `.env`, runs migrations on boot, serves `/api/v1`).
- `internal/handlers/` — HTTP handlers, one file per resource (auth, missions, events, telemetry, admin, …).
- `internal/models/` — GORM models; **JSON field names (snake_case) here are the API contract**.
- `internal/db/migrations/` — SQL run before AutoMigrate (extensions, enums, indexes, leaderboard MV).
- `internal/services/`, `internal/middleware/`, `internal/realtime/` (SSE hub).
- `frontend/src/` — `api/` (axios client + single-flight refresh), `hooks/` (queries.ts, mutations.ts, useAuthBootstrap), `pages/`, `components/`, `store/useAuthStore.ts`, `types/` (hand-written API types).

## Run it locally
Everything is configured in `.env` (`APP_ENV=development`, DB on port 5434, `FRONTEND_URL=http://localhost:5173`). Go lives at `/var/home/Samuel/.local/go/bin` (not on PATH).

```bash
make db-up        # start local Postgres (podman/docker compose), port 5434
make api          # run Go API on :8080 (migrates on boot)
make web          # run Vite dev server on :5173 (proxies /api -> :8080)
make test-it      # Go integration tests (needs db-up; sets TEST_DATABASE_URL)
make db-down      # stop Postgres (keeps volume)
```

Frontend checks: `cd frontend && npm run build` (tsc + vite), `npm run lint`.

### Dev login (no Discord needed)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=admin|mission_maker|enlisted`.
It mints a real session and 302-redirects to the SPA callback exactly like Discord —
open it in the browser to log in, or curl it and read `access_token` from the
`Location` fragment for API testing.

## Conventions
- API JSON is **snake_case** (from GORM struct tags). Frontend `types/` are hand-written
  to match — when changing a model, update the matching TS type. The mission **export**
  JSON (`/missions/:id/export`) is the one camelCase exception.
- List endpoints return `{data, total, limit, offset}` (audit logs use a `next_cursor`).
- Auth tiers: public, `RequireAuth` (JWT), `RequireMinRole(admin|mission_maker)`,
  `RequireServiceToken` (`X-Service-Token`, for game-server ingest).
- Refresh tokens are **single-use** (rotated + revoked each call). All refreshes go
  through one single-flight helper (`frontend/src/api/refresh.ts`) so the token is
  never double-spent.
- Git: **commit directly to `main`; never create a branch.** End commit messages with
  the `Co-Authored-By` trailer. Commits are tagged `T-00x`.
- Docs: see **§Documentation** — sync before commit. Frontend deferred work uses
  **FD-0xx** — see [`docs/TAGS.md`](docs/TAGS.md).

## Documentation

Keep docs in sync **in the same commit** as the code change (or immediately before — never merge stale docs).

**Before every T-0xx commit, check what changed:**

| Change type | Update |
|-------------|--------|
| Shipped feature / milestone | **§Status** — new T-0xx bullet; bump "latest feature work" line |
| New/changed route | Matching `frontend/docs/pages/*.md` + row in `frontend/docs/INDEX.md`; verify against `frontend/src/router.tsx` |
| UI surface (no new route) | Relevant page doc + `Live source:` path to `frontend/src/pages/` or `features/` |
| API / model change | `internal/models/` tags + matching `frontend/src/types/`; note handler if behavior changed |
| Mission Creator | MC README, `05` Decisions log, and/or `06` inventory — only if editor contract or Eden parity changed |
| Frontend deferred work | **FD-0xx** in `frontend/docs/TRACKING.md` — never reuse T-0xx for deferred items |

**Doc hub:** [`docs/README.md`](docs/README.md) → [`docs/frontend/README.md`](docs/frontend/README.md) / [`docs/backend/README.md`](docs/backend/README.md). Tag glossary: [`docs/TAGS.md`](docs/TAGS.md).

**Do not update** blueprint HTML, stitch exports, or mock-up HTML — archive tier only. Live UI = `frontend/src/pages` + `features/`.

**Doc-only commits** (reorgs, typo fixes) get their own T-0xx tag and a §Status note if structure or authority changed.

## Status (latest feature work: T-043 — 2026-06-20)
T-005..T-007 between T-004 and T-008 are documentation/seed only; the status below is current.

**Done:**
- T-043 **Platform documentation reorg** — [`docs/README.md`](docs/README.md) hub with
  frontend/backend/archive master indexes; platform docs moved to `docs/platform/` and
  `docs/backend/architecture.md`; Mission Creator corpus reorg (`eden/`, `reference/`,
  `02_roadmap.md`); FD-0xx vs T-0xx split in [`docs/TAGS.md`](docs/TAGS.md); frontend
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
  `.bg-grid-overlay` utilities. New reusable primitives in `frontend/src/components/ui/`
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
  feature modules `frontend/src/features/tactical-map/` (terrain-agnostic engine) +
  `frontend/src/features/mission-creator/` (editor wrapper), code-split lazy route
  `/missions/:id/edit` (mission_maker+, `fullBleed`). Execution authority is
  `Design_Docs/Mission_Creator_Architecture/05_agent_execution_plan.md` (Eden docked-shell UX +
  Decisions log; `04_eden_ux_spec.md` restates it); `03_engineering_ultra_plan.md` remains
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
    `y-indexeddb` persistence (per-mission, keyed `tbd-mission-<id>`, via
    `hooks/useMissionDoc.ts`). Entities render through a GPU `IconLayer` (the 200-slot
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
    (`layers/useSelectionLayer.ts` + GPU `pickObjects`), middle/right-drag = pan. `ydoc`
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
    download). Autosave stays **local** (y-indexeddb) — the versions API is immutable (unique
    semver, no overwrite). Live-verified: POST 201, dup semver 409, ORBAT round-trips.
  - **T-039 / T-040 — wiring fixes:** Save Version surfaces the backend `response.data.error` +
    an invalid-mission-id banner (T-039); the `/missions/create` wizard now sends `max_players`,
    uses the real weather enums, and navigates to `/missions/:id/edit` (T-040).

**Not yet built / next (Mission Creator):** the Eden editor (phases 3.5/7b/7a/9) is shipped;
the remaining phases are blocked on external assets/backend.
- Phase 2 **DEM / Z-axis** — blocked on hosted heightmap assets (see Ultra Plan §0.3).
- Phase 5/6 **Asset Registry worker + Arsenal** (needs backend `GET /api/v1/registry`); until then
  the Asset Palette catalog + slot `loadout` export are mock/empty.
- Phase 8 **tools & objectives** (ruler/LoS/viewshed GLSL; needs DEM for LoS).
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
