//! Role: Module boundary for streaming/scheduler.
//! Position: `streaming/scheduler` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Residency.
pub mod residency;

/// Viewport.
#[cfg(feature = "streaming")]
pub mod viewport;

/// State.
#[cfg(feature = "streaming")]
pub mod state;

/// Budget.
#[cfg(feature = "streaming")]
pub mod budget;

/// Queries.
#[cfg(feature = "streaming")]
pub mod queries;

/// The chunk-granular, class-filterable point index over the resident world objects.
#[cfg(feature = "streaming")]
pub mod world_object_index;

/// The residency's manifest, prefab and chunk-index loads and its chunk ingest.
#[cfg(feature = "streaming")]
pub mod chunk_ingest;
