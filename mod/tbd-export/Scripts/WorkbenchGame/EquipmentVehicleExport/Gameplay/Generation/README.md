# Equipment gameplay generation

The generation that applies the gameplay selection policy to the exporter's discovery census and
writes the gameplay dataset under
`$profile:TBD_Export/equipment_vehicle_exports/gameplay/generations/<generation id>/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Generation/
└── TBD_GameplayExportGeneration.c  resource capture, the reference queue, the index and the reports
```

## How it works

`TBD_GameplayExportGeneration` extends `TBD_SourceExportGeneration`, keeping its run, queue,
environment and reader verification, and overrides three steps:

- `Destination` points the run at `equipment_vehicle_exports/gameplay/generations/`.
- `CaptureResource` loads one resource, reads it with a `TBD_SourceContainerReader` that carries the
  selection policy, and writes the gameplay resource document: under `resources/` for equipment and
  vehicle roots, under `shared_configurations/` for a dependency. It adds an index entry with the
  resource's domains and queues every reference the policy marks as gameplay.
- `Finish` writes `resource_index.json`, `field_definitions.json`, `native_types.json`,
  `selection_report.json` and `generation.json` (document type `gameplay_generation`, status
  `failed` when any error was recorded, and the policy digest).

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_SourceExportGeneration` and `TBD_SourceExportEnvironment` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Generation/`,
  `TBD_SourceContainerReader` in `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Source/`,
  `TBD_SourceCapabilityBuilder` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Capabilities/`, and the policy
  and writers in `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/`.
- Used by: `TBD_EquipmentVehicleExportPlugin` and the `start` action of `EMCP_WB_SourceExport`.
- Rules: a resource is captured once and identified by its GUID or, without one, its exact resource
  name; an unreviewed class or field makes the generation `failed`.
