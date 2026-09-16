//! Role: Module boundary for renderers/primitives.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Compose.
pub mod compose;

/// Triangulate.
pub mod triangulate;

/// Hairlines.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod hairlines;

/// Vector lines.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod vector_lines;

/// Selection.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod selection;
