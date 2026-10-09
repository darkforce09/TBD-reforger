# Equipment gameplay commands

The xtask side of the equipment gameplay dataset: loading the reviewed selection policy, generating
the export addon's selection tables from it, projecting a full diagnostic generation through it,
and validating a gameplay generation before it publishes.

## Contents

```text
tools/commands/mod_operations/src/equipment_gameplay/
├── mod.rs                  the module tree and the two command entries, `project_command` and `generate_command`
├── model.rs                the source and compact document shapes: facts, nodes, resources
├── policy.rs               `Policy::load`: the manifest, the class files, the digest and the lookups
├── policy_codegen.rs       writes, or with `--check` compares, the Workbench selection tables
├── projection.rs           `mod project-equipment-gameplay`: a complete source generation into a gameplay one
├── resource_projection.rs  one resource's projection: selected nodes, fields, sections and links
├── tests/                  unit tests for numeric tokens, broken publications and the policy's selections
├── validation.rs           the gameplay generation check the validate and publish commands call
└── validation_resource.rs  one gameplay resource's check: location, identity, fields and references
```

## How it works

`Policy::load` reads `contracts/rules/equipment-gameplay/policy.json` (policy version 1) and the
class files it lists, refuses an unsafe file name, and hashes the manifest and every class file
into the policy digest. Three users share it:

- `generate_command` (`cargo xtask mod generate-equipment-gameplay-policy [--check]`) writes one
  `TBD_GameplayPolicy_<section>_<n>` class per 180 table lines into the export addon's
  `Gameplay/Policy/Generated/` folder, plus `TBD_GameplayPolicyGenerated`, which carries the digest
  and applies every table. With `--check` it writes nothing and fails on any difference or on a file
  it did not generate.
- `project_command` (`cargo xtask mod project-equipment-gameplay --input <dir> --output <dir>`)
  takes a complete source generation (schema version 2) and writes a gameplay generation to a new
  folder: `generation.json` with extraction method `diagnostic_projection`, the resource index,
  field definitions, native types, the selection report and one compact document per resource.
- `validate` is called by `mod validate-equipment-vehicle-export` and
  `mod publish-equipment-vehicle-export` when a generation's `document_type` is
  `gameplay_generation`. It requires a completed, error-free generation whose reader verification
  passed and whose policy digest equals the loaded policy's, no unreviewed field, a decision per
  field that matches the policy, and resource documents whose location, identity, fields and
  references agree with the index; every gameplay reference must resolve inside the generation.

## Boundaries

- Depends on: `contracts/rules/equipment-gameplay/`; `ValidationReport` and the file readers of
  `tools/commands/mod_operations/src/equipment_vehicle_export/`; `tool_test_support`;
  `serde_json`, `content_digest`, `walkdir` and the crate's `Error`.
- Used by: `tools/commands/mod_operations/src/mod_dispatch.rs`, for the two commands;
  `tools/commands/mod_operations/src/equipment_vehicle_export/mod.rs`, which hands gameplay
  generations to `validate`.
- Rules: the generated tables are the only files in their folder and equal what the policy
  generates (`--check`); a gameplay generation built under another policy digest never validates;
  numeric source tokens survive projection unchanged and a broken publication is refused
  (`gameplay_preserves_numeric_tokens_and_rejects_broken_publication` in `tests/catalog.rs`); the policy
  keeps gameplay controls and drops presentation
  (`gameplay_policy_retains_gameplay_controls_and_excludes_presentation`).

## Related documentation

- [Mod commands](/tools/commands/mod_operations/src/README.md) — the `mod` group these commands
  belong to.
- [Equipment gameplay selection policy](/contracts/rules/equipment-gameplay/README.md) — the
  reviewed rows the policy loads.
- [Equipment gameplay dataset](/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/README.md)
  — the Workbench side that runs the generated tables.
