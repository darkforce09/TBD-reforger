# renderers/text

Draw pipelines, batching, polygon and line composition, glyph atlas layout, and text submission.

## Contents

- `atlas.rs`
- `font.rs`
- `lanes.rs`
- `layout`
- `metrics.rs`
- `mod.rs`
- `packing.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)

The atlas uses the existing bitmap font; there is no MSDF text implementation.
