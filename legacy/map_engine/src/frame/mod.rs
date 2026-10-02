//! Role: Module boundary for frame.
//! Position: `frame` in the map engine.
//! Signals & state: the engine object itself — its GPU resources, its persistent batch list,
//! and the belts that refill them.
//! Invariants: this is the module that speaks `graphics_engine`'s frame vocabulary.
//! Gate rule 3a pins that naming to THIS FILE — not merely to this directory. Every other
//! module in the crate, `frame/`'s own submodules included, reaches the vocabulary through
//! `crate::frame::…`. The GPU-free half of the renderer's vocabulary — the POD instance layouts,
//! the bit-packing, the frame ids, the camera uniform and damage tracking — is
//! `render_primitives`, which belts import directly beside their data (§2C.1 Kind B).
//!
//! The engine's own GPU lifecycle lives in this folder: `engine.rs` holds the engine state,
//! `boot.rs` the adapter and device bring-up, `bindings.rs` the lane bind-group policy and
//! `cull.rs` the indirect-draw compaction. `cull.rs` is **not** a spatial query: it is GPU
//! indirect-draw bookkeeping over a fixed lane list, and the frustum arithmetic it uses lives in
//! the graphics engine.

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
// `LanePool` and `ReadbackLane` are GPU buffer bookkeeping that belongs to graphics-engine and
// lives there; this crate names them only because `RenderEngine`, which holds them, is defined
// here. This alias is one of the two `device`/`pipeline` seams engine-layer rule 3b allows, named
// once here rather than at its three call sites, which spell it `crate::frame::buffers`. The
// pinned list is `RULE3B_PIN` in
// `tools/verification_core/src/repository_laws/engine_layers/rules.rs`.
pub use graphics_engine::device::buffers;

/// Pipelines.
// The second seam rule 3b allows. Eighteen call sites — `frame/boot.rs`, twelve
// `diagnostics/readback/*` probes and `diagnostics/probes/runner.rs` — build render pipelines
// against `RenderEngine`'s own shader module and bind-group layouts. Without this alias they
// would spell graphics-engine's pipeline module eighteen times instead of once; the pipelines
// themselves are built by graphics-engine code. The alias closes only when `RenderEngine` is
// defined in graphics-engine.
pub use graphics_engine::pipeline as pipelines;

// ── THE PACKET VOCABULARY ────────────────────────────────────────────────────────────────────
//
// §2C.1 Kind C. Everything the renderer's `frame` module publishes that this crate consumes,
// enumerated. **The five `pub use` lines below are the only places in `map_engine`
// that spell the path they spell** — engine-layer rule 3a (`RULE3A_PIN` in
// `tools/verification_core/src/repository_laws/engine_layers/rules.rs`) pins that in both
// directions, at exactly five, so a sixth import is a diff to this list and a lost one is a
// stale pin. Every other module of the crate reads `use crate::frame::DrawBatch;`. (This prose
// never spells the path, so the pinned count is exactly the size of the re-export list and a
// comment edit cannot turn the gate red.)
//
// Enumerated and never a glob, because the value of the chokepoint is not the indirection —
// re-exports cost nothing at runtime and a glob would compile identically. The value is that
// the crate's entire graphics interface is a list you can read in one screen, and that widening
// it is a diff to this file rather than an import somewhere in `world/` nobody reviews.
//
// Every item listed names a `wgpu` type, so each is `cfg(target_arch = "wasm32")` in the graphics
// engine: `atlas`, `batch`, `buffers`, `packet`, `present` and `text`. The arithmetic and newtypes
// beside them — the camera uniform, damage tracking and the frame ids — are GPU-free and live in
// `render_primitives::frame`, which callers import directly. This mirror gates on the target and
// NOT on `render`: the condition is whether the item exists, and `render` is a question about
// whether this crate draws.
//
// `IndexedMesh` and `VertexStream` are in the list without a `use` site today on purpose. They
// are what `draw::polygons::upload_*` and `draw::lines::upload_*` hand back and what
// `DrawPayload::{Indexed, Lines}` carry — part of the vocabulary this crate speaks whether or
// not a belt happens to need the name spelled. Leaving them out would make the list a census of
// current imports rather than a statement of the interface.

/// Ordered-insert and removal over a persistent `Vec<DrawBatch>` (rule 1's real case).
#[cfg(target_arch = "wasm32")]
pub use graphics_engine::frame::packet;

/// Swapchain acquire, submit and present.
#[cfg(target_arch = "wasm32")]
pub use graphics_engine::frame::present;

/// One frame's complete draw list, and the draws in it.
#[cfg(target_arch = "wasm32")]
pub use graphics_engine::frame::{DrawBatch, DrawPayload, FramePacket, IndirectDraw};

/// The buffer handles and ranges a payload carries. Rule 2: handles and ranges, never geometry.
#[cfg(target_arch = "wasm32")]
pub use graphics_engine::frame::{IndexedMesh, InstanceBuffer, VertexStream};

/// A run of packed glyph instances, and the two cell atlases a run can index.
#[cfg(target_arch = "wasm32")]
pub use graphics_engine::frame::{
    GlyphAtlasGpu, TextAtlasGpu, TextRun, create_glyph_atlas, create_text_atlas,
};

/// The GPU frustum compaction of packed sprite instances — wasm only, because it needs a device;
/// its CPU oracle is `render_primitives::draw::cull::oracle`.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use graphics_engine::draw::cull::compute;

/// Re-export `crate::frame::engine::RenderEngine`.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use engine::RenderEngine;

/// Re-export `graphics_engine::r#loop::{FrameTarget, RafPump}`.
// The frontend must not depend on `graphics_engine` (one arrow, not two): it reaches
// the renderer through this crate, and this is the door. Spelled beside `RenderEngine` above
// because a caller that has one always wants the other. `pump.rs` explains why the
// `FrameTarget` impl cannot live on the graphics side.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub use pump::{FrameTarget, RafPump};

/// A mounted engine, or an empty slot during initialization and teardown.
// The one spelling of the engine slot that every holder, the frontend included, shares.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub type EngineHandle = std::rc::Rc<std::cell::RefCell<Option<RenderEngine>>>;

// Damage discipline. `RenderDamage`'s own state machine is tested in
// `render_primitives`; this asserts that this crate still consults it — that `render()`
// refuses an undamaged frame, that every lane mutation marks damage, and that the packet
// borrows the persistent batch list instead of rebuilding one per frame. Not gated on
// `render`: it reads source text, never a GPU.
#[cfg(test)]
#[path = "tests/damage_discipline.rs"]
mod damage_discipline;
