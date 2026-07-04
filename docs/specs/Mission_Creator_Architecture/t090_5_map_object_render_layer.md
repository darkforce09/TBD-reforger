# T-090.5 — Map object render layer (Eden-like static world)

**Ticket:** T-090 · **Slice:** T-090.5  
**Status:** Spec ready — blocked on **T-090.3** + **T-090.10.1** plan · **v2: A3 density-gate LOD, no world clustering**  
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

### Buildings (`PolygonLayer`)

**Normative shipped geometry:** oriented bounding **rectangle** from `spatial.halfExtentsM` +
`rotationDeg`. Real **footprint polygon rings** are populated only when T-090.3.0 proves Enfusion
footprint export; when present, polygons supersede OBB rectangles for render. Fill
`rgba(120,120,130,0.35)`, stroke `#888`; `buildingClass === 'military'` tint `#a08060`;
military/tower/bunker get a center badge at deckZoom ≥ `BUILDING_BADGE_MIN_ZOOM` (N2).

### Trees (`IconLayer` + glyph atlas)

Below `WORLD_CLUSTER_MAX_ZOOM` (deckZoom ≤ 0): **cluster discs** via a **separate world** cluster index
(*not* the slot `slotClusterIndex` singleton); above it, individual glyphs from the `world-glyphs` atlas
by `render.iconKey`, rotated by `rotationDeg` (north-up SVG, clockwise yaw), sized
`baseSizePx * 2^(deckZoom − REF_ZOOM)` (N2/N4). See [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md)
+ [`t090_world_objects_worker.md`](t090_world_objects_worker.md).

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
