# Equipment gameplay selection policy

The Workbench side of the gameplay selection policy: the class that answers which classes and
fields enter the gameplay dataset, and the tables generated from the reviewed rules in
`contracts/rules/equipment-gameplay/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/
├── Generated/                      the generated tables, one folder per policy section
└── TBD_GameplaySelectionPolicy.c   class and field decisions, unreviewed keys and the selection report
```

## How it works

`TBD_GameplaySelectionPolicy` fills itself on construction through
`TBD_GameplayPolicyGenerated.Apply`, which calls every generated section table. `AddClass` files a
native class under a section and `AddRule` gives a set of (property, native type) fields a
disposition, a section, a reason and whether the reference is followed.

The source reader asks `Selected` for each container class, so a class filed under `excluded`
stays out and its fields are only counted, and asks `Rule` for each property it reads. A class or
field with no decision is recorded once as unreviewed and becomes a generation error.
`GameplayReference` limits followed references to `.et`, `.conf` and `.gamemat` resources, and
`Report` writes `selection_report.json` with every decision, how often it was observed and the
unreviewed keys.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the generated tables in `Generated/`; `TBD_SourceExportJson` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Serialization/`.
- Used by: `TBD_GameplayExportGeneration` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Generation/`,
  `TBD_GameplayResourceWriter` and `TBD_GameplayFieldDefinitions` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Serialization/`, and
  `TBD_SourceContainerReader` in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Source/`.
- Rules: `Generated/` is written only by `cargo xtask mod generate-equipment-gameplay-policy` from
  `contracts/rules/equipment-gameplay/policy.json` and its class files, one section folder per
  policy section with at most 180 table lines per class; `--check` fails on drift or on any file
  the generator did not write.

## Related documentation

- [Equipment gameplay selection policy](/contracts/rules/equipment-gameplay/README.md) — the
  reviewed rows, their dispositions and sections.
