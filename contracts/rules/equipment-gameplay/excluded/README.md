# Excluded classes of the gameplay policy

The policy rows of classes no gameplay consumer reads: presentation, engine infrastructure, AI and
cosmetic configuration. Every field here is excluded with its reason, so the export never walks into
these classes.

## Contents

```text
contracts/rules/equipment-gameplay/excluded/
└── classes_*.json  the section's class rows, one JSON array per file
```

## How it works

The rows run across twelve files so each stays small; `policy.json` lists every one in
`class_files`, and their order there is the order the policy digest hashes them. Every rule here has
the disposition `exclude` and the reason "No gameplay-catalog consumer: presentation, engine
infrastructure, AI, or cosmetic configuration". A class in this section is not selected, so the
export records no component branch of it; when a retained class requires a relationship to one, the
published graph keeps the target's identity and exclusion reason and none of its values.

## Format

- Encoding: UTF-8 JSON, one array of class rows per file, named `classes_NN.json` and numbered from
  `01`.
- Schema: the class row the
  [equipment gameplay selection policy](/contracts/rules/equipment-gameplay/README.md#how-it-works)
  describes; every row here has the section `excluded`.
- Adding a file: list it in the `class_files` of
  `contracts/rules/equipment-gameplay/policy.json`, then run
  `cargo xtask mod generate-equipment-gameplay-policy` and commit the regenerated tables.

## Producers and consumers

- Producers: people, reviewing the classes a complete diagnostic generation reports for this
  section.
- Consumers: `Policy::load` in `tools/commands/mod_operations/src/equipment_gameplay/policy.rs`,
  through which the xtask equipment gameplay and equipment export commands read the rows;
  `cargo xtask mod generate-equipment-gameplay-policy` writes them into
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/excluded/`.

## Boundaries

- Depends on: the native class, property and type names of the section's Enfusion classes, and the
  manifest `contracts/rules/equipment-gameplay/policy.json`, which lists these files.
- Used by: the policy loader and, through the generated selection tables, the export addon's
  gameplay selection.
- Rules: every row's `section` is `excluded`; a class or a (class, property, native type) field
  listed here appears nowhere else in the policy (`Policy::load`).
