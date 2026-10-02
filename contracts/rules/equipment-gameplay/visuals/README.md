# Visuals gameplay policy

The policy rows of preview render attributes, material assignments and mesh objects. Most of their
fields stay out; the references and the few values a catalog preview needs enter the gameplay
dataset.

## Contents

```text
contracts/rules/equipment-gameplay/visuals/
└── classes_*.json  the section's class rows, one JSON array per file
```

## Format

- Encoding: UTF-8 JSON, one array of class rows per file, named `classes_NN.json` and numbered from
  `01`.
- Schema: the class row the
  [equipment gameplay selection policy](/contracts/rules/equipment-gameplay/README.md#how-it-works)
  describes; every row here has the section `visuals`, and a rule may still file a mixed-purpose
  field under `excluded`.
- Adding a file: list it in the `class_files` of
  `contracts/rules/equipment-gameplay/policy.json`, then run
  `cargo xtask mod generate-equipment-gameplay-policy` and commit the regenerated tables.

## Producers and consumers

- Producers: people, reviewing the classes a complete diagnostic generation reports for this
  section.
- Consumers: `Policy::load` in `tools/xtask/src/commands/mod_ops/equipment_gameplay/policy.rs`,
  through which the xtask equipment gameplay and equipment export commands read the rows;
  `cargo xtask mod generate-equipment-gameplay-policy` writes them into
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/visuals/`.

## Boundaries

- Depends on: the native class, property and type names of the section's Enfusion classes, and the
  manifest `contracts/rules/equipment-gameplay/policy.json`, which lists these files.
- Used by: the policy loader and, through the generated selection tables, the export addon's
  gameplay selection.
- Rules: every row's `section` is `visuals`; a class or a (class, property, native type) field
  listed here appears nowhere else in the policy (`Policy::load`).
