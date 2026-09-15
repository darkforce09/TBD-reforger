//! Role: Module boundary for core/buffers.
//! Position: `core/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Pool.
pub mod pool;

/// Readback.
pub mod readback;
