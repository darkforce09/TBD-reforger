//! Role: lib.
//! Position: `apps/website/graphics-engine/src` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Architecture.
pub mod architecture;

/// Camera.
pub mod camera;

/// Core.
pub mod core;

/// Diagnostics.
pub mod diagnostics;

/// Doll.
pub mod doll;

/// Environment.
pub mod environment;

/// Formats.
pub mod formats;

/// Renderers.
pub mod renderers;

/// Spatial.
pub mod spatial;

/// Streaming.
pub mod streaming;

/// Symbology.
pub mod symbology;

/// Terrain.
pub mod terrain;

#[cfg(test)]
#[path = "tests/feature_gate_tripwire.rs"]
mod feature_gate_tripwire;
