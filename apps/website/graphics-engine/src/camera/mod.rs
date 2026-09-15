//! Role: Module boundary for camera.
//! Position: `camera` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Math.
pub mod math;

/// Orbit.
pub mod orbit;

/// Ortho.
pub mod ortho;
