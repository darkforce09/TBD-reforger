//! Role: texture.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::ortho::state::OrthoCamera;
use crate::diagnostics::readback::scene::map_read_4;
use crate::diagnostics::readback::scene::padded_bytes_per_row;
use crate::frame::engine::CLEAR_COLOR;
use crate::frame::engine::RenderEngine;
use crate::frame::pipelines::textured::create_textured_pipeline;
use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::layout::QuadInstance;

#[wasm_bindgen]
impl RenderEngine {
    /// Texture self check.
    pub fn texture_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let quad_layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let tex_bgl = self.tex_bind_group_layout.clone();
        let sampler = self.sampler.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;

            let texels: [u8; 16] = [
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ];
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tex-self-check"),
                size: wgpu::Extent3d {
                    width: 2,
                    height: 2,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                texture.as_image_copy(),
                &texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: Some(2),
                },
                wgpu::Extent3d {
                    width: 2,
                    height: 2,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tex-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tex-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });
            let tex_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tex-self-check-tex"),
                layout: &tex_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let inst = QuadInstance {
                min: [-400.0, -300.0],
                max: [400.0, 300.0],
                color: [1.0, 1.0, 1.0, 1.0],
            };
            let inst_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tex-self-check-quad"),
                contents: bytemuck::cast_slice(&[inst]),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let tex_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("tex-self-check"),
                bind_group_layouts: &[Some(&cam_bgl), Some(&tex_bgl)],
                immediate_size: 0,
            });
            let pipeline = create_textured_pipeline(&device, &tex_layout, &shader, fmt);
            let _ = &quad_layout;

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tex-self-check-target"),
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
                label: Some("tex-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tex-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("tex-self-check"),
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
                pass.set_bind_group(1, &tex_bind, &[]);
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
                    100,
                    100,
                    [255, 0, 0, 255],
                    "NW (north-up proof: red, not blue/green)",
                ),
                (700, 100, [0, 255, 0, 255], "NE"),
                (100, 500, [0, 0, 255, 255], "SW"),
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
