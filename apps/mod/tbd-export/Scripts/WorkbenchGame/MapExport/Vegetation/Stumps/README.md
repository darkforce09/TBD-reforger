# Stump and deadwood export

Finds every placed tree stump, cut trunk, log, root base and woodpile in the open world in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and writes each one's kind, wood species,
position, rotation, scale, size and diameter to `stumps.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/Stumps/
├── TBD_MapExportStumps.c     the export: classifies stump and log prefabs and writes `stumps.json`
└── TBD_StumpsExportPlugin.c  the standalone plugin that runs the stump export
```

## How it works

`TBD_MapExportStumps.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default), taking each entity whose origin lies inside the world and in the cell, so no entity counts
twice. It keeps the entities between 0.05 m and 5 m tall that `IsStumpPrefab` accepts, holds one
record per entity (prefab, stump type, species, variant, position, `GetAngles` rotation, scale and
world-box width, height and depth) and then writes
`$profile:TBD_Export/<map>/vegetation/stumps.json`: `mapName`, `worldSize`, `totalStumps`,
`stumpTypeCounts`, `speciesCounts` and the `stumps` array. Each record also carries a `diameter`,
the larger of the box's width and depth.

`IsStumpPrefab` rejects map descriptors, editor comments, and bush, plant, vegetable, crop, rock,
furniture, crate, garbage, construction, structure, industrial and pallet prefabs. It accepts a
forest woodpile prop, a tree debris prefab, or a prefab whose name says stump, cut trunk, wood log,
woodpile, tree base, root base or stem root. The type is `wood_pile`, `trunk_segment`, `cut_stump`,
`rooted_base`, `fallen_deadwood` or `weathered_stump`, from the name; the species comes from the
tree genus in the name.

`TBD_StumpsExportPlugin` builds its own config and context and runs the export; `Configure` opens
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
  `m_bExportStumps` is set. No committed tool reads `stumps.json`.
- Rules: `IsStumpPrefab` is the one place that decides what this layer holds, and it rejects the
  folders and names the other vegetation layers take; the file is written as JSON text by hand and
  keeps its field names. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.
