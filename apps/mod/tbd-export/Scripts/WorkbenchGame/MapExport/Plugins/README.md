# Full map export plugin

The one plugin that runs every map-export layer in a single pass over the world open in
[Workbench](/documentation_v2/glossary.md#workbench): terrain, vegetation, objects and
infrastructure, locations and anchors, and the prefab and arsenal registries.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/
└── TBD_MapExportPlugin.c  `TBD_MapExportPlugin`: runs every enabled layer in order
```

## How it works

`TBD_MapExportPlugin` is a `WorkbenchPlugin`. `Configure` opens its `TBD_MapExportConfig` in a
dialog; `Run` calls `ExecuteExport`, which builds a `TBD_MapExportContext`, creates
`<destination>/<map>/`, and runs each layer whose switch is on, in this order:

| Step | Switch | Exporter | Folder |
|---|---|---|---|
| 1 | `m_bExportDEM` | `TBD_MapExportDEM.Export` | `Terrain/DEM/` |
| 2 | `m_bExportSatellite` | `TBD_MapExportSatellite.Export` | `Terrain/Satellite/` |
| 3 | `m_bExportRoads` | `TBD_MapExportRoads.Export` | `Terrain/Roads/` |
| 4 | `m_bExportWater` | `TBD_MapExportWater.Export` | `Terrain/Water/` |
| 5 | `m_bExportVegetation` | `TBD_MapExportVegetation.Export` | `Vegetation/` |
| 6 | `m_bExportObjects` | `TBD_MapExportObjects` | `Objects/` |
| 7–10 | `m_bExportFences`, `m_bExportBridges`, `m_bExportAviation`, `m_bExportPowerlines` | `TBD_MapExportFences`, `…Bridges`, `…Aviation`, `…Powerlines` | `Objects/Infrastructure/` |
| 11 | `m_bExportLocations` | `TBD_MapExportLocations` | `Locations/` |
| 12 | `m_bExportAnchors` | `TBD_MapExportAnchors` | `Locations/Anchors/` |
| 13 | `m_bExportPrefabs` | `TBD_MapExportPrefabs` | `Registry/` |
| 14 | `m_bExportArsenal` | `TBD_MapExportArsenal` | `Registry/` |

The folders are relative to `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/`. A failed layer
marks the run failed and the next layer still runs; `ExecuteExport` returns true only when every
layer it ran succeeded and logs the elapsed time. The building blueprint exporter
(`Objects/Buildings/`), the props exporter (`Objects/Props/`) and the full world-object export
(`Objects/TBD_WorldFullExportPlugin.c`) are not steps of this plugin.

Its `[WorkbenchPluginAttribute]` (menu "Export All Map Data (Full Suite)", category `TBD`) is
commented out, so Workbench lists it in no menu, and no committed MCP handler or `cargo xtask`
command calls `ExecuteExport`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the export core in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`, and
  the exporter classes of the `Terrain/`, `Vegetation/`, `Objects/`, `Locations/` and `Registry/`
  folders beside it.
- Used by: nothing; no script, handler or tool refers to `TBD_MapExportPlugin`.
- Rules: each layer runs behind its own config switch and writes its own category folder, so a
  layer can be switched off without touching the others; lines added stay ASCII.
  `cargo xtask mod compile` compiles only the framework addon, so this script compiles only when
  Workbench loads `tbd-export`.
