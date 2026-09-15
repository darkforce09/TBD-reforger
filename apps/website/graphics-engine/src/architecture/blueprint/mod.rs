//! Role: Module boundary for architecture/blueprint.
//! Position: `architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Model.
#[cfg(feature = "formats")]
pub mod model;

/// Structure.
#[cfg(feature = "formats")]
pub mod structure;

/// Footprint.
#[cfg(feature = "formats")]
pub mod footprint;

/// Archive.
#[cfg(feature = "formats")]
pub mod archive;

/// Attribution 1.
#[cfg(feature = "formats")]
pub mod attribution_1;

/// Attribution 2.
#[cfg(feature = "formats")]
pub mod attribution_2;

/// Geometry.
#[cfg(feature = "formats")]
pub mod geometry;
