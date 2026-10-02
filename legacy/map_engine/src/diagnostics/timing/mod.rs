//! Role: Module boundary for diagnostics/timing.
//! Position: `diagnostics/timing` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Gpu.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod gpu;
