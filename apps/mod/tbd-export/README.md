# TBD Export

`TBD_Export`, the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) tooling addon of the
[mod](/documentation_v2/glossary/g_to_m.md#mod): exporters that read Arma Reforger's loaded worlds,
prefabs and configs and write the terrain, building, equipment, vehicle and item
[registry](/documentation_v2/glossary/n_to_z.md#registry) data the platform ingests. Nothing here
ships to players or servers; it runs inside Workbench, and its road exporter inside a world that
plays the export [mission header](/documentation_v2/glossary/g_to_m.md#mission-header).

## Contents

```text
apps/mod/tbd-export/
├── addon.gproj           the `TBD_Export` project: GUID, title, dependencies on vanilla and `TBD_EMCP`
├── Missions/             the export mission header that boots the export world
├── Prefabs/              the export game mode prefab that carries the road export component
├── resourceDatabase.rdb  the addon's resource database, which Workbench maintains
├── Scripts/              the exporters: the runtime road export and the Workbench module
└── worlds/               the export world, a sub-scene of vanilla Everon with the export game mode
```

## How it works

`addon.gproj` depends on vanilla Arma Reforger and on `TBD_EMCP`, the addon in
`apps/mod/tbd-emcp/` that carries the Net API handlers the Enfusion MCP tools call, and on nothing
else: opening this project gives Workbench a standalone export session in which `TBD_Framework` is
not loaded.

```text
                     addon.gproj dependencies
TBD_Export ──▶ vanilla 58D0FB3206B6F859, TBD_EMCP D4E5F6A7B8C90123   (never TBD_Framework)

Workbench, apps/mod/tbd-export/addon.gproj open
   ├─ Scripts/WorkbenchGame/ exporters ──▶ $profile:TBD_Export/…, $profile:TBD_WorldExport_full.jsonl,
   │                                       $profile:TBD_RegistryItems.json, $profile:TBD_RegistryCompat.json
   └─ Net API :5775 ◀── cargo xtask mcp wbcall  (EMCP_WB_TbdBlueprint, EMCP_WB_SourceExport, TBD_EMCP's handlers)

Missions/TBD_Export_Everon.conf ──▶ worlds/TBD_Export_Everon.ent ──▶ Prefabs/Systems/TBD_Export_GameMode.et
   └─ played: Scripts/Game/ road export ──▶ $profile:TBD_Export/everon/roads/
```

The [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) under `Scripts/` holds two modules:
`Scripts/Game/` compiles into the game and holds only the runtime road network export, which the
export game mode component runs once the export world plays; `Scripts/WorkbenchGame/` compiles
into Workbench and holds the map layer exporters, the equipment and vehicle source exporter with
its diagnostic menu entries, and the item registry export. `Missions/`, `worlds/` and `Prefabs/`
exist for the road export: the header boots the export world, a sub-scene of vanilla
[Eden](/documentation_v2/glossary/a_to_f.md#eden) (Everon), whose layer places the export game mode
and Eden's AI world. The developer tools read the exports from the Workbench profile folder, which
under Proton is Steam's prefix for app 1874910
(`…/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/`).
A local `tools/` folder may hold PNG helper scripts, which git ignores (`.gitignore`,
`apps/mod/tbd-export/tools/*.mjs`).

## Getting started

Run these from the repository root. The first brings Workbench up on this addon; the rest need it
open with the Net API on port 5775.

```bash
cargo xtask mod dev-bootstrap        # installs the MCP package, launches Workbench on this addon's addon.gproj
cargo xtask mcp wbcall EMCP_WB_SourceExport '{"action":"verify"}'   # checks the source reader
```

`mod dev-bootstrap` prints what to do by hand and exits 1 while Workbench's Net API does not
answer. A new script file needs a Workbench cold restart before its class exists; an edited one
needs a script reload. Then run an exporter from the menu (Plugins → TBD → Export Equipment and
Vehicles) or over the Net API, and hand its output to the tools:

```bash
cargo xtask mod validate-equipment-vehicle-export --input <generation_directory>
cargo xtask mod publish-equipment-vehicle-export --input <generation_directory>
cargo xtask map ingest-blueprints    # building blueprints from the profile into assets_v2/terrains/
cargo xtask db registry-import       # the registry catalogs, once copied into contracts_v2/catalogs/
```

`cargo xtask mod compile` compiles only the framework addon
(`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`), so these scripts compile only when
Workbench, or a game, loads `tbd-export`.

## Configuration

- `addon.gproj`: the ID `TBD_Export`, GUID `C3D4E5F6A7B89012`, title "TBD Export", the dependencies
  `58D0FB3206B6F859` (vanilla Arma Reforger) and `D4E5F6A7B8C90123` (`TBD_EMCP`), and PC and
  HEADLESS configurations that use vanilla's `chimeraMenus.conf`; Workbench and the game read it
  when they load the addon.
- `$TBD_Export:exporter_revision.txt`: optional and untracked; its first line becomes the
  `exporter_revision` of every equipment and vehicle generation, which is `unavailable` without it
  (`Scripts/WorkbenchGame/EquipmentVehicleExport/Generation/TBD_SourceExportEnvironment.c`).
- `TBD_MapExportConfig`: the map exporters' destination (default `$profile:TBD_Export/`), map name
  and chunk size, set per run in code or a plugin dialog
  (`Scripts/WorkbenchGame/MapExport/Core/`).
- The Workbench profile folder (`$profile:`), where every export lands; the tools that read it take
  a path override where they need one (`cargo xtask map ingest-blueprints --src <dir>`).

## Public surface

- Workbench menu entries: "Export Equipment and Vehicles" in `TBD`, and eighteen "Diagnostic
  Export: …" entries in `TBD Diagnostics`. The map layer and registry plugins have their menu
  attributes commented out.
- Net API handlers: `EMCP_WB_TbdBlueprint` (building recon, blueprints, voxel dumps, sight parity)
  and `EMCP_WB_SourceExport` (the source export, its steps and probes), both reached with
  `cargo xtask mcp wbcall`.
- Files in the Workbench profile: `TBD_Export/<map>/…` and `TBD_WorldExport_full.jsonl` from the
  map exporters, `TBD_Export/everon/roads/` from the road export,
  `TBD_Export/equipment_vehicle_exports/` from the source exporter, and `TBD_RegistryItems.json`
  with `TBD_RegistryCompat.json` from the registry export.
- The export mission header `Missions/TBD_Export_Everon.conf`, opened by hand in Workbench.

## Boundaries

- Depends on: vanilla Arma Reforger, including the Eden world and its AI world prefab; `TBD_EMCP`
  in `apps/mod/tbd-emcp/`, for the Net API bridge; the export schemas in `contracts_v2/definitions/`
  that its files follow. Nothing from `apps/mod/tbd-framework/`.
- Used by: `cargo xtask mod dev-bootstrap`, which opens this project
  (`tools_v2/xtask/src/commands/mod_ops/development_bootstrap.rs`); `cargo xtask mcp wbcall`; the
  map commands and world export pipeline that read the map exports
  (`tools_v2/xtask/src/commands/map/`, `tools_v2/developer-tools/src/world_export_pipeline/`); the
  equipment and vehicle validation in `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/`;
  and, through the copied catalogs in `contracts_v2/catalogs/`, `cargo xtask db registry-import`.
- Rules: the dependencies stay vanilla and `TBD_EMCP`, never `TBD_Framework`, and the addon holds
  no copy of a framework class; the addon never ships: `cargo xtask deploy staging` excludes
  `apps/mod/tbd-export/` (`tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`);
  `resourceDatabase.rdb` belongs to Workbench and is never edited by hand; no upstream framework
  identifier or reused GUID enters the scripts (`cargo xtask verify no-crf-leak`).

## Related documentation

- [Export addon documentation](/documentation_v2/mod/tbd-export/README.md) — every exporter, its
  entry point and its documents.
- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  every map layer and the pipeline to committed terrain data.
- [Terrain export runbook](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md)
  — a full world-object export through to rebuilt terrain data.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — bringing Workbench and
  the MCP bridge up, and calling Net API handlers.
- [Workbench MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — how the bridge
  loads through this addon's dependency.
