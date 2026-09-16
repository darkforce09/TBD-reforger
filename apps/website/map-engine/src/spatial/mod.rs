//! Role: Module boundary for spatial.
//! Position: `spatial` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bvh.
#[cfg(feature = "bvh")]
pub mod bvh;

/// Indexing.
pub mod indexing;

/// Terrain los.
pub mod terrain_los;

/// World los.
#[cfg(feature = "streaming")]
pub mod world_los;
