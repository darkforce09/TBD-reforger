//! Role: text.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::ortho::state::OrthoCamera;
use crate::diagnostics::readback::scene::map_read_4;
use crate::diagnostics::readback::scene::padded_bytes_per_row;
use crate::frame::engine::CLEAR_COLOR;
use crate::frame::engine::RenderEngine;

use crate::frame::pipelines::text::create_text_pipeline;
use crate::frame::upload::text::text_uniform_bytes;
use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Text self check.
    pub fn text_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let text_bgl = self.text_bind_group_layout.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("text-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("text-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            let (rgba, aw, ah) =
                crate::overlay::symbology::text_metrics::atlas::bake_ascii_atlas_rgba();
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("text-self-check-atlas"),
                size: wgpu::Extent3d {
                    width: aw,
                    height: ah,
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
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aw * 4),
                    rows_per_image: Some(ah),
                },
                wgpu::Extent3d {
                    width: aw,
                    height: ah,
                    depth_or_array_layers: 1,
                },
            );
            let u_bytes = text_uniform_bytes();
            let text_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("text-self-check-uniform"),
                contents: &u_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let samp = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("text-self-check-samp"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..wgpu::SamplerDescriptor::default()
            });
            let tview = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let text_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("text-self-check-atlas"),
                layout: &text_bgl,
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
                        resource: text_uniform.as_entire_binding(),
                    },
                ],
            });

            let inst = website_graphics_engine::layout::IconInstance {
                pos: [0.0, 0.0],
                size: 160.0,
                yaw: 0,
                glyph: 23,
                tint: 0xFFFF_FFFF,
            };
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("text-self-check-inst"),
                contents: bytemuck::bytes_of(&inst),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("text-self-check"),
                bind_group_layouts: &[Some(&cam_bgl), None, Some(&text_bgl)],
                immediate_size: 0,
            });
            let pipeline = create_text_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("text-self-check-target"),
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
                label: Some("text-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("text-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("text-self-check"),
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
                pass.set_bind_group(2, &text_bg, &[]);
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
                (
                    377,
                    252,
                    [240, 240, 230, 255],
                    "top bar (tx=11,ty=6) = glyph color",
                ),
                (
                    392,
                    312,
                    [240, 240, 230, 255],
                    "descender (tx=14,ty=18) = glyph color",
                ),
                (
                    407,
                    312,
                    [16, 21, 29, 255],
                    "halo (tx=17,ty=18) = TEXT_HALO_RGBA (U-mirror trap: mirrored ink lands here)",
                ),
                (
                    377,
                    347,
                    [51, 68, 85, 255],
                    "upright-empty (tx=11,ty=25) = CLEAR (V-flip trap)",
                ),
                (400, 50, [51, 68, 85, 255], "far exterior = CLEAR_COLOR"),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                let pass = got == *expect;
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
