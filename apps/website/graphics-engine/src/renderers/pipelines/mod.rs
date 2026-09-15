//! Role: Module boundary for renderers/pipelines.
//! Position: `renderers/pipelines` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Quad.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod quad;

/// Textured.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod textured;

/// Vector.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod vector;

/// Text.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod text;

/// Icon.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod icon;

/// Building.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod building;
