//! Role: Module boundary for architecture/compound.
//! Position: `world/architecture/compound` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Scene.
#[cfg(feature = "io")]
pub mod scene;

/// Transform.
pub mod transform;

/// Assembly.
#[cfg(feature = "io")]
pub mod assembly;

/// Instances.
#[cfg(feature = "io")]
pub mod instances;

/// Doors.
#[cfg(feature = "io")]
pub mod doors;
