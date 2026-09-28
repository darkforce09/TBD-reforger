# Attachment gameplay policy

The policy rows of weapon attachment classes: attachment types (bayonets, muzzle devices, optics
mounts, stocks, underbarrel launchers), attachment and ejector slots, and magazine wells. Their
native configuration enters the gameplay dataset; a mixed-purpose slot container keeps only its
gameplay settings.

## Contents

```text
contracts_v2/rules/equipment-gameplay/attachment/
└── classes_*.json  the section's class rows, one JSON array per file
```

## Format

- Encoding: UTF-8 JSON, one array of class rows per file, named `classes_NN.json` and numbered from
  `01`.
- Schema: the class row the
  [equipment gameplay selection policy](/contracts_v2/rules/equipment-gameplay/README.md#how-it-works)
  describes; every row here has the section `attachment`, and a rule may still file a mixed-purpose
  field under `excluded`.
- Adding a file: list it in the `class_files` of
  `contracts_v2/rules/equipment-gameplay/policy.json`, then run
  `cargo xtask mod generate-equipment-gameplay-policy` and commit the regenerated tables.

## Producers and consumers

- Producers: people, reviewing the classes a complete diagnostic generation reports for this
  section.
- Consumers: `Policy::load` in `tools_v2/xtask/src/commands/mod_ops/equipment_gameplay/policy.rs`,
  through which the xtask equipment gameplay and equipment export commands read the rows;
  `cargo xtask mod generate-equipment-gameplay-policy` writes them into
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/attachment/`.

## Boundaries

- Depends on: the native class, property and type names of the section's Enfusion classes, and the
  manifest `contracts_v2/rules/equipment-gameplay/policy.json`, which lists these files.
- Used by: the policy loader and, through the generated selection tables, the export addon's
  gameplay selection.
- Rules: every row's `section` is `attachment`; a class or a (class, property, native type) field
  listed here appears nowhere else in the policy (`Policy::load`).
