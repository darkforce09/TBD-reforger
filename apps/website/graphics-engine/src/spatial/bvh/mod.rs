//! Role: Module boundary for spatial/bvh.
//! Position: `spatial/bvh` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Sidecar.
pub mod sidecar;

/// Tree.
pub mod tree;

/// Node.
#[cfg(feature = "bvh")]
pub mod node;

/// Surface.
#[cfg(feature = "bvh")]
pub mod surface;

/// Traversal.
#[cfg(feature = "bvh")]
pub mod traversal;
