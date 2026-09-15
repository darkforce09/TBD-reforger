//! Role: Module boundary for symbology.
//! Position: `symbology` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Atlas.
pub mod atlas;

/// Instances.
pub mod instances;

/// Labels.
pub mod labels;

/// Links.
pub mod links;

/// Roles.
pub mod roles;
