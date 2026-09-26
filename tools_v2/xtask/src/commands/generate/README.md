# Code generators

The `cargo xtask gen` group, and the contract codegen behind `cargo xtask schema codegen`: the
JSON Schemas in `contracts_v2/definitions/` become the Rust serde types the website
[API](/documentation_v2/glossary/a_to_f.md#api) compiles, with no Node in the pipeline. Developers run
the codegen after changing a schema, and CI checks its output is fresh.

## Contents

```text
tools_v2/xtask/src/commands/generate/
├── cli.rs            the `GenCmd` clap enum: `font-table`
├── dispatch.rs       routes `font-table` to its generator in the verifications tree
├── mod.rs            the module tree
├── schema_types/     partitions typify's output by definition and renders the module files
├── schema_types.rs   the schema-to-module table, `codegen`, `verify_fresh`, typify and rustfmt
└── tests/            unit tests for the codegen's module folders and the freshness check
```

## How it works

`schema_types.rs` holds `TARGETS`, a table from each schema file to the generated module folder
of its owning domain under `apps/website/api_v2/src/`, such as
`missions/contract/generated/registry_items` or `operations/models/generated/event_hub`. For each
target it parses the schema, runs `typify` (deriving `Debug`; the date-time strings of
`current-profile.schema.json` stay `String` to keep their exact precision), splits and renders the
result through `schema_types/`, and formats every file with `rustfmt --edition 2024`.

```text
contracts_v2/definitions/<schema>.json
  └─ typify ─▶ module_plan::partition ─▶ module_files::render_files ─▶ rustfmt
       ├─ codegen:      write the folder, delete stray .rs files, empty folders and a <target>.rs
       └─ verify_fresh: compare in memory; a missing, changed or stray file is an error
```

The loadout export model is not generated: its versioned root `oneOf` does not survive typify, so
`apps/website/api_v2/src/missions/contract/loadout_projection.rs` is written by hand, and
`codegen` says so when it finishes.

## Commands

### gen font-table

- Synopsis: `cargo xtask gen font-table <bdf>`
- Does: reads a Spleen 16x32 BDF font and prints to stdout a Rust module with `FONT_GLYPH_W`,
  `FONT_GLYPH_H` and `FONT_16X32`: thirty-two sixteen-bit pixel rows for each ASCII glyph from
  U+0020 to U+007E, plus an empty slot 95, under the font's BSD-2-Clause notice. Four sample glyphs
  are drawn on stderr. The generator lives in
  `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/repository_access.rs`.
- Exit codes: 0 printed; 1 (`xtask:` error) an unreadable file, a glyph that is not a full 16x32
  cell or has a short bitmap, or a missing glyph.
- Example: `cargo xtask gen font-table spleen-16x32.bdf`

### schema codegen and the freshness check

- Synopsis: `cargo xtask schema codegen` (also the `schema-codegen` row of `cargo xtask ci`);
  `cargo xtask ci verify-codegen-fresh`. Both are defined in their own groups and call
  `schema_types::codegen` and `schema_types::verify_fresh`.
- Does: `codegen` rewrites every target folder and prints one line per schema with its file
  count; `verify_fresh` renders every target in memory and compares, writing nothing and seeing
  untracked files too.
- Exit codes: 0 written, or fresh; 1 a schema that fails to parse or convert, a generated file
  over 500 lines, `rustfmt` failing, or, for the check, the first missing, stale or stray file.
- Example: `cargo xtask schema codegen`

## Boundaries

- Depends on: `developer_tools::repository_layout::contract_definitions_dir`;
  `crate::core::repository_root`; `crate::verifications::language_bans::node_and_file_limits` for
  `font-table`; the `typify`, `schemars`, `syn`, `serde_json` and `walkdir` crates; `rustfmt` on
  `PATH`.
- Used by:
  - `tools_v2/xtask/src/cli/dispatch.rs`, for `cargo xtask gen`;
  - `tools_v2/xtask/src/commands/schema/dispatch.rs` (`schema codegen`) and
    `tools_v2/xtask/src/commands/ci/task_definitions.rs` (`ci schema-codegen`);
  - `verify_codegen_fresh` in `tools_v2/xtask/src/commands/ci/editor_api.rs`
    (`ci verify-codegen-fresh`, a step of `ci ci-local-schema` and so of `ci ci-local`).
- Rules: the `generated/` folders under `apps/website/api_v2/src/` are written only by
  `schema codegen` and never edited by hand, which `ci verify-codegen-fresh` checks; a target
  never keeps a single-file form beside its folder
  (`schema_codegen_removes_what_the_schemas_no_longer_produce` in `tests/schema_types.rs`); every
  generated file stays within the production line limit
  (`schema_split_keeps_every_typify_item_and_bounds_every_file`); the check sees a missing,
  changed or stray file without git
  (`schema_freshness_detects_missing_changed_and_stray_outputs_without_git`).

## Related documentation

- [Contract definitions](/contracts_v2/definitions/README.md) — the schemas the codegen reads.
