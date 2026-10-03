# Schema tooling

The `schema_tooling` crate: the contract tooling behind `cargo xtask schema` and `cargo xtask gen`.
It renders the Rust contract types of the `contract_schema_types` crate from the JSON Schemas in
`contracts/definitions/`, runs the contract gates that hold those schemas, every fixture and
terrain document that follows them and every `@contract` citation in code to one another,
flattens a [mission](/documentation/glossary/g_to_m.md#mission)'s ORBAT template into its slots,
and generates the Spleen font table.

## Contents

```text
tools/commands/schema_tooling/
├── Cargo.toml  the `schema_tooling` library package: typify, the schema gates' dependencies, layout tier 5
└── src/        the generators, the contract schema gates, the ORBAT slot flattening and the errors
```

## How it works

`cargo xtask schema <command>` and `cargo xtask gen <command>` parse their arguments in the xtask
binary and call one entry of this crate, which reads the checkout through `repository_layout` and
returns the exit code (0 pass, 1 findings). The entries split into:

- the contract codegen (`schema codegen`, `ci schema-codegen`) and its freshness check
  (`ci verify-codegen-fresh`), in `src/generate/`: typify renders each schema into its module
  folder of `crates/contracts/contract_schema_types/src/generated/`, and rustfmt formats it;
- the document suite (`schema validate`) and the one-file mission check (`schema
  validate-file`), in `src/schema_checks/contract_validation/`;
- the code-to-schema check (`schema citations`), which walks the top-level folder of every
  workspace member and `apps/mod/`;
- the map-object gates (`schema map-object-enums`, `schema type-inventory`, `schema map-glyphs`),
  which keep the map data, the rules and the glyph set inside the closed enums of
  `map-object-enums.schema.json`;
- `schema flatten-orbat-slots`, which writes a mission's `slots[]` from its ORBAT template;
- `gen font-table`, which prints the Rust glyph table of a Spleen 16x32 BDF file.

`cargo xtask ci schema-validate` runs `validate`, `map-glyphs`, `map-object-enums` and
`type-inventory` from here with the map asset gates `map-object-golden` and `height-labels` of
xtask's map asset verifications, and `cargo xtask schema list-gates` prints that set. The unit
tests go beyond the gates: they check that every property of the mission schema's objective spine
appears as an identifier in the [mod](/documentation/glossary/g_to_m.md#mod)'s objective scripts,
that the keys of the hand-staged schemaVersion 1.3 golden are members of the objective reader's
structs, and the side-fallback branches of the objective sources. `src/README.md` describes each
module.

## Getting started

Run from the repository root:

```bash
cargo test -p schema_tooling     # the codegen layout, the gate pins, the citation scope and the flattening
cargo xtask schema validate      # the document suite over the committed contracts
cargo xtask ci schema-codegen    # regenerate the contract types after a schema change
```

## Configuration

No feature and no environment variable. The gates read `contracts/`, the terrain documents under
`assets/terrains/` and the mod sources under `apps/mod/tbd-framework/`; the codegen runs `rustfmt`
from `PATH` in the checkout root.

## Public surface

- At the crate root: `codegen`, `verify_fresh`; `validate_all`, `validate_file`, `citations`,
  `map_object_enums`, `type_inventory`, `map_glyphs`; `flatten_orbat_slots`; `GenCmd` and
  `run_gen_command`; `Error` and `Result`.
- `prelude`: every entry above but `Error` and `Result`.

## Boundaries

- Depends on: `repository_layout` (the contract folders), `repository_laws` (the workspace
  members the citation scan derives its roots from), `process_runner` (rustfmt), `content_digest`,
  `ticket_registry` (the empty-write refusal), `typify`, `schemars`, `syn`, `prettyplease`, `heck`,
  `jsonschema`, `regex`, `walkdir`, `serde_json`, `clap`, `thiserror`, and as a temporary edge the
  `developer_tools` library (`world_export_pipeline::INSTANCE_KINDS`); `tool_test_support` in
  tests.
- Used by: `tools/xtask/src/commands/schema/dispatch.rs`, `tools/xtask/src/cli/dispatch.rs`
  (`gen`) and `tools/commands/ci_task_catalog/src/` (`schema-validate`, `schema-codegen`,
  `verify-citations`, `verify-codegen-fresh`, and through them `ci-local-schema` and `ci-local`);
  the platform [wave](/documentation/glossary/n_to_z.md#wave) gate's schema step; the `schema` job
  of `.github/workflows/ci.yml` and `.github/workflows/schema.yml`.
- Rules:
  - tier 5 of `tools/commands` (`cargo xtask verify crate-tiers`).
  - A gate that examined nothing fails: an unreadable workspace, a missing citation root or zero
    citations, a missing enum `$defs`, a missing fixture file.
  - Two pins run inside the gates as well as in the tests, so every gate run checks them:
    `instance_kinds_lockstep_failures` inside `type_inventory` and `unread_wire_field_failures`
    inside `validate_all`.
  - The `schema-validate` gate set changes together with the wave gate's list and the `ci.yml`
    schema job (`cargo xtask verify ci-schema-parity`).
  - Everything under the generated folder is generator output: the codegen removes a Rust file it
    does not render and the freshness check refuses one.

## Related documentation

- [Contract definitions](/contracts/definitions/README.md) — the schemas these gates hold the
  code and data to.
- [Schema command group](/tools/xtask/src/commands/schema/README.md) — the `schema` commands and
  their flags.
- [Contract schema types](/crates/contracts/contract_schema_types/README.md) — the crate the
  codegen writes.
