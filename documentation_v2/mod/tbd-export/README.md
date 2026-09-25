**Status:** live

# Export addon documentation

The deeper documents of `TBD_Export`, the [Workbench](/documentation_v2/glossary.md#workbench)
addon of the [mod](/documentation_v2/glossary.md#mod) that exports the terrain, object, building,
equipment, vehicle and item [registry](/documentation_v2/glossary.md#registry) data the platform
ingests. Nothing in the addon ships to players or servers.

## Contents

```text
documentation_v2/mod/tbd-export/
└── Scripts/  the exporter documents, mirroring the addon's script modules
```

## How it works

The addon's code READMEs describe each folder; the documents here cover what spans folders: the map
export pipeline and its runbook, and the frozen acceptance evidence of the equipment and vehicle
exporter. Start from the exporter you need:

| Exporter | Code | Entry point today | Documents |
|---|---|---|---|
| Map layers: elevation, raster, roads, water, vegetation, objects, infrastructure, places, lists | [MapExport](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/README.md) | none: every plugin's menu entry is commented out | [map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) |
| Full world-object export | [Objects](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/README.md) | none | [terrain export runbook](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md) |
| Building blueprints, voxel dumps, sight parity | [Buildings](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/README.md) | `cargo xtask mcp wbcall EMCP_WB_TbdBlueprint` | [map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) |
| Runtime road network | [Export game scripts](/apps/mod/tbd-export/Scripts/Game/TBD/Export/README.md) | playing the export mission header | [map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) |
| Equipment and vehicle source export | [EquipmentVehicleExport](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md) | menu "Export Equipment and Vehicles"; Net API `EMCP_WB_SourceExport` | [acceptance evidence](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md) |
| Equipment and vehicle diagnostics | [EquipmentExport](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/README.md), [VehicleExport](/apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/README.md) | their menu entries | the code READMEs |
| Registry items and compatibility | `apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c` | none: its menu entry is commented out | the [contract catalogs](/contracts_v2/catalogs/README.md) it feeds |

The addon also carries its own export world and
[mission header](/documentation_v2/glossary.md#mission-header) (`apps/mod/tbd-export/worlds/`,
`apps/mod/tbd-export/Missions/`) and the export game mode prefab
(`apps/mod/tbd-export/Prefabs/Systems/`), which the runtime road export needs.

```text
                     addon.gproj dependencies
TBD_Export ──▶ vanilla 58D0FB3206B6F859, TBD_EMCP D4E5F6A7B8C90123   (never TBD_Framework)

Workbench, apps/mod/tbd-export/addon.gproj open
   ├─ WorkbenchGame exporters ──▶ $profile:TBD_Export/…, $profile:TBD_WorldExport_full.jsonl
   └─ Net API :5775 ◀── cargo xtask mcp wbcall / mcp call (handlers from TBD_EMCP and this addon)
```

## Code

- [Export addon](/apps/mod/tbd-export/) — the addon project, its scripts, world, mission header
  and prefabs.
- [Map commands](/tools_v2/xtask/src/commands/map/) and the
  [world export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/) — the tools that
  read the map exports.
- [Equipment and vehicle validation](/tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/)
  — `cargo xtask mod validate-equipment-vehicle-export` and `publish-equipment-vehicle-export`.

## Boundaries

- Depends on: the code READMEs under `apps/mod/tbd-export/`, which hold each folder's detail, and
  the [feature doc](/documentation_v2/standards/templates/feature_doc.md),
  [runbook](/documentation_v2/standards/templates/runbook.md) and
  [folder index](/documentation_v2/standards/templates/readme_documentation_folder.md) templates.
- Used by: the [mod documentation index](/documentation_v2/mod/README.md), and the export addon's
  code READMEs, which link these documents.
- Rules: the addon depends on vanilla and `TBD_EMCP` only (`apps/mod/tbd-export/addon.gproj:5-8`);
  documents mirror the code folder they cover; evidence stays frozen in `verification_evidence/`.

## Related documentation

- [Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — how Workbench
  loads the bridge through this addon's dependency, and the Net API calls.
- [Terrain datasets](/assets_v2/terrains/README.md) — the committed data the map exports feed.
- [Contract catalogs](/contracts_v2/catalogs/README.md) — the item registry catalogs.
