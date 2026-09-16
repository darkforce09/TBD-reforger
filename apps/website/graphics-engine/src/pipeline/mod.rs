//! Role: Module boundary for renderers/pipelines.
//! Position: `pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Quad.
#[cfg(target_arch = "wasm32")]
pub mod quad;

/// Textured.
#[cfg(target_arch = "wasm32")]
pub mod textured;

/// Vector.
#[cfg(target_arch = "wasm32")]
pub mod vector;

/// Text.
#[cfg(target_arch = "wasm32")]
pub mod text;

/// Icon.
#[cfg(target_arch = "wasm32")]
pub mod icon;

/// Building.
#[cfg(target_arch = "wasm32")]
pub mod building;
