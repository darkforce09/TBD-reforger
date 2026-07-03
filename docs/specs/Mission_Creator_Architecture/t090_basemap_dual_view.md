# T-090 — Dual basemap views (Map + Satellite)

**Status:** Spec ready — **vital product requirement**  
**Tickets:** T-090.1 (Satellite first) · **T-090.1.1** (Map view pyramid)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md)

---

## In one sentence

Mission Creator must offer **two switchable bottom raster views** — **Satellite** (aerial / SAP ortho) and **Map** (cartographic / styled map) — like **Google Maps**, same alignment and bounds, one active at a time, with world-object vector layers on top of either.

---

## Product requirement (locked)

| View | Google Maps analogue | TBD source | Use when |
|------|---------------------|------------|----------|
| **Satellite** | Satellite | Color ortho / SAP tile pyramid from Workbench | Aligning slots to real geography, tree lines, building roofs |
| **Map** | Map (default roadmap) | Cartographic / stylized tile pyramid (roads, terrain tints, no photo) | Editing roads/objects, readability, lower visual noise |

**Both are vital.** Grid-only fallback is not acceptable once export ships — at minimum **Satellite** must load; **Map** ships in **T-090.1.1** immediately after Satellite gate passes (same export pipeline, second pyramid).

---

## UX (normative)

```text
Mission Settings  (or map chrome — Eden-style layer control)
  Basemap view:  ( ) Satellite   (•) Map     ← mutually exclusive radio
  [ ] Hillshade   (optional overlay — both views)
  Hillshade strength  (slider 0–100% when hillshade on — **shipped T-090.1.2.6** @ `b958e3b4`)
  [ ] Grid
  [ ] Roads / Buildings / Trees …  (T-090.5 vector layers)
```

- **Persist** `basemapView: 'satellite' | 'map'` in `localStorage` (`tbd-mc-basemap-view`).
- Default for new users: **`satellite`** (alignment-first); allow **`map`** as default later via terrain registry.
- Switching view **does not** reload world objects or slots — only swaps bottom `TileLayer` URL template.
- Toggle animation: instant swap (no cross-fade v1 — perf).

---

## Stack order (bottom → top)

```text
1. Active basemap raster  — satellite OR map pyramid (exclusive)
2. Hillshade (optional)   — T-091.2 DEM overlay; works on both views
3. Procedural grid (optional)
4. World objects (T-090.5) — roads, buildings, trees, …
5. Mission slots / selection
```

World object **vector** layers draw **identically** on Satellite and Map — only the **underlay** changes.

---

## Manifest contract (dual pyramids)

Extend `terrain-manifest.schema.json` **`tiles`** block:

```json
{
  "tiles": {
    "tileSize": 256,
    "format": "webp",
    "minZoom": 0,
    "maxZoom": 5,
    "indexOrder": "xyz",
    "satellite": {
      "path": "tiles/satellite",
      "urlTemplate": "/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp"
    },
    "map": {
      "path": "tiles/map",
      "urlTemplate": "/map-assets/everon/tiles/map/{z}/{x}/{y}.webp"
    }
  }
}
```

| Field | Notes |
|-------|-------|
| `satellite` | Aerial / SAP / color ortho — **T-090.1 primary** |
| `map` | Cartographic export — roads, land cover tints, no aerial photo — **T-090.1.1** |
| `urlTemplate` | Same `{z}/{x}/{y}` + **same Y-axis inversion** as [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) |

**Legacy single path** `tiles/{z}/{x}/{y}.webp` → alias to **`satellite`** only; migrate on next export.

---

## Export (T-090.3)

| Pyramid | Workbench source | Notes |
|---------|------------------|-------|
| **Satellite** | Enhanced Map Tool / SAP / color ortho capture | Already specified in T-090.3 §A |
| **Map** | Cartographic map export (EMT “map” layer, or stylized render — roads + terrain palette, **no** aerial photo) | Second pass in same `make map-export` |

Both pyramids:

- Same world bounds (Everon 12800×12800)
- Same tile zoom levels 0–5
- Same XYZ disk layout + **TMS Y flip** at fetch
- Validated **H1/H2/H2b** independently

**Cost:** dual-pyramid storage per the **N10 tile-cache table** below (single source) — ~2× a single pyramid.

---

## Implementation (frontend)

| Piece | Behavior |
|-------|----------|
| `useTerrainBasemapLayer.ts` | Accept `view: 'satellite' \| 'map'`; pick manifest URL template |
| `worldLayerPrefs` or `useMapStore` | `basemapView` slice |
| `MissionSettingsDialog` | Radio: Satellite / Map (extends T-091.2 basemap toggles) |
| `useBaseMapLayer.ts` | Compose active basemap + grid |

**Only one** satellite or map `TileLayer` mounted at a time — not both at 50% opacity (unlike hillshade).

---

## Slice split

| Slice | Delivers |
|-------|----------|
| **T-090.1** | Satellite pyramid + switch stub (Satellite only until map tiles exist) |
| **T-090.1.1** | Map pyramid + UI radio + manifest `tiles.map` + verify both views |

T-090.1 may ship with Map radio **disabled** (“Coming soon”) until T-090.1.1 tiles land — but **spec + manifest slot** must exist @ T-090.1.

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| V1 | Switch Satellite → Map → same landmark @ (x,y) within **≤50 m** on both | H2 on each |
| V2 | Y-axis H2b on **both** pyramids | manual log |
| V3 | `basemapView` persists across reload | localStorage |
| V4 | World object layer visible on both views | manual |
| V5 | Pan ≥55 fps switching views | FpsCounter |
| V6 | Missing map tiles → Satellite still works; Map shows toast + fallback to Satellite or grid | degraded |
| V7 | Schema validates manifest with both `satellite` and `map` | `make schema-validate` |

---

## Tile cache & storage (N10 — single source; identical table in [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md))

| Item | Value |
|------|-------|
| One pyramid (Everon tile zoom 0–5 WebP q≈80) | 200–400 MB LFS |
| Dual pyramid (Satellite + Map) | 400–800 MB LFS |
| Max concurrent tile fetches | 6 |
| Tile LRU cache | 512 tiles (~32 MB) |
| Cold first paint | grid + hillshade ≤500 ms; basemap tiles stream; forest regions ≤3 s @ deckZoom −2 |
| Runtime | only one basemap pyramid mounted at a time |

## Synthesized Map + persistence (N9 / N8)

- **N9 synthesized Map:** if T-090.3.0 finds no Workbench cartographic export, the Map pyramid is
  **synthesized** — DEM hillshade + land-cover LUT + baked road vectors @ tile zoom 0–3 — and the
  manifest sets `tiles.map.source: “synthesized-cartographic”`; the UI labels it **”Synthetic map”**. Map
  ships for real, never a permanently-disabled radio.
- **N8 persistence:** `basemapView` → `localStorage` `tbd-mc-basemap-view`; world-layer toggles →
  `localStorage` `tbd-mc-world-layers` (per-user, global). Grid + hillshade stay per-mission
  `meta.environment` (the existing `MissionSettingsDialog` model). One Mission Settings section documents
  both: view/world-layer prefs are per-user; grid/hillshade travel with the mission.

## Relationship to other “layers”

| Feature | Layer type | Not the same as |
|---------|------------|-----------------|
| Satellite / Map | **Basemap view** (raster swap) | World object toggles |
| Hillshade | Optional **overlay** | Basemap view |
| Grid | Optional **overlay** | Basemap view |
| Roads/buildings (T-090.5) | **Vector data** on top | Cartographic map tiles (complementary — map tiles may show roads too; vectors add editor semantics) |

**Map view tiles** show geography in a **readable cartographic style**. **T-090.5 road vectors** show **typed** roads from export (`roadClass`) for editing — both can coexist; Map view is the “clean underlay”, vectors are the “editor truth”.

---

## Related

- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md) — Cartesian tiles + Y flip
- [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md) — dual pyramid export
- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) — Mission Settings pattern
- [`t090_eden_map_reference.md`](t090_eden_map_reference.md)
