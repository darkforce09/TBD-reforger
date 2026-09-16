# environment/vegetation

World building footprints, vegetation density and canopy geometry, location labels, and classification.

## Contents

- `buffers.rs`
- `canopy.rs`
- `density.rs`
- `loader.rs`
- `mass.rs`
- `mod.rs`
- `regions.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

Vegetation uses the existing density, footprint, and glyph paths; tree-trunk cylinders are absent.
