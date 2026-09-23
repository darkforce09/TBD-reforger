# TBD Export

Workbench map-export and extraction tooling for the TBD Reforger platform.
Nothing here ships to players or servers: the addon exists to run inside Workbench (and, for the road
exporter, inside a simulation server pointed at the export scenario).

Mod GUID: `C3D4E5F6A7B89012` · ID `TBD_Export` · Dependencies: `58D0FB3206B6F859` (vanilla),
`D4E5F6A7B8C90123` (TBD_EMCP — the enfusion-mcp bridge). It has no dependency on `TBD_Framework`.

## What is here

| Path | Role |
|---|---|
| `Scripts/WorkbenchGame/MapExport/**` | The map-export plugins (terrain DEM / satellite / water / roads, vegetation, objects, props, infrastructure, buildings + blueprints, locations, registry / arsenal). `Plugins,TBD,…` in Workbench. |
| `Scripts/WorkbenchGame/EquipmentExport/**` | Shared export path/JSON infrastructure and the unfiltered equipment discovery census. [Hub README](Scripts/WorkbenchGame/EquipmentExport/README.md). |
| `Scripts/WorkbenchGame/EquipmentExport/WeaponExport/**` | Twelve per-domain weapon extractors (ammunition, attachments, bayonets, handguards, illuminators, M16, muzzles, optics, rifles, stocks, underbarrel, master arsenal). [Hub README](Scripts/WorkbenchGame/EquipmentExport/WeaponExport/README.md). |
| `Scripts/WorkbenchGame/EquipmentExport/WearableExport/**` | Universal wearable, clothing, armor, load-bearing rig, and gear extractors across ten categories. [Hub README](Scripts/WorkbenchGame/EquipmentExport/WearableExport/README.md). |
| `Scripts/WorkbenchGame/EquipmentExport/ItemExport/**` | Universal inventory item extractor across eleven categories (medical, radios, navigation, tools, explosives, survival, etc.). [Hub README](Scripts/WorkbenchGame/EquipmentExport/ItemExport/README.md). |
| `Scripts/WorkbenchGame/VehicleExport/**` | Vehicle extractor plugins. |
| `Scripts/WorkbenchGame/MapExport/Objects/Buildings/EMCP_WB_TbdBlueprint.c` | Net API handler that drives the blueprint recon / trace / parity extractors from `cargo xtask mcp call`. Lives outside `EnfusionMCP/` on purpose: the MCP's `wb_cleanup` deletes that directory. |
| `Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c`, `TBD_RegistryScan.c` | Registry item export (`contracts_v2/catalogs/registry-items.workbench.json`); use the `TBD_ExportJson` / `TBD_ExportPaths` aliases from `MapExport/Core`. |
| `Scripts/Game/TBD/Export/*.c` | `TBD_RoadExportComponent` — runtime road-network exporter (game-mode component). |
| `Prefabs/Systems/TBD_Export_GameMode.et` (+ `.meta`) | Dedicated standalone export game mode prefab carrying `TBD_RoadExportComponent`. |
| `worlds/TBD_Export_Everon.ent` (+ `.meta`, layers) | Dedicated SubScene of vanilla Eden (`{853E92315D1D9EFE}worlds/Eden/Eden.ent`) with AIWorld and export game mode. |
| `Missions/TBD_Export_Everon.conf` | The export scenario (standalone export world `worlds/TBD_Export_Everon.ent`, game mode `TBD Export`). |
| `tools/*.mjs` | PNG helpers (gitignored). |

## Workbench

Open `apps/mod/tbd-export/addon.gproj`: Workbench resolves vanilla Reforger and TBD_EMCP as dependencies,
providing a completely standalone export environment without requiring TBD_Framework. Export output lands under
`$profile:TBD_Export/…` (Proton: `…/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/TBD_Export/`);
`cargo xtask map ingest-blueprints` reads it from there.

New `.c` files need a Workbench cold restart (the script list is built at load). Headless compile gate:
`cargo xtask mod compile` compiles `Scripts/Game` of `TBD_Framework`; `Scripts/WorkbenchGame` compiles
inside Workbench.

## History

Until 2026-09-12 this addon was a file-for-file mirror of tbd-framework, held in lockstep by the compile
gate. It was briefly configured as a dependency addon over tbd-framework, but that prevented launching
export tooling independently. It is now fully decoupled: all export scripts and assets are standalone.

