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

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
