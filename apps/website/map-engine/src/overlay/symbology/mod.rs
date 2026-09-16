//! Role: Module boundary for symbology.
//! Position: `overlay/symbology` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Atlas.
pub mod atlas;

/// Instances.
// T-0xx Phase 2A: `website-graphics-engine` is optional from `streaming` up, so the belts that
// name a graphics layout type are gated with it.
#[cfg(feature = "streaming")]
pub mod instances;

/// Labels.
pub mod labels;

/// Links.
pub mod links;

/// Markers.
#[cfg(feature = "streaming")]
pub mod markers;

/// Roles.
pub mod roles;
