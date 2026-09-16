# diagnostics/readback

Readback self-checks, frame benchmarks, timing queries, hardware probes, and browser console output.

## Contents

- `compute_cull.rs`
- `doll.rs`
- `marquee.rs`
- `mod.rs`
- `road_centerline.rs`
- `scene.rs`
- `sea_band.rs`
- `text.rs`
- `texture.rs`
- `tree_glyph.rs`
- `world_building.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
