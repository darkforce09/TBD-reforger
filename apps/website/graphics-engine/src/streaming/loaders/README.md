# streaming/loaders

Asset fetching, chunk residency, upload budgets, packed buffers, memory accounting, and host callbacks.

## Contents

- `chunk.rs`
- `chunk_bin.rs`
- `fetch.rs`
- `manifest.rs`
- `mod.rs`
- `occluder_loader.rs`
- `prefab.rs`
- `residency.rs`
- `store.rs`
- `tests`
- `world_loader`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
