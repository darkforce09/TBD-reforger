# renderers/pipelines

Draw pipelines, batching, polygon and line composition, glyph atlas layout, and text submission.

## Contents

- `building.rs`
- `icon.rs`
- `mod.rs`
- `quad.rs`
- `text.rs`
- `textured.rs`
- `vector.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
