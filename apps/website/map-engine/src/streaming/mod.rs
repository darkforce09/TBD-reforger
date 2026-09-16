//! Role: Module boundary for streaming.
//! Position: `streaming` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bridge.
pub mod bridge;

/// Buffers.
#[cfg(feature = "streaming")]
pub mod buffers;

/// Loaders.
#[cfg(feature = "streaming")]
pub mod loaders;

/// Memory.
#[cfg(feature = "streaming")]
pub mod memory;

/// Scheduler.
#[cfg(feature = "streaming")]
pub mod scheduler;

/// Host.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod host;
