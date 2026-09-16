//! Role: marquee.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::ortho::state::OrthoCamera;
use crate::core::context::state::CLEAR_COLOR;
use crate::core::context::state::RenderEngine;
use crate::diagnostics::readback::scene::map_read_4;
use crate::diagnostics::readback::scene::padded_bytes_per_row;

use crate::renderers::batching::scene::ANCHOR;
use crate::renderers::pipelines::vector::create_line_pipeline;
use crate::renderers::pipelines::vector::create_polygon_pipeline;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Non-α-1 blends round through the GPU's float pipeline, so interior/border assert **±1 per channel** against the f64-computed expectation (documented GPU-R-adv, unlike the α=1 byte-exact checks). Resolves to JSON.
    pub fn marquee_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("marquee-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("marquee-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            let fill_c = [173.0_f32 / 255.0, 198.0 / 255.0, 1.0, 40.0 / 255.0];
            let line_c = [173.0_f32 / 255.0, 198.0 / 255.0, 1.0, 200.0 / 255.0];
            let (x0, y0, x1, y1) = (-100.0_f32, -100.0, 100.0, 100.0);
            let fill_verts = [
                crate::renderers::batching::lanes::LineVertex {
                    pos: [x0, y0],
                    color: fill_c,
                },
                crate::renderers::batching::lanes::LineVertex {
                    pos: [x1, y0],
                    color: fill_c,
                },
                crate::renderers::batching::lanes::LineVertex {
                    pos: [x1, y1],
                    color: fill_c,
                },
                crate::renderers::batching::lanes::LineVertex {
                    pos: [x0, y1],
                    color: fill_c,
                },
            ];
            let fill_idx: [u32; 6] = [0, 1, 2, 0, 2, 3];
            let ring = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
            let mut line_verts = Vec::with_capacity(8);
            for e in 0..4 {
                let a = ring[e];
                let b = ring[(e + 1) % 4];
                line_verts.push(crate::renderers::batching::lanes::LineVertex {
                    pos: a,
                    color: line_c,
                });
                line_verts.push(crate::renderers::batching::lanes::LineVertex {
                    pos: b,
                    color: line_c,
                });
            }
            let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-self-check-fill"),
                contents: bytemuck::cast_slice(&fill_verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-self-check-idx"),
                contents: bytemuck::cast_slice(&fill_idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let lbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-self-check-line"),
                contents: bytemuck::cast_slice(&line_verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let polygon = create_polygon_pipeline(&device, &layout, &shader, fmt);
            let line = create_line_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("marquee-self-check-target"),
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
                label: Some("marquee-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("marquee-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("marquee-self-check"),
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
                pass.set_pipeline(&polygon);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..6, 0, 0..1);
                pass.set_pipeline(&line);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, lbuf.slice(..));
                pass.draw(0..8, 0..1);
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

            let clear = [51.0_f64, 68.0, 85.0];
            let blend = |src: [f64; 3], alpha: f64, dst: [f64; 3]| -> [f64; 3] {
                [
                    src[0] * alpha + dst[0] * (1.0 - alpha),
                    src[1] * alpha + dst[1] * (1.0 - alpha),
                    src[2] * alpha + dst[2] * (1.0 - alpha),
                ]
            };
            let prim = [173.0_f64, 198.0, 255.0];
            let interior_f = blend(prim, 40.0 / 255.0, clear);

            let border_over_fill_f = blend(prim, 200.0 / 255.0, interior_f);
            let border_over_clear_f = blend(prim, 200.0 / 255.0, clear);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let to_u8 = |v: [f64; 3]| -> [u8; 3] {
                [
                    v[0].round().clamp(0.0, 255.0) as u8,
                    v[1].round().clamp(0.0, 255.0) as u8,
                    v[2].round().clamp(0.0, 255.0) as u8,
                ]
            };
            let interior_e = to_u8(interior_f);
            let border_fill_e = to_u8(border_over_fill_f);
            let border_clear_e = to_u8(border_over_clear_f);
            let within = |got: [u8; 4], expect: [u8; 3]| -> bool {
                (0..3).all(|i| got[i].abs_diff(expect[i]) <= 1)
            };

            let read = |px: u32, py: u32| {
                let offset = u64::from(py * padded + px * 4);
                (px, py, offset)
            };
            let mut json = Vec::new();
            let mut all_pass = true;

            let (px, py, off) = read(400, 300);
            let got = map_read_4(&device, &read_buf, off)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            let pass = within(got, interior_e);
            all_pass &= pass;
            json.push(format!(
                "{{\"px\":{px},\"py\":{py},\"expect\":[{},{},{}],\"got\":[{},{},{},{}],\"tol\":1,\"pass\":{pass},\"label\":\"fill interior (adv ±1)\"}}",
                interior_e[0], interior_e[1], interior_e[2], got[0], got[1], got[2], got[3],
            ));

            let mut border_pass = false;
            let mut border_got = [0u8; 4];
            for bx in [300u32, 299] {
                let (_, _, off) = read(bx, 300);
                let got = map_read_4(&device, &read_buf, off)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                if within(got, border_fill_e) || within(got, border_clear_e) {
                    border_pass = true;
                    border_got = got;
                    break;
                }
                border_got = got;
            }
            all_pass &= border_pass;
            json.push(format!(
                "{{\"px\":\"300|299\",\"py\":300,\"expect\":\"[{},{},{}] over fill | [{},{},{}] over clear\",\"got\":[{},{},{},{}],\"tol\":1,\"pass\":{border_pass},\"label\":\"border column (adv ±1)\"}}",
                border_fill_e[0], border_fill_e[1], border_fill_e[2],
                border_clear_e[0], border_clear_e[1], border_clear_e[2],
                border_got[0], border_got[1], border_got[2], border_got[3],
            ));

            let (px, py, off) = read(600, 300);
            let got = map_read_4(&device, &read_buf, off)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            let pass = got == [51, 68, 85, 255];
            all_pass &= pass;
            json.push(format!(
                "{{\"px\":{px},\"py\":{py},\"expect\":[51,68,85,255],\"got\":[{},{},{},{}],\"pass\":{pass},\"label\":\"exterior = CLEAR_COLOR (byte-exact)\"}}",
                got[0], got[1], got[2], got[3],
            ));

            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }
}
