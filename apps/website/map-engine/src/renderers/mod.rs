//! Role: Module boundary for renderers.
//! Position: `renderers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Batching.
#[cfg(feature = "streaming")]
pub mod batching;

/// Engine.
pub mod engine;

/// Pipelines.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
//
// T-0xx Phase 2B: the second of the two `device`/`pipeline` sites gate rule 3b still allows.
// Eighteen call sites — `core/context/device_2.rs`, twelve `diagnostics/readback/*` probes
// and `diagnostics/probes/runner.rs` — build render pipelines against `RenderEngine`'s own
// shader module and bind-group layouts. Deleting this alias would spell
// `website_graphics_engine::pipeline` eighteen times instead of once; the pipelines
// themselves are already built by graphics-engine code. What has not happened is
// `RenderEngine` crossing, which is what would let this line go. See
// `xtask/src/gate_engine_layers.rs` for the pinned list.
pub use website_graphics_engine::pipeline as pipelines;

/// Primitives.
pub mod primitives;

/// Text.
#[cfg(feature = "streaming")]
pub mod text;
