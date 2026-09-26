# Export addon Workbench module

The export addon's [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) script module: the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) that Workbench compiles into the editor
and nowhere else. It holds every exporter that reads the loaded game data from inside the editor:
the map layers, the equipment and vehicle source export with its diagnostics, and the item
[registry](/documentation_v2/glossary/n_to_z.md#registry) export.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/
├── EquipmentExport/                 equipment diagnostic menu entries over the source exporter
├── EquipmentVehicleExport/          the equipment and vehicle source exporter and its Net API handler
├── MapExport/                       the map layer exporters and the building blueprint handler
├── TBD_RegistryItemsExportPlugin.c  the item registry export plugin: writes the items and compat files
├── TBD_RegistryScan.c               `TBD_RegistryScanner`: classes loaded prefabs, derives compat edges
└── VehicleExport/                   vehicle diagnostic menu entries over the source exporter
```

## How it works

Enfusion compiles each addon's `Scripts/WorkbenchGame/` into Workbench's own module, so these
classes exist only while Workbench has the addon loaded. Three kinds of entry point run them:

| Exporter | Entry point | Writes under `$profile:` |
|---|---|---|
| Map layers (`MapExport/`) | none in the menu: every plugin attribute is commented out; the Net API handler `EMCP_WB_TbdBlueprint` | `TBD_Export/<map>/…`, `TBD_WorldExport_full.jsonl` |
| Equipment and vehicle source export (`EquipmentVehicleExport/`) | the menu entry "Export Equipment and Vehicles" (`TBD`); the Net API handler `EMCP_WB_SourceExport` | `TBD_Export/equipment_vehicle_exports/…` |
| Equipment and vehicle diagnostics (`EquipmentExport/`, `VehicleExport/`) | the "Diagnostic Export: …" entries (`TBD Diagnostics`) | `TBD_Export/equipment_vehicle_exports/generations/…` |
| Item registry (`TBD_RegistryItemsExportPlugin.c`) | none: its plugin attribute is commented out | `TBD_RegistryItems.json`, `TBD_RegistryCompat.json` |

A menu entry is a `WorkbenchPlugin` class registered by `[WorkbenchPluginAttribute]`; a Net API
handler is a `NetApiHandler` class that `cargo xtask mcp wbcall <class> '<json>'` reaches through
Workbench's Net API.

The registry export scans the `Prefabs` tree of every loaded addon (`TBD_RegistryScanner` in
`TBD_RegistryScan.c`), classes each prefab into a registry item kind by the components it holds,
refines kinds and arsenal types from the addons' entity catalogs, derives compatibility edges from
engine data only (magazine wells, attachment slot types, vehicle weapon slot chains, character
loadout slots), and writes the items file and then the compat file, which doubles as the
run-complete marker. It refuses to write an empty item or edge list. The two files are copied by
hand to `contracts_v2/catalogs/registry-items.workbench.json` and
`contracts_v2/catalogs/registry-compat.workbench.json`. It writes JSON through `TBD_ExportJson`, the
short alias of the map export's JSON helper in `MapExport/Core/`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's `WorkbenchPlugin`, `NetApiHandler`, `WorldEditorAPI` and resource
  search; the engine's world, container and file classes; the loaded addons' prefabs, configs and
  entity catalogs; nothing from `tbd-framework`.
- Used by: people, through the Workbench menu; `cargo xtask mcp wbcall`, which reaches the two Net
  API handlers (`tools_v2/xtask/src/commands/mcp/`); the developer tools that read the map exports
  (listed in the [map export README](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/README.md)); `cargo xtask mod validate-equipment-vehicle-export` and
  `publish-equipment-vehicle-export`, which read the source export
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/`); and, through the copied
  catalogs, `cargo xtask db registry-import` and `cargo xtask schema validate`.
- Rules: nothing under `Scripts/Game/` may name a class from here, because a game never compiles
  this module; the scripts compile only when Workbench loads `tbd-export`, since
  `cargo xtask mod compile` compiles the framework addon alone
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`); a new script file needs a
  Workbench cold restart before its class exists; a Net API handler stays out of any `EnfusionMCP/`
  folder, which the MCP's `wb_cleanup` deletes.

## Related documentation

- [Export addon Workbench exporter documentation](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/README.md)
  — the exporter families and their documents.
- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  every map layer, its entry point and the pipeline to committed data.
- [Contract catalogs](/contracts_v2/catalogs/README.md) — the item registry catalogs the registry
  export feeds.
