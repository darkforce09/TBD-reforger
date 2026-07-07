//! `self_check` — GPU rendering made numeric (T-151 plan §S4c). Renders the calibration
//! scene with a **fixed local camera** (independent of the live view state) into a dedicated
//! offscreen `Rgba8Unorm` 800×600 target, reads the pixels back, and byte-compares the seven
//! probe points. Every expectation is `==` — no tolerances — legitimate because all quad
//! edges land on integer pixel coordinates (pixel centers at half-integers decide coverage
//! at ≥ 0.5 px from every edge, vs < 1e-3 px worst-case f32 displacement) and all colors are
//! forced through the unorm8 rounding margin (plan §S4 margin arguments).
//!
//! Exposed as a synchronous method returning a JS `Promise`: the wasm-bindgen borrow on the
//! engine ends before the async body runs (all wgpu handles are cheap `Arc` clones), so the
//! page's rAF loop can keep calling `render()` while the readback is in flight — an async
//! `&self` method would hold the WasmRefCell borrow across the await and panic on reentry.

use map_engine_core::camera::OrthoCamera;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::engine::{CLEAR_COLOR, RenderEngine, create_quad_pipeline};
use crate::scene::ANCHOR;

const PROBE_W: u32 = 800;
const PROBE_H: u32 = 600;
/// `align(800·4, COPY_BYTES_PER_ROW_ALIGNMENT=256)` — 3200 → 3328; rows carry 128 pad bytes.
const PADDED_BYTES_PER_ROW: u32 = 3328;

const GREEN: [u8; 4] = [0, 255, 0, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const CLEAR_BYTES: [u8; 4] = [51, 68, 85, 255];

/// The seven probes (plan §S4c). Screen rects at the probe camera (800×600, zoom 0, target
/// (6400,6400) = anchor): G x∈[300,500] y∈[200,400]; R x∈[450,490] y∈[210,250].
const PROBES: &[(u32, u32, [u8; 4], &str)] = &[
    (400, 300, GREEN, "center of G"),
    (302, 302, GREEN, "2px inside G NW corner"),
    (498, 398, GREEN, "2px inside G SE corner"),
    (298, 300, CLEAR_BYTES, "2px west of G (clear)"),
    (400, 198, CLEAR_BYTES, "2px north of G (clear)"),
    // Orientation kill-shot: if the y-axis were flipped these two swap, so a sign error
    // cannot pass — R sits NE of center and must render UP-and-right (smaller pixel y).
    (470, 230, RED, "inside R (NE quadrant, north-up proof)"),
    (470, 370, GREEN, "mirror of R probe (must be G, not R)"),
];

/// Yield to the browser macrotask queue — lets the GL fence / map callbacks progress.
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[wasm_bindgen]
impl RenderEngine {
    /// Run the byte-exact readback self-check; resolves to the JSON report
    /// `{"backend", "probes": [{px, py, expect, got, pass, label}], "pass"}`.
    pub fn self_check(&self) -> js_sys::Promise {
        // Clone the shared handles now so no engine borrow crosses an await point.
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let pipeline_layout = self.pipeline_layout.clone();
        let bind_group_layout = self.bind_group_layout.clone();
        let unit_quad_buf = self.unit_quad_buf.clone();
        let calibration_buf = self.calibration_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            run_self_check(
                &device,
                &queue,
                &shader,
                &pipeline_layout,
                &bind_group_layout,
                &unit_quad_buf,
                &calibration_buf,
                &backend,
            )
            .await
            .map(JsValue::from)
            .map_err(|e| JsValue::from_str(&e))
        })
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_self_check(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    bind_group_layout: &wgpu::BindGroupLayout,
    unit_quad_buf: &wgpu::Buffer,
    calibration_buf: &wgpu::Buffer,
    backend: &str,
) -> Result<String, String> {
    // Fixed probe camera — independent of the live view state, so the check is
    // deterministic regardless of window size or current pan/zoom.
    let camera = OrthoCamera::new(f64::from(PROBE_W), f64::from(PROBE_H), 6400.0, 6400.0, 0.0);
    let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);

    let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe-mvp"),
        size: 64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buf, 0, bytemuck::cast_slice(&mvp));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("probe-mvp"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe-target"),
        size: wgpu::Extent3d {
            width: PROBE_W,
            height: PROBE_H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let pipeline = create_quad_pipeline(
        device,
        pipeline_layout,
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );

    let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe-readback"),
        size: u64::from(PADDED_BYTES_PER_ROW) * u64::from(PROBE_H),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("probe"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("probe"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
        pass.set_vertex_buffer(1, calibration_buf.slice(..));
        pass.draw(0..4, 0..2);
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read_buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PADDED_BYTES_PER_ROW),
                rows_per_image: Some(PROBE_H),
            },
        },
        wgpu::Extent3d {
            width: PROBE_W,
            height: PROBE_H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    // Async map with a bounded poll/yield loop (no blocking poll exists on wasm; the GL
    // path needs poll ticks for its fence — plan §S4c).
    let done = Rc::new(Cell::new(0u8)); // 0 pending, 1 ok, 2 error
    {
        let done = done.clone();
        read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                done.set(if res.is_ok() { 1 } else { 2 });
            });
    }
    let mut ticks = 0;
    while done.get() == 0 {
        let _ = device.poll(wgpu::PollType::Poll);
        sleep_ms(4).await;
        ticks += 1;
        if ticks > 2000 {
            return Err("probe-map-timeout: readback did not complete within ~8s".to_owned());
        }
    }
    if done.get() == 2 {
        return Err("probe-map-failed".to_owned());
    }

    // Extract probes from the padded rows and byte-compare.
    let mut probes_json = Vec::with_capacity(PROBES.len());
    let mut all_pass = true;
    {
        let data = read_buf.slice(..).get_mapped_range();
        for &(px, py, expect, label) in PROBES {
            let base = (py * PADDED_BYTES_PER_ROW + px * 4) as usize;
            let got: [u8; 4] = data[base..base + 4].try_into().expect("4 bytes");
            let pass = got == expect;
            all_pass &= pass;
            probes_json.push(format!(
                concat!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],",
                    "\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}"
                ),
                px,
                py,
                expect[0],
                expect[1],
                expect[2],
                expect[3],
                got[0],
                got[1],
                got[2],
                got[3],
                pass,
                label,
            ));
        }
    }
    read_buf.unmap();

    Ok(format!(
        "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
        backend,
        probes_json.join(","),
        all_pass,
    ))
}
