# camera/orbit

Orthographic map and orbit-camera coordinates, projection matrices, unprojection, and viewport controls.

## Contents

- `camera.rs`
- `mod.rs`
- `projection.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on mission-core or Leptos; browser I/O is gated to WebAssembly.

Only the existing yaw control is supported; orbit pitch, dolly, and pan have no source implementation.
