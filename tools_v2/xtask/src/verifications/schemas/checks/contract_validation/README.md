# Contract validation suite

The bodies of `cargo xtask schema validate` and `cargo xtask schema validate-file`: JSON Schema
validation of every committed fixture, sample and terrain document against the contract
definitions in `contracts_v2/definitions/`, and the check of one
[mission](/documentation_v2/glossary.md#mission) file. The parent file
`tools_v2/xtask/src/verifications/schemas/checks/contract_validation.rs` declares both and
re-exports their entries.

## Contents

```text
tools_v2/xtask/src/verifications/schemas/checks/contract_validation/
├── validate_all.rs   `validate_all`: the full suite, one PASS or FAIL line per document, the failure count
└── validate_file.rs  `validate_file`: one mission file or stdin against `mission.schema.json` and its ceilings
```

## How it works

`validate_all` registers the map-object, terrain-registry and mission schemas by their `$id` so
cross-file `$ref`s resolve, compiles one validator per contract, and prints a titled section per
document family. The mission goldens, the unread 1.3 wire fields, the byte ceiling, the kit alias
cross-reference and the negative goldens run in
`tools_v2/xtask/src/verifications/schemas/checks/mission_validation.rs`, and the registry, items
and compat documents with their reference walks in `registry_validation.rs` beside it. This file
then validates the faction library, the loadout exports, the [Mission Creator](/documentation_v2/glossary.md#mission-creator) payload, the bridge
message samples, the Everon terrain manifest, locations, height labels and anchors example, the
[Enfusion](/documentation_v2/glossary.md#enfusion) DTO fixtures (each against the mission schema's `#/$defs/<name>`), the map-object
samples and catalog bundle, the terrain registry and manifests, and the type inventories. Its last
section reads `TBD_MissionValidator.c` and requires the five unconsumed-key warnings and their
marker comments, and the `empty-warning-fields.json` golden that authors every such key. It exits
1 when any document failed and 0 with `All contracts valid.`

`validate_file` reads a path, or stdin for `-`, and exits 1 when the text is not JSON, is larger
than the schema's `x-tbd-missionFileMaxBytes` (8 MiB when absent), breaks `mission.schema.json`,
or, for schemaVersion `1.1`, has a [slot](/documentation_v2/glossary.md#slot) count that differs from the [ORBAT](/documentation_v2/glossary.md#orbat)'s role counts or a
repeated slot id. It prints `ok` and exits 0 otherwise.

## Boundaries

- Depends on: the parent module's imports (`read_json`, `schema_root`, the `developer_tools`
  repository layout functions for the contract, catalog, fixture and terrain paths);
  `jsonschema`; `serde_json`.
- Used by: `tools_v2/xtask/src/commands/schema/dispatch.rs` (`schema validate`,
  `schema validate-file`); the `schema-validate` row of
  `tools_v2/xtask/src/commands/ci/task_definitions.rs`; the platform [wave](/documentation_v2/glossary.md#wave) gate's schema step
  (`tools_v2/xtask/src/commands/platform/wave_execution/schema.rs`);
  `.github/workflows/schema.yml`.
- Rules: every family prints its own section and a failure never stops the suite, so one run
  reports every broken document; a schema that fails to compile or a document that fails to read
  aborts the run with an error rather than a pass.
