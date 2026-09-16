# streaming/host

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `bootstrap.rs`
- `mod.rs`
- `preferences.rs`
- `queries.rs`
- `state.rs`
- `terrain.rs`
- `viewport.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
