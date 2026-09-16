# streaming/memory/budget

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `accounting.rs`
- `ledger.rs`
- `mod.rs`
- `model.rs`
- `platform.rs`
- `satellite.rs`
- `stats.rs`
- `t938_6`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
