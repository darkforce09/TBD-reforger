//! Role: Module boundary for terrain/relief.
//! Position: `terrain/relief` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Contours.
pub mod contours;

/// Hillshade.
pub mod hillshade;

/// Sea band.
pub mod sea_band;

/// Host.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod host;
