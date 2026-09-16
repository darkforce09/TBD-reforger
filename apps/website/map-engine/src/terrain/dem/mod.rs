//! Role: Module boundary for terrain/dem.
//! Position: `terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Grid.
pub mod grid;

/// Manifest.
pub mod manifest;

/// Png.
#[cfg(feature = "world")]
pub mod png;

/// Raw.
#[cfg(feature = "io")]
pub mod raw;

/// Sample.
pub mod sample;

/// Sampling.
pub mod sampling;

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
