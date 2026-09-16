# camera/orbit

Orthographic map and orbit-camera coordinates, projection matrices, unprojection, and viewport controls.

## Contents

- `camera.rs`
- `mod.rs`
- `projection.rs`

## Boundaries

This module owns graphics data and computation. It does not depend on Leptos or on any editor application state; browser I/O is gated to WebAssembly. (It said "does not depend on mission-core" until T-0xx Phase 2A folded that crate in as `data/`; the sentence named a crate that no longer exists.)

Only the existing yaw control is supported; orbit pitch, dolly, and pan have no source implementation.
