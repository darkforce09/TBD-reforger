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

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)
