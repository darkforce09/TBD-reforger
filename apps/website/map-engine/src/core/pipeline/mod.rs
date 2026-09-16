//! Role: Module boundary for core/pipeline.
//! Position: `core/pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Damage.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
pub use website_graphics_engine::frame::damage;

/// Frame packet pipeline / bind-group tables and the lane → binding policy.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bindings;

/// Draw order.
pub mod draw_order;

/// Stable vector and texture upload identifiers.
pub mod roles;
