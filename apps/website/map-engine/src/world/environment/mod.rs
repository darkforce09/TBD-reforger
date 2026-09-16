//! Role: Module boundary for environment.
//! Position: `world/environment` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Buildings.
pub mod buildings;

/// Classify.
#[cfg(feature = "streaming")]
pub mod classify;

/// Locations.
pub mod locations;

/// Vegetation.
pub mod vegetation;
