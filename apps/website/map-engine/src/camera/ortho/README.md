# camera/ortho

Orthographic map and orbit-camera coordinates, projection matrices, unprojection, and viewport controls.

## Contents

- `camera`
- `controllers.rs`
- `mod.rs`
- `projection.rs`
- `state.rs`
- `unproject.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.
