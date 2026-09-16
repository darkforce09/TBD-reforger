//! Role: Module boundary for renderers/primitives.
//! Position: `renderers/primitives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Compose.
pub mod compose;

/// Triangulate.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
pub use website_graphics_engine::draw::triangulate;

/// Hairlines.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod hairlines;

/// Vector lines.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod vector_lines;

/// Selection.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod selection;
