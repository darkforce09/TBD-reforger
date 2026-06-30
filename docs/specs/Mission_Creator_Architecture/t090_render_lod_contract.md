# T-090 — Render LOD contract (canonical zoom + LOD authority)

**Status:** Spec ready — **single source of truth** for map render LOD
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · constants N1–N3
**Consumers:** [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md) · [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md) · [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) · [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md)

---

## In one sentence

Every map render LOD decision is expressed in **Deck orthographic zoom** (the live `useOrthographicView`
band), never tile-pyramid zoom; this file holds the **only** copy of the master LOD table and the
render constants — all other specs link here and must not restate the numbers.

---

## N1 — Zoom authority (locked)

LOD, render, hover and cluster gates use **Deck orthographic zoom**, band **MIN −6 … MAX +6, default
−2** (`apps/website/frontend/src/features/tactical-map/view/useOrthographicView.ts`). The basemap tile
pyramid index (0–5) is **fetch/index only** and never appears in a LOD table without the mapping below.

### deckZoom ↔ tileZ mapping (mandatory — the only place tile index meets render zoom)

| deckZoom | Approx map width (Everon 12800 m) | tileZ (basemap fetch) |
|----------|-----------------------------------|-----------------------|
| −6 | full terrain | 0 |
| −4 | ~3200 m | 1 |
| −2 | ~800 m (default) | 2 |
| 0 | ~200 m | 3 |
| +2 | ~50 m | 4 |
| +4 … +6 | detail | 5 |

The basemap `TileLayer` selects tileZ from deckZoom via this table (Deck does this internally from the
viewport); world-object LOD below is independent and keyed on deckZoom only.

---

## N2 — World render constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `REF_ZOOM` | **3** | glyph size: `displayPx = baseSizePx * 2^(deckZoom − REF_ZOOM)` |
| `WORLD_CLUSTER_MAX_ZOOM` | **0** | deckZoom ≤ 0 → tree **cluster discs**; deckZoom > 0 → individual tree glyphs |
| `FOREST_REGION_MAX_ZOOM` | **1** | deckZoom ≤ 1 → forest **polygon fill** visible |
| `PROP_MIN_ZOOM` | **3** | deckZoom ≥ 3 → prop/rock glyphs |
| `BUILDING_BADGE_MIN_ZOOM` | **1** | deckZoom ≥ 1 → military/tower/bunker badge |
| `ROAD_PATH_MIN_ZOOM` | **−6** | all road classes except `path` (footpath ≥ **+4**) |

**Why separate from slot clustering:** slot clustering gates on `ZOOM_CLUSTER_MAX = −4` /
`CLUSTER_SLOT_THRESHOLD = 500` (`state/constants.ts`). World tree density reads **`type-inventory.json`
`byKind.tree.instances`** (exact integer once census lands) — orders of magnitude above slot clustering.
of magnitude denser than authored slots, so world clustering must persist to a higher deckZoom
(`WORLD_CLUSTER_MAX_ZOOM = 0`) and forests replace tree points entirely below it. The two systems share
no constants; world layers carry their own `WORLD_*` values.

---

## N3 — Master LOD table (canonical — do not duplicate these numbers elsewhere)

Bands are Deck orthographic zoom. `α` is fill opacity.

| deckZoom | forest | tree | building | road | prop/rock |
|----------|--------|------|----------|------|-----------|
| −6…−3 | region fill α=0.45 | none (inside region) | centroid dot 4 px | highway + paved | hidden |
| −3…−1 | region fill α=0.35 + density shade | cluster disc (count) | OBB thin rect 1 px | + dirt + track | hidden |
| −1…+1 | outline only, fill fading | cluster→glyph transition | OBB rect | all classes | hidden |
| +1…+3 | context α=0.12 | rotated tree glyph | OBB + class badge | all + path | rock glyph |
| +3…+6 | hidden | full-size tree glyph | OBB (+ footprint ring if exported) + badge | all | prop + rock glyph |

- **Forest fill color:** default `rgba(34,120,60,α)` (see [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md)).
- **Building geometry (N6):** **Normative shipped geometry:** oriented bounding **rectangle** from
  `spatial.halfExtentsM` + `rotationDeg`. Real **footprint polygon rings** are populated only when
  T-090.3.0 proves Enfusion footprint export; when present, polygons supersede OBB rectangles for render.
- **Tree clusters vs glyphs:** below `WORLD_CLUSTER_MAX_ZOOM` (deckZoom ≤ 0) trees render as cluster
  discs via the **separate world cluster index** (not the slot `slotClusterIndex` singleton; see
  [`t090_world_objects_worker.md`](t090_world_objects_worker.md)); above it, individual rotated glyphs.

---

## Glyph size (N2 `REF_ZOOM`)

```ts
// deckZoom-driven; baseSizePx from the glyph manifest (N4 in glyphs spec)
getSize: (d, { zoom }) => d.render.baseSizePx * Math.pow(2, zoom - 3 /* REF_ZOOM */)
```

Optional cap: scale a tree glyph by `spatial.heightM` up to 1.5×.

---

## Road dashing (GAP-M4)

Deck `PathLayer` has **no native dash**. Dashed classes (`road_dirt`, `track`, `path`) use
`PathStyleExtension` from `@deck.gl/extensions` (`{ dash: true }` + `getDashArray`), or a 1-px dash
texture fallback. Solid classes (`highway_paved`, `road_paved`, `runway`) need no extension.

| roadClass | color | width px @ deckZoom 0 | style | min deckZoom |
|-----------|-------|-----------------------|-------|--------------|
| highway_paved | `#c8c8c8` | 4 | solid | −6 |
| road_paved | `#a0a0a0` | 2.5 | solid | −6 |
| road_dirt | `#8b6914` | 2 | dash | −2 |
| track | `#6b5010` | 1.5 | dash | −2 |
| path | `#5a4a3a` | 1 | dash | +4 |
| runway | `#ffffff` | 6 | solid | −6 |

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| LOD1 | This file is the only `t090*.md` containing the master α/px LOD numbers | `make t090-spec-verify` |
| LOD2 | All zoom thresholds expressed as deckZoom (−6…+6), never bare tile z | `make t090-spec-verify` (gate 3) |
| LOD3 | At default deckZoom −2: forests render as polygons, trees as cluster discs (no per-tree icons at scale, no empty map) | vitest (T-090.5) |
| LOD4 | `REF_ZOOM`, `WORLD_CLUSTER_MAX_ZOOM`, `FOREST_REGION_MAX_ZOOM`, `PROP_MIN_ZOOM` exported from one module | code review (T-090.5) |

---

## Related

- [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md)
- [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md)
- [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md)
- [`t090_world_objects_worker.md`](t090_world_objects_worker.md)
- [`t065_cluster_lod.md`](t065_cluster_lod.md) — slot clustering (separate system)
