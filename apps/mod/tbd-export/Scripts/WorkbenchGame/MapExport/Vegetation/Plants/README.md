# Wild plant export

Finds every placed wild plant, weed strip and seaweed in the open world in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and writes each one's species, environment,
position, rotation, scale and size to `plants.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/Plants/
├── TBD_MapExportPlants.c     the export: classifies plant prefabs and writes `plants.json`
└── TBD_PlantsExportPlugin.c  the standalone plugin that runs the plant export
```

## How it works

`TBD_MapExportPlants.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default), taking each entity whose origin lies inside the world and in the cell, so no entity counts
twice. It keeps the entities between 0.1 m and 4 m tall that `IsPlantPrefab` accepts, holds one
record per entity (prefab, species, variant, environment, position, `GetAngles` rotation, scale and
world-box width, height and depth) and then writes
`$profile:TBD_Export/<map>/vegetation/plants.json`: `mapName`, `worldSize`, `totalPlants`,
`speciesCounts`, `environmentCounts` and the `plants` array.

`IsPlantPrefab` rejects map descriptors, editor comments, and bush, tree, vegetable, crop, rock,
prop, decoration, flowerpot, stump, log and deadwood prefabs. It accepts a prefab under a `Plant/`
folder, or one under `Vegetation/` whose file name starts `p_` or holds `CurbsideWeeds`. Curbside
weeds get the species `curbside_weeds`; other names give `<genus>_<species>`, and `fucus` becomes
`fucus_vesiculosus` with the environment `marine`; every other plant is `terrestrial`.

`TBD_PlantsExportPlugin` builds its own config and context and runs the export; `Configure` opens
the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no
menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportVegetation` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/`, when the config's
  `m_bExportPlants` is set. No committed tool reads `plants.json`.
- Rules: `IsPlantPrefab` is the one place that decides what this layer holds, and it rejects the
  folders and names the other vegetation layers take; the file is written as JSON text by hand and
  keeps its field names. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.
