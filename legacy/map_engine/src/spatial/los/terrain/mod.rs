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

#[cfg(test)]
#[path = "tests/viewshed_fixtures.rs"]
mod viewshed_fixtures;

#[cfg(test)]
#[path = "tests/sampler_tests.rs"]
mod sampler_tests;

#[cfg(test)]
#[path = "tests/viewshed_tests.rs"]
mod viewshed_tests;

#[cfg(test)]
#[path = "tests/scheduler_tests.rs"]
mod scheduler_tests;
