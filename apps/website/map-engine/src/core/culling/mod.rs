//! Role: Module boundary for core/culling.
//! Position: `core/culling` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Compute.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod compute;

/// Lod.
#[cfg(feature = "streaming")]
pub mod lod;

/// Oracle.
pub mod oracle;

/// Engine.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod engine;
