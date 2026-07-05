# T-090.5.4 verify log — Map Engine v2 sea-band + DEM contours

**Slice:** T-090.5.4 · **Executor:** claude-code · **Branch:** `main`
**Authority:** plan §7 row T-090.5.4 + `t090_render_lod_contract.md` §N3

Worker-side sea band + elevation contours, computed at runtime from the shipped 6400² DEM
(`packages/map-assets/everon/dem/everon-dem-16bit.png`) — the A3 `DrawSea` + `DrawCountlines`
analogues. Two new Deck layers: `world-sea` (slot 2, above the satellite field / below hillshade)
and `world-contours` (slot 5, after land-cover / before roads).

## What shipped

**Pure geometry (node-tested):**
- `worldmap/demGrid.ts` — `DemVectorGrid` + `downsampleDemGrid`/`downsampleDemGridBand` (4× box
  average of the 6400² meters cache → 1600² @ 8 m cells, fresh buffer, endpoints anchored on
  0/12800 so no edge sliver) + `reduceGrid2x` (per-interval contour pyramid).
- `worldmap/seaBand.ts` — `buildSeaBandGeometry`: nested hypsometric fills (inside = elev ≤ iso),
  row-RLE span merge for full-inside cells + boundary marching-squares walk, per-vertex RGBA,
  appended shallow→deep for painter-order deepening. `seaFillAlpha` fade ladder + `SEA_BAND_LEVELS`.
- `worldmap/contours.ts` — `contourSegments` single-sweep multi-level marching squares (per cell
  min/max once, march only crossing levels; O(cells+crossings)), `contourLevels` (positive only),
  `contourGridReductions` (100 m→2×, 50 m→1×, 20/10 m→base).

**Worker (off main thread):** `workers/worldObjectsCore.ts` gains `setDemGrid` / `buildSeaBand` /
`buildContours(intervalM)` (DEM-grid state fully orthogonal to the objects manifest — works on
DEM-only terrains where `loadManifest` returns null; cleared in `reset()`; contour pyramid
memoized). Shell + client transfer all result buffers; client transfers the grid buffer main→worker.

**Main-thread store:** `worldmap/demVectorStore.ts` (forestMassStore pattern) — DEM ready →
banded downsample (yield every 256 rows) → push grid → build sea band + current contour interval.
Grid never retained main-side; a null worker reply (worker restarted on mission unmount) triggers
one re-downsample + re-push + retry. Per-interval contour cache; previous composite kept until the
new one commits (no blanking); degraded DEM (Arland) → empty composites, layers cleanly absent.

**Layers + wiring:** `seaBandLayer.ts` (`world-sea` SolidPolygon, per-vertex color, opacity =
fade ladder, `pickable:false`), `contourLayer.ts` (`world-contours` LineLayer interleaved,
`pickable:false`). `lodGates.ts` additive (`SEA_FILL_MAX_ZOOM=3`, `'sea'` MAX gate, `'contour'`
min −6, `contourIntervalForZoom`). `useWorldMapLayers` returns `{sea, world}`; `TacticalMap`
splices `sea` below hillshade. Respects `worldLayerPrefs` `sea` + `contours` toggles.

## Decisions (for Cursor doc sync)

1. **Contour interval ladder — ticket-text vs §N3 divergence.** Ticket prose reads `20 m @ 0…+3`;
   §N3 (cited authority) + plan §5 read `20 m @ 0…+1, 10 m @ +1…+3`. Implemented **§N3** (spec wins
   on conflict): `z<−4→100; −4≤z<−2.5→50; −2.5≤z<+1→20; z≥+1→10` (edges to finer band). The
   ticket's vitest pin (−2 → 20 m) holds. Noted in a `contourIntervalForZoom` code comment. Cursor
   to reconcile the ticket prose.
2. **Positive-only contour levels** (interval…≤ maxElev, never the 0 line). Bathymetry is the sea
   band's job (A3 parity). No negative isolines.
3. **Contours as `LineLayer`** (not PathLayer) — same deviation `world-forest-outline` already
   shipped; zero-repack interleaved buffers.
4. **Whole-island static geometry** (landcover precedent): sea band once per terrain, contours once
   per interval band (cached). Zero per-pan work — no viewport streaming for these layers.

## Watch items (operator / future)

- **Sea intra-layer alpha stacking:** nested translucent fills rely on painter order (data order,
  LEQUAL depth) within one SolidPolygonLayer. If coplanar z-fighting appears, add
  `parameters: { depthCompare: 'always' }` to the sea layer. Not applied yet — watch in the browser.
- **Colors provisional** (`SEA_BAND_LEVELS`, `CONTOUR_RGBA`) — operator visual pass tunes.

## Gate results (all exit 0)

```
make schema-validate          → OK (36 spec files + authority docs, 12 gates pass; verify-t090/n6/n10 OK)
npm run test -- --run seaBand contours lodGates   → PASS
npx vitest run (full)         → 23 files, 223 tests PASS
npm run build                 → tsc + vite built in ~1.3s (no type errors)
npm run lint                  → clean (contourSegments refactored under complexity 15)
```

New vitest coverage: `lodGates.test.ts` (sea MAX gate, contour min gate, interval ladder incl.
−2→20 pin), `seaBand.test.ts` (all-land empty, all-ocean RLE spans, ring closure, shallow→deep
color order, iso-tie ≤ convention, island-in-ocean marching, edge-flush spans, `seaFillAlpha`,
downsample identity/box-average/fresh-buffer), `contours.test.ts` (levels list, grid-factor,
ramp segment counts, linear interpolation, cone closed-loop even-degree, flat-cell skip,
`reduceGrid2x`), `worldObjectsCore.test.ts` (setDemGrid→build roundtrip, manifest-orthogonal,
null before push / after unload), `demVectorStore.test.ts` (push-once, interval cache, stale
composite kept, terrain switch reset, degraded no-op, worker-restart re-push, late DEM readiness).

## Boot smoke (dev server)

`VITE_WORLDMAP_ENABLED=1 vite` → root 200, `map-assets/everon/manifest.json` 200,
`dem/everon-dem-16bit.png` 206 (image/png via the `make map-assets-link` symlink), new source
modules transform (200, no Vite errors). Production `npm run build` bundles the worker + layers.

## Manual (operator, GPU — node vitest has no WebGL)

Pending an operator browser pass (`VITE_WORLDMAP_ENABLED=1 make web`, hard refresh):
- **M-shore** — shoreline band aligns with the satellite water-composite coast (Everon spot-check).
- **Contours** — interval steps 100→50→20→10 m as zoom goes −6 → +6.
- **Z-pan** — no main-thread hitch on pan (only the one-time yielded downsample at DEM-ready).
- **R5** — FpsCounter ≥55 fps with sea + contours on.
- **M-reg** — flag OFF unchanged; `sea` / `contours` toggles hide independently.

**Status: code + automated gates PASS. Ready for Cursor doc sync.**
