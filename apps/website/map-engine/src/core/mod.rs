//! Role: Module boundary for core.
//! Position: `core` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Buffers.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
pub use website_graphics_engine::device::buffers;

/// Context.
pub mod context;

/// Culling.
pub mod culling;

/// Pipeline.
pub mod pipeline;
