//! Role: Module boundary for spatial/indexing.
//! Position: `spatial/indexing` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Cluster.
pub mod cluster;

/// Picking.
pub mod picking;

/// Point index.
pub mod point_index;

/// World.
#[cfg(feature = "streaming")]
pub mod world;
