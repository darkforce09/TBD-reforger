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

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
