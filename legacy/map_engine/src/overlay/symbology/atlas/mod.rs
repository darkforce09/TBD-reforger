//! Role: Module boundary for symbology/atlas.
//! Position: `overlay/symbology/atlas` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Gpu.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod gpu;

/// Raster.
pub mod raster;
