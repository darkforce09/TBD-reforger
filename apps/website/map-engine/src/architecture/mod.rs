//! Role: Module boundary for architecture.
//! Position: `architecture` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Blueprint.
pub mod blueprint;

/// Compound.
pub mod compound;

/// Los.
pub mod los;

/// Section.
pub mod section;
