//! Role: Module boundary for environment/buildings.
//! Position: `environment/buildings` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Obb.
#[cfg(feature = "streaming")]
pub mod obb;

/// Prefab.
#[cfg(feature = "streaming")]
pub mod prefab;

/// Buffers.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod buffers;

/// Footprint.
#[cfg(feature = "streaming")]
pub mod footprint;
