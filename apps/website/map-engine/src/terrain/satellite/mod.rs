//! Role: Module boundary for terrain/satellite.
//! Position: `terrain/satellite` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Textures.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod textures;

/// Quadtree.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod quadtree;

/// Streamer.
#[cfg(feature = "streaming")]
pub mod streamer;
