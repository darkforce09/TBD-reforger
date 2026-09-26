# Terrain elevation export

Samples the open world's ground height on a regular grid in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and writes it as an ASCII 16-bit height matrix
with a metadata file. The developer tools pack that matrix into the terrain's elevation model.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/DEM/
├── TBD_DEMExportPlugin.c  the standalone plugin that runs the DEM export
└── TBD_MapExportDEM.c     the export: samples ground height, writes the height matrix and metadata
```

## How it works

`TBD_MapExportDEM.Export` takes the shared export context and config from
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`:

1. The grid is `round(worldSize / m_fDemMetersPerPixel)` samples square (2 m per pixel by default,
   so 6400 × 6400 on a 12,800 m world). Sample `(px, py)` lies at `x = px · worldSize / (w − 1)`,
   `z = py · worldSize / (h − 1)`, so the grid spans both edges.
2. Each sample is `WorldEditorAPI.GetTerrainSurfaceY(x, z)`, encoded as
   `round((y − min) / (max − min) · 65535)` and clamped to 0–65535. `min` and `max` are the terrain
   bounds' height range when the minimum is below zero, and otherwise the fixed range −204.78 m to
   375.53 m (`DEFAULT_HMIN`, `DEFAULT_HMAX`).
3. It writes `heightmap.txt`: one text row per `py`, row 0 at `z = 0`, values separated by single
   spaces, flushed every 8,000 characters. A failed write deletes the file and fails the export.
4. It writes `dem_meta.json`: `method` `mod-getsurfacey-resample`, the width and height in pixels,
   the resolution, the encoding range, the sampled minimum and maximum, the terrain bounds,
   `rasterFile` `heightmap.txt`, `rasterFormat` `ascii-uint16-rows`, and `anchors`: the surface
   height probed at the centre and at the four points 15 % and 85 % along each axis, plus three
   fixed bridgehead points when the world is at least 12,800 m wide.

Both files land in `$profile:TBD_Export/<map>/terrain/`, where `<map>` is the lowercase map name the
context derives from the world path (`eden` becomes `everon`) or the config's override.

`TBD_DEMExportPlugin` builds its own `TBD_MapExportConfig` and context, and runs the export;
`Configure` opens the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench
lists it in no menu. `TBD_MapExportPlugin` in
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` calls `TBD_MapExportDEM.Export` as
its first layer, and its attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; Workbench's
  `WorldEditorAPI.GetTerrainSurfaceY` and `FileIO`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  The two files are the input of
  `world raw-u16-dem-png --raster <matrix> --meta <meta> --out <png>`, run through
  `cargo run -p developer-tools --bin world --`
  (`tools_v2/developer-tools/src/world_export_pipeline/export_preparation/dem_elevation.rs`). It
  reads `widthPx`, `heightPx` and the `heightRange*` keys, and writes the 16-bit PNG that
  `assets_v2/terrains/everon/dem/` commits, with an `elevation.dem` beside it. No committed command
  moves the files out of the profile; the operator passes their paths.
- Rules: the encoding range written to `dem_meta.json` is the one the samples were quantised
  against, and the converter falls back to the same fixed range when the keys are absent; the matrix
  holds exactly `widthPx × heightPx` values, which the converter checks; lines added stay ASCII.
  `cargo xtask mod compile` compiles only the framework addon, so these scripts compile only when
  Workbench loads `tbd-export`.

## Related documentation

- [Everon elevation model](/assets_v2/terrains/everon/dem/README.md) — the committed image this
  export feeds, and its consumers.
- [Elevation model](/apps/website/map-engine/src/world/terrain/dem/README.md) — how the map engine
  decodes and samples it.
- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  the elevation row order and the missing staging step.
