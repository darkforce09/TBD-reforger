**Status:** live

# Map export

The [Workbench](/documentation_v2/glossary.md#workbench) side of the terrain data pipeline: the
`tbd-export` [mod](/documentation_v2/glossary.md#mod) addon's exporters read a world open in the
editor (ground height, rasters, roads, water, vegetation, placed objects, buildings, places and
prefab lists) and write files to the Workbench profile, where the developer tools pick up the ones
the committed terrain data is built from. Developers and agents rebuilding a terrain read it.

## Where it lives

- Code: [`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/README.md),
  one folder per layer, and the runtime road export in
  [`apps/mod/tbd-export/Scripts/Game/TBD/Export/`](/apps/mod/tbd-export/Scripts/Game/TBD/Export/README.md).
  The export world and its [mission header](/documentation_v2/glossary.md#mission-header) are
  [`apps/mod/tbd-export/worlds/`](/apps/mod/tbd-export/worlds/README.md) and
  [`apps/mod/tbd-export/Missions/`](/apps/mod/tbd-export/Missions/README.md).
- Entry: the Net API handler `EMCP_WB_TbdBlueprint`
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/EMCP_WB_TbdBlueprint.c`),
  called by `cargo xtask mcp wbcall`, and the `TBD_RoadExportComponent` of the export game mode,
  started by playing `apps/mod/tbd-export/Missions/TBD_Export_Everon.conf`. Every other exporter is
  a `WorkbenchPlugin` class with no menu entry (see [Entry points](#entry-points)).
- Related: the [terrain export runbook](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md),
  the [world export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md) that
  turns the full export into object and road data, and the
  [terrain datasets](/assets_v2/terrains/README.md) it feeds.

## Behaviour

### Layers

Every layer writes below `$profile:TBD_Export/<map>/<category>/`, where `<map>` is the lowercase map
name taken from the world path (`eden` becomes `everon`) or the config's override; the full
world-object export writes to the profile root instead. The layer READMEs linked below give each
file's format.

| Layer | Code folder | Writes | Read by |
|---|---|---|---|
| Elevation | [`Terrain/DEM/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/DEM/README.md) | `terrain/heightmap.txt`, `terrain/dem_meta.json` | `world raw-u16-dem-png`, with the paths passed by hand |
| Cartographic raster | [`Terrain/Satellite/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Satellite/README.md) | `satellite/rasterization.tga` and its meta | nothing |
| Road network | [`Terrain/Roads/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Roads/README.md) | `roads/`, one file per road class | nothing |
| Water | [`Terrain/Water/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/README.md) | `water/` rasters, lakes, rivers, ponds | nothing; `map water` expects other names (see below) |
| Vegetation | [`Vegetation/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Vegetation/README.md) | `vegetation/`, one file per class | nothing |
| Full world objects | [`Objects/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/README.md) | `TBD_WorldExport_full.jsonl`, `TBD_WorldExport_full_meta.json` in the profile root | `world copy-export-profile --full` |
| Classified objects | [`Objects/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/README.md) | `buildings/`, `props/`, `vegetation/trees.jsonl` and `rocks.jsonl` | nothing |
| Building blueprints and parity | [`Objects/Buildings/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/README.md) | `prefabs/buildings/`, `prefabs/dumps/`, `prefabs/debug/` | `cargo xtask map ingest-blueprints`, `map blueprint-from-voxels`, `map parity-report`, `map world-los`, `map instances-verify` |
| Infrastructure | [`Objects/Infrastructure/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Infrastructure/README.md) | `infrastructure/`: fences, bridges and piers, aviation, power grid | nothing |
| Places | [`Locations/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Locations/README.md) | `locations/locations.json` | nothing |
| Landmark anchors | [`Locations/Anchors/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Locations/Anchors/README.md) | `anchors/verification.json` | nothing |
| Prefab and arsenal lists | [`Registry/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Registry/README.md) | `registry/prefabs.json`, `registry/arsenal.json` | nothing |
| Runtime road network | [`Scripts/Game/TBD/Export/`](/apps/mod/tbd-export/Scripts/Game/TBD/Export/README.md) | `everon/roads/`, the Workbench road file names | nothing |

The committed road archive does not come from either road export: `world build-roads` decodes the
road topology from the game paks. The item [registry](/documentation_v2/glossary.md#registry)
catalogs in `contracts_v2/catalogs/` come from a separate plugin,
`apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c`, whose two
`$profile:TBD_Registry*.json` files are copied into the catalog folder by hand.

### Entry points

Three kinds of entry point exist in the code, and only two run today:

1. **Net API handler (live).** `EMCP_WB_TbdBlueprint` answers
   `cargo xtask mcp wbcall EMCP_WB_TbdBlueprint '{"action":"<action>", …}'` with one of the actions
   `recon`, `extract`, `parity`, `probe`, `dump` and `world-parity`. It is the one map-export entry
   point that runs from outside the editor. It lives beside the building exporters, not in an
   `EnfusionMCP/` folder, because the MCP's `wb_cleanup` deletes that folder in the addon it names.
2. **Export mission header (live).** Playing `TBD_Export_Everon.conf` in Workbench or on a server
   that loads `TBD_Export` starts the export game mode, whose `TBD_RoadExportComponent` writes the
   runtime road files 500 ms after it initialises, on the server only.
3. **Plugins (no entry).** `TBD_MapExportPlugin` in `Plugins/` would run layers 1 to 14 in order,
   and each layer folder has a standalone plugin, but every `[WorkbenchPluginAttribute]` under
   `MapExport/` is commented out (32 of them), so Workbench lists none in a menu. No committed MCP
   handler or `cargo xtask` command runs a plugin, and a `wb_execute_action` call with a
   `Plugins,TBD,…` menu path finds nothing to run.

The same holds for the registry plugin: its attribute is commented out too.

### From a full export to committed object data

The object and road data under `assets_v2/terrains/<terrain>/objects/` and `roads/` is built from
the full world-object export in five stages; the
[terrain export runbook](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md)
gives the commands.

```text
TBD_WorldFullExportPlugin (Workbench, no menu entry)
    └─▶ $profile:TBD_WorldExport_full.jsonl, then TBD_WorldExport_full_meta.json (sentinel, last)
          │  world copy-export-profile --terrain <t> --full --profile <dir>
          ▼
assets_v2/scratch/<t>/export/raw-entities.jsonl, export-meta.json, staged-meta.json
          │  cargo xtask map export-terrain <t> --phase <P>
          │    world phase-gate ─▶ world build-objects --patch-manifest --ops-log ─▶ world build-roads --ops-log
          ▼
assets_v2/terrains/<t>/objects/, roads/  ─▶  world verify-phase --terrain <t> --phase <P>
          │  PASS
          ▼
raise importPhaseMax in assets_v2/terrains/terrain-registry.json to allow the next phase
```

1. The plugin sweeps the world in 512 m cells, keeps each entity only in the cell of its origin
   (the same clamp as the developer tools' `cell_of`), and writes the meta file last; a failed
   write deletes the partial file and writes no meta.
2. `copy-export-profile --full` refuses to stage without the meta, or when the meta's `keptCount`
   differs from the file's line count, and never overwrites a staged export with an empty file.
3. `export-terrain` refuses a phase above the registry's `importPhaseMax` (exit 1), stops with the
   operator steps when the staged file is missing (exit 2), and otherwise builds the objects and
   roads. The phase order is `P1_buildings` to `P5_props`, then `P6_roads_highway` to `P10_full`;
   `build-objects` and `verify-phase` implement `P1_buildings` to `P5_props` only.
4. `verify-phase` runs the mathematical gates over the staged export and the committed artifacts.
5. The registry bump is a hand edit: the gate's own refusal reads "advance only after
   map-verify-phase PASS + registry bump". Everon stands at `P5_props` and Arland at
   `P1_buildings` in the committed registry.

### Elevation orientation

`TBD_MapExportDEM` writes one text row per sample row `py`, row 0 at world z = 0, the south edge
(`TBD_MapExportDEM.c:127-130`), and the map engine samples row 0 at the bounds' minimum z. The
committed elevation image keeps that order.

### Known discrepancies

- The operator steps `cargo xtask map export-terrain` prints
  (`tools_v2/xtask/src/commands/map/terrain_export.rs:61-84`) say to run the full export from the
  menu "Plugins > TBD > Export TBD World Objects (full)" or through `mcp call wb_execute_action`
  with that menu path — the plugin's attribute is commented out
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/TBD_WorldFullExportPlugin.c:19`),
  so neither path runs it.
- `world copy-export-profile` without `--full` reads `TBD_WorldExport_subregion.jsonl`
  (`tools_v2/developer-tools/src/world_export_pipeline/export_preparation/export_profile.rs:28`) —
  no script in the addon writes that file.
- `copy-export-profile` tells a missing export to "Run the TBD_TerrainWorldExportPlugin"
  (`export_profile.rs:49`) — no such plugin exists; the full export is `TBD_WorldFullExportPlugin`.
- `copy-export-profile` defaults the profile to `$HOME/Documents/Games/ArmaReforgerWorkbench/profile`
  (`export_profile.rs:20-23`) — Workbench under Proton writes to
  `compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile`,
  the path `cargo xtask map ingest-blueprints` defaults to
  (`tools_v2/developer-tools/src/blueprint/ingest.rs:24`), so the stage step needs `--profile`.
- No command stages the other layers: `map water` reads
  `assets_v2/scratch/<terrain>/water/TBD_InlandWaterExport_{vectors,meta}.json` and `_mask.txt`,
  `_depth.txt` (`tools_v2/developer-tools/src/map_raster_pipeline/inland_water_archive.rs:61-67`)
  — the water layer writes `bathymetry_mask.txt`, `bathymetry_depth.txt`, `lakes.json`,
  `rivers.json` and `ponds.json` to the profile
  (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/TBD_MapExportWater.c:94-97`),
  and nothing copies or renames them into the scratch.
- `dem_elevation.rs` says the elevation file's row 0 is the north edge
  (`tools_v2/developer-tools/src/world_export_pipeline/export_preparation/dem_elevation.rs:4`) —
  the exporter writes row 0 at z = 0, the south edge (`TBD_MapExportDEM.c:127-130`).
- The runtime road export and the Workbench road layer write the same file names under
  `everon/roads/`, so whichever runs last replaces the other's files.

## Data

- `$profile:TBD_WorldExport_full.jsonl`: one JSON object per entity (`resourceName`, `className`,
  `x`, `y`, `z`, `headingDeg`, `pitchDeg`, `rollDeg`, `scale`, `halfExtentsM`); the meta carries
  `exportVersion` 2 and the counts `copy-export-profile` checks. Staged as
  `assets_v2/scratch/<terrain>/export/raw-entities.jsonl`, gitignored.
- `assets_v2/terrains/terrain-registry.json` (`contracts_v2/definitions/terrain-registry.schema.json`):
  `importPhaseMax` per terrain, read by `world phase-gate`; `build-objects --patch-manifest` writes
  the phase into the terrain's `manifest.json` as `importPhaseMax` and `importPhaseShipped`.
- Net API: one request per `wbcall`, `APIFunc` `EMCP_WB_TbdBlueprint` with `action`, `filter`,
  `maxEntities`, `cx`, `cy`, `seed` and the probe endpoints; the response is `status`, `action` and
  `message`. The [building blueprint README](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/README.md)
  lists what each action writes.
- The [blueprint compiler](/tools_v2/developer-tools/src/blueprint/README.md) reads the voxel dumps
  and parity pairs offline; the [map commands](/tools_v2/xtask/src/commands/map/README.md) list
  every command that reads these files.

## Design

The exporters hold one pattern: a layer takes a `TBD_MapExportConfig` and a `TBD_MapExportContext`
from `MapExport/Core/`, reads the world through `WorldEditorAPI` and the engine's world queries, and
writes only through `TBD_MapExportPaths` below the config's destination. Most layers sweep square
cells of `m_fObjectChunkSizeM` (512 m by default). No visual design applies: the feature has no UI
beyond Workbench's own plugin dialogs.

## Open work

- [T-1081 — Decide whether Workbench registry and map export plugins stay unregistered](/.ai/tickets/T-1081.toml)
  (idea, no plan): either the plugin menu entries come back, or the headers, READMEs and the
  export-terrain operator steps stop naming them; the subregion path and the nonexistent plugin
  name in `copy-export-profile` get fixed either way.
- [T-1100 — Fix map water export output unreadable by map water](/.ai/tickets/T-1100.toml) (idea,
  no plan): the water layer's files and `map water`'s expected names and river fields agree.
- [T-1101 — Fix map export road classes, spline transforms and DEM result](/.ai/tickets/T-1101.toml)
  (idea, no plan): road classes stop overlapping, spline points follow entity rotation, and the
  DEM export fails when its meta file fails.
- [T-1102 — Rewrite stale map export comments and drop unused WORLD_PATH](/.ai/tickets/T-1102.toml)
  (idea, no plan): the `dem_elevation.rs` row-0 comment and the water, vegetation and road comments
  match the code.
- [T-1117 — Stream map export-terrain world output while each step runs](/.ai/tickets/T-1117.toml)
  (idea, no plan): `export-terrain` shows the world tool's output as it runs.
- [T-1148 — Fix map export files overwriting each other and cell-edge duplicates](/.ai/tickets/T-1148.toml)
  (idea, no plan): one run stops overwriting `vegetation_meta.json` and blueprint files, and the
  AABB sweeps stop recording cell-crossing entities twice.
- [T-1149 — Tidy tbd-export: hard-coded arsenal, no-op props, road component, unused code](/.ai/tickets/T-1149.toml)
  (idea, no plan): the arsenal list, the props placeholder and the runtime road component's fixed
  map name and world size are resolved.
- [T-294 — Arland has a manifest and no object data](/.ai/tickets/T-294.toml) (ready,
  [plan](/documentation_v2/tickets/plans/t-294_plan.md)): Arland gets a full export and object
  data through this pipeline.

## Decisions

- The export addon depends on vanilla and `TBD_EMCP` only (`apps/mod/tbd-export/addon.gproj:5-8`),
  never on the framework: the export tooling opens in Workbench on its own, and the shipping mod
  carries no editor code.
- The full export writes its meta file last, as a completion sentinel: a crashed run leaves no meta,
  so `copy-export-profile --full` never stages a partial file.
- Blueprint extraction stays uninterpreted in the addon: the handler dumps raw trace data, and every
  heuristic runs offline in `tools_v2/developer-tools/src/blueprint/`, so rules change without a
  Workbench recompile.
- The Net API handler lives outside `EnfusionMCP/`, which the MCP's `wb_cleanup` deletes.
- The elevation matrix quantises against the terrain's own height range when it dips below zero,
  and a fixed −204.78 m to 375.53 m range otherwise; the converter falls back to the same range, so
  the image and the elevation file decode to the same grid.
