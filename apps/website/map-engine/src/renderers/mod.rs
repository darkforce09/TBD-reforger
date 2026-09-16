//! Role: Module boundary for renderers.
//! Position: `renderers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Batching.
#[cfg(feature = "streaming")]
pub mod batching;

/// Engine.
pub mod engine;

/// Pipelines.
pub mod pipelines;

/// Primitives.
pub mod primitives;

/// Text.
#[cfg(feature = "streaming")]
pub mod text;
