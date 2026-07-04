# T-090.10 — Map Engine v2 (A3-aligned render spine)

**Ticket:** T-090 · **Slice:** T-090.10 (architecture) · **T-090.10.1** (detailed plan — Claude Code)  
**Status:** **T-090.10.1 shipped** @ `a222a146` · **Plan:** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](../../../.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md)  
**Active implementation:** **T-090.3.1** (export) · **T-090.5.1** (render scaffold, parallel)  
**Authority:** [`.ai/artifacts/t144_arma3_map_architecture_report.md`](../../../.ai/artifacts/t144_arma3_map_architecture_report.md) · [`t144_arma3_map_architecture_study.md`](t144_arma3_map_architecture_study.md)  
**Hub:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · **Legacy (frozen):** [`t090_legacy_raster_pipeline.md`](t090_legacy_raster_pipeline.md)

---

## In one sentence

Replace the **raster-compose readability pipeline** with an **A3-structural** map engine: one frozen satellite photo field + **synthesized cartography + vector data layers** drawn in Deck.gl with **density-gate LOD** (no world-object clustering).

---

## Why this slice exists

**T-144.1** proved A3 never bakes roads, forests, or land-cover into tiles. Readability = **structured world records + immediate-mode vector draw** over one terrain texture. Our dual-pyramid + ImageMagick compose path was a **bridge**; extending it (T-090.1.2.9, more cartographic bakes) is **cancelled**.

**We keep:** frozen SAP unified texture (`tbd-sat` = A3 `DrawField`), DEM/Z (T-091), pick (T-063), export direction (T-090.3), mission-slot perf (T-057–067).

**We kill:** map tile pyramid as product, raster road/land-cover bakes, dual-basemap as two independent raster pipelines, world-object clustering on the world layer.

---

## A3 spine → our analogue

| A3 (`CStaticMap::DrawBackground`) | Our v2 layer | Source |
|-----------------------------------|--------------|--------|
| Paper texture | Optional subtle tint / neutral base | minimal or none |
| `DrawField` — terrain sat per quad | **Frozen `tbd-sat` GPU texture** (T-090.1.2.8) | SAP — no live engine |
| `DrawSea` — ±5 m height band | **Sea gradient layer** from DEM | T-091 DEM |
| `DrawCountlines` — live iso | **Contour lines** from DEM @ zoom-derived interval | T-091 |
| `DrawForestsNew` — marching squares | **Forest polygons** from density grid iso | T-090.8 |
| `DrawRoads` — vector quads | **PathLayer** by `roadClass` | T-090.3 export |
| `DrawObjects` — footprints + icons | **PolygonLayer + IconLayer** by `mapType` | T-090.3 + type inventory |
| Sat α fade by zoom (`maxSatelliteAlpha`) | **Sat opacity crossfade** + toggle | new in T-090.5 |
| Vectors always on top | Layer order contract | render spine |
| `ptsPerSquare` LOD gates | Deck orthographic zoom density gates | [`t090_render_lod_contract.md`](t090_render_lod_contract.md) v2 |
| Per-square chunk stream + time budget | Chunk loader + hydrate budget | T-090.3 + T-090.5 |
| Mission entities overlay | Existing slot IconLayer | unchanged |

---

## Layer stack (normative order, bottom → top)

```text
1. satellite-field     BitmapLayer / custom — tbd-sat, opacity = f(deckZoom, user toggle)
2. sea-band            PolygonLayer or raster shader — DEM ±5 m (A3 DrawSea analogue)
3. hillshade           existing T-091.2 (optional, user slider)
4. land-cover-regions  PolygonLayer — from export density / mapType (replaces map tile tints)
5. contours            PathLayer — DEM iso, interval ∝ zoom
6. roads               PathLayer — class width/color, no raster bake
7. buildings           PolygonLayer — OBB or footprint when export provides
8. forest-mass         PolygonLayer — marching squares (T-090.8)
9. trees / props       IconLayer — per-instance, gated by importance + zoom
10. procedural-grid    existing useBaseMapLayer
11. mission-slots      existing IconLayer + cluster (slots only — NOT world objects)
12. selection/marquee  existing
```

**View modes:** Satellite-heavy vs Map-heavy = **opacity crossfade** on layer 1 + optional paper tint — **not** switching between two unrelated tile pyramids.

---

## Data authority (export — T-090.3 rescoped)

| Artifact | Role | Replaces |
|----------|------|----------|
| `objects/chunks/{cx}_{cy}.json.gz` | per-square map objects | A3 `MapObjectList` spans |
| `objects/roads.json.gz` | road polylines + class | `.topo` raster strokes |
| `objects/prefabs.json.gz` | `mapType` / importance / icon | SAP color heuristics |
| `objects/density/{cx}_{cy}.bin` (new) | tree/rock corner densities | land-cover mask scripts |
| `manifest.json` | chunk index + LOD config | — |
| ~~`tiles/map/` pyramid~~ | **LEGACY FROZEN** | — |
| `everon-sat.tbd-sat` | photo field only | A3 terrain material stage 0 |

Phased import P1–P10 unchanged in spirit; **no Workbench pass A2 (map pyramid)** on new exports.

---

## LOD contract (world objects — v2 rules)

Adopt A3 **density gates**, not supercluster, for the world layer:

1. Per-class `minPtsPerSquare` equivalent in Deck orthographic zoom space.
2. Per-type `importance` threshold from [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md).
3. **No clustering** for world trees/props (T-065 cluster remains **mission slots only**).
4. Pick radius ∝ zoom; pick only types visible at current LOD band (T-090.9).
5. Chunk hydrate: per-frame time budget + neighbor-radius for oversized objects (A3 `landSave.cpp`).

Full numeric table: Claude Code delivers in **T-090.10.1** plan artifact; then Cursor locks into `t090_render_lod_contract.md` v2.

---

## Implementation program (after T-090.10.1 plan lands)

| Order | Slice | Deliverable |
|-------|-------|-------------|
| 1 | **T-090.10.1** | Detailed plan artifact + rescoped slice specs |
| 2 | **T-090.3** | Data-only export (roads, buildings, density, chunks) |
| 3 | **T-090.5** | Map Engine v2 Deck spine (layers 1–9 stub → full) |
| 4 | **T-090.8** | Forest marching squares from density grid |
| 5 | **T-090.9** | Interaction, pick gates, legend |
| 6 | **T-090.4 / .6** | Audits @ scale (parallel when export ready) |
| — | **T-090.7** | AI read API (after objects addressable) |

---

## T-090.10.1 — Claude Code deliverable (planning slice)

**Output path (mandatory):** [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](../../../.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md)

The plan must include:

1. File-level touch list (`tactical-map/layers/*`, export plugin, schemas).
2. Phased slices with acceptance gates (`make map-verify-phase`, vitest, manual zoom table).
3. Migration: what happens to `tiles/map/`, `build-map-cartographic.mjs`, Mission Settings dual radio.
4. LOD numeric table (A3 `ptsPerSquare` → Deck zoom).
5. Chunk protocol extensions (density grid, time budget).
6. Risk register (perf @ 1M objects, bundle size, fallback if layer N fails).

**No code** in T-090.10.1 — plan only. Implementation starts **T-090.3** after operator + Cursor approve plan.

---

## Cancelled / frozen (do not extend)

See [`t090_legacy_raster_pipeline.md`](t090_legacy_raster_pipeline.md).

| Slice | Disposition |
|-------|-------------|
| T-090.1.2.9 | **cancelled** — roads → T-090.5 vectors |
| T-090.1.2.3 | **cancelled** — no tile pyramid analogue |
| T-090.1.1 / T-090.1.1.1 | **shipped, frozen** — legacy map pyramid; no rebuilds |
| T-143 (idea) | **down-ranked** — A3 sea-only; we exceed |

---

## Verification (engine v2 program)

| Gate | Command / check |
|------|-----------------|
| Plan exists | `.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md` |
| Legacy cancelled | `./scripts/ticket check` — slices `.1.2.9`, `.1.2.3` = `cancelled` |
| Sat field still loads | MC boot — unified texture, no regression |
| Spec consistency | `make verify-t090-spec-consistency` (update in T-090.10.1) |
