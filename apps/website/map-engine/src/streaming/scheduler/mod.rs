//! Role: Module boundary for streaming/scheduler.
//! Position: `streaming/scheduler` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Chunk math.
pub mod chunk_math;

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
