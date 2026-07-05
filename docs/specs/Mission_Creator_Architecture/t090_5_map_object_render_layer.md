# T-090.5 — Map object render layer (Eden-like static world)

**Ticket:** T-090 · **Slice:** T-090.5  
**Status:** **T-090.8.1 shipped** @ `e28d073a` · **T-090.5.4 active** — sea-band + contours · **v2: A3 density-gate LOD, no world clustering**  
**Executor:** **claude-code**  
**Authority:** [`t090_10_map_engine_v2.md`](t090_10_map_engine_v2.md) · [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Add **Deck.gl layers** one **import phase at a time** (P1 buildings → … → P9 roads), with **class glyphs** (SVG → atlas, rotatable + zoom-scalable) per [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md).

---

## Problem

Tiles alone (T-090.1) don't expose **selectable structure** — road class, building footprints, tree locations. Mission makers need vector/instance detail on the map, toggled like Eden layers.

---

## Architecture

```text
MissionCreatorPage
  └─ TacticalMap
       ├─ BasemapTileLayer (T-090.1)
       ├─ MapEngineV2Layers (T-090.5 — see t090_10_map_engine_v2.md)
       │    ├─ SeaBandLayer (DEM ±5m)
       │    ├─ ContourLayer
       │    ├─ RoadLayer (PathLayer)
       │    ├─ BuildingLayer (PolygonLayer)
       │    ├─ ForestMassLayer (marching squares — T-090.8)
       │    └─ TreePropLayer (IconLayer — NO cluster; density-gate LOD)
       └─ SlotIconLayer (existing)
```

### Data loading

| Strategy | When |
|----------|------|
| **Manifest fetch** | `GET /static/map-assets/everon/manifest.json` or bundled import |
| **Chunk loader** | If `objects.chunks` in manifest — fetch by viewport bbox + 1 chunk margin |
| **Spatial index** | rbush in worker (reuse T-063 pattern) for pick/hover later |
| **Never** | Full `JSON.parse` of 500 MB catalog on main thread |

Initial implementation may ship with **sample subset** + chunk pipeline stub; full Everon load gated on T-090.3 export size.

---

## Visual spec (by `kind` / `class`)

**All zoom thresholds are Deck orthographic zoom and live in the canonical
[`t090_render_lod_contract.md`](t090_render_lod_contract.md) §N3 (constants N1–N3) — this section does
not restate the numbers.**

### Forests first (`PolygonLayer`)

Region fills (forest/field/water) render **before** trees — translucent areas at low/mid deckZoom that
dissolve to per-tree glyphs only when zoomed in. See [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md).

### Roads (`PathLayer`)

Color, width, min-deckZoom and dash style per the road table in the render contract. Dashing uses
`@deck.gl/extensions` **PathStyleExtension** (GAP-M4) — `highway_paved`/`road_paved`/`runway` solid;
`road_dirt`/`track`/`path` dashed.

**Shipped (T-090.5.2–.2.2):** `extractRoadCenterline` converts export quad-soup polylines to centerlines;
width from measured pair geometry (`widthM`). **`world-roads-casing`** under **`world-roads`** (near-black
stroke +40% width). Full `.topo` type mapping (types 1+2 = asphalt highway) @ **T-090.3.3** → 888 segments.

### Buildings (`PolygonLayer`)

**Normative shipped geometry:** oriented bounding **rectangle** from `spatial.halfExtentsM` +
`rotationDeg`. Real **footprint polygon rings** are populated only when T-090.3.0 proves Enfusion
footprint export; when present, polygons supersede OBB rectangles for render.

**Measured OBBs (T-090.3.3):** export `halfExtentsM` = per-axis median of raw engine samples (not rule-template constants); footprints 0–4,066 m² on Everon.

**Shipped fills (T-090.5.2.1–.2.2):** default solid dark `rgba(38,38,44,0.72)` / stroke `rgba(150,150,158,0.8)`;
per-class tints (bridge, pier/dock, ruin, castle, lighthouse, container, tent, military, …). **`water`**
piers/docks render as footprints alongside buildings. Military/tower/bunker badges @ `BUILDING_BADGE_MIN_ZOOM`.

### Trees (`IconLayer` + glyph atlas)

At deckZoom **≥ `TREE_GLYPH_MIN_ZOOM` (0)** per [`t090_render_lod_contract.md`](t090_render_lod_contract.md) v2: individual glyphs from the `world-glyphs` atlas by `render.iconKey`, rotated by `rotationDeg`, sized `baseSizePx * 2^(deckZoom − REF_ZOOM)`. **Below 0:** trees **hidden** — forest mass polygons (T-090.8.1) carry readability. **No world supercluster.**

### Props / rocks / utility

Same atlas path as trees; default **off** in toggles (noise); visible at deckZoom ≥ `PROP_MIN_ZOOM` (N2).

---

## Layer toggles (UI)

**Mission Settings** or **map overlay menu** (match Eden):

| Toggle | Default | Layer ids |
|--------|---------|-----------|
| Basemap tiles | on | `basemap-tiles` |
| Roads | on | `world-roads` |
| Buildings | on | `world-buildings` |
| Vegetation | on | `world-trees` |
| Props | off | `world-props` |

Persist in `localStorage` key `tbd-mc-world-layers`.

---

## Z rendering

- Use export `z` when present and T-090.4 `severity !== fail`.
- If `z` missing or fail: render at `demZ` (T-091 sampler) — **visual only**.
- **Do not** block slot placement on world object Z.

---

## Performance budget

| Metric | Target |
|--------|--------|
| Pan/zoom @ 50k visible world instances | ≥55 fps (match T-057) |
| Initial chunk load | <500 ms |
| Memory | <50 MB world index in tab |

Techniques: viewport cull, kind-specific LOD, worker parse, **T-112** GPU cull (future).

---

## Deliverables

| # | Path |
|---|------|
| 1 | `features/tactical-map/layers/worldObjectLayers.ts` |
| 2 | `features/tactical-map/layers/worldGlyphAtlas.ts` |
| 3 | `packages/map-assets/glyphs/` — SVG + atlas — [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) |
| 4 | `features/tactical-map/state/worldLayerPrefs.ts` |
| 5 | `features/mission-creator/WorldLayerToggles.tsx` |
| 6 | Wire into `TacticalMap` below slots |
| 7 | `scripts/map-assets/build-glyph-atlas.mjs` + `make map-glyphs-verify` |

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| R1 | Roads visible with basemap on | manual |
| R2 | Toggle off roads → layer hidden | manual |
| R3 | Buildings @ medium zoom | manual |
| R4 | Tree cluster @ zoom out | manual |
| R5 | Pan ≥55 fps with sample catalog | FpsCounter |
| R7 | Every `render.iconKey` in phase catalog has SVG + atlas entry | `make map-glyphs-verify` |
| R8 | Rotated instance @ 90° ≠ 0° in atlas pick test | vitest |

---

## Out of scope

- Editing/moving world objects in MC — **Workbench only** (read-only context). Hover/inspect/filter/legend ship in **T-090.9** ([`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md)) — not deferred.
- Floor selector (**T-129**)
- T-110 binary base + mission deltas @ millions

---

## Related

- [`t090_eden_map_reference.md`](t090_eden_map_reference.md)
- [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md)
- [`t063_spatial_index.md`](t063_spatial_index.md)
- [`t065_cluster_lod.md`](t065_cluster_lod.md)

---

## Claude Code prompt — T-090.5.4 (copy-paste)

Authority: plan §7 row T-090.5.4 + [`t090_render_lod_contract.md`](t090_render_lod_contract.md) §N3 contour interval ladder. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-090.5.4** — Map Engine v2 sea-band + DEM contours (worker-side geometry).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_10_map_engine_v2_implementation_plan.md — §4.2 slots 2+5, §7 row T-090.5.4, A3 DrawSea ±5 m
  2. docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md (this file)
  3. docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md — §N3 sea/land-cover + contour interval column
  4. apps/website/frontend/src/features/tactical-map/dem/* — loadDemForTerrain, sampleElevation, DemTexture decode
  5. apps/website/frontend/src/features/tactical-map/worldmap/useWorldMapLayers.ts — layer insertion (sea before hillshade; contours after landcover)
  6. apps/website/frontend/src/features/tactical-map/worldmap/forestMassStore.ts — store pattern for worker-streamed geometry
  7. apps/website/frontend/src/features/tactical-map/workers/worldObjectsCore.ts — extend worker API if DEM/contour gen belongs here
  8. packages/map-assets/everon/manifest.json — dem.path, demNativeMetersPerPixel

═══ PROBLEM ═══
Island zoom still lacks cartographic context below the photo field: ocean/shore band and elevation
contours must read like A3 DrawSea + DrawCountlines — computed from the shipped 6400² DEM, not
offline raster compose.

═══ SHIPPED (do not reopen) ═══
  T-090.5.3 @ 155651b9 — worker + chunkStore streaming
  T-090.8.1 @ e28d073a — forest mass + landcover (slots 4, 8); reuse forestMassStore patterns
  T-091.0 / T-091.1 — DEM PNG on disk + main-thread DemController loader

═══ LOCKED ═══
  - Layer ids: `world-sea` (slot 2, under sat field), `world-contours` (slot 5, after landcover)
  - Sea: DEM → ocean polygon + shore gradient band (A3 ±5 m analogue); pickable:false
  - Contours: iso polylines per §N3 interval ladder (100 m @ −6…−4, 50 m @ −4…−2.5, 50→20 m @ −2.5…0, 20 m @ 0…+3, 10 m @ +3…+6); fade/off per band table
  - Pure geometry in `worldmap/seaBand.ts` + `worldmap/contours.ts` (node vitest); heavy work off main thread (worker or async cache keyed by interval band)
  - Respect `worldLayerPrefs` `sea` + `contours` toggles (defaults on)
  - No legacy `tiles/map/` changes; no mapStyle branch deletion (T-090.10.2)
  - Do NOT add tree glyphs (T-090.5.5) or touch forestMass/road/building layers

═══ DO ═══
  1. seaBand.ts — pure DEM samples → ocean fill + shore band polygons (vitest on small fixture grid)
  2. contours.ts — pure marching squares / iso extraction per interval (vitest: known fixture → expected segment count)
  3. seaBandLayer.ts + contourLayer.ts — thin Deck builders (SolidPolygon/PathLayer, binary typed arrays, pickable:false)
  4. Wire DEM decode path in worker OR reuse main-thread meters cache with worker transfer — perf gate: contour gen never blocks pan
  5. Integrate in useWorldMapLayers; memo keys on zoom band + toggle state (not raw zoom every frame)
  6. Vitest: interval ladder spot checks; sea visible @ default −2; contours match band at −2 (20 m branch)
  7. Write .ai/artifacts/t090_5_4_verify_log.md
  8. Tag **T-090.5.4** · commit prefix **T-090.5.4:**

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE status markers
  - Rewrite forestMass, chunkStore core, lodGates forest gates, or export pipeline
  - Commit apps/mod/tbd-framework/resourceDatabase.rdb

═══ VERIFY (all exit 0) ═══
  make schema-validate
  cd apps/website/frontend
  npm run test -- --run seaBand contours lodGates
  npm run build
  npm run lint

═══ MANUAL (VITE_WORLDMAP_ENABLED=1 make web — hard refresh) ═══
  M-shore: shoreline band visually aligns with sat water-composite coast (Everon spot-check)
  Contours: interval steps match §N3 when zooming −6 → +6
  Z-pan: no main-thread hitch on pan; layers stable
  R5: FpsCounter ≥55 fps with sea + contours on
  M-reg: flag OFF unchanged; toggles hide sea/contours independently

═══ RETURN ═══
  - Commit SHA + tag T-090.5.4
  - .ai/artifacts/t090_5_4_verify_log.md
  - Vitest + build/lint output (PASS)
  - Manual shoreline/contour notes
  - **Ready for Cursor doc sync.**
```
