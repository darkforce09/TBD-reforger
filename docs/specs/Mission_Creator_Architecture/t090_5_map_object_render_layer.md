# T-090.5 — Map object render layer (Eden-like static world)

**Ticket:** T-090 · **Slice:** T-090.5  
**Status:** **T-090.5.5 shipped** @ `2b1a0dda` · **v2: A3 density-gate LOD, no world clustering**  
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

## Shipped — T-090.5.4 @ `bd481cf1`

Runtime `world-sea` (slot 2, spliced below hillshade in `TacticalMap`) + `world-contours` (slot 5). Pure modules: `demGrid.ts`, `seaBand.ts`, `contours.ts`; worker `setDemGrid` / `buildSeaBand` / `buildContours`; main-thread `demVectorStore.ts`. Vitest **223/223**. Contour interval authority: §N3 (`20 m @ 0…+1`, `10 m @ +1…+3`) via `contourIntervalForZoom` — edges belong to the finer band. Verify: [`.ai/artifacts/t090_5_4_verify_log.md`](../../../.ai/artifacts/t090_5_4_verify_log.md).

---

## Claude Code prompt archive — T-090.5.4 (shipped @ `bd481cf1`)

Full prompt retired — see [`.ai/artifacts/t090_5_4_verify_log.md`](../../../.ai/artifacts/t090_5_4_verify_log.md).

---

## Shipped — T-090.5.5 @ `2b1a0dda`

Individual tree/veg/prop glyphs via `IconLayer` completed.
- `FpsCounter.tsx` — HUD + `Ctrl+Alt+D` toggle for debugging.
- `worldLayerPrefs.ts` — Persisted `worldmapDebug` flag via Zustand (+ test).
- `treeStore.ts` — `getTreeStreamDebug()` surface for telemetry.
- `worldObjectsCore.ts` — `loadManifest` deduplication applied.
Working tree clean, debug HUD verified, trees render properly.

---

## Claude Code prompt archive — T-090.5.5 (shipped @ `2b1a0dda`)

Authority: plan §7 row T-090.5.5 + [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md). **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-090.5.5** — Map Engine v2 individual tree / vegetation / prop glyphs (IconLayer).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_10_map_engine_v2_implementation_plan.md — §4.2 slots 9–10, §7 row T-090.5.5
  2. docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md (this file)
  3. docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md — atlas, iconKey, rotation, baseSizePx
  4. docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md — §N2/N3 tree/veg/prop gates
  5. apps/website/frontend/src/features/tactical-map/worldmap/lodGates.ts — TREE_GLYPH_MIN_ZOOM, VEGETATION_MIN_ZOOM, PROP_MIN_ZOOM, INSTANCE_BUDGET
  6. apps/website/frontend/src/features/tactical-map/workers/worldObjectsCore.ts — visibleInstances (budget-capped SoA)
  7. apps/website/frontend/src/features/tactical-map/worldmap/chunkStore.ts — viewport streaming pattern
  8. apps/website/frontend/src/features/tactical-map/layers/worldGlyphAtlas.ts + buildingLayer badge pattern
  9. packages/map-assets/everon/objects/type-inventory.json — PH-P2 tree census (501,861 instances)

═══ PROBLEM ═══
501k trees are indexed in the worker but never drawn. Forest mass polygons carry island zoom;
individual glyphs must appear only above TREE_GLYPH_MIN_ZOOM (0) per LOD v2 — no world supercluster.

═══ SHIPPED (do not reopen) ═══
  T-090.5.3 @ 155651b9 — worker streaming + visibleInstances API
  T-090.8.1 @ e28d073a — forest mass (glyphs hidden below zoom 0 by design)
  T-090.5.4 @ bd481cf1 — sea + contours complete the cartographic underlay
  T-090.5.2.x — roads/buildings + 28-glyph atlas (extend for tree/veg/prop iconKeys)

═══ LOCKED ═══
  - Layer ids: `world-trees` (slot 9), `world-props` (slot 10); vegetation shares tree layer or sub-filter per glyphs spec
  - IconLayer + worldGlyphAtlas; getSize = baseSizePx * 2^(zoom − REF_ZOOM); optional heightM cap 1.5×
  - Data from worker visibleInstances(bbox, deckZoom) — viewport cull + INSTANCE_BUDGET (150k)
  - Respect lodGates + render.importanceZoom per prefab; pickable:false until T-090.9
  - NO world supercluster (contract LOD5); forest mass stays below zoom 0
  - Respect worldLayerPrefs trees/props toggles

═══ DO ═══
  1. worldmap/treePropLayer.ts — buildTreeLayers / buildPropLayers from VisibleSet typed arrays
  2. Add missing glyph SVGs for PH-P2…P5 kinds (make map-glyphs-build + map-glyphs-verify)
  3. Wire viewport stream in useWorldMapLayers (chunkStore or dedicated poll of visibleInstances)
  4. Vitest: LOD3 @ −2 trees hidden; @ 0 trees visible; INSTANCE_BUDGET; R8 rotation fixture
  5. Write .ai/artifacts/t090_5_5_verify_log.md
  6. Tag **T-090.5.5** · commit prefix **T-090.5.5:**

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE status markers
  - Add world supercluster or cluster discs
  - Rewrite seaBand/contours/forestMass/road/building layers
  - Commit apps/mod/tbd-framework/resourceDatabase.rdb

═══ VERIFY (all exit 0) ═══
  make schema-validate
  make map-glyphs-verify
  cd apps/website/frontend
  npm run test -- --run treeProp lodGates worldObjectsCore
  npm run build
  npm run lint

═══ MANUAL (VITE_WORLDMAP_ENABLED=1 make web — hard refresh) ═══
  LOD3: @ −2 no tree icons; zoom to 0+ → glyphs appear; forest polygons fade per gate
  R5: ≥55 fps in tree-visible band (viewport with trees)
  Toggle: trees/props off → layers hidden
  M-reg: flag OFF unchanged

═══ RETURN ═══
  - Commit SHA + tag T-090.5.5
  - .ai/artifacts/t090_5_5_verify_log.md
  - Vitest + map-glyphs-verify + build/lint (PASS)
  - Manual LOD3/R5 notes
  - **Ready for Cursor doc sync.**
```
