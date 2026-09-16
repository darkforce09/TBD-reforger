//! Role: Module boundary for environment/vegetation.
//! Position: `environment/vegetation` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Canopy.
#[cfg(feature = "streaming")]
pub mod canopy;

/// Density.
pub mod density;

/// Mass.
pub mod mass;

/// Regions.
#[cfg(feature = "streaming")]
pub mod regions;

/// Buffers.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
// T-0xx Phase 2A: `website-graphics-engine` is optional from `streaming` up, so the belts that
// name a graphics layout type are gated with it.
#[cfg(feature = "streaming")]
pub mod buffers;

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
