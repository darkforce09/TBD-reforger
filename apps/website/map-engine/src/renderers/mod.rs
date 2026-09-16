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
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
pub use website_graphics_engine::pipeline as pipelines;

/// Primitives.
pub mod primitives;

/// Text.
#[cfg(feature = "streaming")]
pub mod text;
