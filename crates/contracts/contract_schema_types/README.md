# Contract schema types

The `contract_schema_types` crate: the Rust serde types `typify` generates from the JSON Schemas
in `contracts/definitions/`, one module per API domain and one module per schema below it. Every
file under `src/generated/` is written by `cargo xtask schema codegen` (also the `schema-codegen`
row of `cargo xtask ci`) and never edited by hand; `cargo xtask ci verify-codegen-fresh` proves
the tree equals a fresh render of the schemas.

## Contents

```text
crates/contracts/contract_schema_types/
├── Cargo.toml  the package: `chrono`, `regress`, `serde`, `serde_json`, `uuid`; layout tier 0
└── src/        the hand-written crate root and prelude, and the generated module tree
```

## How it works

The codegen (`tools/commands/schema_tooling/src/generate/schema_types.rs`) maps each schema file to a
module path: the API domain that serves or reads the schema, then the schema's own module, such
as `operations::event_hub` for `event-hub.schema.json` or
`community_content::equipment_data_viewer::dataset` for
`equipment-data-viewer/dataset.schema.json`. A schema module holds one file per schema definition
(with its derived types and impls) and typify's `error` module, and re-exports every definition's
types from its `mod.rs`; a definition too large for one 500-line file becomes a folder. The
codegen also writes the `mod.rs` of the generated root, of each domain and of each grouping
folder, so adding a schema is a row in the codegen's table and a regeneration.

Every type implements `Serialize`, `Deserialize`, `Clone` and `Debug`. A string the schema
constrains is a newtype whose `FromStr`, `TryFrom` and `Deserialize` check the constraint (the
`pattern` with `regress`, the ECMA-262 engine JSON Schema patterns are written for); `date-time`
strings are `chrono::DateTime<Utc>`, except in `current-profile.schema.json`, whose timestamps
stay `String` to keep their exact fractional precision through a round trip; `uuid` strings are
`uuid::Uuid`. The loadout export model is not generated: its versioned root `oneOf` does not
survive typify, so the API keeps it by hand in
`crates/api/api_missions/src/contract/loadout_projection.rs`.

## Getting started

Run from the repository root:

```bash
cargo xtask schema codegen               # regenerate src/generated/ after changing a schema
cargo xtask ci verify-codegen-fresh      # check src/generated/ equals a fresh render
cargo clippy -p contract_schema_types    # the crate compiles under the workspace lints
```

## Configuration

None: the crate reads no environment variable and declares no feature.

## Public surface

- One module per API domain, re-exported from the crate root: `administration`,
  `community_content`, `identity_and_access`, `match_telemetry`, `missions`, `operations`,
  `server_infrastructure`. Each holds one module per schema, named after the schema file
  (`event-hub.schema.json` is `operations::event_hub`).
- `prelude`: the domain modules, for a glob import.

## Boundaries

- Depends on: `chrono`, `regress`, `serde`, `serde_json` and `uuid`; no workspace crate.
- Used by: `crates/api/api_server` (the registry import decodes registry envelopes into `missions`, the
  equipment data viewer and media upload handlers re-read their answers as their contract types)
  and the API's contract tests, which decode live answers into these types.
- Rules:
  - `src/generated/` is codegen output: `cargo xtask ci verify-codegen-fresh` fails on a missing,
    changed or stray Rust file, and its files are exempt from the typed id rule of
    `cargo xtask verify crate-anatomy` by their `generated` folder;
  - `lib.rs` allows the lints typify's output trips on the `generated` module (`missing_docs`,
    `clippy::unwrap_used`, `clippy::module_inception`, `clippy::derivable_impls`,
    `rustdoc::invalid_html_tags`) and says why
    beside the attribute; every other workspace lint applies;
  - every generated file stays within 500 lines (`cargo xtask verify file-length`): the codegen
    splits a large definition and refuses a file it cannot split;
  - contracts tier, so the crate depends on no workspace crate outside the foundation tier
    (`cargo xtask verify crate-tiers`).

## Related documentation

- [Contract definitions](/contracts/definitions/README.md) — the schemas the types are generated
  from.
- [Code generators](/tools/commands/schema_tooling/src/generate/README.md) — the codegen and its
  freshness check.
- [Schema evolution policy](/documentation/contracts/schema_evolution_policy.md) — how a schema
  change reaches its generated types.
