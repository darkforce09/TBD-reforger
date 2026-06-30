# T-090.1 — Aligned Cartesian basemap

**Ticket:** T-090 · **Slice:** T-090.1 **(Satellite basemap + tile pyramid export)**  
**Status:** **shipped** @ `564419e` — interim `MapDataExporter.ExportRasterization` + pyramid LOD (aligned). **High-detail SAP ortho:** **T-090.1.2** [`t090_1_2_sap_supertexture_satellite.md`](t090_1_2_sap_supertexture_satellite.md).  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · spike ops log [`.ai/artifacts/map_export_everon.json`](../../../.ai/artifacts/map_export_everon.json)  
**Follow-ons:** Map view **T-090.1.1** · world objects **T-090.2–.5** · UX ref [`t090_eden_map_reference.md`](t090_eden_map_reference.md)

---

## In one sentence

Render aligned Everon **dual basemap views** (Satellite + Map) under the procedural grid in `<TacticalMap>` — Google Maps-style switch, same bounds — with grid-only fallback when tiles 404. See [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md).

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-090.3.0 spike** | Shipped @ `b342c35` — [`.ai/artifacts/map_export_everon.json`](../../../.ai/artifacts/map_export_everon.json) |
| **Satellite source (K3)** | `system/terrain/defSatMap_BCR.edds` + per-terrain SAP/satellite background (wiki *2D Map Creation*) — **this slice extracts + retiles** |
| **T-090.0** | Manifest `tiles.satellite`, `alignmentOrigin`, `bounds` validate |
| **Dev serve** | `make map-assets-link` → tiles at `/map-assets/everon/tiles/satellite/...` |

**Spike lesson (S0):** `wb_state` entity count is **not** a loaded-world signal — confirm a populated world (e.g. `TBD_Dev_POC.ent`) before export work.

**Out of scope for T-090.1:** Map cartographic tiles (`.topo` / Export Map Data) → **T-090.1.1** (source already found in spike; N9 synth **not** required).

---

## Problem

[`useBaseMapLayer.ts`](../../../apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts) draws only a procedural grid. Mission makers cannot visually align slots with in-game geography.

---

## Goal

1. Load terrain manifest via `manifestUrl` from [`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts).
2. Add **Satellite** basemap (`tiles.satellite`) — **T-090.1 ship target**.
3. Stub **Map** basemap (`tiles.map`) + UI radio — full tiles @ **T-090.1.1** ([`t090_basemap_dual_view.md`](t090_basemap_dual_view.md)).
4. **Cartesian** TileLayer/BitmapLayer — **never Web Mercator** ([`engineering_plan.md`](engineering_plan.md) §4.1).
5. Grid overlay on top (semi-transparent); clip to `worldBounds`.
6. Degraded: 404 → grid-only + toast.

---

## Out of scope

- DEM / Z (**T-091**)
- Hillshade (**T-091.2**)
- Arland assets (defer until Everon gate PASS)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Coordinate system | `COORDINATE_SYSTEM.CARTESIAN`, `flipY: false`, origin bottom-left |
| Tile URL | **`tiles.satellite.urlTemplate`** — default `/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp` |
| Map view URL | **`tiles.map.urlTemplate`** — `/map-assets/everon/tiles/map/{z}/{x}/{y}.webp` (**T-090.1.1**) |
| Basemap mode | **`basemapView`**: `'satellite' \| 'map'` — mutually exclusive (Google Maps pattern) |
| Zoom range | Manifest `minZoom`–`maxZoom` (default 0–5) |
| Layer order | Basemap **below** grid lines |
| Toggle | **Satellite / Map** radio in Mission Settings — [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md); hillshade/grid remain separate toggles (T-091.2) |
| **Tile Y-axis (XYZ vs TMS)** | Exported WebP pyramids use **XYZ** tile indexing: **`y=0` = northernmost** row. Deck.gl `TileLayer` under `COORDINATE_SYSTEM.CARTESIAN` + `flipY: false` maps tile **`y=0` to the southernmost** world edge. **Never pass raw `{y}` into the URL template on a `TileLayer`.** |
| **TileLayer Y fix** | Override `getTileData` (or equivalent fetch hook): `tmsY = 2 ** z - 1 - y` (i.e. `Math.pow(2, z) - 1 - y`); request `{z}/{x}/{tmsY}.webp`. |
| **BitmapLayer fallback** | Single-image path: **no Y flip** — `bounds: [minX, minY, maxX, maxY]` (Everon: `[0, 0, 12800, 12800]`) maps the **top** row of the image to the **north** (`maxY`) bound. |

---

## Implementation specification

### Tile Y-axis trap (mandatory — do not skip)

Our editor uses **`COORDINATE_SYSTEM.CARTESIAN`** with **`flipY: false`** (bottom-left origin, +Y north). That matches Arma world meters — but it **conflicts** with how most map tile pyramids are stored on disk.

| System | Tile `y=0` row |
|--------|----------------|
| **XYZ export** (our WebP pyramid from T-090.3) | **Northernmost** edge of the map |
| **Deck.gl `TileLayer`** @ Cartesian + `flipY: false` | **Southernmost** world edge |

If the implementing agent passes Deck's tile index `y` straight into `tiles.urlTemplate` (`…/{z}/{x}/{y}.webp`), the basemap will load but appear **vertically scrambled** (north/south flipped). H2 landmark tests may still “sort of” pass at a glance — treat **explicit Y inversion** as a hard requirement, not an optimization.

#### `TileLayer` path (pyramid)

When using `@deck.gl/geo-layers` `TileLayer` (or any tiled fetch keyed by `{z,x,y}`):

1. Let `y` be the index Deck.gl requests (southern-first in our Cartesian setup).
2. Before building the URL, compute the on-disk XYZ row:

```typescript
const tmsY = Math.pow(2, z) - 1 - y;
const url = urlTemplate.replace('{z}', String(z)).replace('{x}', String(x)).replace('{y}', String(tmsY));
```

3. Prefer encapsulating this in **`getTileData`** (or a single `fetchTile(z, x, y)` helper) so no code path can accidentally use raw `y`.
4. Document in code **why** — future refactors have removed this flip and shipped upside-down maps before.

**Do not** enable default Web Mercator tiling or assume TMS/XYZ “just works” with Cartesian bounds.

#### `BitmapLayer` path (single extent)

When using one full-terrain image instead of a pyramid:

- **No Y-index flip** is required.
- Set `bounds` to manifest world extents, e.g. Everon **`[0, 0, 12800, 12800]`** — Deck.gl maps the **top** scanline of the bitmap to the **north** (`maxY`) bound and the bottom scanline to the south (`minY`), which matches a north-up export aligned to world bounds.

Use this path for z0-only prototypes; production expects the pyramid (`TileLayer` + Y inversion above).

### Files to touch

| File | Change |
|------|--------|
| `layers/useBaseMapLayer.ts` | Fetch manifest; compose basemap + grid layers |
| `layers/useTerrainBasemapLayer.ts` | **New** — BitmapLayer or Cartesian TileLayer |
| `coords/terrainManifest.ts` | **New** — parse manifest JSON (shared with T-091.1) |
| `TacticalMap.tsx` | Pass `terrain` id; remount on change (existing `key`) |

### Horizontal alignment tests (required @ verify)

| ID | Test | Method |
|----|------|--------|
| **H1** | Grid origin | World (0,0) pixel = southwest map corner tile pixel |
| **H2** | Landmark | Pick 1 Biki-known coordinate (e.g. airfield center) — tile color/feature within **≤50 m** of expected world point @ z3 |
| **H2b** | **Y-axis / north-up** | Known **north vs south** feature (e.g. Everon north coast vs south coast) matches Reforger — **not** mirror-flipped. Fail if only east/west looks right. |
| **H3** | Bounds clip | Pan beyond 12800 m — basemap does not draw outside bounds |

Document measured H1/H2 results in PR / manual verify log.

---

## Verification gate (mandatory)

**Ship T-090.1 only when ALL PASS.** Closes spike **K3** (sample + pyramid path proven).

### Automated

```bash
make ci-local-frontend
make verify-terrain
make schema-validate
test -f packages/map-assets/everon/tiles/satellite/0/0/0.webp   # K3 sample tile minimum
bash scripts/map-assets/verify-spike-ops-log.mjs TERRAIN=everon  # optional: update gates.K3 pass in ops log after tile lands
```

### Manual (browser)

| ID | Step | Pass condition |
|----|------|----------------|
| M1 | Open `/missions/:id/edit` Everon | Basemap visible under grid |
| M2 | Compare to Reforger map screenshot | Coastline/airfield roughly aligned @ default zoom |
| M3 | Rename tile dir → 404 | Grid-only + toast; editor usable |
| M4 | Pan/zoom | ≥55 fps with basemap (no regression vs T-057) |
| M5 | Switch terrain Arland | Grid only (no Everon tiles on wrong terrain) |

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Build/lint | `make ci-local-frontend` exit 0 |
| S2 | Cartesian | No `@deck.gl/geo-layers` TileLayer without explicit bbox **and** XYZ→fetch Y inversion (`tmsY = 2**z - 1 - y`) |
| S3 | H1/H2/H2b | Manual log attached — horizontal **and north/south** alignment documented |
| S4 | Degraded | M3 pass |
| S5 | Perf | M4 pass |

---

## Related

- [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md) — **Satellite + Map views (vital)**
- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — tile export runbook
- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) — basemap toggle in settings
