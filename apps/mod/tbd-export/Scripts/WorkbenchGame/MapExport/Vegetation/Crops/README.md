# Crop export

Finds every placed garden crop and vegetable row in the open world in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and writes each one's crop, layout, position,
rotation, scale and size to `crops.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/Crops/
├── TBD_CropsExportPlugin.c  the standalone plugin that runs the crop export
└── TBD_MapExportCrops.c     the export: classifies crop prefabs and writes `crops.json`
```

## How it works

`TBD_MapExportCrops.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default), taking each entity whose origin lies inside the world and in the cell, so no entity counts
twice. It keeps the entities that `IsCropPrefab` accepts, holds one record per entity (prefab, crop
type, variant, layout, position, `GetAngles` rotation, scale and world-box width, height and depth)
and then writes `$profile:TBD_Export/<map>/vegetation/crops.json`: `mapName`, `worldSize`,
`totalCrops`, `cropCounts`, `layoutCounts` and the `crops` array.

`IsCropPrefab` rejects map descriptors, editor comments, and bush, plant, tree, rock, prop,
decoration, flowerpot, stump, log and deadwood prefabs. It accepts a prefab under a `Vegetables/`
folder, or one whose path says crop and whose file name holds `Crop`. The layout is `shortline`,
`longline`, `line` or `patch` from the file name; the crop type is tomato, potato, cabbage, capsicum
or pumpkin when the name says so, and otherwise the first part of the name without `Crop`. No height
filter applies.

`TBD_CropsExportPlugin` builds its own config and context and runs the export; `Configure` opens the
config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportVegetation` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/`, when the config's
  `m_bExportCrops` is set. No committed tool reads `crops.json`.
- Rules: `IsCropPrefab` is the one place that decides what this layer holds, and it rejects the
  folders and names the other vegetation layers take; the file is written as JSON text by hand and
  keeps its field names. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.
