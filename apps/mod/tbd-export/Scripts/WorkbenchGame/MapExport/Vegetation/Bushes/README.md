# Bush export

Finds every placed bush in the open world in [Workbench](/documentation_v2/glossary.md#workbench)
and writes each one's species, position, rotation, scale and size to `bush.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/Bushes/
├── TBD_BushesExportPlugin.c  the standalone plugin that runs the bush export
└── TBD_MapExportBushes.c     the export: classifies bush prefabs and writes `bush.json`
```

## How it works

`TBD_MapExportBushes.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default), taking each entity whose origin lies inside the world and in the cell, so no entity counts
twice. It keeps the entities between 0.2 m and 6 m tall that `IsBushPrefab` accepts, holds one
record per entity (prefab, species, variant, position, `GetAngles` rotation, scale and world-box
width, height and depth) and then writes `$profile:TBD_Export/<map>/vegetation/bush.json`:
`mapName`, `worldSize`, `totalBushes`, `speciesCounts` and the `bushes` array.

`IsBushPrefab` rejects map descriptors, editor comments, and plant, vegetable, tree, stump, log,
deadwood and water prefabs. It accepts a prefab under a `Bush/` folder, or one under `Vegetation/`
whose file name starts `b_`. The species is the first two parts of the name after `b_`
(`<genus>_<species>`), the variant its last part.

`TBD_BushesExportPlugin` builds its own config and context and runs the export; `Configure` opens
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
  `m_bExportBushes` is set. No committed tool reads `bush.json`.
- Rules: `IsBushPrefab` is the one place that decides what this layer holds, and it rejects the
  folders and names the other vegetation layers take; the file is written as JSON text by hand and
  keeps its field names. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.
