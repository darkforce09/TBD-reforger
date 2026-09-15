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

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

The atlas uses the existing bitmap font; there is no MSDF text implementation.
