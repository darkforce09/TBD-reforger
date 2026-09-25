# Map export

The [Workbench](/documentation_v2/glossary.md#workbench) scripts that read a terrain's data out of
the world open in the editor: ground height, rasters, roads and water, vegetation, placed objects
and buildings, places, and prefab catalogs. They write text, JSON and image files to the Workbench
profile, where the developer tools pick up the ones the platform's terrain data is built from.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/
├── Core/        the shared config, world context, output paths and JSON helpers
├── Locations/   named towns and villages, and landmark anchors with their heights
├── Objects/     the full world-object export, classified objects, buildings, infrastructure
├── Plugins/     the plugin that runs every layer in one pass
├── Registry/    a placed-prefab taxonomy and a coded weapon and magazine list
├── Terrain/     ground height, the cartographic raster, the road network and water
└── Vegetation/  trees, rocks, bushes, plants, crops and stumps
```

## How it works

Every layer follows one pattern. An exporter class takes a `TBD_MapExportConfig` and a
`TBD_MapExportContext` from `Core/`, reads the open world through Workbench's `WorldEditorAPI` and
the engine's world queries, and writes into `<destination>/<map>/<category>/`: the destination is
`$profile:TBD_Export/` unless the config names another, and `<map>` is the lowercase map name taken
from the world path (`eden` becomes `everon`) or the config's override. Most layers sweep the world
in square cells of `m_fObjectChunkSizeM` (512 m by default).

```text
plugin (Run / Configure dialog)  or  Net API request (cargo xtask mcp wbcall)
        │
        ▼
TBD_MapExportConfig + TBD_MapExportContext (Core/)
        │
        ▼
exporter class ──▶ $profile:TBD_Export/<map>/<category>/…
                   $profile:TBD_WorldExport_full.jsonl (the full world-object export)
```

Three kinds of entry point exist:

- `TBD_MapExportPlugin` in `Plugins/` runs every enabled layer in order, each behind its config
  switch.
- Each layer folder has a standalone `WorkbenchPlugin` that builds its own config and context and
  runs that layer alone.
- `EMCP_WB_TbdBlueprint` in `Objects/Buildings/` is a Net API handler: `cargo xtask mcp wbcall
  EMCP_WB_TbdBlueprint '{"action":…}'` runs the building recon, blueprint extraction, voxel dump,
  probe and parity actions, and the world sight-parity sampler.

Every `[WorkbenchPluginAttribute]` under this folder is commented out, so Workbench lists none of
the plugins in its menus. No committed MCP handler or `cargo xtask` command runs a plugin, and the
Net API handler is the one entry point that runs from outside the editor.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's `WorldEditor` module and `WorldEditorAPI`; the engine's world, road,
  water, trace and file classes; nothing from `tbd-framework`.
- Used by: the developer tools, which read these files from the profile or from the export scratch
  under `assets_v2/scratch/<terrain>/`:
  - `world copy-export-profile --full` stages the full world-object export for
    `cargo xtask map export-terrain`, which builds `assets_v2/terrains/<terrain>/objects/` and
    `roads/`; without `--full` it reads a `TBD_WorldExport_subregion.jsonl` that no script here
    writes;
  - `world raw-u16-dem-png` packs the elevation files into the terrain's elevation model;
  - the map tool's `water` command (`tools_v2/developer-tools/src/map_raster_pipeline/`) reads
    water rasters from the export scratch, under file names the water layer does not write;
  - `cargo xtask map ingest-blueprints`, `map blueprint-from-voxels`, `map parity-report`,
    `map world-los` and `map instances-verify` read the building and parity outputs;
  - `cargo xtask mcp wbcall` calls the Net API handler.
  No committed tool reads the road, vegetation, classified object, infrastructure, location,
  anchor or catalog files.
- Rules: a layer writes only below the config's destination, through `TBD_MapExportPaths`; the
  addon holds no copy of a framework class; sources stay ASCII. `cargo xtask mod compile` compiles
  only the framework addon
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`), so these scripts compile only when
  Workbench loads `tbd-export`.

## Related documentation

- [Terrain datasets](/assets_v2/terrains/README.md) — the committed terrain data these exports
  feed, and the tools between them.
- [World export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md) — staging
  the full export and building the object and road data.
- [Blueprint compiler](/tools_v2/developer-tools/src/blueprint/README.md) — the offline building
  blueprint steps.
- [Map commands](/tools_v2/xtask/src/commands/map/README.md) — the `cargo xtask map` commands that
  read these exports.
- [MCP commands](/tools_v2/xtask/src/commands/mcp/README.md) — `cargo xtask mcp wbcall`.
