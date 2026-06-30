# T-090 — World object glyphs (SVG / atlas, rotatable + scalable)

**Status:** Spec ready — ships with **T-090.5** render (per phased import)  
**Authority:** [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md) · [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md)

---

## In one sentence

Each **object class** (not each of 1M instances) gets a **glyph** — SVG source → GPU **texture atlas** — drawn **rotatable** (`rotationDeg`) and **zoom-scalable** on the map, keyed by `prefab.render.iconKey`.

---

## Why this matters

Tiles show geography; **glyphs** show *what* is there at editor zoom — trees, bunkers, fences — like Eden map icons. Without glyphs, buildings are flat polygons and trees are anonymous dots. **Eden AI** can still read JSON semantics without glyphs, but **humans + AI map reasoning** (“the pine line along the road”) need visible, oriented symbols.

**You do need SVG (or equivalent)** for point-like kinds. **You do not need** one SVG per prefab @ 8k types — dedupe like catalog v1.

---

## What uses glyphs vs geometry (locked)

| `kind` | Primary visual | Glyph? | Rotation |
|--------|----------------|--------|----------|
| `building` | **Footprint polygon** (world `geometry` or OBB) | Optional **center badge** for `military`, `tower`, `bunker` | Polygon + badge angle |
| `tree` | **IconLayer** glyph | **Yes** — required | **Yes** — `rotationDeg` |
| `vegetation` | IconLayer | **Yes** | Yes |
| `rock` | IconLayer (large) or small polygon | **Yes** @ small rocks | Yes |
| `prop` | IconLayer | **Yes** | Yes |
| `utility` | IconLayer | **Yes** | Yes |
| `road` | **PathLayer** stroke | **No** — width/color by `roadClass` | Polyline tangent |
| `water` | Polygon or icon | Optional | Yes if icon |

Roads and building footprints are **vector geometry**, not billboards. Glyphs supplement, not replace, footprints.

---

## Glyph identity (`iconKey`)

**Normative key:** `{kind}-{class}` unless overridden.

| `kind` | `class` | `iconKey` example |
|--------|---------|-------------------|
| `tree` | `conifer` | `tree-conifer` |
| `building` | `military` | `building-military` |
| `prop` | `fence` | `prop-fence` |
| `rock` | `boulder` | `rock-boulder` |

Stored on prefab:

```json
"render": {
  "iconKey": "tree-conifer",
  "lodMinZoom": 3,
  "baseSizePx": 24,
  "anchor": [0.5, 1.0],
  "tintable": true,
  "defaultColor": "#2d5a27"
}
```

| Field | Required | Notes |
|-------|----------|-------|
| `iconKey` | yes* | *Required when kind uses IconLayer; buildings may omit (polygon-only) |
| `lodMinZoom` | yes | Hide individual glyph below this zoom (cluster replaces @ low zoom) |
| `baseSizePx` | yes | Size @ reference zoom; scales with map zoom |
| `anchor` | yes | `[x,y]` 0–1 — default `[0.5, 1.0]` = base of tree on ground |
| `tintable` | no | If true, multiply by `defaultColor` or class color |
| `defaultColor` | no | Aegis token or hex |

**Optional prefab override:** `iconKey: "tree-pinus-custom"` only when class glyph is wrong — rare; adds one SVG, not automatic per prefab.

Default rule in classify export:

```typescript
iconKey = `${kind}-${class}`.replace(/_/g, '-');
```

---

## Glyph base sizes (N4) + building geometry (N6)

`baseSizePx` is the size at **deckZoom = `REF_ZOOM` (3)**; on-screen size is `baseSizePx * 2^(deckZoom − 3)`.

| iconKey | baseSizePx |
|---------|------------|
| tree-conifer | 18 |
| tree-deciduous | 16 |
| tree-palm | 16 |
| rock-large | 14 |
| prop-utility | 12 |
| building-badge-military | 10 |
| building-badge-tower | 10 |

**Normative shipped geometry:** oriented bounding **rectangle** from `spatial.halfExtentsM` +
`rotationDeg`. Real **footprint polygon rings** are populated only when T-090.3.0 proves Enfusion
footprint export; when present, polygons supersede OBB rectangles for render. Building badges
(military/tower/bunker) are center glyphs drawn over the rectangle at deckZoom ≥ `BUILDING_BADGE_MIN_ZOOM`.

**Rotation handedness (L2):** glyph `0°` = map north (+y), clockwise yaw. Enfusion yaw handedness + the
`localUp → world Z` bounds remap are measured by the **T-090.3.0** spike (S6/K6) and applied identically
here and in the geometry audit ([`t090_6_geometry_placement_audit.md`](t090_6_geometry_placement_audit.md)).

## Asset pipeline (SVG → atlas)

```text
packages/map-assets/glyphs/
  manifest.json           # iconKey → metadata + atlas pointers
  svg/                    # source SVGs (version controlled, small)
    tree-conifer.svg
    building-military.svg
    prop-fence.svg
    ...
  atlas/
    world-glyphs.webp       # baked atlas (Git LFS if large)
    world-glyphs.json       # { iconKey: { x, y, width, height } }
```

Build step (T-090.5):

```bash
make map-glyphs-build
# → scripts/map-assets/build-glyph-atlas.mjs (sharp / svg2img)
```

**Why atlas:** Deck.gl `IconLayer` expects **one texture** + sub-rectangles — same pattern as mission **slot icons**. Raw per-frame SVG fetch @ 400k trees = death.

**SVG authoring rules:**

- **North-up** in file; rotation applied via `getAngle` / `angle` instance attribute
- **Simple fills** — no filters, no embedded bitmaps
- Viewbox square or fixed aspect; **anchor at bottom center** for ground props
- Target **24×24** or **32×32** design units; atlas packs power-of-two

---

## Rotation (normative)

For IconLayer instances:

```typescript
// rotationDeg: 0 = icon points map north (+y), clockwise
getAngle: (d) => (d.rotationDeg ?? 0) * (Math.PI / 180),
```

- Instance `rotationDeg` from export (yaw).
- If export omits rotation, use prefab default `0`.
- **Pitch/roll** not shown on 2D map glyph (Eden parity) — 3D attitude out of scope.

Building **footprint polygon**: rotate ring by `rotationDeg` around pivot before draw.

Road **PathLayer**: no icon rotation; stroke follows polyline.

---

## Scale / LOD (normative)

**LOD is Deck orthographic zoom — canonical ladder in [`t090_render_lod_contract.md`](t090_render_lod_contract.md) §N3; this table is illustrative.**

| deckZoom | Trees / props | Buildings |
|----------|---------------|-----------|
| ≤ 0 (`WORLD_CLUSTER_MAX_ZOOM`) | **Cluster disc** — no per-tree SVG | OBB rect (thin) |
| 0 … +3 | glyph `baseSizePx * 2^(deckZoom − REF_ZOOM)` | OBB rect + class badge |
| ≥ +3 (`PROP_MIN_ZOOM`) | full glyph; props layer on | OBB rect (+ footprint ring if exported) + badge |

```typescript
getSize: (d, { zoom }) =>
  d.render.baseSizePx * Math.pow(2, zoom - REF_ZOOM),
```

Optional: scale tree glyph by `spatial.heightM` clamp — tall trees slightly larger icon (cap 1.5×).

**Cluster:** a **separate world** supercluster index for `kind=tree` (built in the worker — **not** the
slot `slotClusterIndex` singleton; see [`t090_world_objects_worker.md`](t090_world_objects_worker.md)) —
the cluster disc shows count, not stacked SVGs. Below `WORLD_CLUSTER_MAX_ZOOM` (deckZoom ≤ 0), forest
**regions** replace individual trees entirely ([`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md)).

---

## Phased glyph rollout (matches import phases)

| Phase | Glyphs required before ship |
|-------|------------------------------|
| **P1** buildings | Polygon style + optional badges: `building-*` for each **non-empty** `buildingClass` in inventory |
| **P2** trees | `tree-conifer`, `tree-deciduous`, `tree-palm`, `tree-dead`, `tree-unknown` |
| **P3** vegetation | `vegetation-bush`, `vegetation-grass`, … |
| **P4–P5** | `rock-*`, `prop-*` |
| **P6–P9** | No new glyphs (roads = strokes) |

Verify:

```bash
make map-glyphs-verify   # every iconKey referenced in prefabs has svg + atlas entry
```

---

## Mathematical verification

| ID | Check | Pass |
|----|-------|------|
| G1 | `∀ prefab with IconLayer kind: render.iconKey defined` | script |
| G2 | `∀ iconKey in prefabs: manifest.glyphs[iconKey] exists` | script |
| G3 | `∀ svg/*.svg: valid XML, viewBox present` | script |
| G4 | Atlas json rects ⊆ atlas image bounds | script |
| G5 | Golden screenshot @ fixed view: tree @ 0° vs 90° rotation visually distinct | test fixture |
| G6 | `baseSizePx > 0`, `anchor[0], anchor[1] ∈ [0,1]` | script |

Part of `make map-verify-phase`.

---

## Relationship to AI

- **AI tactical data** = JSON (`gameplay`, `spatial`) — no glyph required.
- **AI vision / map description** (future): optional ortho **sprite capture** from Workbench — **not** v1; v1 uses glyphs for human editor only.
- `render.iconKey` in prefab lets AI say: *“instances use the military building glyph (fort icon)”* via `type-inventory` + manifest.

---

## Deliverables (T-090.5)

| # | Path |
|---|------|
| 1 | `packages/map-assets/glyphs/manifest.json` |
| 2 | `packages/map-assets/glyphs/svg/*.svg` (one per **class** glyph, ~40–80 files total) |
| 3 | `scripts/map-assets/build-glyph-atlas.mjs` |
| 4 | `features/tactical-map/layers/worldGlyphAtlas.ts` — load atlas once |
| 5 | `worldObjectLayers.ts` — IconLayer wired to atlas + rotation + size |
| 6 | `make map-glyphs-build` + `make map-glyphs-verify` |

---

## Out of scope

- Photorealistic top-down renders per object
- 8420 unique SVGs (one per prefab)
- 3D mesh preview on map
- Animated glyphs

---

## Related

- [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md)
- [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md)
- [`t090_phased_object_import.md`](t090_phased_object_import.md)
- [`t065_cluster_lod.md`](t065_cluster_lod.md)
