# Map export core

The shared base of every map-export layer: the settings a run takes, the handle on the world open in
[Workbench](/documentation_v2/glossary.md#workbench), and the path and JSON helpers every exporter
writes through.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/
├── TBD_MapExportConfig.c   `TBD_MapExportConfig`: the destination, map name and per-layer switches
├── TBD_MapExportContext.c  `TBD_MapExportContext`: the open world, its bounds, map name and prefabs
└── TBD_MapExportPaths.c    `TBD_MapExportPaths`, `TBD_MapExportJson`: output folders, JSON text
```

## How it works

Every export follows one sequence. A plugin builds a `TBD_MapExportConfig`, whose `[Attribute]`
fields are what its `Configure` dialog (`Workbench.ScriptDialog`) edits, and a
`TBD_MapExportContext`, then hands both to an exporter class's `Export`:

1. `TBD_MapExportContext.Init` takes the `WorldEditor` module and its `WorldEditorAPI`, finds the
   `BaseWorld` through the first editor entity that resolves to one, and reads the terrain bounds;
   when `GetTerrainBounds` fails it uses 0 to 12,800 m across and −204.78 m to 375.53 m in height.
   The world size is the larger of the bounds' x and z maxima (12,800 m when that is not positive).
2. `DeriveMapName` turns the world path into the map name: the parent folder of the `.ent` file, or
   the file name when the parent is `worlds`, `Worlds` or `MP`, lowercased, with `eden` read as
   `everon` and `everon` as the fallback. `GetMapName` returns the config's `m_sMapNameOverride`,
   lowercased, whenever it is set.
3. `GetEntityResourceName` (also `ResolvePrefab`) names an entity's prefab from its prefab data, or
   from its editor source's ancestor when the entity has none.
4. The exporter writes through `TBD_MapExportPaths.BuildCategoryPath(destination, map, category,
   file)`, which creates `<destination>/<map>/<category>/` folder by folder (a `$profile:` prefix
   included) and returns the file path; an empty map name becomes `default`, and an empty
   destination `$profile:TBD_Export/`. `ResolveNativeOsPath` maps a `$profile:` path to the Proton
   prefix's Windows path (`PROFILE_WIN`) for the engine calls that need a native path.
5. Records are built as JSON text with `TBD_MapExportJson.Escape` and written with
   `TBD_MapExportJson.Write`, which logs and returns false when a write fails so the exporter can
   stop.

### Settings

| Field | Default | Read by |
|---|---|---|
| `m_sDestinationDir` | `$profile:TBD_Export/` | every exporter, as the root of `<map>/<category>/` |
| `m_sMapNameOverride` | empty (derive from the world path) | `TBD_MapExportContext.GetMapName` |
| `m_bExportDEM`, `m_bExportSatellite`, `m_bExportRoads`, `m_bExportWater` | on | `TBD_MapExportPlugin`, one terrain layer each |
| `m_bExportHighways`, `m_bExportPavedRoads`, `m_bExportDirtRoads`, `m_bExportTracks`, `m_bExportPaths`, `m_bExportRunways` | on | the road coordinator, one class each |
| `m_bExportVegetation` | on | `TBD_MapExportPlugin` |
| `m_bExportTrees`, `m_bExportRocks`, `m_bExportBushes`, `m_bExportPlants`, `m_bExportCrops`, `m_bExportStumps` | on | the vegetation coordinator, one layer each |
| `m_bCullBuriedRocks` | off | the rocks layer |
| `m_bExportObjects`, `m_bExportFences`, `m_bExportBridges`, `m_bExportAviation`, `m_bExportPowerlines` | on | `TBD_MapExportPlugin`, the objects and infrastructure exporters |
| `m_bExportLocations`, `m_bExportAnchors` | on | `TBD_MapExportPlugin`, the locations and anchors exporters |
| `m_bExportPrefabs`, `m_bExportArsenal` | on | `TBD_MapExportPlugin`, the registry exporters |
| `m_fDemMetersPerPixel` | 2.0 | the elevation and water layers |
| `m_fWaterMetersPerPixel` | 1.0 | the water layer |
| `m_fObjectChunkSizeM` | 512.0 | every exporter that sweeps the world in square cells |

`TBD_ExportPaths` and `TBD_ExportJson` are empty subclasses of the two helpers under shorter
names; `TBD_RegistryItemsExportPlugin` in
`apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c` writes through
`TBD_ExportJson`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's `WorldEditor` module and `WorldEditorAPI`, `Workbench.ScriptDialog`, the
  engine's `BaseWorld`, `EntityPrefabData` and `FileIO`.
- Used by: every exporter and plugin under `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/`
  (`Terrain/`, `Vegetation/`, `Objects/`, `Locations/`, `Registry/`, `Plugins/`), and
  `TBD_RegistryItemsExportPlugin` through the `TBD_ExportJson` name.
- Rules: an exporter writes only below the config's destination, through `BuildCategoryPath`, so
  every output lands in `<destination>/<map>/<category>/`; `$profile:` paths stay virtual except
  where `ResolveNativeOsPath` is required. Lines added stay ASCII. `cargo xtask mod compile`
  compiles only the framework addon, so these scripts compile only when Workbench loads
  `tbd-export`.
