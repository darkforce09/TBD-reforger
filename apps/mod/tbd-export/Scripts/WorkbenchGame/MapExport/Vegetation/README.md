# Vegetation export

The vegetation layers of the map export: trees, rocks, bushes, wild plants, crops, and stumps with
other deadwood, each found among the placed entities of the open world in
[Workbench](/documentation_v2/glossary.md#workbench) and written as one JSON file per layer, with a
summary file for the whole run.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/
├── Bushes/                       bushes, written to `bush.json`
├── Crops/                        garden crops and vegetable rows, written to `crops.json`
├── Plants/                       wild plants, weed strips and seaweed, written to `plants.json`
├── Rocks/                        rocks and cliffs with burial measures, written to `rocks.json`
├── Stumps/                       stumps, logs, root bases and woodpiles, written to `stumps.json`
├── TBD_MapExportVegetation.c     runs every layer and writes `vegetation_meta.json`
├── TBD_VegetationExportPlugin.c  the standalone plugin that runs the whole vegetation export
└── Trees/                        standing trees by class and species, written to `trees.json`
```

## How it works

```text
TBD_VegetationExportPlugin / TBD_MapExportPlugin
        |
        v
TBD_MapExportVegetation.Execute
        |-- m_bExportTrees  -> TBD_MapExportTrees  -> trees.json
        |-- m_bExportRocks  -> TBD_MapExportRocks  -> rocks.json, rocks_meta.json
        |-- m_bExportBushes -> TBD_MapExportBushes -> bush.json
        |-- m_bExportPlants -> TBD_MapExportPlants -> plants.json
        |-- m_bExportCrops  -> TBD_MapExportCrops  -> crops.json
        |-- m_bExportStumps -> TBD_MapExportStumps -> stumps.json
        v
all succeeded -> vegetation_meta.json
```

Every layer works the same way. It sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m
by default), takes each entity whose origin lies in the cell and inside the world, resolves its
prefab, and keeps it when the layer's classifier accepts the prefab path and file name and its world
box passes the layer's size filter, where it has one. Each classifier also rejects the folders and
names the other layers take. Every kept entity becomes one record with its prefab, classification,
position, `GetAngles` rotation, scale and box size, and every file carries per-class counts beside
the records. All files land in `$profile:TBD_Export/<map>/vegetation/`.

`TBD_MapExportVegetation` writes `vegetation_meta.json` only when every layer it ran succeeded:
`method` `mod-vegetation-suite-export`, the map name, the world size, which layers the config
enabled, and the run time.

`TBD_VegetationExportPlugin` builds its own config and context and runs the coordinator; `Configure`
opens the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in
no menu. `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`
runs the coordinator when `m_bExportVegetation` is set, and its attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB` and Workbench's `WorldEditorAPI.GetTerrainSurfaceY`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads these files: the vegetation in the committed object chunks
  (`assets_v2/terrains/everon/objects/`) comes from the full world-object export in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/` and `world build-objects`.
- Rules: a layer's classifier decides its membership, keyed on prefab paths and file names; each
  file is written as JSON text by hand, flushed every 8,000 characters. Lines added stay ASCII, and
  the scripts compile only when Workbench loads `tbd-export`.

## Related documentation

- [Everon terrain dataset](/assets_v2/terrains/everon/README.md) — the committed terrain data,
  object chunks included, and the tools that build it.
