//! Role: encoder.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::core::pipeline::draw_order::lane_order;
use crate::renderers::batching::batch::Batch;
use crate::renderers::batching::batch::BatchPayload;
use crate::renderers::batching::batch::IndirectIcon;
use crate::renderers::batching::scene::ANCHOR;
use wasm_bindgen::prelude::*;

/// Draw batches.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_batches<'a>(
    batches: &'a [Batch],
    pass: &mut wgpu::RenderPass<'a>,
    bind_group: &'a wgpu::BindGroup,
    unit_quad_buf: &'a wgpu::Buffer,
    quad_pipeline: &'a wgpu::RenderPipeline,
    textured_pipeline: &'a wgpu::RenderPipeline,
    forest_density_pipeline: &'a wgpu::RenderPipeline,
    line_pipeline: &'a wgpu::RenderPipeline,
    building_pipeline: &'a wgpu::RenderPipeline,
    polygon_pipeline: &'a wgpu::RenderPipeline,
    icon_pipeline: &'a wgpu::RenderPipeline,
    text_pipeline: &'a wgpu::RenderPipeline,
    glyph_atlas_bind: Option<&'a wgpu::BindGroup>,
    text_atlas_bind: Option<&'a wgpu::BindGroup>,
    slot_base_bind: Option<&'a wgpu::BindGroup>,
    slot_drag_bind: Option<&'a wgpu::BindGroup>,
    indirect_icons: &[IndirectIcon<'a>],
) {
    let mut icons_emitted = vec![false; indirect_icons.len()];
    let emit_due =
        |pass: &mut wgpu::RenderPass<'a>, emitted: &mut [bool], before: Option<LaneRole>| {
            for (i, d) in indirect_icons.iter().enumerate() {
                if emitted[i] {
                    continue;
                }
                if let Some(role) = before
                    && lane_order(role) <= lane_order(d.role)
                {
                    continue;
                }
                emitted[i] = true;
                pass.set_pipeline(d.pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_bind_group(2, d.atlas_bind, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, d.instances.slice(..));
                pass.draw_indirect(d.indirect, 0);
            }
        };

    for batch in batches {
        emit_due(pass, &mut icons_emitted, Some(batch.role));
        if !batch.visible {
            continue;
        }
        match &batch.payload {
            BatchPayload::Instanced { instances, count } => {
                pass.set_pipeline(quad_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.draw(0..4, 0..*count);
            }
            BatchPayload::Textured(l) => {
                let pipe = if batch.role == LaneRole::ForestFill {
                    forest_density_pipeline
                } else {
                    textured_pipeline
                };
                pass.set_pipeline(pipe);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_bind_group(1, &l.bind_group, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, l.instances.slice(..));
                pass.draw(0..4, 0..1);
            }
            BatchPayload::Lines(l) => {
                pass.set_pipeline(line_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, l.verts.slice(..));
                pass.draw(0..l.count, 0..1);
            }
            BatchPayload::BuildingInstanced { instances, count } => {
                pass.set_pipeline(building_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.draw(0..4, 0..*count);
            }
            BatchPayload::Polygon(l) => {
                pass.set_pipeline(polygon_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, l.verts.slice(..));
                pass.set_index_buffer(l.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..l.index_count, 0, 0..1);
            }
            BatchPayload::IconInstanced { instances, count } => {
                if batch.role == LaneRole::WorldLabels
                    || batch.role == LaneRole::WorldRoadLabels
                    || batch.role == LaneRole::WorldTownLabels
                {
                    let Some(text_bg) = text_atlas_bind else {
                        continue;
                    };
                    pass.set_pipeline(text_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_bind_group(2, text_bg, &[]);
                    pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                    pass.set_vertex_buffer(1, instances.slice(..));
                    pass.draw(0..4, 0..*count);
                    continue;
                }
                let atlas_bg = match batch.role {
                    LaneRole::SlotDrag => slot_drag_bind,

                    LaneRole::Slots
                    | LaneRole::Clusters
                    | LaneRole::SlotPlacePreview
                    | LaneRole::MissionVehicles
                    | LaneRole::MissionMarkers
                    | LaneRole::MissionComments => slot_base_bind,
                    _ => glyph_atlas_bind,
                };
                let Some(atlas_bg) = atlas_bg else {
                    continue;
                };
                pass.set_pipeline(icon_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_bind_group(2, atlas_bg, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.draw(0..4, 0..*count);
            }
            BatchPayload::MarkerComposite {
                icons,
                icon_count,
                captions,
            } => {
                if let Some(slot_bg) = slot_base_bind {
                    pass.set_pipeline(icon_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_bind_group(2, slot_bg, &[]);
                    pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                    pass.set_vertex_buffer(1, icons.slice(..));
                    pass.draw(0..4, 0..*icon_count);
                }

                if let (Some((cap_buf, cap_count)), Some(text_bg)) = (captions, text_atlas_bind) {
                    pass.set_pipeline(text_pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    pass.set_bind_group(2, text_bg, &[]);
                    pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                    pass.set_vertex_buffer(1, cap_buf.slice(..));
                    pass.draw(0..4, 0..*cap_count);
                }
            }
        }
    }

    emit_due(pass, &mut icons_emitted, None);
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

            let glyph_bg = self.glyph_atlas.as_ref().map(|a| &a.bind_group);
            let text_bg = self.text_atlas.as_ref().map(|a| &a.bind_group);
            let slot_base = self.slot_atlas.as_ref().map(|a| &a.base_bind_group);
            let slot_drag = self.slot_atlas.as_ref().map(|a| &a.drag_bind_group);

            let indirect_icons = if do_compute {
                self.collect_indirect_icons()
            } else {
                Vec::new()
            };
            draw_batches(
                &self.batches,
                &mut pass,
                &self.bind_group,
                &self.unit_quad_buf,
                &self.surface_pipeline,
                &self.textured_pipeline,
                &self.forest_density_pipeline,
                &self.line_pipeline,
                &self.building_pipeline,
                &self.polygon_pipeline,
                &self.icon_pipeline,
                &self.text_pipeline,
                glyph_bg,
                text_bg,
                slot_base,
                slot_drag,
                &indirect_icons,
            );
        }
        do_compute
    }
}
