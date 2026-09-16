//! Role: Module boundary for renderers/batching.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Lanes.
pub mod lanes;

/// Scene.
pub mod scene;

// T-0xx Phase 1D: `batch.rs` is GONE, not moved. `Batch`, `BatchPayload` and `IndirectIcon`
// were this crate's private draw vocabulary; they are now `website-graphics-engine`'s
// `frame::{DrawBatch, DrawPayload, IndirectDraw}`, which every lane upload builds directly.
// `BatchPayload::Textured(TexLane)` did not survive the crossing: a payload carries a
// `BindGroupId`, and the texture handle, basemap mode and tile count stay in
// `RenderEngine::tex_lanes`.

/// Encoder.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod encoder;
