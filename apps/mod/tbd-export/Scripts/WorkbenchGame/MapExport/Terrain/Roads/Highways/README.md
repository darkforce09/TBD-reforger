# Highway export

Finds the open world's highways and major arterial roads in
[Workbench](/documentation_v2/glossary.md#workbench) and writes their centrelines, widths and
endpoint connections to `highways.json`. The folder also holds a diagnostic plugin that prints what
the engine's road network holds.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/Highways/
├── TBD_HighwaysExportPlugin.c    the standalone plugin that runs the highway export
├── TBD_MapExportHighways.c       the export: finds highway entities, writes their segments
└── TBD_RoadNetworkProbePlugin.c  a diagnostic plugin that logs the road network and three roads
```

## How it works

`TBD_MapExportHighways.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default) and keeps each entity whose origin falls in the cell and that `IsHighwayEntity` accepts: no
map descriptor or editor comment, no vegetation, prop, fence, decal or narrow pavement prefab, and
either a `Material` with a dashed centre line or `road_asphalt_e_01`, `road_asphalt_e_02` at 7.5 m
or wider, `road_asphalt_e_03` at 8 m or wider, or a prefab named highway, main road or wide asphalt.
The width is the entity's `Width`, at least 8 m for the named cases.

It then looks for the engine's `RoadNetworkManager` through `GetGame().GetAIWorld()` or a
`ChimeraAIWorld` in the editor's entity tree. With one, each highway entity takes the closest
`BaseRoad` within 25 m, once per road, and its `GetPoints` become a segment. Without one, or when
that yields nothing, the entities themselves are chained: from each unvisited one, the next is the
nearest within 200 m, with a penalty for turning and no reversal sharper than about 100°.

Segments whose endpoints lie within 5 m share a node id and list each other as connected. The file
lands in `$profile:TBD_Export/<map>/roads/highways.json` in the shared road dataset format, with
`roadClass` `highway_paved` and ids `highway_<n>`.

`TBD_RoadNetworkProbePlugin` makes the same context, reports whether a game and an AI world are
running, searches the editor tree for the AI world, and for the first three `RoadEntity` objects
prints their `Width`, `Material` and, with a road network, the closest `BaseRoad` and its first five
points. It writes no file.

Both plugins build their own config and context. Their `[WorkbenchPluginAttribute]` blocks are
commented out, so Workbench lists neither in a menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `ChimeraAIWorld`, `RoadNetworkManager`, `BaseRoad` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportRoads` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/`, which merges the segments
  into the network-wide junction graph. No committed tool reads `highways.json`.
- Rules: `IsHighwayEntity` is the one place that decides what a highway is; the file keeps the field
  set every road class writes. Lines added stay ASCII, and the scripts compile only when Workbench
  loads `tbd-export`.
