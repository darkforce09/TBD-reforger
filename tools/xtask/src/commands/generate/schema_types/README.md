# Contract codegen module layout

The parts of the contract codegen that turn one schema's `typify` output into a module folder
(which types go together, and which files they are written to) and that render the module tree
above those folders. `tools/xtask/src/commands/generate/schema_types.rs` declares the three
files, runs `typify` and `rustfmt`, and writes or compares the tree.

## Contents

```text
tools/xtask/src/commands/generate/schema_types/
├── module_files.rs  renders the partitioned output as files under the 500-line production limit
├── module_plan.rs   partitions typify's output by schema definition, and strips the quoted schema
├── module_tree.rs   renders the `mod.rs` of the generated root, each domain and each grouping folder
└── tests/           unit tests for the partition, the documentation strip and the module tree
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

`module_tree::render_tree_files` reads the codegen's schema table and writes one `mod.rs` for the
generated root and for every folder between it and a schema's module, each opening with the
generated-code banner and declaring its children, every declaration with a documentation line
(the schema file for a schema module, the domain or the `contracts/definitions/` subfolder for a
folder). Two schemas sharing a module path, or a schema module that would also hold another
module, is an error.

## Boundaries

- Depends on: `syn` (with its visitor), `prettyplease`, `heck` for name casing, `serde_json`; the
  formatter from `tools/xtask/src/commands/generate/schema_types.rs`.
- Used by: `render_module` and `render_tree` in
  `tools/xtask/src/commands/generate/schema_types.rs`, behind `cargo xtask schema codegen` and
  `cargo xtask ci verify-codegen-fresh`.
- Rules: no generated file passes the production line limit that `cargo xtask verify file-length`
  enforces; a type belongs to the longest definition name it starts with
  (`derived_types_belong_to_the_longest_definition_they_extend` in `tests/module_plan.rs`); an
  unterminated schema quote is refused (`an_unterminated_schema_quote_is_refused`); every tree
  folder declares exactly its children (`every_folder_declares_exactly_its_children` in
  `tests/module_tree.rs`) and a schema module never doubles as a folder
  (`a_schema_module_never_doubles_as_a_folder`).
