//! Role: Module boundary for diagnostics/bench.
//! Position: `diagnostics/bench` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Frame 1.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod frame_1;

/// Frame 2.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod frame_2;
