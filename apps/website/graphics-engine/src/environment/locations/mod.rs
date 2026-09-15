//! Role: Module boundary for environment/locations.
//! Position: `environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Peaks.
pub mod peaks;

/// Routes.
#[cfg(feature = "streaming")]
pub mod routes;

/// Towns.
#[cfg(feature = "streaming")]
pub mod towns;

/// Route placement.
#[cfg(feature = "streaming")]
pub mod route_placement;

/// Route labels.
#[cfg(feature = "streaming")]
pub mod route_labels;

/// Route geometry.
#[cfg(feature = "streaming")]
pub mod route_geometry;

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
