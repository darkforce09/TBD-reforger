//! Role: Module boundary for architecture/los.
//! Position: `architecture/los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Walker.
#[cfg(feature = "formats")]
pub mod walker;

/// Wash.
#[cfg(feature = "formats")]
pub mod wash;
