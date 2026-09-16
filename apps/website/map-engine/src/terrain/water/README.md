# terrain/water

Terrain decoding and sampling, hillshade and contour geometry, satellite selection and downloads, roads, and water.

## Contents

- `loader.rs`
- `mesh.rs`
- `mod.rs`
- `tests`
- `vectors.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

Water uses imported geometry and sea-band fills; procedural water is absent.
