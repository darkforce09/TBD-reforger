//! Role: Module boundary for diagnostics/probes.
//! Position: `diagnostics/probes` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Runner.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod runner;
