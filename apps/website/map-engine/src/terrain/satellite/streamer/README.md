# terrain/satellite/streamer

Terrain decoding and sampling, hillshade and contour geometry, satellite selection and downloads, roads, and water.

## Contents

- `archive.rs`
- `header.rs`
- `mod.rs`
- `model.rs`
- `selection.rs`
- `t935_10`
- `validation.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
