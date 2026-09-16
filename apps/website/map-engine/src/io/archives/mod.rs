//! Role: Module boundary for formats/archives.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Codec.
pub mod codec;

/// Models.
pub mod models;

/// Version.
#[cfg(feature = "io")]
pub mod version;

/// Roads.
#[cfg(feature = "io")]
pub mod roads;

/// Labels.
#[cfg(feature = "io")]
pub mod labels;

/// Water.
#[cfg(feature = "io")]
pub mod water;

/// Prefabs.
#[cfg(feature = "io")]
pub mod prefabs;

/// Forest.
#[cfg(feature = "io")]
pub mod forest;

/// Blueprints.
#[cfg(feature = "io")]
pub mod blueprints;

/// Satellite.
#[cfg(feature = "io")]
pub mod satellite;
