//! Role: world building.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::ortho::state::OrthoCamera;
use crate::diagnostics::readback::scene::map_read_4;
use crate::diagnostics::readback::scene::padded_bytes_per_row;
use crate::frame::engine::CLEAR_COLOR;
use crate::frame::engine::RenderEngine;

use crate::frame::pipelines::building::create_building_pipeline;
use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// World building self check.
    pub fn world_building_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;

            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bld-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bld-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            let rad = (37.0_f64 * std::f64::consts::PI) / 180.0;
            let inst = website_graphics_engine::layout::BuildingInstance {
                center: [0.0, 0.0],
                half: [40.0, 20.0],
                basis: [rad.cos() as f32, rad.sin() as f32],
                color: [38.0 / 255.0, 38.0 / 255.0, 44.0 / 255.0, 1.0],
            };
            let inst_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bld-self-check-inst"),
                contents: bytemuck::cast_slice(core::slice::from_ref(&inst)),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let pipeline = create_building_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bld-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let tview = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bld-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bld-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bld-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, unit_quad.slice(..));
                pass.set_vertex_buffer(1, inst_buf.slice(..));
                pass.draw(0..4, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            let probes: [(u32, u32, [u8; 4], &str); 3] = [
                (
                    400,
                    300,
                    [38, 38, 44, 255],
                    "center fill byte-exact (FILL_DEFAULT rgb)",
                ),
                (460, 300, [51, 68, 85, 255], "exterior = CLEAR_COLOR"),
                (
                    425,
                    310,
                    [38, 38, 44, 255],
                    "orientation +37 (inside +37, outside -37)",
                ),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                let pass = got == expect;
                all_pass &= pass;
                json.push(format!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}",
                    px, py, expect[0], expect[1], expect[2], expect[3],
                    got[0], got[1], got[2], got[3], pass, label,
                ));
            }
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }
}
