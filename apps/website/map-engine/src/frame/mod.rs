//! Role: Module boundary for frame.
//! Position: `frame` in the map engine.
//! Signals & state: the engine object itself — its GPU resources, its persistent batch list,
//! and the belts that refill them.
//! Invariants: this is the module that speaks `website_graphics_engine`'s frame vocabulary.
//! Gate rule 3a (Phase 2C) will pin that naming here; nothing outside `frame/` should reach
//! a graphics type except through `crate::layout`'s POD contract.
//!
//! T-0xx Phase 2B.1: `core/` and `renderers/` are gone and this is where most of them landed.
//! `core/context/state.rs` is `engine.rs`, `device_{1,2}.rs` are `boot.rs`,
//! `core/pipeline/bindings.rs` is `bindings.rs`, `core/culling/engine.rs` is `cull.rs` — that
//! last one is **not** a spatial query, it is GPU indirect-draw bookkeeping over a fixed lane
//! list, and the frustum arithmetic it uses crossed to graphics-engine in Phase 1.

/// Lane → pipeline and bind-group policy, travelling on the batch.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bindings;

/// Adapter, device, surface and every lifetime-scoped GPU resource.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod boot;

/// Indirect-draw compaction across the fixed lane list.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod cull;

/// Encoding the packet into a render pass.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod encode;

/// `RenderEngine` — the GPU resources, the camera, and the persistent batch list.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod engine;

/// Resize, render, stats, disposal.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lifecycle;

/// The engine's `FrameTarget` impl, and the app's route to the shared rAF loop.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod pump;

/// The belts that fill a lane's buffers and upsert its batch.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod upload;

/// Buffers.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
//
// T-0xx Phase 2B: one of the two `device`/`pipeline` sites gate rule 3b still allows, and it
// is allowed at one named seam rather than at its three call sites. `LanePool` and
// `ReadbackLane` are GPU buffer bookkeeping that belongs to graphics-engine and lives there;
// this crate has to *name* them only because `RenderEngine` holds them, and `RenderEngine`
// did not cross in Phase 1. See `xtask/src/gate_engine_layers.rs` for the pinned list.
pub use website_graphics_engine::device::buffers;

/// Pipelines.
// T-0xx Phase 2B: the second allowed site. Eighteen call sites — `frame/boot.rs`, twelve
// `diagnostics/readback/*` probes and `diagnostics/probes/runner.rs` — build render pipelines
// against `RenderEngine`'s own shader module and bind-group layouts. Deleting this alias would
// spell graphics-engine's pipeline module eighteen times instead of once; the pipelines
// themselves are already built by graphics-engine code. What has not happened is
// `RenderEngine` crossing, which is what would let this line go.
pub use website_graphics_engine::pipeline as pipelines;

/// Damage tracking — which frames need submitting at all.
pub use website_graphics_engine::frame::damage;

/// The CPU frustum oracle for packed sprite instances.
pub use website_graphics_engine::draw::cull::oracle;

/// Its GPU compute twin — wasm only, because it needs a device.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use website_graphics_engine::draw::cull::compute;

/// Re-export `crate::frame::engine::RenderEngine`.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use engine::RenderEngine;

/// Re-export `website_graphics_engine::r#loop::{FrameTarget, RafPump}`.
// The frontend must not depend on `website-graphics-engine` (one arrow, not two): it reaches
// the renderer through this crate, and this is the door. Spelled beside `RenderEngine` above
// because a caller that has one always wants the other. `pump.rs` explains why the
// `FrameTarget` impl cannot live on the graphics side.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use pump::{FrameTarget, RafPump};

/// A mounted engine, or an empty slot during initialization and teardown.
// T-0xx Phase 2B.1: from `core/context/handles.rs`, a ten-line file whose eleven importers —
// five of them in the frontend — all wanted exactly this alias.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub type EngineHandle = std::rc::Rc<std::cell::RefCell<Option<RenderEngine>>>;

// T-0xx Phase 2C: the rule-3 pin. `RenderDamage`'s own state machine is tested in
// `website-graphics-engine`; this asserts that this crate still consults it — that `render()`
// refuses an undamaged frame, that every lane mutation marks damage, and that the packet
// borrows the persistent batch list instead of rebuilding one per frame. Not gated on
// `render`: it reads source text, never a GPU.
#[cfg(test)]
#[path = "tests/damage_discipline.rs"]
mod damage_discipline;
