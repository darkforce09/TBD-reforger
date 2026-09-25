# Terrain export

The terrain layers of the map export: ground height, a cartographic rasterization, the road network
and water, each read from the open world in [Workbench](/documentation_v2/glossary.md#workbench) and
written as text, JSON or image files to the export profile.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/
├── DEM/        ground height sampled into an ASCII 16-bit matrix and its metadata
├── Roads/      the road network, one JSON file per road class plus a junction graph
├── Satellite/  the engine's cartographic rasterization of the world, as a TGA image
└── Water/      water class and depth rasters, and river, lake and pond records
```

## How it works

Every layer follows one pattern. An exporter class takes the `TBD_MapExportContext` and
`TBD_MapExportConfig` of `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`, reads the
world through Workbench's `WorldEditorAPI` and the engine's world queries, and writes into
`<destination>/<map>/<layer>/`: the destination is the config's `m_sDestinationDir`,
`$profile:TBD_Export/` by default; `<map>` is the lowercase map name taken from the world path
(`eden` becomes `everon`) or the config's override; the layer is `terrain`, `satellite`, `roads` or
`water`. Each folder also holds a standalone plugin class that builds its own config and context and
runs its layer.

`TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs the
elevation, rasterization, road and water raster layers in that order, each behind its config switch.
Every plugin's `[WorkbenchPluginAttribute]`, here and there, is commented out, so Workbench lists
none of them in its menus, and no committed MCP handler or `cargo xtask` command starts them.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the export core in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`;
  Workbench's `WorldEditorAPI`; the engine's `MapDataExporter`, water surface query, road network,
  river, spline shape and entity query classes.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  Outside the addon, the elevation files feed the world tool's `raw-u16-dem-png` command
  (`tools_v2/developer-tools/src/world_export_pipeline/`) and the water rasters feed the map tool's
  `water` command (`tools_v2/developer-tools/src/map_raster_pipeline/`), which reads them under
  other names from the export scratch; no committed tool reads the road or rasterization files, and
  no committed step moves any of them out of the profile.
- Rules: the layers write only below the config's destination folder, through `TBD_MapExportPaths`;
  sources stay ASCII. `cargo xtask mod compile` compiles only the framework addon, so these scripts
  compile only when Workbench loads `tbd-export`.

## Related documentation

- [Everon terrain dataset](/assets_v2/terrains/everon/README.md) — the committed terrain data and
  the tools that build it.
