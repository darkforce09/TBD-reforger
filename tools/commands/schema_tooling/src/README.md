# Schema tooling source

The modules of the `schema_tooling` crate: the generators, the contract schema gates, the ORBAT
slot flattening, the crate's errors and the unit tests of the gates and the flattening.

## Contents

```text
tools/commands/schema_tooling/src/
├── error.rs               `Error` and `Result`, the context extension and the `refuse!` macro
├── generate/              the contract codegen, the font-table generator and the `gen` group
├── generate.rs            declares the generators and the `gen` group's arguments and dispatch
├── lib.rs                 the crate root: module header, `mod` lines and the re-exports
├── mission_flattening.rs  `flatten_orbat_slots`: a mission's ORBAT template into its `slots[]`
├── prelude.rs             every public entry for glob import
├── schema_checks/         one module per contract gate, the validation suite and the shared helpers
├── schema_checks.rs       the gates' shared constants and pins, the submodule wiring and the entries
└── tests/                 unit tests for the gates' pins, the citation scope and the flattening
```

## How it works

- `generate`: `schema_types::codegen` renders every schema of its `TARGETS` table with typify into
  the generated folder of `contract_schema_types` and formats each file with rustfmt;
  `verify_fresh` renders in memory and compares. `font_table::gen_font_table` prints the Spleen
  font's Rust glyph table. `generate/README.md` holds the commands and their exit codes.
- `schema_checks`: each gate entry reads the contracts tree and prints `<gate>: OK` or
  `<gate>: FAIL (<n>)`; `schema_checks/README.md` lists what each gate checks.
- `mission_flattening`: reads a mission, writes `slots[]` from its ORBAT template while keeping
  each slot's loadout and uid, refuses to write an empty `slots[]` over a non-empty one, and prints
  the result or writes it in place.
- `error`: a refusal displays its whole text; a context line displays `context: cause`, so
  `xtask: {error:#}` prints the chain as the command always has.

## Boundaries

- Depends on: the crates `Cargo.toml` names; `rustfmt` on `PATH` for the codegen.
- Used by: the crate root's re-exports, called by the xtask binary's `schema`, `gen` and `ci`
  groups.
- Rules: `tests/schema_checks/` holds the pins the gates also run (the instance-kind lockstep, the
  unread wire fields) and the citation scope contract; `tests/mission_flattening.rs` the
  flattening's preserve and refuse rules; `generate/tests/` and `generate/schema_types/tests/` the
  codegen's layout and freshness.
