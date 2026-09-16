//! Role: Module boundary for terrain/water.
//! Position: `terrain/water` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Vectors.
#[cfg(feature = "streaming")]
pub mod vectors;

/// Triangulated sea-band fills.
// T-0xx Phase 2A: `website-graphics-engine` is optional from `streaming` up, so the belts that
// name a graphics layout type are gated with it.
#[cfg(feature = "streaming")]
pub mod mesh;

/// Loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
