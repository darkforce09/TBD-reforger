//! Role: Module boundary for core.
//! Position: `core` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Buffers.
pub mod buffers;

/// Context.
pub mod context;

/// Culling.
pub mod culling;

/// Pipeline.
pub mod pipeline;
