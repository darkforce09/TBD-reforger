//! Role: Module boundary for renderers/batching.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Lanes.
pub mod lanes;

/// Scene.
pub mod scene;

/// Batch.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod batch;

/// Encoder.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod encoder;
