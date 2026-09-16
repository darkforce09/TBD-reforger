//! Role: Module boundary for architecture/blueprint.
//! Position: `world/architecture/blueprint` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Model.
#[cfg(feature = "io")]
pub mod model;

/// Structure.
#[cfg(feature = "io")]
pub mod structure;

/// Footprint.
#[cfg(feature = "io")]
pub mod footprint;

/// Archive.
#[cfg(feature = "io")]
pub mod archive;

/// Attribution 1.
#[cfg(feature = "io")]
pub mod attribution_1;

/// Attribution 2.
#[cfg(feature = "io")]
pub mod attribution_2;

/// Geometry.
#[cfg(feature = "io")]
pub mod geometry;
