//! Role: Module boundary for architecture/los.
//! Position: `spatial/los/interior` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Walker.
#[cfg(feature = "io")]
pub mod walker;

/// Wash.
#[cfg(feature = "io")]
pub mod wash;
