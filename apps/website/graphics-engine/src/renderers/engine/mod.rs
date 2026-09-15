//! Role: Module boundary for renderers/engine.
//! Position: `renderers/engine` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::core::context::state::RenderEngine`.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use crate::core::context::state::RenderEngine;

/// Lifecycle.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lifecycle;
