# Tree export

Finds every standing tree in the open world in [Workbench](/documentation_v2/glossary.md#workbench)
and writes each one's class, species, position, rotation, scale and size to `trees.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/Trees/
├── TBD_MapExportTrees.c     the export: classifies tree prefabs and writes `trees.json`
└── TBD_TreesExportPlugin.c  the standalone plugin that runs the tree export
```

## How it works

`TBD_MapExportTrees.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default), taking each entity whose origin lies inside the world and in the cell, so no entity counts
twice. It sweeps twice so that no record is held in memory: the first pass counts, the second writes
each accepted entity straight to `$profile:TBD_Export/<map>/vegetation/trees.json`. It keeps
entities between 1.2 m and 60 m tall that `IsTreePrefab` accepts. The file holds `mapName`,
`worldSize`, `totalTrees`, `classCounts`, `speciesCounts` and the `trees` array, each with a running
`id`, prefab, tree class, species, variant, position, `GetAngles` rotation, scale and world-box
size.

`IsTreePrefab` rejects map descriptors, editor comments, and bush, plant, vegetable, crop, debris,
stump, log, woodpile, root base, fallen, branch, stem, rock, structure, building, prop, furniture
and water prefabs, and file names starting `b_` or `p_`. It accepts a prefab under a `Tree/` folder,
a file name starting `t_` or `tree_` or naming a common genus, or anything else under `Vegetation/`.
Known genera map to a species and a class (`conifer`, `deciduous` or `palm`); other names give
`<genus>_<species>` and a class from keywords, `deadwood` for a dead variant.

`TBD_TreesExportPlugin` builds its own config and context and runs the export; `Configure` opens the
config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportVegetation` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/`, when the config's
  `m_bExportTrees` is set. No committed tool reads `trees.json`.
- Rules: `IsTreePrefab` is the one place that decides what this layer holds, and it rejects the
  folders and names the other vegetation layers take; the file is written as JSON text by hand and
  keeps its field names. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.
