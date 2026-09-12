# TBD Export

Workbench map-export tooling for the TBD Reforger platform, as a thin addon over **TBD_Framework**.
Nothing here ships to players or servers: the addon exists to run inside Workbench (and, for the road
exporter, inside a headless server pointed at the export scenario).

Mod GUID: `C3D4E5F6A7B89012` · ID `TBD_Export` · Dependencies: `58D0FB3206B6F859` (vanilla),
`B2C3D4E5F6A78901` (TBD_Framework), `D4E5F6A7B8C90123` (TBD_EMCP — the enfusion-mcp bridge)

## What is here (and only here)

| Path | Role |
|---|---|
| `Scripts/WorkbenchGame/MapExport/**` | The map-export plugins (terrain DEM / satellite / water / roads, vegetation, objects, props, infrastructure, buildings + blueprints, locations, registry / arsenal). `Plugins,TBD,…` in Workbench. |
| `Scripts/WorkbenchGame/MapExport/Objects/Buildings/EMCP_WB_TbdBlueprint.c` | Net API handler that drives the blueprint recon / trace / parity extractors from `cargo xtask mcp call`. Lives outside `EnfusionMCP/` on purpose: the MCP's `wb_cleanup` deletes that directory. |
| `Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c`, `TBD_RegistryScan.c` | Registry item export (`packages/tbd-schema/registry/registry-items.workbench.json`); use the `TBD_ExportJson` / `TBD_ExportPaths` aliases from `MapExport/Core`. |
| `Scripts/Game/TBD/Export/*.c` | `TBD_RoadExportComponent` — runtime road-network exporter (game-mode component). |
| `Prefabs/Systems/TBD_GameMode.et` (+ `.meta`) | Path-override of the framework prefab (same resource GUID `7A5B8572ECC15707`) that adds `TBD_RoadExportComponent`. This addon loads last, so its copy wins; the shipping prefab never carries the component. |
| `Missions/TBD_Export_Everon.conf` | The export scenario (framework world `{F652B97A6F497348}worlds/TBD_Dev_POC.ent`, game mode `TBD`). |
| `tools/*.mjs` | PNG helpers (gitignored). |

Everything else — scripts, layouts, configs, worlds, data — comes from tbd-framework through the
dependency. Do not copy framework files in here: `cargo xtask mod compile` fails on any relative path
that exists in two addons (the only allowed overlap is the prefab above).

## Workbench

Open `apps/mod/tbd-export/addon.gproj` (`cargo xtask mod dev-bootstrap` does it): Workbench resolves
TBD_Framework and TBD_EMCP as dependencies, so this is the full dev session. Export output lands under
`$profile:TBD_Export/…` (Proton: `…/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/TBD_Export/`);
`cargo xtask map ingest-blueprints` reads it from there.

New `.c` files need a Workbench cold restart (the script list is built at load). Headless compile gate:
`cargo xtask mod compile` compiles `Scripts/Game` of all three addons; `Scripts/WorkbenchGame` compiles
only inside Workbench.

## History

Until 2026-09-12 this addon was a file-for-file mirror of tbd-framework, held in lockstep by the compile
gate. The mirror was dropped for the dependency model above — see `apps/mod/README.md` and
`docs/mod/MCP_TOOLING.md`.
