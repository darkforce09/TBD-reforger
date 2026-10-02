//! Role: Module boundary for camera/orbit.
//! Position: `camera/orbit` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Camera.
pub mod camera;

/// Projection.
pub mod projection;
