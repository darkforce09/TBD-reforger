# Equipment gameplay resource files

The two writers of the gameplay dataset's documents: one compact file per resource, and the
definition of every field those files hold.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Serialization/
├── TBD_GameplayFieldDefinitions.c  each selected field once: class, property, field name, section, rule
└── TBD_GameplayResourceWriter.c    the `gameplay_resource` document of one captured resource
```

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_SourceContainerReader` and its nodes in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Source/`,
  `TBD_SourceCapabilityBuilder` and `TBD_SourceCapabilityRules` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Capabilities/`,
  `TBD_SourceExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Serialization/`,
  and the selection policy in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/`.
- Used by: `TBD_GameplayExportGeneration` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Generation/`.
- Rules: a resource document holds its nodes, its fields grouped by policy section, its English
  names (the writer switches the language to `en_us` and restores it), its references and its
  parent prefab; the documents follow `contracts/definitions/equipment-gameplay/resource.schema.json`
  and `contracts/definitions/equipment-gameplay/field-definitions.schema.json`.
