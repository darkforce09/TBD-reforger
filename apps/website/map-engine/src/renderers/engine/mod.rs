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

/// Pump — the engine's `FrameTarget` impl, and the app's route to the shared rAF loop.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod pump;

/// Re-export `website_graphics_engine::r#loop::{FrameTarget, RafPump}`.
// The frontend must not depend on `website-graphics-engine` (one arrow, not two): it reaches
// the renderer through this crate, and this is the door. Spelled beside `RenderEngine` above
// because a caller that has one always wants the other.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use pump::{FrameTarget, RafPump};
