//! Role: Module boundary for doll/renderer.
//! Position: `doll/renderer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Engine.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod engine;

/// Pack.
pub mod pack;

/// Lifecycle 1.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lifecycle_1;

/// Lifecycle 2.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lifecycle_2;

/// Pipeline.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod pipeline;

/// Pass.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod pass;
