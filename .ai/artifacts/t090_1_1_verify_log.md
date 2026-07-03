# T-090.1.1 — Map cartographic view: verify log

Date: 2026-07-03 · Executor: Claude Code · Spec: `docs/specs/Mission_Creator_Architecture/t090_1_1_map_cartographic_view.md`

## P0 spike verdict

**G1-A WINNER** — `MapDataExporter.ExportRasterization` TGA (4096², north-up, T-090.1 @ `564419e`) already on disk at `packages/map-assets/everon/staging/spike/TBD_SatExport_everon.tga`; fully offline. G1-B rejected (global low-res engine fallback, not Everon — per T-090.1.2.4 spike). G1-D not needed (same tier, artifact local; requires live Workbench). G1-C not needed (real source found; no offline land-cover LUT anyway). Full verdict: [`t090_1_1_source_spike.json`](t090_1_1_source_spike.json).

**Product composition** (raw TGA has no roads + misses most inland water): G1-A base upscaled 4096→12800 (Lanczos) + inland-water tint from the frozen T-090.1.2.5.2 classifier mask (read-only reuse, TGA ocean teal `#2E5266`) + `.topo` road network per tier (airfield/primary/secondary/minor/track) via `decode-topo.mjs`. **New decode finding:** `.topo` road records embed a per-vertex one-road-width out-and-back excursion (geometry-baked width, exactly 8.0 m on secondaries); `build-map-cartographic.mjs despike()` recovers centerlines (196,440 → 51,045 verts). Manifest `tiles.map.source` stays `workbench-cartographic` (G1-A base raster).

## Pipeline

- `make map-cartographic-everon`: staging ortho → pyramid (z0–6, 256 px, WebP q80 lossy, + `full.webp`) → manifest patch (`source`, `encoding: webp-lossy`) → `make map-cartographic-verify`. 5461 tiles, ~22 MB, local-only (gitignored).
- Schema: `terrain-manifest.schema.json` `tiles.map` gained the `encoding` enum (was `additionalProperties: false` → M2/M8 red until allowed; satellite block untouched).
- Ops: ImageMagick spill must not hit tmpfs `/tmp` (observed SIGKILL + multi-GB `/tmp/magick-*`); script pins `MAGICK_TEMPORARY_PATH=/var/tmp` + `-limit memory 3GiB -limit map 6GiB`.

## Gates M1–M9

| ID | Check | Result |
|----|-------|--------|
| M1 | `make map-cartographic-verify` (`VIEW=map` pyramid complete) | **PASS** — `verify-tile-pyramid: OK everon — levels [0,1,2,3,4,5,6], 5461 tiles, 256px` |
| M2 | `make schema-validate` (dual tiles manifest) | **PASS** — full suite green (118 PASS/OK rows incl. `everon-dual-tiles`, `everon-legacy-tiles`, `everon-unified-satellite`) |
| M3 | Landmark alignment Satellite ↔ Map ≤50 m | **PASS (quantitative)** — coastline crossings on final ortho vs SAP ortho: row 6400 west x=1000 vs 1001 (**1 m**), east 7458 vs 7451 (**7 m**); col 6400 north y=2891 vs 2892 (**1 m**), south 10983 vs 10965 (**18 m**). Bridgehead anchor (4839,6621) lake + secondary road overlay SAT counterparts on the contact sheet: `packages/map-assets/everon/staging/map/contact-sheet-m3-m4.png`. Operator H2 eyeball to confirm. |
| M4 | Y-axis H2b on map pyramid | **PASS** — full-island contact sheet: north airfield peninsula top, NE islet top-right, south peninsula bottom in BOTH views; no mirror. No `--flip-v` used (north-up TGA → XYZ y=0=north; frontend `tileUrl()` is the single inversion, `tileUrl.test.ts` green). |
| M5 | `basemapView` persists `'map'` across reload | **PASS (unit)** — new `state/basemapView.test.ts` (persisted `'map'` honored, garbage→default, set persists); vitest 47/47. Browser reload check = operator. |
| M6 | Map 404 → degraded toast + fallback | **Code-path shipped, operator to confirm** — map resolve probes `tiles/map/0/0/0.webp` → on miss falls to `full.webp` → `none` → `onDegraded('map')` → toast “Map basemap unavailable — showing grid only.” (grid fallback per V6). Simulate: temporarily move `packages/map-assets/everon/tiles/map/` aside, open editor, switch to Map. |
| M7 | FE build + lint | **PASS** — `npm run build` ✓ (1.21 s), `npm run lint` ✓ clean, vitest 47/47 |
| M8 | `make verify-terrain` | **PASS** — `maxDeltaM=0.204 thresholdM=1`, `verify-terrain-alignment: OK` (DEM untouched) |
| M9 | Pan ≥55 fps on Map view | **Operator** — pyramid path is the same `MAX_VISIBLE_BASEMAP_TILES=64` LOD machinery satellite ships ≥55 fps on; map adds no per-frame work. FpsCounter check @ default zoom. |

## Frontend

- `basemapView.ts`: T-127 `map→satellite` coercion removed; real read of `tbd-mc-basemap-view`.
- `MissionSettingsDialog.tsx`: Map button enabled (shared `BasemapViewButton`, active-state styling).
- `useTerrainBasemapLayer.ts`: per-view resolve (`viewFields`/`resolveBasemapMode`); Map = pyramid LOD only (no unified branch); separate `mapResolved` state so a view switch never destroys the loaded `.tbd-sat` GPU texture; view-scoped layer ids `basemap-map-*`.
- `types.ts` + `MissionCreatorPage.tsx`: `onBasemapDegraded(view)` + per-view toast copy.

## Operator checklist (browser)

1. `make web` → open a mission → Mission Settings → **Map**: cartographic view renders (roads + lakes + relief); pan/zoom sharp tiles; FpsCounter ≥55 (M9).
2. Compare a landmark vs Satellite (M3 confirm) and north-coast orientation (M4 confirm).
3. Reload page — Map view persists (M5 confirm).
4. Move `tiles/map/` aside → toast + grid; restore (M6 confirm).
