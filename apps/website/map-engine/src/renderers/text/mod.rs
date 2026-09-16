//! Role: Module boundary for renderers/text.
//! Position: `renderers/text` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Font.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
pub use website_graphics_engine::text::font;

/// Layout.
pub mod layout;

/// Lanes.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lanes;

/// Atlas.
#[cfg(feature = "streaming")]
pub use website_graphics_engine::text::atlas;

/// Metrics.
#[cfg(feature = "streaming")]
pub mod metrics;

/// Packing.
#[cfg(feature = "streaming")]
pub mod packing;
