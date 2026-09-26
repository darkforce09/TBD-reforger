# Track export

Finds the open world's forestry and farm tracks in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and writes their centrelines, widths and
endpoint connections to `tracks.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/Tracks/
├── TBD_MapExportTracks.c     the export: finds track entities, writes their segments
└── TBD_TracksExportPlugin.c  the standalone plugin that runs the track export
```

## How it works

`TBD_MapExportTracks.Export` sweeps the world in `m_fObjectChunkSizeM` cells (512 m by default) and
runs `IsTrackEntity` on each entity whose origin falls in the cell. For each accepted entity it
looks for a spline shape: the entity's parent, a child, the entity itself, or the first one within
25 m. A found spline becomes one segment of its points, each the spline's origin plus the point's
local offset; an entity without one becomes a loose waypoint. The loose waypoints are then chained:
from each unvisited one, the next is the nearest within 200 m, with a penalty for turning and no
reversal sharper than about 100°. Segment endpoints within 5 m share a node id and list each other
as connected.

`IsTrackEntity` rejects map descriptors, editor comments, vegetation, props, fences, footprints,
patches and pavement prefabs; decal materials other than dirt tracks; trail, `road_dirt_01`, asphalt
and cobblestone materials; and asphalt, paved or highway prefabs. It accepts a `Material` holding
`road_forest`, `dirttracks`, `road_dirt_02` or `road_concretepanel`, or a prefab named tractor,
field track, forest track, two track or dirt tracks. The width is the entity's `Width`, 3.5 m when
unset.

The file lands in `$profile:TBD_Export/<map>/roads/tracks.json` in the shared road dataset format,
with `roadClass` `track` and ids `track_<n>`.

`TBD_TracksExportPlugin` builds its own config and context and runs the export. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `SplineShapeEntity` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportRoads` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/`. No committed tool reads
  `tracks.json`.
- Rules: `IsTrackEntity` is the one place that decides what this class holds; the file keeps the
  field set every road class writes. Lines added stay ASCII, and the scripts compile only when
  Workbench loads `tbd-export`.
