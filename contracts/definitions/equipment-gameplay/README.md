# Equipment gameplay contracts

The schemas of a gameplay generation: the purpose-selected native gameplay data the
[Workbench](/documentation/glossary/n_to_z.md#workbench) equipment export writes, and the
receipt its publication records. The xtask equipment export commands embed them and validate
every generation against them before it is published.

## Contents

```text
contracts/definitions/equipment-gameplay/
├── field-definitions.schema.json    `field_definitions.json`: the fields a generation retains
├── generation.schema.json           `generation.json`: the generation's identity, scope, status and errors
├── publication-receipt.schema.json  the receipt a publication writes: sizes and diagnostic basis
├── resource-index.schema.json       `resource_index.json`: every resource and its file
└── resource.schema.json             one file under `resources/`: a resource's nodes and facts
```

## How it works

`cargo xtask mod validate-equipment-vehicle-export` and `publish-equipment-vehicle-export` read
the generation folder's `generation.json`; when its `document_type` is `gameplay_generation`,
`tools/commands/mod_operations/src/equipment_gameplay/validation.rs` validates
`generation.json`, `resource_index.json` and `field_definitions.json` against their schemas and
every resource file the index names (each under `resources/`) against `resource.schema.json`,
then checks the files against each other. Publishing builds the publication receipt in
`tools/commands/mod_operations/src/equipment_vehicle_export/gameplay_receipt.rs`, validates it
against `publication-receipt.schema.json`, and checks that its generation id and byte counts
match the validated generation. Every schema is embedded with `include_str!`, so a schema change
takes effect on the next xtask build.

## Format

- Encoding: UTF-8 JSON Schema (draft-07), one file per document kind, named in lowercase
  hyphenated words after that document with the `.schema.json` suffix.
- Schema: every root is closed (`"additionalProperties": false`) and pins `schema_version` to 1;
  `generation.json`, the resource files and the receipt also pin their `document_type`
  (`gameplay_generation`, `gameplay_resource`, `gameplay_publication_receipt`).
- Adding a file: add the schema here, embed it where the document is validated (the `SCHEMAS`
  table of `validation.rs`, or `gameplay_receipt.rs` for the receipt), and run
  `cargo xtask mod validate-equipment-vehicle-export` over a generation that carries it.

## Producers and consumers

- Producers: people write the schemas. The documents they describe come from the Workbench
  gameplay export in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/`,
  and the receipt from `publish-equipment-vehicle-export`.
- Consumers: `equipment_gameplay/validation.rs` and
  `equipment_vehicle_export/gameplay_receipt.rs` under `tools/commands/mod_operations/src/`,
  run by `cargo xtask mod validate-equipment-vehicle-export` and
  `cargo xtask mod publish-equipment-vehicle-export`.

## Boundaries

- Depends on: the gameplay selection policy in `contracts/rules/equipment-gameplay/`, which
  decides which native fields a generation retains.
- Used by: the xtask equipment export validation and publication named above.
- Rules: a schema change lands with the Workbench export that writes the new shape and with the
  validation that reads it; the embedded paths are pinned by `include_str!`, so a rename updates
  both Rust files in the same change.
