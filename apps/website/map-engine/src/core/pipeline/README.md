# core/pipeline

WebGPU context ownership, persistent GPU buffers, visibility culling, damage tracking, and the ordered draw-lane contract.

## Contents

- `damage.rs`
- `draw_order.rs`
- `mod.rs`
- `roles.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

There is no pipeline cache in the current implementation.
