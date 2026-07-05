# T-090.5.2 — Map Engine v2: roads + buildings live — verify log

**Date:** 2026-07-05 · **Executor:** claude-code · **Slice spec:** `docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md` · **Plan:** `.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md` §4.1–4.2, §5, §7 row T-090.5.2 · **LOD authority:** `t090_render_lod_contract.md` v2 · **Glyphs:** `t090_world_object_glyphs.md`

## Scope shipped

First live world-object Deck layers on the T-090.5.1 spine: **`world-roads`** (PathLayer, 766 segments), **`world-buildings`** (PolygonLayer, 5,606 OBB footprints), **`world-building-badges`** (IconLayer, military/tower/bunker) — flag-gated behind `VITE_WORLDMAP_ENABLED=1`, all `pickable: false` (pick = T-090.9). NO trees/forest/contours/sea/chunk-streaming (T-090.5.3/.5.4/.5.5/.8.1). Export pipeline, Workbench, docs, registry untouched.

| # | Deliverable | Path |
|---|---|---|
| 1 | Glyph atlas builder (`make map-glyphs-build`, stub → real) | `scripts/map-assets/build-glyph-atlas.mjs` + root `Makefile` |
| 2 | P1 building glyph set (9 SVGs, every non-empty buildingClass) | `packages/map-assets/glyphs/svg/building-{residential,civic,agricultural,industrial,commercial,hangar,bunker,tower,military}.svg` + `manifest.json` entries |
| 3 | Built atlas artifacts (committed, plain git) | `packages/map-assets/glyphs/atlas/world-glyphs.webp` (1024×512, 11.8 KB, lossless) + `world-glyphs.json` (Deck-ready mapping + meta) |
| 4 | Glyph verify gate extended (G2-everon / G4-atlas / G6-fields) | `packages/tbd-schema/scripts/verify-map-glyphs-manifest.mjs` |
| 5 | Road layer builder + pure style/parse | `worldmap/roadLayer.ts` (+ test) |
| 6 | Building OBB + badge builders + pure geometry/filter | `worldmap/buildingLayer.ts` (+ test) |
| 7 | World-object data loader (roads one-shot + indexed chunk fetch) | `worldmap/worldData.ts` |
| 8 | Glyph atlas load-once | `layers/worldGlyphAtlas.ts` |
| 9 | Layer assembly (toggles + gates, slots 6–7 order) | `worldmap/useWorldMapLayers.ts` (extended, spine intact) |
| 10 | Zoom/terrain wire | `TacticalMap.tsx` (`useWorldMapLayers({ terrain, deckZoom })`) |
| 11 | `@deck.gl/extensions ^9.3.4` (PathStyleExtension dashes) | `apps/website/frontend/package.json` + lock |

## Decisions

- **Chunk enumeration via export index:** `objects/chunks/manifest.json` (`{chunkSizeM, cells:[{cx,cy,path,instanceCount}]}`, 270 Everon cells) ships with the T-090.3 export — the loader fetches exactly the files that exist. Fallback (index missing): full-grid sweep via `chunkMath` with misses read as empty. Worker + chunkStore LRU streaming still land in T-090.5.3; this loader is the interim main-thread path sanctioned for P1 scale (5.6k buildings).
- **Trees never retained:** mixed P2 chunks are filtered through the building-prefab lookup at parse time; 501,861 tree rows are parsed and dropped (render = T-090.5.5).
- **Badge key derived from `class`**, not prefab `render.iconKey` — everon bunker prefabs (84/99/329) ship without an iconKey; contract badge rule is class-based (military/tower/bunker → `building-badge-*`).
- **Road widths:** contract table is px @ deckZoom 0 where 1 px = 1 m → `widthUnits:'meters'` reproduces it exactly and scales geometrically; `widthMinPixels: 1` = contract clamp. Dash `[8,6]` m for dirt/track/path, `[0,0]` solid.
- **Badge size:** `sizeUnits:'meters'` with size = baseSizePx/2^REF_ZOOM ≡ N2 formula (displayPx = base·2^(zoom−3)) with zero per-frame updates; `sizeMinPixels: 8` readability floor (plan §4.4 glyph min-px clamp).
- **Zoom → layers without churn:** hook memoizes on derived band state (visible road-class key + building/badge booleans), so continuous pan/zoom rebuilds nothing; band crossings swap filtered data (roads) / flip `visible` (buildings, badges — data stays on GPU).
- **gz decode:** gzip magic sniff + `DecompressionStream('gzip')`; Vite serves `.gz` with `Content-Encoding: gzip` (browser auto-decompresses → sniff falls through to plain parse) — both paths verified. SPA HTML fallback (missing file → 200 text/html) detected and treated as absent.
- **Atlas:** 128 px cells (crisp to ~8× base @ z+6), row-major sorted keys, power-of-two 1024×512 ≤ 4096² (GL-G4), magick RSVG rasterize @ density 512 + libwebp lossless. Committed plain (not LFS — `.gitattributes` map-assets rules cover png/r16/tbd-sat only; webp is 11.8 KB).
- **R11 empty state:** no `manifestUrl` / no manifest `objects` block (Arland, custom) → loader resolves empty, zero layers, no error.

## Automated verification — ALL PASS

```
make map-glyphs-build    → build-glyph-atlas: OK — 19 glyphs → 1024×512 atlas (11.8 KB)
make map-glyphs-verify   → verify-map-glyphs: OK (19 glyphs, golden + everon iconKeys covered, atlas rects verified)
make schema-validate     → all gates incl. verify-t090-specs: OK (36 spec files, 12 gates)
npm run test -- --run    → Test Files 13 passed · Tests 102 passed (85 pre-existing + 17 new; zero regressions)
npm run build            → tsc -b + vite clean (chunk-size warning pre-existing)
npm run lint             → clean
prettier --check         → slice files clean
```

**New vitest coverage** (`worldmap/roadLayer.test.ts` 8 tests, `worldmap/buildingLayer.test.ts` 9 tests):
- Road class table verbatim (colors/widths/dash) + N3 bands: −6…−4 highway/paved/runway only · dirt+track join @ −2 · path @ +4 only (LOD vitest gate).
- `parseRoadsPayload` narrows real payload shape; drops unknown-class/degenerate/malformed rows and non-JSON.
- `obbCorners` @ 0° vs 90° distinct (**R8** rotation-distinct), 360°≡0°, shoelace area rotation-invariant (L2 clockwise-from-north handedness).
- Buildings hidden @ −3, visible @ −2.5/−2 (**LOD3** building half); badges hidden @ 0, visible @ +1.
- `badgeIconKey` incl. bunker-without-prefab-iconKey; prefab lookup keeps buildings only; chunk filter drops trees/unknown/non-finite.
- Construction smoke: `world-roads`/`world-buildings`/`world-building-badges` ids, `pickable:false` everywhere, badge layer null without atlas (R5 degrade), null on empty data.

**One-off integration proof against the real committed export** (temp vitest, run then removed — census parity is the export pipeline's regression surface, not FE's):

```
roads: 766/766 segments parse (394 road_dirt / 367 road_paved / 5 runway)
buildings: 310-prefab lookup → 5,606 instances decoded from 270 indexed chunks
badges: count == type-inventory military+tower+bunker sum
chunk index cells == chunk files on disk (exact set)
```

**Dev-server asset smoke** (`make map-assets-link` + Vite): chunk index / glyph mapping / roads.gz / chunk gz / terrain manifest all 200; missing chunk path returns SPA HTML (200 text/html) — loader guard covers; `.gz` served `Content-Encoding: gzip`.

**Editorconfig:** all slice files pass. Repo-wide `make verify-editorconfig` fails on 4 **pre-existing, untouched** files (`missions_compiled_integration_test.go` spaces-vs-tabs, `everon-sat.tbd-sat` locally-materialized LFS binary, `sap-seam-metrics.mjs`, vendored `bcdec.h`) — predates this slice, not introduced here.

## Manual gates — PENDING-OPERATOR (in-browser)

`VITE_WORLDMAP_ENABLED=1 make web` → dev-login → open any mission in the editor (Everon):

| ID | Check | Expected |
|----|-------|----------|
| R1 | Roads @ default zoom −2, basemap on | paved (grey) + dirt (brown dash) + runway strokes over sat |
| R2 | `tbd-mc-world-layers` roads toggle off | `world-roads` gone (buildings stay) |
| R3 | Buildings @ ≥ −2.5 | grey OBB rects, #888 stroke; military tint #a08060 |
| R5 | Pan sweep, FpsCounter | ≥55 fps with P1 data loaded |
| Z1–Z6 | Screenshots @ −6/−4/−2/0/+3/+6 | per N3 rows: −6 highway/paved/runway only, no buildings · −4 same · −2 +dirt/track +building rects · 0 same · +3 badges visible (≥+1) · +6 all but nothing extra (path class absent from Everon export) |
| M-reg | Flag OFF (`make web`) | zero world layers, sat/hillshade/grid byte-identical to T-090.5.1 behavior |

Code-level equivalents of R1–R3 + LOD bands are covered by the vitest gates above (precedent: T-090.5.1 M-gates, T-090.1.1 M6/M9). Note Z-ladder: Everon export has no `highway_paved`/`track`/`path` segments (394 dirt / 367 paved / 5 runway), so band steps visible in practice are paved+runway (all zooms) → +dirt @ −2.

## Follow-ups (not this slice)

- T-090.5.3: worker chunk streaming + chunkStore LRU + `visibleInstances` (this loader shrinks to the manifest gate); INSTANCE_BUDGET census vitest.
- T-090.5.5: tree/veg/prop glyph layers (atlas already carries tree/veg/rock/prop/utility sprites).
- T-090.9: pick wiring (`PICK_RADIUS_PX`, worker `pickNearest/pickRect`).
- `WorldLayerToggles.tsx` UI (prefs are live via localStorage; surfaced control ships with the toggles panel deliverable).
