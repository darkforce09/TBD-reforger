//! **Role:** encoding the frame packet into the main render pass: the compute cull of the sprite
//! lanes, the refill of the packet's pipeline and bind-group tables, and the pass itself.
//! **Position:** the map renderer; `render` calls `encode_main_pass` between the acquire and the
//! submit, the render diagnostics redraw the scene offscreen through `pipeline_table` and the
//! scene view's bind-group table.
//! **Signals & state:** the engine's two persistent packet tables, which this module clears and
//! refills.
//! **Invariants:** neither table nor the batch list is rebuilt per frame: the packet borrows the
//! engine's persistent batch list and tables, both refills reuse the allocation the last frame
//! paid for, and only the indirect draw list (which borrows the compute cull's buffers, so it
//! cannot be an engine field) is a local, allocated only when the compute cull runs.
//!
//! The refills are free functions with an `&mut Vec` out-param rather than `&mut self` methods:
//! `bind_group_table` reads four engine fields while writing a fifth, which the borrow checker
//! splits per field for a free function and refuses (E0502) for a method; and the tables stay
//! plain data with one writer, never a `RefCell` whose runtime borrow flag could panic on the
//! hot path.

use crate::bindings;
use crate::engine::RenderEngine;
use gpu_frame::frame::{FramePacket, IndirectDraw, TextAtlasGpu};
use map_coordinates::terrain_frames::ANCHOR;
use render_primitives::frame::ids::LaneId;
use renderer_core::packet_bindings;
use symbology_layers_gpu::glyph_atlas_gpu::GlyphAtlasGpu;
use symbology_layers_gpu::slot_symbology::SlotSymbologyGpu;
use world_layers_gpu::textured_lane::TexLane;

/// Refill `out`, the pipeline table a frame packet addresses by
/// [`render_primitives::frame::ids::PipelineId`], in the slot order of
/// `renderer_core::packet_bindings`.
///
/// The choice of pipeline is made where a lane means something (the typed layers and the upload
/// belts) and travels on the batch as an id; this is only the lookup table those ids index.
/// `out` is the caller's, kept across frames: the nine `Arc` bumps stay, since they are what a
/// `RenderPipeline` handle is, but the allocation behind them does not repeat. The render
/// diagnostics fill their offscreen packet's table with it too.
#[allow(clippy::too_many_arguments)]
pub fn pipeline_table(
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
    out.reserve(packet_bindings::PIPELINE_SLOTS);
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
/// the renderer: it skips the batch.
///
/// `out` is the caller's: `clear()` + `resize(.., None)` writes `BIND_SLOTS` `None`s over the
/// allocation the last frame already paid for, and drops the previous frame's handles at the top
/// of the refill, so no GPU resource outlives the frame after the one that last bound it.
pub(crate) fn bind_group_table(
    camera: &wgpu::BindGroup,
    glyph_atlas: &GlyphAtlasGpu,
    text_atlas: Option<&TextAtlasGpu>,
    slot_symbology: &SlotSymbologyGpu,
    tex_lanes: &[(LaneId, TexLane)],
    out: &mut Vec<Option<wgpu::BindGroup>>,
) {
    out.clear();
    out.resize(bindings::BIND_SLOTS, None);
    out[packet_bindings::BIND_CAMERA.0 as usize] = Some(camera.clone());
    out[packet_bindings::BIND_GLYPH_ATLAS.0 as usize] = glyph_atlas.bind_group().cloned();
    out[packet_bindings::BIND_TEXT_ATLAS.0 as usize] = text_atlas.map(|a| a.bind_group.clone());
    out[packet_bindings::BIND_MOVABLE_SPRITE_ATLAS.0 as usize] =
        slot_symbology.atlas_bind_group().cloned();
    out[packet_bindings::BIND_MOVABLE_SPRITE_ATLAS_DRAGGED.0 as usize] =
        slot_symbology.dragged_atlas_bind_group().cloned();
    for (lane, tex) in tex_lanes {
        out[packet_bindings::tex_bind_id(*lane).0 as usize] = Some(tex.bind_group().clone());
    }
}

impl RenderEngine {
    /// Encode the main pass into `view`: the compute cull when it runs, the table refills, and
    /// the packet over the persistent batch list. Returns whether the pass ran the compute cull.
    pub(crate) fn encode_main_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        take_timing: bool,
    ) -> bool {
        let do_compute = self.icon_cull.enabled() && self.icon_cull.has_any_source();
        if do_compute {
            let world = self.camera.visible_world_rect();

            let frustum = [
                world[0] - ANCHOR[0],
                world[1] - ANCHOR[1],
                world[2] - ANCHOR[0],
                world[3] - ANCHOR[1],
            ];
            self.icon_cull
                .encode_cull(encoder, self.gpu.device(), self.gpu.queue(), frustum);
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
            &self.glyph_atlas,
            self.text_atlas.as_ref(),
            &self.slot_symbology,
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
        let mut indirect: Vec<IndirectDraw<'_>> = Vec::new();
        if do_compute {
            self.collect_indirect_icons(&mut indirect);
        }

        // The matrix is re-composed rather than threaded down from `render()`: the packet's
        // job is to state what the frame draws with, and `encode_main_pass` keeps the argument
        // list it had before the split. Composing a 4x4 ortho twice a frame is not measurable;
        // a packet carrying a default camera would be a lie.
        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        let packet = FramePacket {
            camera: render_primitives::frame::camera::CameraUniform::new(mvp),
            clear: self.clear_color,
            batches: &self.batches,
            text: &[],
            indirect: &indirect,
            pipelines: &self.frame_pipelines,
            bind_groups: &self.frame_bind_groups,
            camera_bind: packet_bindings::BIND_CAMERA,
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
                        query_set: t.query_set(),
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            gpu_frame::draw::encode::encode(&mut pass, &packet);
        }
        do_compute
    }
}
