//! Role: Module boundary for terrain.
//! Position: `terrain` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Dem.
pub mod dem;

/// Relief.
pub mod relief;

/// Roads.
pub mod roads;

/// Satellite.
pub mod satellite;

/// Water.
pub mod water;
