# Equipment diagnostic exports

[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) menu entries that export a chosen slice of
the installed equipment (all of it, inventory items, wearables, weapons and their parts) for
inspection. Each one selects resources and hands them to the shared equipment and vehicle source
exporter, so a diagnostic uses the same reader, identity, snapshots, capability fields, dependency
traversal and schema as the complete export.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/
├── ItemExport/                  the inventory item diagnostic
├── TBD_EquipmentExportPlugin.c  "Diagnostic Export: All Equipment": every discovered equipment prefab
├── WeaponExport/                the weapon, ammunition and attachment diagnostics
└── WearableExport/              the wearables and gear diagnostic
```

## How it works

Every plugin here is a `WorkbenchPlugin` in the `TBD Diagnostics` menu category whose `Run` calls
`TBD_SourceDiagnosticExport.Run("equipment", nativeTypes, folder)` from
`apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Plugins/`. The runner starts
from the equipment census of the exporter's discovery and keeps the prefabs under `folder`, when
one is given, whose effective nodes inherit one of `nativeTypes`, when any are given; "All
Equipment" passes neither and takes the whole census. The selection and its gameplay dependencies
go into a generation of scope `diagnostic` under
`$profile:TBD_Export/equipment_vehicle_exports/generations/`, and the plugin prints its directory.

A diagnostic generation is for inspection only: the publisher rejects it, so it can never become
the current bundle, and no plugin here writes a catalog or summary of its own. A complete export
is the "Export Equipment and Vehicles" entry under `TBD`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_SourceDiagnosticExport` and the rest of the source exporter in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`; Workbench's
  `WorkbenchPlugin`.
- Used by: people, through the Workbench menu; nothing in the repository calls these plugins.
- Rules: a diagnostic only selects resources and never reads or writes a fact itself; its
  generation cannot publish (`partial_and_unverified_exports_cannot_publish` in
  `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`). The scripts
  compile only when Workbench loads `tbd-export`, and a new plugin class appears in the menu after
  a Workbench cold restart.

## Related documentation

- [Equipment and vehicle source export](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — the shared exporter, its records, and the validate and publish commands.
