//! Role: Module boundary for core/culling.
//! Position: `core/culling` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Compute.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use website_graphics_engine::draw::cull::compute;

/// Lod.
#[cfg(feature = "streaming")]
pub mod lod;

/// Oracle.
pub use website_graphics_engine::draw::cull::oracle;

/// Engine.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod engine;
