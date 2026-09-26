# Contract schema gates

The checks behind the contract gates of `cargo xtask schema`: `validate`, `validate-file`,
`citations`, `map-object-enums`, `type-inventory` and `map-glyphs`. They hold the JSON Schemas in
`contracts_v2/definitions/`, every fixture and terrain document that follows them, and every
`@contract` citation in code to one another.

## Contents

```text
tools_v2/xtask/src/verifications/schemas/
├── checks/    one module per gate, the validation suite, and the shared JSON and verdict helpers
├── checks.rs  the shared constants and pins, the submodule wiring, and the six gate entries
├── mod.rs     the module tree
└── tests/     unit tests for the instance-kind lockstep, unread wire fields, the objective spine, goldens
```

## How it works

`cargo xtask schema <gate>` calls one entry of `checks`, which reads the contracts tree through
`developer_tools::repository_layout` and returns 0 or 1. The six gates split into:

- the document suite (`validate`) and the one-file [mission](/documentation_v2/glossary/g_to_m.md#mission) check (`validate-file`), in
  `checks/contract_validation/`;
- the code-to-schema check (`citations`), which walks `apps/` and `tools_v2/`;
- the map-object gates (`map-object-enums`, `type-inventory`, `map-glyphs`), which keep the map
  data, the rules and the glyph set inside the closed enums of `map-object-enums.schema.json`.

`cargo xtask ci schema-validate` runs `validate`, `map-glyphs`, `map-object-enums` and
`type-inventory` from here with the map asset gates `map-object-golden` and `height-labels` from
`tools_v2/xtask/src/verifications/map_assets/`, and `cargo xtask schema list-gates` prints that
set. The unit tests in `tests/` go beyond the gates: they check that every property of the
mission schema's objective spine appears as an identifier in the [mod](/documentation_v2/glossary/g_to_m.md#mod)'s objective scripts, that the
keys of the hand-staged schemaVersion 1.3 golden are members of the objective reader's structs,
and the side-fallback branches of the objective sources.

The Rust contract types generated from the same schemas come from `cargo xtask schema codegen`
(`tools_v2/xtask/src/commands/generate/`), and `schema flatten-orbat-slots` lives in
`tools_v2/xtask/src/commands/schema/mission_flattening.rs`.

## Public surface

- `checks::validate_all`, `checks::validate_file`, `checks::citations`,
  `checks::map_object_enums`, `checks::type_inventory` and `checks::map_glyphs`: the gate entries
  `tools_v2/xtask/src/commands/schema/dispatch.rs` and `tools_v2/xtask/src/commands/ci/task_definitions.rs`
  call.

## Boundaries

- Depends on: `developer_tools` (`repository_layout`, `world_export_pipeline::INSTANCE_KINDS`);
  `crate::core::repository_root`; `jsonschema`, `regex`, `walkdir`, `serde_json`; the schemas,
  fixtures, rules and catalogs under `contracts_v2/`, the terrain documents under
  `assets_v2/terrains/`, and the mod sources under `apps/mod/tbd-framework/`.
- Used by: `tools_v2/xtask/src/commands/schema/dispatch.rs`;
  `tools_v2/xtask/src/commands/ci/task_definitions.rs` (`schema-validate`, `verify-citations`, and
  through them `ci-local-schema` and `ci-local`); the platform [wave](/documentation_v2/glossary/n_to_z.md#wave) gate's schema step
  (`tools_v2/xtask/src/commands/platform/wave_execution/schema.rs`); the `schema` job of
  `.github/workflows/ci.yml` and `.github/workflows/schema.yml`.
- Rules:
  - A gate that examined nothing fails: a missing citation root or zero citations, a missing enum
    `$defs`, a missing fixture file.
  - Two pins run inside the gates as well as in the tests, so every gate run checks them:
    `instance_kinds_lockstep_failures` inside `type_inventory` and `unread_wire_field_failures`
    inside `validate_all`.
  - The `schema-validate` gate set changes together with the wave gate's list and the `ci.yml`
    schema job (`cargo xtask verify ci-schema-parity`).

## Related documentation

- [Contract definitions](/contracts_v2/definitions/README.md) — the schemas these gates hold the
  code and data to.
- [Schema command group](/tools_v2/xtask/src/commands/schema/README.md) — the `schema` commands and
  their flags.
