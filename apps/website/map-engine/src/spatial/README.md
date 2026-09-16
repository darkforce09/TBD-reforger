# spatial

Point indexing, BVH construction and traversal, terrain ray marching, viewshed jobs, and streamed world occlusion.

## Contents

- `bvh`
- `indexing`
- `mod.rs`
- `terrain_los`
- `world_los`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
