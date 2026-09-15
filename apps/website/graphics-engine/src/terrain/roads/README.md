# terrain/roads

Terrain decoding and sampling, hillshade and contour geometry, satellite selection and downloads, roads, and water.

## Contents

- `airfield.rs`
- `cartographic_strip.rs`
- `mesh.rs`
- `mod.rs`
- `network.rs`
- `styling.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
