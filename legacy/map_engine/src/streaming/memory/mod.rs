//! Role: Module boundary for streaming/memory.
//! Position: `streaming/memory` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Budget.
#[cfg(feature = "streaming")]
pub mod budget;
