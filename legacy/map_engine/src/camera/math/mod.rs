//! Role: Module boundary for camera/math.
//! Position: `camera/math` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Dimensions.
pub mod dimensions;

/// Glmat4.
pub mod glmat4;

/// Shaping.
pub mod shaping;
