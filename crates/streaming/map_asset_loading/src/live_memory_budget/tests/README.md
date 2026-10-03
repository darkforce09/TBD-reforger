# Live memory budget tests

The native test of the live ledger's page readers: with no page, the budget is the default and
no linear memory is measured. The ledger's own rules are tested with the model in
`crates/streaming/map_streaming_model/src/memory_budget/tests/`.

## Contents

```text
crates/streaming/map_asset_loading/src/live_memory_budget/tests/
└── platform_tests.rs  the native budget and heap readers
```

## Boundaries

- Depends on: the parent module's surface through `use super::*` (`configured_budget_bytes`,
  `heap_bytes`); `map_streaming_model::memory_budget` (`DEFAULT_BUDGET_MB`, `MIB`).
- Used by: nothing outside the folder; `mod.rs` compiles it only in test builds.
- Rules:
  - a native build takes the default budget and measures no linear memory
    (`a_native_build_takes_the_default_budget_and_measures_no_linear_memory`).
