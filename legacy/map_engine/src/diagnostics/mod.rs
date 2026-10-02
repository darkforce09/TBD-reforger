//! Role: Module boundary for diagnostics.
//! Position: `diagnostics` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bench.
pub mod bench;

/// Platform.
pub mod platform;

/// Probes.
pub mod probes;

/// Readback.
pub mod readback;

/// Timing.
pub mod timing;
