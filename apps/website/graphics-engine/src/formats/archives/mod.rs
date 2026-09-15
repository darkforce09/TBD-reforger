//! Role: Module boundary for formats/archives.
//! Position: `formats/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Codec.
pub mod codec;

/// Models.
pub mod models;

/// Version.
#[cfg(feature = "formats")]
pub mod version;

/// Roads.
#[cfg(feature = "formats")]
pub mod roads;

/// Labels.
#[cfg(feature = "formats")]
pub mod labels;

/// Water.
#[cfg(feature = "formats")]
pub mod water;

/// Prefabs.
#[cfg(feature = "formats")]
pub mod prefabs;

/// Forest.
#[cfg(feature = "formats")]
pub mod forest;

/// Blueprints.
#[cfg(feature = "formats")]
pub mod blueprints;

/// Satellite.
#[cfg(feature = "formats")]
pub mod satellite;
