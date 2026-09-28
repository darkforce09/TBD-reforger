# Equipment gameplay dataset

The part of the equipment and vehicle exporter that produces the published gameplay dataset: the
reviewed selection policy compiled into [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
tables, the generation that applies it, and the compact per-resource files it writes.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/
├── Generation/     the gameplay generation: resource capture, reference queue, index and reports
├── Policy/         the selection policy and the tables generated from the reviewed rules
└── Serialization/  the gameplay resource document and the field definitions
```

## How it works

```text
contracts_v2/rules/equipment-gameplay/
        │  cargo xtask mod generate-equipment-gameplay-policy
        ▼
Policy/Generated/ selection tables, compiled into the addon
        │
"Export Equipment and Vehicles" / EMCP_WB_SourceExport start
        ▼
Generation/ TBD_GameplayExportGeneration: discovery census ─▶ per resource:
   Source/ reader with the Policy/ selection ─▶ Serialization/ resource document
   approved references (.et .conf .gamemat) join the queue
        ▼
$profile:TBD_Export/equipment_vehicle_exports/gameplay/generations/<generation id>/
   generation.json · resource_index.json · field_definitions.json · native_types.json
   selection_report.json · resources/… · shared_configurations/…
```

The generation reuses the discovery census, the source reader and the JSON encoding of the
exporter; the policy decides, class by class and field by field, what enters the dataset. A class
or a (class, property, native type) field the policy has not reviewed is an error of the
generation, recorded in its report, so a new engine field cannot enter or leave the dataset
unnoticed. `generation.json` carries the policy's SHA-256 digest, which the generated tables hold.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the rest of the source exporter in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`: `TBD_SourceExportGeneration`
  (its generation folder), `TBD_SourceResourceDiscovery` (discovery), `TBD_SourceContainerReader`
  (source reader), `TBD_SourceCapabilityBuilder` and `TBD_SourceCapabilityRules` (capabilities) and
  `TBD_SourceExportJson` (serialization).
- Used by: `TBD_EquipmentVehicleExportPlugin` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Plugins/` and the `start`
  action of `EMCP_WB_SourceExport` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Verification/`; the source reader,
  which consults the policy when a generation hands it one.
- Rules: the tables under `Policy/Generated/` are generated and never edited by hand
  (`cargo xtask mod generate-equipment-gameplay-policy --check` refuses drift and any extra file);
  a gameplay generation is validated and published by
  `cargo xtask mod validate-equipment-vehicle-export` and `publish-equipment-vehicle-export`, which
  read its `gameplay_generation` document type; the files follow the schemas in
  `contracts_v2/definitions/equipment-gameplay/`.

## Related documentation

- [Equipment gameplay selection policy](/contracts_v2/rules/equipment-gameplay/README.md) — the
  reviewed rows the tables are generated from.
- [Equipment gameplay commands](/tools_v2/xtask/src/commands/mod_ops/equipment_gameplay/README.md)
  — table generation, projection and validation of gameplay generations.
