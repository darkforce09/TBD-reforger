# core/pipeline

WebGPU context ownership, persistent GPU buffers, visibility culling, damage tracking, and the ordered draw-lane contract.

## Contents

- `damage.rs`
- `draw_order.rs`
- `mod.rs`
- `roles.rs`
- `tests`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)

There is no pipeline cache in the current implementation.
