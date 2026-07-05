# T-090.3 — Map asset export (Workbench → repo)

**Ticket:** T-090 · **Slice:** T-090.3  
**Status:** **export through P2 + T-090.3.3** @ `887a6ed1` · **render through T-090.5.2.2** @ `346a31c9` · **next:** **T-090.5.3** chunk streaming  
**Executor:** **claude-code** (automation + Workbench plugin)  
**Authority:** [`t090_10_map_engine_v2.md`](t090_10_map_engine_v2.md) · [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**One command:** [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md)  
**Legacy raster:** [`t090_legacy_raster_pipeline.md`](t090_legacy_raster_pipeline.md) — §A2 map pyramid **removed** in v2

---

## In one sentence

Implement **`make map-export TERRAIN=<id> PHASE=Pn`** and **`make map-verify-phase`** — phased, cumulative object import with **mathematical gates only** (no eyeball sign-off). Full 1M export blocked until **P10**. See [`t090_phased_object_import.md`](t090_phased_object_import.md).

---

## Operator experience (target)

```bash
make map-export TERRAIN=everon PHASE=P1_buildings
make map-verify-phase TERRAIN=everon PHASE=P1_buildings   # exit 0 before P2
# … P2 trees → P9 footpaths → P10_full
make map-export TERRAIN=arland PHASE=P1_buildings         # same phases, new map
```

Phased import: [`t090_phased_object_import.md`](t090_phased_object_import.md). **No full 1M until P10.**

---

## Prerequisites

| Gate | Owner |
|------|-------|
| T-090.2 schema + `prefab-classify.json` rules stub | claude-code |
| `terrain-registry.json` with terrain row | claude-code |
| Workbench + `TBD_TerrainWorldExportPlugin.c` | claude-code / one-time human install |
| DEM plugin (T-091.0) already shipped | done |
| `make map-assets-link` dev serve | done |

---

## Workbench export (§A — v2 data-only)

> **v2 pivot (T-090.10):** Pass **A2 (map cartographic pyramid)** is **retired**. Export produces **world data** for Map Engine v2 layers — not readability rasters. Satellite photo field remains **frozen `tbd-sat`** (T-090.1.2.8), not re-exported each phase unless operator requests full rebuild.

`TBD_TerrainWorldExportPlugin.c` writes **structured map objects** per terrain:

| Pass | Output | Workbench source |
|------|--------|------------------|
| **Objects P1–P10** | `staging/{terrainId}/objects/` | World entities by phase |
| **Roads** | `objects/roads.json.gz` | Road network |
| **Density grid** (new) | `objects/density/` | Tree/rock density per land square |
| ~~**A2 — Map tiles**~~ | ~~`tiles/map/`~~ | **CANCELLED** — use Deck vector layers |

**Legacy §A dual pyramid** (below) retained for historical reference only — do not implement on new work.

| Pass | Output dir | Workbench source | Slice |
|------|------------|------------------|-------|
| **A1 — Satellite** | `staging/{terrainId}/tiles/satellite/` | Enhanced Map Tool / SAP / color ortho capture | **T-090.1** |
| **A2 — Map** | `staging/{terrainId}/tiles/map/` | Cartographic map export (EMT “map” layer or stylized render — roads + terrain palette, **no** aerial photo) | **T-090.1.1** |

Post-processing (`build-tile-pyramid.sh`) runs **twice** — once per pyramid — then copies into `packages/map-assets/{terrainId}/tiles/{satellite|map}/`.

Both passes share:

- Same `alignmentOrigin` and `worldBounds` from terrain registry
- Same zoom range and tile size (256 WebP)
- Independent **H1/H2/H2b** validation (see [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md))

**Legacy:** single `tiles/{z}/{x}/{y}.webp` staging → migrate to `tiles/satellite/` only; Map pass added on next full export.

---

## Export artifacts

Same layout for **every** terrain — only `terrainId` and bounds change.

| Artifact | Path | Format | LFS |
|----------|------|--------|-----|
| Terrain registry | `packages/map-assets/terrain-registry.json` | JSON | no |
| Tile pyramid (Satellite) | `packages/map-assets/{terrainId}/tiles/satellite/{z}/{x}/{y}.webp` | SAP / color ortho | **yes** |
| Tile pyramid (Map) | `packages/map-assets/{terrainId}/tiles/map/{z}/{x}/{y}.webp` | Cartographic / styled map | **yes** |
| Prefab table | `.../objects/prefabs.json.gz` | catalog v1 + **ai** blocks | yes if >1 MB |
| Instance chunks | `.../objects/chunks/{cx}_{cy}.json.gz` | compact placements | yes |
| Road network | `.../objects/roads.json.gz` | polylines | optional |
| Golden / CI | `packages/tbd-schema/golden/map-objects-everon-sample.json` | small bundle | no |
| Export manifest | `.../manifest.json` | JSON | no |
| Z audits | `.../objects/z-audit.json`, `z-audit-geometry.json` | JSON | regenerated |
| AI ops log | `.ai/artifacts/map_export_{terrainId}.json` | JSON | no |

### `manifest.json` (extends terrain manifest)

```json
{
  "terrainId": "everon",
  "alignmentOrigin": { "x": 0, "y": 0 },
  "dem": { "path": "dem/everon-dem-16bit.png", "width": 4096, "height": 4096 },
  "tiles": {
    "tileSize": 256,
    "format": "webp",
    "minZoom": 0,
    "maxZoom": 5,
    "satellite": {
      "path": "tiles/satellite",
      "urlTemplate": "/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp"
    },
    "map": {
      "path": "tiles/map",
      "urlTemplate": "/map-assets/everon/tiles/map/{z}/{x}/{y}.webp"
    }
  },
  "objects": {
    "schemaVersion": "1.0.0",
    "format": "catalog-v1",
    "prefabsPath": "objects/prefabs.json.gz",
    "prefabCount": 0,
    "instanceCount": 0,
    "chunksPath": "objects/chunks",
    "chunkSizeM": 512,
    "roadsPath": "objects/roads.json.gz",
    "exportedAt": "2026-06-26T00:00:00Z"
  }
}
```

---

## Pipeline scripts (claude-code deliverables)

| Script | Purpose |
|--------|---------|
| `scripts/map-assets/export-terrain.sh` | **Entry point** — `make map-export` |
| `scripts/map-assets/export-all-terrains.sh` | Loop registry |
| `scripts/map-assets/workbench-export.sh` | Headless Workbench / MCP wrapper |
| `scripts/map-assets/build-tile-pyramid.sh` | Raw ortho → z/x/y WebP |
| `scripts/map-assets/classify-prefab.ts` | Rules + overrides → prefabs + **ai** metadata |
| `scripts/map-assets/build-catalog-v1.ts` | Dedupe, chunk, gzip |
| `scripts/map-assets/run-z-audit.ts` | T-090.4 |
| `scripts/map-assets/run-geometry-audit.ts` | T-090.6 |
| `scripts/map-assets/validate-manifest.sh` | Per-terrain validate |
| `scripts/map-assets/write-export-ops-log.ts` | `.ai/artifacts/map_export_*.json` |

**Classification rules (shared, all maps):** `packages/tbd-schema/rules/prefab-classify.json`  
**Overrides (rare):** `packages/tbd-schema/rules/prefab-overrides.json`

---

## Verification gates

| ID | Check | Pass |
|----|-------|------|
| E1 | `make map-export TERRAIN=everon PHASE=P1_buildings` → exit 0 + ops log | script |
| E1b | `make map-verify-phase TERRAIN=everon PHASE=P1_buildings` → exit 0 (G1–G12 + P1-*) | script |
| E2 | Second terrain in registry uses **identical** script path | script |
| E3 | `prefabs.json.gz` — every row has `ai.*` | script |
| E4 | `instanceCount` = sum of chunks; all `prefabId` resolve | script |
| E5 | `make schema-validate` + `make verify-terrain` | exit 0 |
| E6 | Re-export → stable prefab ordering (deterministic diff) | diff |

---

## T-121 absorption

**T-121** = DEM refresh only. Tiles + world objects → this slice + [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md).

---

## Out of scope

- In-browser tile loader (**T-090.1**)
- Deck render (**T-090.5**)
- Mod spawn (**T-092**)
- Per-map custom export scripts (**forbidden**)

---

## Related

- [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md)
- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md)
- [`t090_4_z_placement_audit.md`](t090_4_z_placement_audit.md)
- [`t121_terrain_dem_export_automation.md`](t121_terrain_dem_export_automation.md)
