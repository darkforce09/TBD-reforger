# doll

Character mannequin geometry, equipment-region appearance, camera projection, rendering, and picking.

## Contents

- `interaction`
- `mod.rs`
- `renderer`
- `scene`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
