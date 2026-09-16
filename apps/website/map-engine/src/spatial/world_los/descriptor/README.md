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

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
