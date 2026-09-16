# core

WebGPU context ownership, persistent GPU buffers, visibility culling, damage tracking, and the ordered draw-lane contract.

## Contents

- `buffers`
- `context`
- `culling`
- `mod.rs`
- `pipeline`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
