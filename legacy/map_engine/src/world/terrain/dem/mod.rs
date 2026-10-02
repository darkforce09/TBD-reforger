//! Role: Module boundary for terrain/dem.
//! Position: `world/terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Full-resolution elevation raster: the native `u16` samples and their bilinear height lookup.
pub mod full_resolution;

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

/// Sampling.
pub mod sampling;

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
