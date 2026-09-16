# terrain/relief

Terrain decoding and sampling, hillshade and contour geometry, satellite selection and downloads, roads, and water.

## Contents

- `contours.rs`
- `hillshade.rs`
- `host.rs`
- `mod.rs`
- `sea_band.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
