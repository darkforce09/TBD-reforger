//! Role: Module boundary for spatial/terrain_los.
//! Position: `spatial/los/terrain` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// March.
pub mod march;

/// Overlay.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod overlay;

/// Sampler.
pub mod sampler;

/// Scheduler.
pub mod scheduler;

/// Viewshed.
pub mod viewshed;
