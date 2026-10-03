//! **Role:** the sea band readback self-check: the sea polygon's band colour at its centre and near its
//! corner.
//! **Position:** `readback` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes it as `window.__selfChecks.sea_band_self_check`; it reads the engine only through
//! `map_renderer::diagnostic_accessors`.
//! **Signals & state:** none; each run builds and drops its own target, pipeline and buffers.
//! **Invariants:** the check camera is fixed at the world anchor, independent of the live view;
//! expected pixels are exact bytes.

use crate::readback::scene::map_read_4;
use crate::readback::scene::padded_bytes_per_row;
use camera_math::ortho::state::OrthoCamera;
use map_renderer::engine::CLEAR_COLOR;
use map_renderer::engine::RenderEngine;

use gpu_frame::pipeline::vector::create_polygon_pipeline;
use map_coordinates::terrain_frames::ANCHOR;
use wasm_bindgen::JsValue;

/// Draws the sea band polygon and probes its colour at the centre and near a corner byte-exact.
/// Resolves to the JSON report `{"backend", "probes": [...], "pass"}`.
pub fn sea_band_self_check(engine: &RenderEngine) -> js_sys::Promise {
    const PW: u32 = 800;
    const PH: u32 = 600;
    let gpu = engine.diagnostic_device();
    let resources = engine.diagnostic_pipeline_resources();
    let device = gpu.device.clone();
    let queue = gpu.queue.clone();
    let shader = resources.shader.clone();
    let layout = resources.quad_pipeline_layout.clone();
    let cam_bgl = resources.camera_bind_group_layout.clone();
    let backend = gpu.backend_kind.to_owned();

    wasm_bindgen_futures::future_to_promise(async move {
        use wgpu::util::DeviceExt;
        let fmt = wgpu::TextureFormat::Rgba8Unorm;
        let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
        let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sea-self-check-mvp"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
        let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea-self-check-mvp"),
            layout: &cam_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let half_w = f64::from(PW) * 0.5;
        let half_h = f64::from(PH) * 0.5;
        let sea = [72.0 / 255.0, 118.0 / 255.0, 160.0 / 255.0, 1.0];
        let corners = [
            [-half_w as f32, -half_h as f32],
            [half_w as f32, -half_h as f32],
            [half_w as f32, half_h as f32],
            [-half_w as f32, half_h as f32],
        ];
        let mut verts = Vec::with_capacity(4);
        for p in corners {
            verts.push(render_primitives::draw::geometry::LineVertex { pos: p, color: sea });
        }
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sea-self-check-verts"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sea-self-check-idx"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let pipeline = create_polygon_pipeline(&device, &layout, &shader, fmt);

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sea-self-check-target"),
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
            label: Some("sea-self-check-read"),
            size: u64::from(padded) * u64::from(PH),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sea-self-check"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sea-self-check"),
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
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..6, 0, 0..1);
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

        let probes: [(u32, u32, [u8; 4], &str); 2] = [
            (
                400,
                300,
                [72, 118, 160, 255],
                "center = sea <=0m band colour",
            ),
            (50, 50, [72, 118, 160, 255], "corner interior still sea"),
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
