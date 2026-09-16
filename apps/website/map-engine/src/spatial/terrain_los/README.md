# spatial/terrain_los

Point indexing, BVH construction and traversal, terrain ray marching, viewshed jobs, and streamed world occlusion.

## Contents

- `march.rs`
- `mod.rs`
- `overlay.rs`
- `sampler.rs`
- `scheduler.rs`
- `viewshed.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
