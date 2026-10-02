//! Role: Module boundary for architecture/section.
//! Position: `world/architecture/section` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Cutter.
#[cfg(feature = "io")]
pub mod cutter;

/// Index.
#[cfg(feature = "io")]
pub mod index;
