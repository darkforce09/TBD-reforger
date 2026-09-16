# renderers/batching

Draw pipelines, batching, polygon and line composition, glyph atlas layout, and text submission.

## Contents

- `batch.rs`
- `encoder.rs`
- `lanes.rs`
- `mod.rs`
- `scene.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
