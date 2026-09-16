//! Role: Module boundary for terrain/roads.
//! Position: `world/terrain/roads` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Airfield.
#[cfg(feature = "streaming")]
pub mod airfield;

/// Cartographic strip.
#[cfg(feature = "streaming")]
pub mod cartographic_strip;

/// Network.
#[cfg(feature = "streaming")]
pub mod network;

/// Styling.
pub mod styling;

/// Casing and centerline meshes composed from road segments.
pub mod mesh;
