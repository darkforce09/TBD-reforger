//! Role: Module boundary for frame.
//! Position: `frame` in the map engine.
//! Signals & state: the engine object itself — its GPU resources, its persistent batch list,
//! and the belts that refill them.
//! Invariants: this is the module that speaks `website_graphics_engine`'s frame vocabulary.
//! Gate rule 3a pins that naming to THIS FILE — not merely to this directory. Every other
//! module in the crate, `frame/`'s own submodules included, reaches the vocabulary through
//! `crate::frame::…`; the one other legitimate door to the renderer is `graphics_engine::layout`,
//! the POD/bit-packing ABI, which belts import directly beside their data (§2C.1 Kind B).
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
// did not cross in Phase 1. See `tools_v2/xtask/src/verifications/architecture/engine_layer_boundaries.rs` for the pinned list.
pub use website_graphics_engine::device::buffers;

/// Pipelines.
// T-0xx Phase 2B: the second allowed site. Eighteen call sites — `frame/boot.rs`, twelve
// `diagnostics/readback/*` probes and `diagnostics/probes/runner.rs` — build render pipelines
// against `RenderEngine`'s own shader module and bind-group layouts. Deleting this alias would
// spell graphics-engine's pipeline module eighteen times instead of once; the pipelines
// themselves are already built by graphics-engine code. What has not happened is
// `RenderEngine` crossing, which is what would let this line go.
pub use website_graphics_engine::pipeline as pipelines;

// ── THE PACKET VOCABULARY ────────────────────────────────────────────────────────────────────
//
// T-0xx Phase 2C, §2C.1 Kind C. Everything the renderer's `frame` module publishes that this
// crate consumes, enumerated. **The eight `pub use` lines below are the only places in
// `website-map-engine` that spell the path they spell** — gate rule 3a in
// `tools_v2/xtask/src/verifications/architecture/engine_layer_boundaries.rs` pins that in both directions, at exactly eight, so a ninth
// import is a diff to this list and a lost one is a stale pin. The 38 call sites that used to
// spell it for themselves now read `use crate::frame::DrawBatch;`. (The pin counts the
// re-exports and not this prose on purpose: a gate that fails on a typo fix in a comment is a
// gate that gets suppressed.)
//
// Enumerated and never a glob, because the value of the chokepoint is not the indirection —
// re-exports cost nothing at runtime and a glob would compile identically. The value is that
// the crate's entire graphics interface is a list you can read in one screen, and that widening
// it is a diff to this file rather than an import somewhere in `world/` nobody reviews.
//
// The split into two groups is graphics-engine's own: `camera.rs`, `damage.rs` and `ids.rs` are
// ungated there because they are arithmetic and newtypes, while `atlas`, `batch`, `buffers`,
// `packet`, `present` and `text` name `wgpu` types and are `cfg(target_arch = "wasm32")`. This
// mirror gates on the target for the same reason and NOT on `render`: the condition is whether
// the item exists, and `render` is a question about whether this crate draws.
//
// `IndexedMesh` and `VertexStream` are in the list without a `use` site today on purpose. They
// are what `draw::polygons::upload_*` and `draw::lines::upload_*` hand back and what
// `DrawPayload::{Indexed, Lines}` carry — part of the vocabulary this crate speaks whether or
// not a belt happens to need the name spelled. Leaving them out would make the list a census of
// current imports rather than a statement of the interface.

/// Damage tracking — which frames need submitting at all.
pub use website_graphics_engine::frame::damage;

/// Ordered-insert and removal over a persistent `Vec<DrawBatch>` (rule 1's real case).
#[cfg(target_arch = "wasm32")]
pub use website_graphics_engine::frame::packet;

/// Swapchain acquire, submit and present.
#[cfg(target_arch = "wasm32")]
pub use website_graphics_engine::frame::present;

/// The group-0 camera block every draw binds.
pub use website_graphics_engine::frame::CameraUniform;

/// The three opaque ids a batch travels with — lane, pipeline, bind group.
pub use website_graphics_engine::frame::{BindGroupId, LaneId, PipelineId};

/// One frame's complete draw list, and the draws in it.
#[cfg(target_arch = "wasm32")]
pub use website_graphics_engine::frame::{DrawBatch, DrawPayload, FramePacket, IndirectDraw};

/// The buffer handles and ranges a payload carries. Rule 2: handles and ranges, never geometry.
#[cfg(target_arch = "wasm32")]
pub use website_graphics_engine::frame::{IndexedMesh, InstanceBuffer, VertexStream};

/// A run of packed glyph instances, and the two cell atlases a run can index.
#[cfg(target_arch = "wasm32")]
pub use website_graphics_engine::frame::{
    GlyphAtlasGpu, TextAtlasGpu, TextRun, create_glyph_atlas, create_text_atlas,
};

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
