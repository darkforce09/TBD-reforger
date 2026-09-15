# doll/renderer

Character mannequin geometry, equipment-region appearance, camera projection, rendering, and picking.

## Contents

- `lifecycle_1.rs`
- `lifecycle_2.rs`
- `mod.rs`
- `pack.rs`
- `pass.rs`
- `pipeline.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
