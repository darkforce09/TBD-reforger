# T-090.5.1 — Map Engine v2 render spine scaffold — verify log

**Date:** 2026-07-05 · **Executor:** claude-code · **Slice spec:** `docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md` · **Plan:** `.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md` §4.1–4.3, §7 row T-090.5.1

## Scope shipped

Scaffold only — zero Deck world-object layers, zero chunk fetch, export pipeline untouched.

| # | Deliverable | Path |
|---|---|---|
| 1 | Style modes (pure) | `features/tactical-map/worldmap/styleModes.ts` (+ test) |
| 2 | LOD gates (pure, N2/N3 data-form) | `worldmap/lodGates.ts` (+ test) |
| 3 | Export-chunk math (pure) | `worldmap/chunkMath.ts` (+ test) |
| 4 | Feature flag `worldmap.enabled` | `worldmap/config.ts` (`VITE_WORLDMAP_ENABLED=1`, default OFF) |
| 5 | Layer assembly stub (always `[]`) | `worldmap/useWorldMapLayers.ts` |
| 6 | World-layer prefs + legacy migration | `state/worldLayerPrefs.ts` (+ test) — key `tbd-mc-world-layers` |
| 7 | basemapView → shim over worldLayerPrefs | `state/basemapView.ts` (exports unchanged; delete @ T-090.10.2) |
| 8 | Worker + client skeleton (Comlink) | `workers/worldObjects.worker.ts`, `workers/worldObjectsClient.ts` |
| 9 | Sat-field `opacity` prop | `layers/useTerrainBasemapLayer.ts` (satellite-view layers only) |
| 10 | mapStyle wiring + v2 insertion point | `TacticalMap.tsx` (`...worldMapLayers` between hillshade and grid) |
| 11 | 3-way Satellite/Hybrid/Map control | `mission-creator/layout/MissionSettingsDialog.tsx` |
| 12 | Barrel exports | `tactical-map/index.ts` (`MapStyle`, `useMapStyle/getMapStyle/setMapStyle`, `WorldClassToggles`) |

**Decisions**

- `mapStyle` mapping locked per plan §4.3: satellite → sat opacity **1.0** · hybrid → **0.55** · map → **0** + `PAPER_TINT` `#CDC6A3` (provisional Q7 token; visual pass @ T-090.5.4). `mapStyle:'map'` keeps the legacy `tiles/map/` pyramid (Q5) — opacity dimming applies to **satellite-view** layers only; Map tiles always draw at 1.
- Migration: `tbd-mc-world-layers`.mapStyle wins → else legacy `tbd-mc-basemap-view` ('satellite'/'map') → else 'satellite'. `setMapStyle` **dual-writes** the legacy key (hybrid → 'satellite') for rollback safety until T-090.10.2; existing `basemapView.test.ts` passes **unchanged** against the shim.
- `lodGates.ts` exports all 11 N2 constants verbatim (LOD4) + `classVisible` / `visibleWithImportance` / `instanceBudgetCheck`; no `/CLUSTER/` export (LOD5). `forestFill` is the one MAX gate (≤ +1); α-fade ladders stay T-090.8.1 styling.
- `chunkMath.ts`: `{cx}_{cy}` floor(x/512) ids matching on-disk `objects/chunks/` + `objects/density/` stems; border preload = max(5% larger span, 1 chunk) (plan §6); `extraRing` for oversized classes; `chunkSizeM` override (manifest is runtime authority). Separate domain from slot-cull `state/spatialChunks.ts` — documented, not merged.
- Worker skeleton is honest: `ping()` + `getStatus() → { ready: false }` only; chunk RPCs land T-090.5.3. No call site mounts it this slice.
- `verify-t090-spec-consistency.mjs` (plan §8 row assigned to 5.1): ran pre-implementation — **already passes all 12 gates** (Cursor doc sync reconciled cluster asserts), no change needed.

## Automated verification — ALL PASS

**Targeted vitest** (`npm run test -- styleModes lodGates chunkMath worldLayerPrefs basemapView`):

```
 RUN  v4.1.9 apps/website/frontend
 Test Files  5 passed (5)
      Tests  36 passed (36)
   Duration  103ms
```

- `worldmap/styleModes.test.ts` — 4 tests: 3-way mapping values, raster routing (hybrid→satellite, map→map)
- `worldmap/lodGates.test.ts` — 11 tests: N2 constants exact, −2/−6 band spot-checks (buildings rects + forests fill + trees hidden @ −2; outline hidden @ −2), road ladder (path ≥ +4), glyph bands, importance override, budget fn, no-CLUSTER-export
- `worldmap/chunkMath.test.ts` — 9 tests: floor keying, id format, terrain clamp, 5%-vs-1-chunk margin, preload ring, oversized ring, chunkSizeM override
- `state/worldLayerPrefs.test.ts` — 9 tests: defaults (props off), legacy migration both values, own-key precedence, garbage/broken storage, JSON persist + legacy dual-write, hybrid→legacy-'satellite', toggle persist/merge, notify semantics
- `state/basemapView.test.ts` — 4 tests: **pre-existing suite, unchanged, green against the shim** (N8/M5 contract holds)

**Full suite:** `npm run test` → **11 files / 85 tests passed** (49 pre-existing + 36 new; zero regressions).

**Build:** `npm run build` (tsc -b + vite) → clean (chunk-size warning pre-existing).
**Lint:** `npm run lint` → clean.
**Prettier:** all slice files pass `--check`. (Pre-existing drift in 5 untouched files — `satelliteUnified.*`, `flattenModDocument.*`, `RightInspector/fields.tsx` — left alone, not this slice.)
**Spec gates:** `verify-t090-specs: OK (36 spec files + authority docs, all 12 gates pass)`.

## Manual M1–M3

Code-level verification done; in-browser visual confirmation → operator (precedent: T-090.1.1 M6/M9).

- **M1 (flag OFF, satellite unchanged):** PASS by construction — `WORLDMAP_ENABLED` false → `useWorldMapLayers()` returns `[]` (nothing inserted in the Deck array); default style 'satellite' → `satOpacity = 1.0` → BitmapLayers render at today's opacity; shim returns the same `BasemapView` the hook consumed pre-slice; hillshade + grid paths untouched. No new fetches, no layer-id changes → no blank flash. **Operator: browser eyeball pending.**
- **M2 (flag OFF, style switch):** Mission Settings shows Satellite / Hybrid / Map. Hybrid → sat layers at 0.55 (unified/preview/pyramid all carry the prop); Map → legacy `tiles/map/` pyramid exactly as before (view routing via `basemapViewForStyle`, Map tiles pinned at opacity 1); back to Satellite → unified texture survives (per-view resolve state untouched). **Operator: browser eyeball pending.**
- **M3 (flag ON):** `VITE_WORLDMAP_ENABLED=1 make web` — stub still returns `[]`, so zero world layers and no per-frame work added (memoized empty array); fps profile identical to M1. **Operator: browser eyeball pending.**

## Out of scope confirmed untouched

`docs/**`, `.ai/tickets/registry.json`, `docs/TICKET_*.md`, `scripts/map-assets/**`, `packages/map-assets/**`, Workbench plugins, legacy map-view branches in `useTerrainBasemapLayer` (T-090.10.2), `apps/mod/tbd-framework/resourceDatabase.rdb` (pre-existing dirty, not committed).

**Next:** T-090.5.2 roads + buildings live (needs glyph atlas build) — after Cursor doc sync of this slice.
