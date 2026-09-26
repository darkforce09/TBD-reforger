# Terrain rasterization export

Renders the open world into a flat-coloured cartographic image with Enfusion's `MapDataExporter` in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench), and records the call and its parameters in a
metadata file. The image is a shaded land, sea, forest and other-area map, not a photograph of the
ground textures.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Satellite/
├── TBD_MapExportSatellite.c     the export: `MapDataExporter.ExportRasterization` and its metadata
└── TBD_SatelliteExportPlugin.c  the standalone plugin that runs the export
```

## How it works

`TBD_MapExportSatellite.Export` sets six colours on a new `MapDataExporter` (bright and dark land,
bright and dark ocean, forest area, other area), then calls `ExportRasterization` on the context's
world path (`worlds/Eden/Eden.ent` when the path is empty) with fixed parameters: every scale and
intensity 1.0, a 20 m depth blend, generator areas included. The engine writes the image itself, so
the target is a native path: `TBD_MapExportPaths.ResolveNativeOsPath` turns
`$profile:TBD_Export/<map>/satellite/rasterization.tga` into the Workbench profile's path inside the
Proton prefix (`TBD_MapExportPaths.PROFILE_WIN`).

It then writes `satellite_meta.json` beside the image: `method` `mod-maprasterization-export`, the
world path, the engine's return code and its message (from `SCR_WorldMapExportTool`), the output
path, the terrain bounds and the parameters. The metadata is written whatever the return code; the
export succeeds only when the code is 0.

`TBD_SatelliteExportPlugin` builds its own config and context and runs the export; `Configure` opens
the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no
menu. `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs
this export as its second layer, and its attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `MapDataExporter` and `SCR_WorldMapExportTool.GetReportMessage`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads `rasterization.tga` or `satellite_meta.json`; the satellite image the map
  engine streams (`assets_v2/terrains/everon/satellite/`) is built by the map raster pipeline from
  the game's own textures.
- Rules: the image path handed to `MapDataExporter` is a native path, never a `$profile:` path; the
  Proton prefix path is the constant in `TBD_MapExportPaths`. Lines added stay ASCII, and the
  scripts compile only when Workbench loads `tbd-export`.

## Related documentation

- [Everon satellite image](/assets_v2/terrains/everon/satellite/README.md) — the basemap the map
  engine streams, and how it is built.
