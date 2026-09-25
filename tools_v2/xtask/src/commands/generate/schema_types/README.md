# Contract codegen module layout

The two halves of the contract codegen that turn one schema's `typify` output into a module
folder: which types go together, and which files they are written to.
`tools_v2/xtask/src/commands/generate/schema_types.rs` declares both files, runs `typify` and
`rustfmt`, and writes or compares the folders.

## Contents

```text
tools_v2/xtask/src/commands/generate/schema_types/
├── module_files.rs  renders the partitioned output as files under the 500-line production limit
├── module_plan.rs   partitions typify's output by schema definition, and strips the quoted schema
└── tests/           unit tests for the partition and the documentation strip
```

## How it works

`module_plan::definition_names` reads the schema's `definitions`, `$defs` and titled root as Rust
names. `partition` walks the generated items: typify's support modules (`error`) stand alone, and
each type joins the definition whose name is the longest prefix of its own, with every impl
following its type; a type that matches no definition is its own. `strip_schema_document` removes
the `<details><summary>JSON schema</summary>` block typify appends to each type's documentation.

`module_files::render_files` writes one file per support module and one per definition, each
opening with the generated-code banner that names the schema and importing from the module root
exactly the types it names, plus a `mod.rs` that declares the support modules and re-exports every
definition's types. A definition whose file would pass `PRODUCTION_LINE_LIMIT` (500 lines) becomes
a folder, with its own type in `mod.rs` and one file per derived type; a file still over the
limit, two files at one path, or a definition named like a support module is an error. Every file
goes through the formatter the caller passes, `rustfmt --edition 2024`.

## Boundaries

- Depends on: `syn` (with its visitor), `prettyplease`, `heck` for name casing, `serde_json`; the
  formatter from `tools_v2/xtask/src/commands/generate/schema_types.rs`.
- Used by: `render_module` in `tools_v2/xtask/src/commands/generate/schema_types.rs`, behind
  `cargo xtask schema codegen` and `cargo xtask ci verify-codegen-fresh`.
- Rules: no generated file passes the production line limit that `cargo xtask verify file-length`
  enforces; a type belongs to the longest definition name it starts with
  (`derived_types_belong_to_the_longest_definition_they_extend` in `tests/module_plan.rs`); an
  unterminated schema quote is refused (`an_unterminated_schema_quote_is_refused`).
