# spatial/world_los/descriptor

Point indexing, BVH construction and traversal, terrain ray marching, viewshed jobs, and streamed world occlusion.

## Contents

- `archive.rs`
- `bounds.rs`
- `manifest.rs`
- `mod.rs`
- `model.rs`
- `projection.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
