# Water export

Reads the open world's water in [Workbench](/documentation_v2/glossary.md#workbench) two ways: a
full-map raster of water class and depth probed from the engine's water surfaces, and vector records
of the rivers, lakes and ponds the world places. Each lands as ASCII or JSON files in the export
profile.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/
├── TBD_InlandWaterExportPlugin.c  the standalone plugin that runs the vector export
├── TBD_MapExportInlandWater.c     the vector export: rivers, lakes and ponds, then their totals
├── TBD_MapExportLakes.c           lake shorelines, surface heights, areas and depths
├── TBD_MapExportPonds.c           pond perimeters, kinds, surface heights and depths
├── TBD_MapExportRivers.c          river splines, river parameters and river-part boxes
├── TBD_MapExportWater.c           the raster export: water class and depth grids and their metadata
└── TBD_WaterExportPlugin.c        the standalone plugin that runs the raster export
```

## How it works

Two independent exports share the folder, and every file lands in
`$profile:TBD_Export/<map>/water/`. `TBD_WaterExportPlugin` runs the raster export and
`TBD_InlandWaterExportPlugin` the vector export, each with its own config and context; both
`[WorkbenchPluginAttribute]` blocks are commented out, so Workbench lists neither in a menu.
`TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs the
raster export only, and its attribute is commented out too.

### The raster export

`TBD_MapExportWater` walks a square grid of `round(worldSize / m_fWaterMetersPerPixel)` samples (1 m
per pixel by default; the DEM resolution, then 1 m, when the setting is 0.01 or less), row 0 at
`z = 0`. At each sample it reads the ground height and asks `ChimeraWorldUtils.TryGetWaterSurface`
for water just below the ground, then, on dry land above sea level, 0.35 m, 0.9 m and 1.6 m above
it. The engine's surface type sets the class: 1 ocean, 2 pond or lake, 3 river; an unknown type
counts as a pond when the ground is above 0 m and the surface above 1 m, and as ocean otherwise;
ground at or below 0 m with no surface found counts as ocean; everything else is 0, land. Depth is
the surface height minus the ground height in decimetres, at least 0.5 m in a river, capped at
65,535.

| File | Content |
|---|---|
| `bathymetry_mask.txt` | one row per `z`, space-separated class codes 0–3 |
| `bathymetry_depth.txt` | the same grid of depths in decimetres |
| `lakes.json` | `waterBodies`: pond and river cells grouped per 64 m tile, with id, type, surface height, centre, cell count, maximum depth and box |
| `water_meta.json` | grid size, resolution, `depthUnit` `decimeters`, `depthScaleToMeters` 0.1, the class pixel counts, the surface height range and the same bodies with areas |

A failed raster write deletes both rasters and fails the export.

### The vector export

`TBD_MapExportInlandWater` runs the three body exporters in turn and writes `inland_water_meta.json`
(the counts, total river length and part count, total lake and pond areas, and the three file
names).

| Exporter | Finds | Writes |
|---|---|---|
| `TBD_MapExportRivers` | every `RiverEntity` (`GetExistingInstances`) | `rivers.json`: per river its `Width`, `SplineOffsetUp`, `ReverseFlow`, `Material` and `Surface`, the points of its spline, its length and box, and its `RiverPartEntity` parts sorted from the highest centre down |
| `TBD_MapExportLakes` | entities whose name, class or prefab says lake or water, from the top-level editor entities and a 512 m cell sweep | `lakes.json`: per lake its shoreline ring, surface height, area and maximum and mean depth |
| `TBD_MapExportPonds` | entities whose name, class or prefab says pond, pool, tarn or water, the same way | `ponds.json`: per pond its perimeter ring, kind (`farm_pond`, `woodland_pool`, `crater_pool` and the rest), surface height, area and depths |

Lakes and ponds take their ring, and the surface height every ring point carries, from the nearest
spline shape (parent, child or neighbour), or draw an ellipse over the entity's box when there is
none. Area is the ring's shoelace area; the lake export drops pond-named bodies of 15,000 m² or
less, and the pond export drops bodies over 15,000 m² not named as ponds. Depth comes from terrain
samples over the ring's box, with fixed fallbacks (lake 3.5 m and 1.8 m, pond 1.5 m and 0.8 m) when
no sample lies under the surface. Nearby bodies at a similar height merge.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `ChimeraWorldUtils.TryGetWaterSurface`, `RiverEntity`, `RiverPartEntity`, `SplineShapeEntity`,
  `BaseWorld.QueryEntitiesByAABB` and `WorldEditorAPI.GetTerrainSurfaceY`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  The developer tools' `map water` command
  (`tools_v2/developer-tools/src/map_raster_pipeline/inland_water_archive.rs`) builds
  `water_vectors.rkyv` and `bathymetry.tbd-bath` from a water export staged in
  `assets_v2/scratch/<terrain>/water/` under other names: `TBD_InlandWaterExport_mask.txt`,
  `_depth.txt`, `_meta.json` (it reads `widthPx`, `heightPx` and `depthScaleToMeters`) and one
  `_vectors.json` holding `lakes`, `rivers` and `ponds` or `inlandWaterBodies`. No committed step
  renames or merges this folder's files into that layout, and its river reader expects `nodes` with
  `pos` and an `averageWidthM`, which `rivers.json` does not write.
- Rules: 0 is the only dry class code, since `map water` reads every other code as water; the depth
  unit and `depthScaleToMeters` change together; the raster and vector exports both write
  `lakes.json`, so running both keeps whichever ran last. Sources stay ASCII, and they compile only
  when Workbench loads `tbd-export`.

## Related documentation

- [Water: bathymetry, inland water and the sea mesh](/apps/website/map-engine/src/world/terrain/water/README.md)
  — how the map engine reads the water archives built from this export.
