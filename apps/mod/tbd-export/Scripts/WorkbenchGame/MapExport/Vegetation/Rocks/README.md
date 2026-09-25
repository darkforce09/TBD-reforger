# Rock export

Finds every placed rock, boulder and cliff in the open world in
[Workbench](/documentation_v2/glossary.md#workbench), measures how far each one sits above or below
the ground, and writes the records to `rocks.json` with a summary in `rocks_meta.json`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/Rocks/
├── TBD_MapExportRocks.c     the export: classifies rock prefabs, measures burial, writes both files
└── TBD_RocksExportPlugin.c  the standalone plugin that runs the rock export
```

## How it works

`TBD_MapExportRocks.Export` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by
default), taking each entity whose origin lies inside the world and in the cell, so no entity counts
twice. It sweeps twice so that no record is held in memory: the first pass counts, the second writes
each accepted entity straight to `$profile:TBD_Export/<map>/vegetation/rocks.json`. It keeps
entities whose world box lies between 0.05 m and 300 m on every axis and that `IsRockPrefab`
accepts.

For each rock it samples the ground height at the origin and at the four corners of the world box.
From those it derives the exposed peak height (box top minus lowest ground), the burial depth
(highest ground minus box bottom) and the exposure ratio (box top minus mean ground, over the box
height, clamped to 0–1), and a visibility: `fully_buried` when the top is within 0.1 m of the lowest
ground, and otherwise, by exposure ratio, `mostly_buried` under 0.2, `partially_exposed` under 0.7
and `fully_exposed` above. With the config's `m_bCullBuriedRocks` set, fully buried rocks are left
out.

`rocks.json` holds `mapName`, `worldSize`, the totals, `classCounts`, `materialCounts`,
`visibilityCounts` and the `rocks` array: prefab, class, material, variant, position, rotation,
scale, box size, `worldMinY` and `worldMaxY`, a `terrain` block with the samples and the derived
values, and the `apex` point. `rocks_meta.json` holds the method, the counts, whether culling
applied, the world size and the data file name.

`IsRockPrefab` rejects map descriptors, editor comments, and bush, tree, plant, vegetable, crop,
debris, stump, log, structure, building, prop, furniture, fence, wall, vehicle, road, powerline and
water prefabs. It accepts a prefab under a `Rocks/` folder, a class named rock, cliff or boulder, or
a file name that starts `rock_`, `cliff_`, `boulder_`, `scree_`, `pebble_` or `stone_` or names a
stone. The class is `cliff`, `scree`, `pebble`, `outcrop`, `sea_rock` or `boulder`, and the material
granite, limestone, sandstone, slate, basalt or generic, both from the name.

`TBD_RocksExportPlugin` builds its own config and context and runs the export; `Configure` opens the
config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no menu.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB` and `WorldEditorAPI.GetTerrainSurfaceY`.
- Used by: `TBD_MapExportVegetation` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/`, when the config's
  `m_bExportRocks` is set. No committed tool reads `rocks.json`.
- Rules: `IsRockPrefab` is the one place that decides what this layer holds, and it rejects the
  folders and names the other vegetation layers take; the file is written as JSON text by hand and
  keeps its field names. Lines added stay ASCII, and the scripts compile only when Workbench loads
  `tbd-export`.
