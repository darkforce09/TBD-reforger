**Status:** live

# Export addon documentation

The deeper documents of `TBD_Export`, the [Workbench](/documentation/glossary/n_to_z.md#workbench)
addon of the [mod](/documentation/glossary/g_to_m.md#mod) that exports the terrain, object, building,
equipment, vehicle and item [registry](/documentation/glossary/n_to_z.md#registry) data the platform
ingests. Nothing in the addon ships to players or servers.

## Contents

```text
documentation/mod/tbd-export/
└── Scripts/  the exporter documents, mirroring the addon's script modules
```

## How it works

The addon's code READMEs describe each folder; the documents here cover what spans folders: the map
export pipeline and its runbook, and the frozen acceptance evidence of the equipment and vehicle
exporter. Start from the exporter you need:

| Exporter | Code | Entry point today | Documents |
|---|---|---|---|
| Map layers: elevation, raster, roads, water, vegetation, objects, infrastructure, places, lists | [MapExport](/mod/tbd-export/Scripts/WorkbenchGame/MapExport/README.md) | none: every plugin's menu entry is commented out | [map export](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) |
| Full world-object export | [Objects](/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/README.md) | none | [terrain export runbook](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md) |
| Building blueprints, voxel dumps, sight parity | [Buildings](/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/README.md) | `cargo xtask mcp wbcall EMCP_WB_TbdBlueprint` | [map export](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) |
| Runtime road network | [Export game scripts](/mod/tbd-export/Scripts/Game/TBD/Export/README.md) | playing the export mission header | [map export](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) |
| Equipment and vehicle source export | [EquipmentVehicleExport](/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md) | menu "Export Equipment and Vehicles"; Net API `EMCP_WB_SourceExport` | [acceptance evidence](/documentation/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/README.md) |
| Equipment and vehicle diagnostics | [EquipmentExport](/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/README.md), [VehicleExport](/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/README.md) | their menu entries | the code READMEs |
| Ballistics oracle: engine ballistic tables, trajectory simulation and gravity of the vanilla mortar shells | [BallisticsOracle plugin](/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/README.md), [game scripts](/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/README.md) | menu "Ballistics Oracle", then playing the export mission header | [ballistics oracle](/documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md) |
| Registry items and compatibility | `mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c` | none: its menu entry is commented out | the [contract catalogs](/contracts/catalogs/README.md) it feeds |

The addon also carries its own export world and
[mission header](/documentation/glossary/g_to_m.md#mission-header) (`mod/tbd-export/worlds/`,
`mod/tbd-export/Missions/`) and the export game mode prefab
(`mod/tbd-export/Prefabs/Systems/`), which the runtime road export and the ballistics
oracle's simulation run need.

```text
                     addon.gproj dependencies
TBD_Export ──▶ vanilla 58D0FB3206B6F859, TBD_EMCP D4E5F6A7B8C90123   (never TBD_Framework)

Workbench, mod/tbd-export/addon.gproj open
   ├─ WorkbenchGame exporters ──▶ $profile:TBD_Export/…, $profile:TBD_WorldExport_full.jsonl
   └─ Net API :5775 ◀── cargo xtask mcp wbcall / mcp call (handlers from TBD_EMCP and this addon)
```

## Code

- [Export addon](/mod/tbd-export/) — the addon project, its scripts, world, mission header
  and prefabs.
- [Map commands](/tools/xtask/src/commands/map/), the
  [world export pipeline](/tools/map_assets/world_export_pipeline/src/) and the
  [map raster pipeline](/tools/map_assets/map_raster_pipeline/src/) — the tools that read the map
  exports; the raster pipeline's `water-images` and `road-images` draw the water and road exports
  as PNG images.
- [Equipment and vehicle validation](/tools/commands/mod_operations/src/equipment_vehicle_export/)
  — `cargo xtask mod validate-equipment-vehicle-export` and `publish-equipment-vehicle-export`.

## Boundaries

- Depends on: the code READMEs under `mod/tbd-export/`, which hold each folder's detail, and
  the [feature doc](/documentation/standards/templates/feature_doc.md),
  [runbook](/documentation/standards/templates/runbook.md) and
  [folder index](/documentation/standards/templates/readme_documentation_folder.md) templates.
- Used by: the [mod documentation index](/documentation/mod/README.md), and the export addon's
  code READMEs, which link these documents.
- Rules: the addon depends on vanilla and `TBD_EMCP` only (`mod/tbd-export/addon.gproj:5-8`);
  documents mirror the code folder they cover; evidence stays frozen in `verification_evidence/`.

## Related documentation

- [Enfusion MCP bridge](/documentation/mod/tbd-emcp/workbench_mcp_bridge.md) — how Workbench
  loads the bridge through this addon's dependency, and the Net API calls.
- [Terrain datasets](/assets/terrains/README.md) — the committed data the map exports feed.
- [Contract catalogs](/contracts/catalogs/README.md) — the item registry catalogs.
