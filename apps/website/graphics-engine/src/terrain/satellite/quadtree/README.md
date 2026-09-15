# terrain/satellite/quadtree

Terrain decoding and sampling, hillshade and contour geometry, satellite selection and downloads, roads, and water.

## Contents

- `basemap.rs`
- `bootstrap.rs`
- `decode.rs`
- `downloads.rs`
- `mod.rs`
- `preview.rs`
- `retry.rs`
- `selection.rs`
- `upload.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
