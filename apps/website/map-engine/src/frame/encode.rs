//! Role: encoding the frame packet into a render pass.
//! Position: `frame` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::FramePacket;
use crate::frame::LaneId;
use crate::frame::bindings;
use crate::frame::engine::RenderEngine;
use crate::frame::upload::text::TextAtlasGpu;
use crate::overlay::symbology::atlas::gpu::GlyphAtlasGpu;
use crate::overlay::symbology::instances::bridge_1::SlotAtlasGpu;
use crate::world::scene::ANCHOR;
use crate::world::terrain::satellite::textures::TexLane;
use wasm_bindgen::prelude::*;

// ── PHASE 2C §R1 — BOTH TABLES REFILL, NEITHER REALLOCATES ───────────────────────────────────
//
// Rule 1 says: clear and refill, keeping capacity; never allocate a fresh vec per frame. The
// case that MATTERS is `RenderEngine::batches` — the one that scales with scene complexity —
// and that one was already right: `FramePacket` borrows (`batches: &'a [DrawBatch]`), the vec is
// an engine field, and `upsert_lane` / `remove_lane` are the only things that touch it.
//
// These two tables were the violation Phase 1 shipped, inside the module that defines the
// pattern for everything downstream. Both were rebuilt every frame by `encode_main_pass`:
//
//   pipeline_table()     Vec::with_capacity(9)      + 9  Arc clones
//   bind_group_table()   vec![None; BIND_SLOTS=100] + ~5 Arc clones and up
//
// Two heap allocations and roughly twenty atomic refcount operations, at 60 Hz. Introduced in
// 24e22b1b9 — "invert the lanes — the encoder stops asking what a lane is (1D, part 3)" — and
// the doc comment on `pipeline_table` said outright *"Cloning is an `Arc` bump per pipeline per
// frame"*. Seeing it and shipping it anyway is precisely how a rule stops being real.
//
// **This will not move the bench and is not expected to.** Two allocations against a 16.6 ms
// budget is under 0.01% of frame time; landing inside noise is the pass condition here, not
// evidence the hoist did nothing. The reason to do it is that `frame/encode.rs` is where every
// later packet-building belt will look to see what the house style is.
//
// SHAPE: free functions with an `&mut Vec` out-param, and the vecs live as `RenderEngine`
// fields. Not `RefCell` — the tables are plain data with a single writer, and a runtime borrow
// flag to launder a compile-time question is how a hot path acquires a panic. Not a `&mut self`
// method either: `bind_group_table` reads four other fields of `self` while writing one, which
// is E0502 as a method and perfectly ordinary as a free function taking the pieces it reads.
// The out-param is what lets the caller pass `&mut self.frame_bind_groups` alongside
// `self.glyph_atlas.as_ref()` — disjoint field borrows, which the borrow checker splits happily.

/// Refill the pipeline table a frame packet addresses by [`crate::frame::PipelineId`].
///
/// T-0xx Phase 1D: `draw_batches` used to take nine `&RenderPipeline` arguments and choose
/// between them by matching on the lane. The choice is now made where a lane means something
/// — `frame/bindings.rs` — and travels on the batch; this is only the lookup table those ids
/// index.
///
/// T-0xx Phase 2C §R1: `out` is the caller's, kept across frames. The nine `Arc` bumps stay —
/// they are what a `RenderPipeline` handle IS — but the allocation behind them does not repeat.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pipeline_table(
    quad: &wgpu::RenderPipeline,
    textured: &wgpu::RenderPipeline,
    density: &wgpu::RenderPipeline,
    line: &wgpu::RenderPipeline,
    building: &wgpu::RenderPipeline,
    polygon: &wgpu::RenderPipeline,
    icon: &wgpu::RenderPipeline,
    text: &wgpu::RenderPipeline,
    icon_storage32: Option<&wgpu::RenderPipeline>,
    out: &mut Vec<wgpu::RenderPipeline>,
) {
    out.clear();
    out.reserve(bindings::PIPELINE_SLOTS);
    out.push(quad.clone());
    out.push(textured.clone());
    out.push(density.clone());
    out.push(line.clone());
    out.push(building.clone());
    out.push(polygon.clone());
    out.push(icon.clone());
    out.push(text.clone());

    // Slot 8 is only ever named by an indirect draw, and `collect_indirect_icons` emits none
    // unless this pipeline exists — so the fallback is unreachable, not a silent substitute.
    out.push(icon_storage32.unwrap_or(icon).clone());
}

/// Refill the sparse bind-group table a frame packet indexes.
///
/// Five fixed slots for the camera and the three atlases, then one slot per lane id for a
/// textured lane's own texture. `None` is how "this atlas has not been uploaded yet" reaches
/// the renderer: it skips the batch, which is what the old encoder's
/// `continue`-on-missing-bind-group did.
///
/// T-0xx Phase 2C §R1: free rather than a method, and `out` is the caller's. `clear()` +
/// `resize(.., None)` writes the same `BIND_SLOTS` `None`s that `vec![None; BIND_SLOTS]` wrote,
/// over the allocation the last frame already paid for. It also drops the previous frame's
/// handles at exactly the point the old code dropped them — at the top of the next rebuild
/// rather than at the end of the last scope — so no GPU resource outlives what it did before.
pub(crate) fn bind_group_table(
    camera: &wgpu::BindGroup,
    glyph_atlas: Option<&GlyphAtlasGpu>,
    text_atlas: Option<&TextAtlasGpu>,
    slot_atlas: Option<&SlotAtlasGpu>,
    tex_lanes: &[(LaneId, TexLane)],
    out: &mut Vec<Option<wgpu::BindGroup>>,
) {
    out.clear();
    out.resize(bindings::BIND_SLOTS, None);
    out[bindings::BIND_CAMERA.0 as usize] = Some(camera.clone());
    out[bindings::BIND_GLYPH_ATLAS.0 as usize] = glyph_atlas.map(|a| a.bind_group.clone());
    out[bindings::BIND_TEXT_ATLAS.0 as usize] = text_atlas.map(|a| a.bind_group.clone());
    out[bindings::BIND_SLOT_BASE.0 as usize] = slot_atlas.map(|a| a.base_bind_group.clone());
    out[bindings::BIND_SLOT_DRAG.0 as usize] = slot_atlas.map(|a| a.drag_bind_group.clone());
    for (lane, tex) in tex_lanes {
        out[bindings::tex_bind_id(*lane).0 as usize] = Some(tex.bind_group.clone());
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Encode main pass.
    pub(crate) fn encode_main_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        take_timing: bool,
    ) -> bool {
        let do_compute =
            self.gpu_cull_enabled() && self.icon_cull.as_ref().is_some_and(|c| c.has_any_src());
        if do_compute {
            let world = self.camera.visible_world_rect();

            let frustum = [
                world[0] - ANCHOR[0],
                world[1] - ANCHOR[1],
                world[2] - ANCHOR[0],
                world[3] - ANCHOR[1],
            ];
            if let Some(cull) = &mut self.icon_cull {
                cull.encode_cull(encoder, &self.device, &self.queue, frustum);
            }
        }

        // Order is load-bearing, not cosmetic. The two refills take `&mut self.<field>` beside
        // `&self.<other field>`, which the borrow checker splits per field; `collect_indirect_icons`
        // takes `&self` whole, and `indirect` holds that borrow for the rest of the function. So
        // the field writes must finish first. Swapping these three statements is an E0502.
        pipeline_table(
            &self.surface_pipeline,
            &self.textured_pipeline,
            &self.forest_density_pipeline,
            &self.line_pipeline,
            &self.building_pipeline,
            &self.polygon_pipeline,
            &self.icon_pipeline,
            &self.text_pipeline,
            self.icon_pipeline_storage32.as_ref(),
            &mut self.frame_pipelines,
        );
        bind_group_table(
            &self.bind_group,
            self.glyph_atlas.as_ref(),
            self.text_atlas.as_ref(),
            self.slot_atlas.as_ref(),
            &self.tex_lanes,
            &mut self.frame_bind_groups,
        );

        // The third allocation, and the one that cannot become a field: `IndirectDraw<'a>`
        // holds `&'a wgpu::Buffer` pointers INTO `self.icon_cull`, so a `RenderEngine` field of
        // that type would make the struct self-referential — which safe Rust has no way to
        // express, and which no amount of `mem::take` gets around. It gets the out-param shape
        // anyway so the three statements read alike, and the single `reserve` inside collapses
        // what was up to three growth reallocations into at most one. It is also the only one
        // of the three that is conditional: no compute cull, no allocation at all.
        let mut indirect: Vec<crate::frame::IndirectDraw<'_>> = Vec::new();
        if do_compute {
            self.collect_indirect_icons(&mut indirect);
        }

        // The matrix is re-composed rather than threaded down from `render()`: the packet's
        // job is to state what the frame draws with, and `encode_main_pass` keeps the argument
        // list it had before the split. Composing a 4x4 ortho twice a frame is not measurable;
        // a packet carrying a default camera would be a lie.
        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        let packet = FramePacket {
            camera: crate::frame::CameraUniform::new(mvp),
            clear: self.clear_color,
            batches: &self.batches,
            text: &[],
            indirect: &indirect,
            pipelines: &self.frame_pipelines,
            bind_groups: &self.frame_bind_groups,
            camera_bind: bindings::BIND_CAMERA,
            unit_quad: &self.unit_quad_buf,
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: self.timer.as_ref().filter(|_| take_timing).map(|t| {
                    wgpu::RenderPassTimestampWrites {
                        query_set: &t.query_set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            website_graphics_engine::draw::encode::encode(&mut pass, &packet);
        }
        do_compute
    }
}
