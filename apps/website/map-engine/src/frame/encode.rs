//! Role: encoding the frame packet into a render pass.
//! Position: `frame` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::FramePacket;
use crate::frame::bindings;
use crate::frame::engine::RenderEngine;
use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;

/// The pipelines a frame packet addresses by [`crate::frame::PipelineId`].
///
/// T-0xx Phase 1D: `draw_batches` used to take nine `&RenderPipeline` arguments and choose
/// between them by matching on the lane. The choice is now made where a lane means something
/// — `core/pipeline/bindings.rs` — and travels on the batch; this is only the lookup table
/// those ids index. Cloning is an `Arc` bump per pipeline per frame.
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
) -> Vec<wgpu::RenderPipeline> {
    let mut out = Vec::with_capacity(bindings::PIPELINE_SLOTS);
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
    out
}

impl RenderEngine {
    /// The sparse bind-group table a frame packet indexes.
    ///
    /// Five fixed slots for the camera and the three atlases, then one slot per lane id for a
    /// textured lane's own texture. `None` is how "this atlas has not been uploaded yet"
    /// reaches the renderer: it skips the batch, which is what the old encoder's
    /// `continue`-on-missing-bind-group did.
    pub(crate) fn bind_group_table(
        &self,
        camera: &wgpu::BindGroup,
    ) -> Vec<Option<wgpu::BindGroup>> {
        let mut out = vec![None; bindings::BIND_SLOTS];
        out[bindings::BIND_CAMERA.0 as usize] = Some(camera.clone());
        out[bindings::BIND_GLYPH_ATLAS.0 as usize] =
            self.glyph_atlas.as_ref().map(|a| a.bind_group.clone());
        out[bindings::BIND_TEXT_ATLAS.0 as usize] =
            self.text_atlas.as_ref().map(|a| a.bind_group.clone());
        out[bindings::BIND_SLOT_BASE.0 as usize] =
            self.slot_atlas.as_ref().map(|a| a.base_bind_group.clone());
        out[bindings::BIND_SLOT_DRAG.0 as usize] =
            self.slot_atlas.as_ref().map(|a| a.drag_bind_group.clone());
        for (lane, tex) in &self.tex_lanes {
            out[bindings::tex_bind_id(*lane).0 as usize] = Some(tex.bind_group.clone());
        }
        out
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

        let pipelines = pipeline_table(
            &self.surface_pipeline,
            &self.textured_pipeline,
            &self.forest_density_pipeline,
            &self.line_pipeline,
            &self.building_pipeline,
            &self.polygon_pipeline,
            &self.icon_pipeline,
            &self.text_pipeline,
            self.icon_pipeline_storage32.as_ref(),
        );
        let bind_groups = self.bind_group_table(&self.bind_group);
        let indirect = if do_compute {
            self.collect_indirect_icons()
        } else {
            Vec::new()
        };

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
            pipelines: &pipelines,
            bind_groups: &bind_groups,
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
