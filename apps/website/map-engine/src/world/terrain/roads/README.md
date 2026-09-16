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

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
