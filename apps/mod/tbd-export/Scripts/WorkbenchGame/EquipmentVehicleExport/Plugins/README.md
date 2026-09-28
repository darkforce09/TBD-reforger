# Equipment and vehicle export menu entries

The [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) menu entries that run the gameplay
export and the complete source export of equipment and vehicles, and the shared runner that exports
a selected set of resources through the same pipeline.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Plugins/
└── TBD_EquipmentVehicleExportPlugin.c  the gameplay and full-diagnostics menu entries, the diagnostic runner
```

## How it works

Two `WorkbenchPlugin` classes are registered by `[WorkbenchPluginAttribute]` in the `TBD`
category, with the `ResourceManager` and `WorldEditor` modules:

- `TBD_EquipmentVehicleExportPlugin`, "Export Equipment and Vehicles", starts a
  `TBD_GameplayExportGeneration` (from `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/`),
  the compact gameplay dataset.
- `TBD_FullEquipmentVehicleDiagnosticsPlugin`, "Export Full Equipment and Vehicle Diagnostics",
  starts a complete `TBD_SourceExportGeneration`, the full source dataset.

Each `Run` steps its generation to the end in one call, which holds the editor until the export
finishes, then prints the generation directory; each refuses to start while another generation is
unfinished.

`TBD_SourceDiagnosticExport.Run(domain, nativeTypes, folder)` exports a selected set: it runs
discovery, takes the equipment
or the vehicle list, keeps the names that contain `folder` when one is given, and, when native
types are given, keeps the prefabs with an effective node that inherits one of them (read with the
source reader, compared with `TBD_SourceCapabilityRules.IsA`). It then runs a generation of scope
`diagnostic` over that selection, with its gameplay dependencies, carrying any discovery error into
the generation.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`, the
  generation in `Generation/`, the gameplay generation in `Gameplay/`, the discovery in `Discovery/`, the reader in `Source/` and the rules
  in `Capabilities/`; Workbench's `WorkbenchPlugin`.
- Used by: people, through the Workbench menu; no script in the repository calls
  `TBD_SourceDiagnosticExport` at present.
- Rules: every export, gameplay, complete or diagnostic, goes through `TBD_SourceExportGeneration`
  or its gameplay subclass, so there is one reader and one identity; a diagnostic generation never
  publishes, which the
  validator's `partial_and_unverified_exports_cannot_publish` test
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`) holds. A new
  plugin class appears in the menu after a Workbench cold restart.
