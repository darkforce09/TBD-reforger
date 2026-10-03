# Contract schema types source

The source of `contract_schema_types`: a hand-written crate root and prelude, and the module tree
`cargo xtask schema codegen` generates from `contracts/definitions/`.

## Contents

```text
crates/contracts/contract_schema_types/src/
├── generated/  the codegen output: one module per API domain, one module per schema below it; never edited by hand
├── lib.rs      the crate root: module header, the lint allowance on `generated`, its re-export
└── prelude.rs  the domain modules, for a glob import
```

## How it works

`lib.rs` declares the private `generated` module under its lint allowance and re-exports its
domain modules, so callers write `contract_schema_types::operations::event_hub::EventHub`.
`generated/mod.rs`, each domain's `mod.rs` and each grouping folder's `mod.rs` are written by the
codegen together with the schema modules, so neither `lib.rs` nor `prelude.rs` changes when a
schema or a domain is added.

## Boundaries

- Depends on: `chrono`, `regress`, `serde`, `serde_json` and `uuid`, all reached from the
  generated code.
- Used by: the API library and its contract tests.
- Rules: nothing under `generated/` is edited by hand (`cargo xtask ci verify-codegen-fresh`);
  regenerate with `cargo xtask ci schema-codegen`.
