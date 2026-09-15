# streaming/scheduler

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `budget.rs`
- `chunk_math.rs`
- `mod.rs`
- `queries.rs`
- `residency`
- `state.rs`
- `tests`
- `viewport.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
