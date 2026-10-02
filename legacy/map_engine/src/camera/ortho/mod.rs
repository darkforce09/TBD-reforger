//! Role: Module boundary for camera/ortho.
//! Position: `camera/ortho` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Camera.
pub mod camera;

/// Controllers.
pub mod controllers;

/// Projection.
pub mod projection;

/// State.
pub mod state;

/// Unproject.
pub mod unproject;
