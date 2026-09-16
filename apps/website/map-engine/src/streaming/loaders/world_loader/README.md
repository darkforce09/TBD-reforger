# streaming/loaders/world_loader

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `atlas.rs`
- `bootstrap.rs`
- `ingest.rs`
- `metrics.rs`
- `mod.rs`
- `state.rs`
- `terrain.rs`
- `upload.rs`
- `viewport.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
