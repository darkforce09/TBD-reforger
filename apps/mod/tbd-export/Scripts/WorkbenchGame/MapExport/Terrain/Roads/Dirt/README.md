# Dirt road export

Finds the open world's dirt and gravel roads in [Workbench](/documentation_v2/glossary.md#workbench)
and writes their centrelines, widths and endpoint connections to `roads_dirt.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/Dirt/
├── TBD_DirtRoadsExportPlugin.c  the standalone plugin that runs the dirt road export
└── TBD_MapExportDirtRoads.c     the export: finds dirt road entities, writes their segments
```

## How it works

`TBD_MapExportDirtRoads.Export` sweeps the world in `m_fObjectChunkSizeM` cells (512 m by default)
and runs `IsDirtRoadEntity` on each entity whose origin falls in the cell. For each accepted entity
it looks for a spline shape: the entity's parent, a child, the entity itself, or the first one
within 25 m. A found spline becomes one segment of its points, each the spline's origin plus the
point's local offset; an entity without one becomes a loose waypoint. The loose waypoints are then
chained: from each unvisited one, the next is the nearest within 250 m, with a penalty for turning
and no reversal sharper than about 100°. Segment endpoints within 5 m share a node id and list each
other as connected.

`IsDirtRoadEntity` rejects map descriptors, editor comments, vegetation, props, fences, decals,
footprints, patches and pavement prefabs; trail, forest-road, dirt-track and `road_dirt_02`
materials; asphalt and cobblestone materials; and asphalt, paved or highway prefabs. It accepts a
`Material` holding `road_dirt_01` or `dirt_01`, or a prefab named `road_dirt`. The width is the
entity's `Width`, 4.5 m when unset.

The file lands in `$profile:TBD_Export/<map>/roads/roads_dirt.json` in the shared road dataset
format, with `roadClass` `road_dirt` and ids `road_dirt_<n>`.

`TBD_DirtRoadsExportPlugin` builds its own config and context and runs the export. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `SplineShapeEntity` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportRoads` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/`. No committed tool reads
  `roads_dirt.json`.
- Rules: `IsDirtRoadEntity` is the one place that decides what this class holds; the file keeps the
  field set every road class writes. Lines added stay ASCII, and the scripts compile only when
  Workbench loads `tbd-export`.
