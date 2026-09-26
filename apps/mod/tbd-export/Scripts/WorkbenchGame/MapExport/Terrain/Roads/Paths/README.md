# Footpath export

Finds the open world's footpaths and trails in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
and writes their centrelines, widths and endpoint connections to `paths.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/Paths/
├── TBD_MapExportFootpaths.c  the export: finds footpath entities, writes their segments
└── TBD_PathsExportPlugin.c   the standalone plugin that runs the footpath export
```

## How it works

`TBD_MapExportFootpaths.Export` sweeps the world in `m_fObjectChunkSizeM` cells (512 m by default)
and runs `IsFootpathEntity` on each entity whose origin falls in the cell. For each accepted entity
it looks for a spline shape: the entity's parent, a child, the entity itself, or the first one
within 25 m. A found spline becomes one segment of its points, each the spline's origin plus the
point's local offset; an entity without one becomes a loose waypoint. The loose waypoints are then
chained: from each unvisited one, the next is the nearest within 150 m, with a penalty for turning
and no reversal sharper than about 100°. Segment endpoints within 5 m share a node id and list each
other as connected.

`IsFootpathEntity` rejects map descriptors, editor comments, vegetation, props, fences and decals;
`road_asphalt`, cobblestone and `road_dirt_01` materials; and highway, main road, asphalt, paved,
cobblestone, dirt road, runway and airstrip prefabs. It accepts a `Material` holding a trail
material (`traildirt`, `trailgravel`, `trailforest`, `trail_`), or a prefab named path, trail,
footpath, pedestrian, walkway, hiking or stone steps. The width is the entity's `Width`, 1.75 m when
unset.

The file lands in `$profile:TBD_Export/<map>/roads/paths.json` in the shared road dataset format,
with `roadClass` `path` and ids `path_<n>`.

`TBD_PathsExportPlugin` builds its own config and context and runs the export. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `SplineShapeEntity` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportRoads` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/`. No committed tool reads
  `paths.json`.
- Rules: `IsFootpathEntity` is the one place that decides what this class holds; the file keeps the
  field set every road class writes. Lines added stay ASCII, and the scripts compile only when
  Workbench loads `tbd-export`.
