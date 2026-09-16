//! Role: Module boundary for architecture/compound.
//! Position: `architecture/compound` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Scene.
#[cfg(feature = "formats")]
pub mod scene;

/// Transform.
pub mod transform;

/// Assembly.
#[cfg(feature = "formats")]
pub mod assembly;

/// Instances.
#[cfg(feature = "formats")]
pub mod instances;

/// Doors.
#[cfg(feature = "formats")]
pub mod doors;
