# renderers/engine

Draw pipelines, batching, polygon and line composition, glyph atlas layout, and text submission.

## Contents

- `lifecycle.rs`
- `mod.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

There is no composite MapScene abstraction; the existing RenderEngine remains the owner.
