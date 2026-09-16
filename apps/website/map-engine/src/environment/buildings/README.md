# environment/buildings

World building footprints, vegetation density and canopy geometry, location labels, and classification.

## Contents

- `buffers.rs`
- `footprint.rs`
- `mod.rs`
- `obb.rs`
- `prefab.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
