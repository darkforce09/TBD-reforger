//! **Role:** the texture readback self-check: a 2×2 texture drawn north-up through the textured
//! pipeline, probed at three corners.
//! **Position:** `readback` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes it as `window.__selfChecks.texture`; it reads the engine only through
//! `map_renderer::diagnostic_accessors`.
//! **Signals & state:** none; each run builds and drops its own target, pipeline and buffers.
//! **Invariants:** the check camera is fixed at the world anchor, independent of the live view;
//! expected pixels are exact bytes.

use crate::readback::scene::map_read_4;
use crate::readback::scene::padded_bytes_per_row;
use camera_math::ortho::state::OrthoCamera;
use gpu_frame::pipeline::textured::create_textured_pipeline;
use map_coordinates::terrain_frames::ANCHOR;
use map_renderer::engine::CLEAR_COLOR;
use map_renderer::engine::RenderEngine;
use render_primitives::draw::instances::QuadInstance;
use wasm_bindgen::JsValue;

/// Draws a 2×2 red, green, blue, white texture over an 800 × 600 target through the textured
/// pipeline and probes three corners byte-exact (north-up: red at the north-west). Resolves to the
/// JSON report `{"backend", "probes": [{px, py, expect, got, pass, label}], "pass"}`.
pub fn texture_self_check(engine: &RenderEngine) -> js_sys::Promise {
    const PW: u32 = 800;
    const PH: u32 = 600;
    let gpu = engine.diagnostic_device();
    let resources = engine.diagnostic_pipeline_resources();
    let device = gpu.device.clone();
    let queue = gpu.queue.clone();
    let shader = resources.shader.clone();
    let cam_bgl = resources.camera_bind_group_layout.clone();
    let tex_bgl = resources.textured_bind_group_layout.clone();
    let sampler = resources.sampler.clone();
    let unit_quad = resources.unit_quad_buffer.clone();
    let backend = gpu.backend_kind.to_owned();

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
