# renderers/text/layout

Draw pipelines, batching, polygon and line composition, glyph atlas layout, and text submission.

## Contents

- `mod.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
