//! Role: Module boundary for terrain/water.
//! Position: `terrain/water` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Vectors.
#[cfg(feature = "streaming")]
pub mod vectors;

/// Triangulated sea-band fills.
pub mod mesh;

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
