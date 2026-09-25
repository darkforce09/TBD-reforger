# Runway export

Finds the open world's airfield runways and taxiways in
[Workbench](/documentation_v2/glossary.md#workbench) and writes their centrelines, widths and
endpoint connections to `runways.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/Runways/
├── TBD_MapExportRunways.c     the export: finds runway entities, writes their segments
└── TBD_RunwaysExportPlugin.c  the standalone plugin that runs the runway export
```

## How it works

`TBD_MapExportRunways.Export` sweeps the world in `m_fObjectChunkSizeM` cells (512 m by default) and
runs `IsRunwayEntity` on each entity whose origin falls in the cell. For each accepted entity it
looks for a spline shape: the entity's parent, a child, the entity itself, or the first one within
50 m. A found spline becomes one segment of its points, each the spline's origin plus the point's
local offset; an entity without one becomes a loose waypoint. The loose waypoints are then chained:
from each unvisited one, the next is the nearest within 500 m, with a penalty for turning and no
reversal sharper than about 100°. Segment endpoints within 10 m share a node id and list each other
as connected.

`IsRunwayEntity` rejects map descriptors, editor comments, vegetation, lights, lamps, poles, runway
markings, skid marks, borders and decals. It accepts a `Material` holding `runwayconcrete`, or
asphalt at a width of 20 m or more, or a prefab named runway, airstrip or taxiway, which gets 30 m
when its width is under 15 m. The width is the entity's `Width` above 1 m, and 30 m otherwise, so an
asphalt road whose `Width` is unset also passes the 20 m test.

The file lands in `$profile:TBD_Export/<map>/roads/runways.json` in the shared road dataset format,
with `roadClass` `runway` and ids `runway_<n>`.

`TBD_RunwaysExportPlugin` builds its own config and context and runs the export. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `SplineShapeEntity` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportRoads` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/`. No committed tool reads
  `runways.json`.
- Rules: `IsRunwayEntity` is the one place that decides what this class holds; the file keeps the
  field set every road class writes. Lines added stay ASCII, and the scripts compile only when
  Workbench loads `tbd-export`.
