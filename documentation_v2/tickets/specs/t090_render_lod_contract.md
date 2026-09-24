# T-090 — Render LOD contract (canonical zoom + LOD authority)

**Status:** **v2** — locked @ T-090.10.1 plan [`t090_10_map_engine_v2_implementation_plan.md`](../../../.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md) @ `a222a146`  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · constants **N1–N11** (no N12)  
**Consumers:** [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md) · [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md) · [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) · [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) · `worldmap/lodGates.ts` (T-090.5.1)

---

## In one sentence

Every **world-object** LOD decision uses **Deck orthographic zoom** (A3 density-gate model) — **no world supercluster**; forest mass polygons replace individual tree glyphs below the glyph band; **slot** clustering (`ZOOM_CLUSTER_MAX`) stays a separate system for mission slots only.

---

## N1 — Zoom authority (locked)

LOD, render, hover and pick gates use **Deck orthographic zoom**, band **MIN −6 … MAX +6, default −2** (`view/useOrthographicView.ts`). Legacy tile-pyramid index (0–5) is **satellite fallback fetch only** — not a world LOD axis.

### deckZoom ↔ tileZ (basemap fetch only — satellite pyramid fallback)

| deckZoom | Approx map width (Everon 12800 m) | tileZ |
|----------|-----------------------------------|-------|
| −6 | full terrain | 0 |
| −4 | ~3200 m | 1 |
| −2 | ~800 m (default) | 2 |
| 0 | ~200 m | 3 |
| +2 | ~50 m | 4 |
| +4 … +6 | detail | 5 |

---

## N2 — World render constants (v2)

| Constant | Value | Meaning |
|----------|-------|---------|
| `REF_ZOOM` | **3** | glyph size: `displayPx = baseSizePx * 2^(deckZoom − REF_ZOOM)` |
| `TREE_GLYPH_MIN_ZOOM` | **0** | deckZoom ≥ 0 → individual tree glyphs (below: hidden; forest mass only) |
| `FOREST_FILL_MAX_ZOOM` | **+1** | deckZoom ≤ +1 → forest polygon fill visible |
| `FOREST_OUTLINE_MIN_ZOOM` | **−1.5** | deckZoom ≥ −1.5 → forest outline (A3 `ptsPerSquareForEdge≈15`) |
| `BUILDING_FOOTPRINT_MIN_ZOOM` | **−2.5** | deckZoom ≥ −2.5 → building OBB rects (A3 `ptsPerSquareObj≈9`) |
| `BUILDING_BADGE_MIN_ZOOM` | **+1** | deckZoom ≥ +1 → military/tower/bunker badge |
| `VEGETATION_MIN_ZOOM` | **+1.5** | deckZoom ≥ +1.5 → vegetation glyphs |
| `PROP_MIN_ZOOM` | **+3** | deckZoom ≥ +3 → prop/small rock glyphs |
| `ROCK_LARGE_MIN_ZOOM` | **+1** | deckZoom ≥ +1 → large rock landmark glyphs |
| `PICK_RADIUS_PX` | **12** | screen pick radius (A3 2% viewport analogue) |
| `INSTANCE_BUDGET` | **150 000** | max drawn world instances at any zoom (vitest vs census) |

**Deleted (v1 — do not use):** `WORLD_CLUSTER_MAX_ZOOM`, world `supercluster`, `clusterTrees`, `worldClusterIndex.ts`.

**Slot clustering (unchanged):** `ZOOM_CLUSTER_MAX = −4`, `CLUSTER_SLOT_THRESHOLD = 500` (`state/constants.ts`) — **mission slots only**.

Per-type override: `render.importanceZoom` on prefab (schema bump T-090.3.1) — visible when `deckZoom ≥ importanceZoom` even if class gate is higher. Landmarks (lighthouse, transmitter, watertower, military): recommend **−4**.

---

## N3 — Master LOD band table (canonical — v2)

Bands are Deck orthographic zoom. `α` = polygon fill opacity. Zoom is continuous; bands step **feature classes** (A3 model).

| deckZoom | sea / land-cover | contour interval | roads visible | buildings | forest | trees / veg / props |
|----------|------------------|------------------|---------------|-----------|--------|---------------------|
| −6…−4 | fill on | 100 m | highway, paved, runway | — | fill α 0.45 | — |
| −4…−2.5 | fill on | 50 m | + dirt/track @ −2 | — | fill α 0.45 | — |
| −2.5…0 | fill on | 50→20 m | all but path | OBB thin | fill α 0.35 + outline @ ≥−1.5 | — |
| 0…+1 | fill on | 20 m | all but path | rects + fill | fill α 0.35, outline | tree glyphs from 0 |
| +1…+3 | fade fill | 10 m | all but path | + badges @ ≥+1 | fill α 0.12 → outline-only | + vegetation @ ≥+1.5, large rocks @ ≥+1 |
| +3…+6 | off | 10 m | + path @ ≥+4 | + footprint rings (if exported) | outline only | + props @ ≥+3; tree size cap 1.5× by `heightM` |

- **Forest fill color:** `rgba(34,120,60,α)` — [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md).
- **Building geometry (N6):** OBB rectangle from `spatial.halfExtentsM` + `rotationDeg`; footprint rings supersede when export provides them.
- **Trees below `TREE_GLYPH_MIN_ZOOM`:** **hidden** — forest mass polygons carry readability (no cluster discs).

---

## Road class table (unchanged — vector-correct)

| roadClass | color | width px @ deckZoom 0 | style | min deckZoom |
|-----------|-------|-----------------------|-------|--------------|
| highway_paved | `#c8c8c8` | 4 | solid | −6 |
| road_paved | `#a0a0a0` | 2.5 | solid | −6 |
| road_dirt | `#8b6914` | 2 | dash | −2 |
| track | `#6b5010` | 1.5 | dash | −2 |
| path | `#5a4a3a` | 1 | dash | +4 |
| runway | `#ffffff` | 6 | solid | −6 |

Dashing: `PathStyleExtension` (`@deck.gl/extensions`). Widths clamp ≥ 1 px at all zooms.

---

## Glyph size (N2 `REF_ZOOM`)

```ts
getSize: (d, { zoom }) => d.render.baseSizePx * Math.pow(2, zoom - 3 /* REF_ZOOM */)
```

Optional cap: scale tree glyph by `spatial.heightM` up to **1.5×**.

---

## N4 — Pick gates

Pick only classes visible at current `deckZoom` per N3 (`lodGates.classVisible`). Radius = `PICK_RADIUS_PX · mpp` where `mpp = 2^-zoom`. Worker authoritative — Deck GPU pick forbidden on mass layers.

---

## Verification (v2)

| ID | Check | Pass |
|----|-------|------|
| LOD1 | This file is the only `t090*.md` containing the master band table | `cargo xtask schema t090-specs` |
| LOD2 | All thresholds expressed as deckZoom (−6…+6) | `cargo xtask schema t090-specs` |
| LOD3 | At default deckZoom **−2**: forests = **polygons**, trees = **hidden**, buildings = **OBB rects** | vitest (T-090.5.2+) |
| LOD4 | `REF_ZOOM`, `TREE_GLYPH_MIN_ZOOM`, `FOREST_*`, `SEA_FILL_MAX_ZOOM`, `contourIntervalForZoom`, `BUILDING_*`, `INSTANCE_BUDGET` exported from `worldmap/lodGates.ts` | code review |
| LOD5 | No `WORLD_CLUSTER_*` or world supercluster in codebase | grep + vitest |

---

## Related

- Plan: [`.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md`](../../../.ai/artifacts/t090_10_map_engine_v2_implementation_plan.md) §5
- [`t065_cluster_lod.md`](t065_cluster_lod.md) — **slot** clustering only
