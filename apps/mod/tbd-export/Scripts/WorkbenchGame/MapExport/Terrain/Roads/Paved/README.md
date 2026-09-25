# Paved road export

Finds the open world's secondary asphalt and cobblestone roads in
[Workbench](/documentation_v2/glossary.md#workbench) and writes their centrelines, widths and
endpoint connections to `roads_paved.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/Paved/
├── TBD_MapExportPavedRoads.c     the export: finds paved road entities, writes their segments
└── TBD_PavedRoadsExportPlugin.c  the standalone plugin that runs the paved road export
```

## How it works

`TBD_MapExportPavedRoads.Export` works as the highway export does: a sweep of `m_fObjectChunkSizeM`
cells (512 m by default), a classifier, then either the closest `BaseRoad` within 25 m of each
accepted entity from the engine's `RoadNetworkManager`, or, when there is no road network or it
yields nothing, a chain of the entities themselves (next point within 200 m, turns penalised).
Endpoints within 5 m share a node.

`IsPavedRoadEntity` rejects map descriptors, editor comments, vegetation, props, fences, decals and
pavement prefabs; the trail, dirt, forest, dirt-track and concrete-panel materials; runway materials
and prefabs, and any asphalt road with a `Width` of 20 m or more. It accepts a `Material` holding
asphalt or cobblestone, a prefab named road asphalt, road paved or cobblestone, or a `RoadEntity`
with no material. The default width is 6 m. The classifier holds no highway exclusion, so an asphalt
highway entity lands here as well as in the highway export.

The file lands in `$profile:TBD_Export/<map>/roads/roads_paved.json` in the shared road dataset
format, with `roadClass` `road_paved` and ids `road_paved_<n>`.

`TBD_PavedRoadsExportPlugin` builds its own config and context and runs the export. Its
`[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `ChimeraAIWorld`, `RoadNetworkManager`, `BaseRoad` and `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportRoads` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/`. No committed tool reads
  `roads_paved.json`.
- Rules: `IsPavedRoadEntity` is the one place that decides what a paved road is; the file keeps the
  field set every road class writes. Sources stay ASCII, and they compile only when Workbench loads
  `tbd-export`.
