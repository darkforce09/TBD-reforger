# Vehicle diagnostic exports

[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) menu entries that export the installed
vehicles, or the BTR-70 family alone, for inspection. Each one hands its selection to the shared
equipment and vehicle source exporter, so its records are the same as a complete export's, and its
generation is diagnostic and can never replace the published bundle.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/
├── TBD_BTRExportPlugin.c             "Diagnostic Export: BTR-70 Resources": the BTR70 folder's vehicles
├── TBD_VehicleCatalogExportPlugin.c  "Diagnostic Export: Vehicle Catalog": every discovered vehicle
└── TBD_VehicleDeepExportPlugin.c     "Diagnostic Export: Vehicle Systems": every discovered vehicle
```

## How it works

Each plugin is a `WorkbenchPlugin` in the `TBD Diagnostics` menu category whose `Run` calls
`TBD_SourceDiagnosticExport.Run("vehicle", nativeTypes, folder)` from
`apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Plugins/` with no native type
filter. The BTR-70 entry keeps the discovered vehicles whose names contain
`Prefabs/Vehicles/Wheeled/BTR70/`; the catalog and systems entries pass no folder, so both take the
whole vehicle census and produce the same selection. The selection and its gameplay dependencies go
into a generation of scope `diagnostic`, which the publisher rejects. A complete export is the
"Export Equipment and Vehicles" entry under `TBD`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_SourceDiagnosticExport` and the rest of the source exporter in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`; Workbench's
  `WorkbenchPlugin`.
- Used by: people, through the Workbench menu; nothing in the repository calls these plugins.
- Rules: a diagnostic only selects resources; its generation cannot publish
  (`partial_and_unverified_exports_cannot_publish` in
  `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`). The scripts
  compile only when Workbench loads `tbd-export`.

## Related documentation

- [Equipment and vehicle source export](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — the shared exporter, its records, and the validate and publish commands.
