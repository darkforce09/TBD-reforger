# streaming/scheduler/residency

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `mod.rs`
- `t151_11_3_tests`
- `t152_3_tests`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
