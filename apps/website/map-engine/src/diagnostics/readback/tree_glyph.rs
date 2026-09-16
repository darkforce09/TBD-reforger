//! Role: tree glyph.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::ortho::state::OrthoCamera;
use crate::core::context::state::CLEAR_COLOR;
use crate::core::context::state::RenderEngine;
use crate::diagnostics::readback::scene::map_read_4;
use crate::diagnostics::readback::scene::padded_bytes_per_row;

use crate::renderers::batching::scene::ANCHOR;
use crate::renderers::pipelines::icon::create_icon_pipeline;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Tree glyph self check.
    pub fn tree_glyph_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let icon_bgl = self.icon_bind_group_layout.clone();
        let icon_layout = self.icon_pipeline_layout.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tree-glyph-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tree-glyph-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tree-glyph-self-check-atlas"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
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
                tex.as_image_copy(),
                &[255u8, 255, 255, 255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );

            let u_bytes = RenderEngine::pack_icon_uniforms(&[0.0f32, 0.0, 1.0, 1.0], 0.0, 0.0, 1.0);
            let uv_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tree-glyph-self-check-uv"),
                contents: &u_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let samp = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("tree-glyph-self-check-samp"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..wgpu::SamplerDescriptor::default()
            });
            let tview = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let atlas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tree-glyph-self-check-atlas"),
                layout: &icon_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tview),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&samp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uv_buf.as_entire_binding(),
                    },
                ],
            });

            let tint = 74u32 | (122u32 << 8) | (50u32 << 16) | (255u32 << 24);
            let inst = crate::renderers::batching::scene::IconInstance {
                pos: [0.0, 0.0],
                size: 40.0,
                yaw: 0,
                glyph: 0,
                tint,
            };
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tree-glyph-self-check-inst"),
                contents: bytemuck::bytes_of(&inst),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let pipeline = create_icon_pipeline(&device, &icon_layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tree-glyph-self-check-target"),
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
            let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tree-glyph-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tree-glyph-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("tree-glyph-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
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
                pass.set_bind_group(2, &atlas_bg, &[]);
                pass.set_vertex_buffer(0, unit_quad.slice(..));
                pass.set_vertex_buffer(1, ibuf.slice(..));
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

            let probes: &[(u32, u32, [u8; 4], &str)] = &[
                (400, 300, [74, 122, 50, 255], "glyph center = forest tint"),
                (400, 50, [51, 68, 85, 255], "far exterior = CLEAR_COLOR"),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;

                let pass = if *label == "glyph center = forest tint" {
                    got[3] > 0 && got[0] == expect[0] && got[1] == expect[1] && got[2] == expect[2]
                } else {
                    got == *expect
                };
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
