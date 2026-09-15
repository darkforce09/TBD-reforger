# spatial/bvh

Point indexing, BVH construction and traversal, terrain ray marching, viewshed jobs, and streamed world occlusion.

## Contents

- `mod.rs`
- `node.rs`
- `sidecar.rs`
- `surface.rs`
- `tests`
- `traversal.rs`
- `tree`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
