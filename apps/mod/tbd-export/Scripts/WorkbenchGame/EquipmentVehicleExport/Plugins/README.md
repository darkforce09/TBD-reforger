# Equipment and vehicle export menu entries

The [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) menu entry that runs a complete
equipment and vehicle export, and the shared runner every diagnostic menu entry of the addon calls
to export a selected set of resources through the same pipeline.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Plugins/
└── TBD_EquipmentVehicleExportPlugin.c  the complete-export menu entry and the diagnostic runner
```

## How it works

`TBD_EquipmentVehicleExportPlugin` is a `WorkbenchPlugin` registered by
`[WorkbenchPluginAttribute]` as "Export Equipment and Vehicles" in the `TBD` category, with the
`ResourceManager` and `WorldEditor` modules. `Run` starts a complete generation and steps it to the
end in one call, which holds the editor until the export finishes, then prints the generation
directory. It refuses to start while another generation is unfinished.

`TBD_SourceDiagnosticExport.Run(domain, nativeTypes, folder)` serves the "Diagnostic Export: …"
plugins in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and
`apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`. It runs discovery, takes the equipment
or the vehicle list, keeps the names that contain `folder` when one is given, and, when native
types are given, keeps the prefabs with an effective node that inherits one of them (read with the
source reader, compared with `TBD_SourceCapabilityRules.IsA`). It then runs a generation of scope
`diagnostic` over that selection, with its gameplay dependencies, carrying any discovery error into
the generation.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`, the
  generation in `Generation/`, the discovery in `Discovery/`, the reader in `Source/` and the rules
  in `Capabilities/`; Workbench's `WorkbenchPlugin`.
- Used by: people, through the Workbench menu; the diagnostic plugins in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and
  `apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`, which call `TBD_SourceDiagnosticExport`.
- Rules: every export, complete or diagnostic, goes through `TBD_SourceExportGeneration`, so there
  is one reader, one identity and one schema; only the complete export can publish, which the
  validator's `partial_and_unverified_exports_cannot_publish` test
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`) holds. A new
  plugin class appears in the menu after a Workbench cold restart.
