//! Role: Module boundary for spatial.
//! Position: `spatial` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bvh.
#[cfg(feature = "bvh")]
pub mod bvh;

/// Indexing.
pub mod indexing;

/// Line of sight — three layers, adjacent and deliberately not merged. See `los/mod.rs`.
pub mod los;
