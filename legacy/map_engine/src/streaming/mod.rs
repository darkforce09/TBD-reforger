//! Role: Module boundary for streaming.
//! Position: `streaming` in the map engine, behind the `streaming` feature.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bridge.
pub mod bridge;

/// Buffers.
pub mod buffers;

/// Loaders.
pub mod loaders;

/// Memory.
pub mod memory;

/// Scheduler.
pub mod scheduler;

/// Host.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod host;
