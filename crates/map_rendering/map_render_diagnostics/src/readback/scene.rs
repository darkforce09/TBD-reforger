//! **Role:** the shared readback helpers (padded rows, the 4 ms sleep, the four-byte map read) and the
//! live scene readback: `readback_rgba` redraws the engine's batch list offscreen and reads one
//! pixel.
//! **Position:** `readback` of the map render diagnostics; every check and the benchmark use the
//! helpers; the Mission Creator's viewport bridge publishes `readback_rgba` on
//! `window.__selfChecks`; the scene is read only through `map_renderer::diagnostic_accessors`.
//! **Signals & state:** none; each readback builds and drops its own target, pipelines and buffers.
//! **Invariants:** a scene readback fills local pipeline and bind-group tables, never the engine's,
//! so the next real frame never binds an offscreen target; a map read gives up after 2000 polls
//! 4 ms apart.

use gpu_frame::pipeline::icon::create_icon_pipeline;
use gpu_frame::pipeline::oriented_quad::create_oriented_quad_pipeline;
use gpu_frame::pipeline::quad::create_quad_pipeline;
use gpu_frame::pipeline::text::create_text_pipeline;
use gpu_frame::pipeline::textured::create_density_pipeline;
use gpu_frame::pipeline::textured::create_textured_pipeline;
use gpu_frame::pipeline::vector::create_line_pipeline;
use gpu_frame::pipeline::vector::create_polygon_pipeline;
use map_coordinates::terrain_frames::ANCHOR;
use map_renderer::encode::pipeline_table;
use map_renderer::engine::RenderEngine;
use renderer_core::packet_bindings;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsValue;

/// The bytes of one `width`-pixel RGBA row, padded up to wgpu's copy row alignment.
pub(crate) fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(align) * align
}

/// Resolves after `ms` milliseconds of the page's `setTimeout`.
pub(crate) async fn readback_sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Maps `read_buf`, polling `device` every 4 ms for up to 2000 polls, and returns the four bytes
/// at `offset`; errs `readback-map-timeout` or `readback-map-failed`.
pub(crate) async fn map_read_4(
    device: &wgpu::Device,
    read_buf: &wgpu::Buffer,
    offset: u64,
) -> Result<[u8; 4], String> {
    let done = Rc::new(Cell::new(0u8));
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
        readback_sleep_ms(4).await;
        ticks += 1;
        if ticks > 2000 {
            return Err("readback-map-timeout".to_owned());
        }
    }
    if done.get() == 2 {
        return Err("readback-map-failed".to_owned());
    }
    let out: [u8; 4] = {
        let data = read_buf.slice(..).get_mapped_range();
        let b = offset as usize;
        data[b..b + 4].try_into().expect("4 bytes")
    };
    read_buf.unmap();
    Ok(out)
}

/// The offscreen pipelines a scene readback draws the live batch list with, one per packet
/// pipeline slot.
struct ReadbackPipelines {
    /// Quads.
    quad: wgpu::RenderPipeline,

    /// Textured rectangles.
    textured: wgpu::RenderPipeline,

    /// The forest density texture.
    density: wgpu::RenderPipeline,

    /// Lines.
    line: wgpu::RenderPipeline,

    /// Oriented quads (buildings).
    oriented_quad: wgpu::RenderPipeline,

    /// Polygons.
    polygon: wgpu::RenderPipeline,

    /// Sprites.
    icon: wgpu::RenderPipeline,

    /// Glyph runs.
    text: wgpu::RenderPipeline,
}

/// Draws the engine's live batch list into a `w` × `h` `Rgba8Unorm` target with offscreen
/// pipelines and a camera bind group of its own, and returns the buffer the target is copied into
/// (rows padded to `padded` bytes).
pub(crate) fn encode_scene_readback(
    engine: &RenderEngine,
    w: u32,
    h: u32,
    padded: u32,
) -> wgpu::Buffer {
    let gpu = engine.diagnostic_device();
    let resources = engine.diagnostic_pipeline_resources();
    let scene = engine.diagnostic_scene();
    let device = gpu.device;
    let shader = resources.shader;
    let fmt = wgpu::TextureFormat::Rgba8Unorm;
    let quad = create_quad_pipeline(device, resources.quad_pipeline_layout, shader, fmt);
    let tex_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("readback-textured"),
        bind_group_layouts: &[
            Some(resources.camera_bind_group_layout),
            Some(resources.textured_bind_group_layout),
        ],
        immediate_size: 0,
    });
    let textured = create_textured_pipeline(device, &tex_layout, shader, fmt);
    let density = create_density_pipeline(device, &tex_layout, shader, fmt);
    let line = create_line_pipeline(device, resources.quad_pipeline_layout, shader, fmt);
    let oriented_quad =
        create_oriented_quad_pipeline(device, resources.quad_pipeline_layout, shader, fmt);
    let polygon = create_polygon_pipeline(device, resources.quad_pipeline_layout, shader, fmt);

    let mvp = scene.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
    let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback-mvp"),
        size: 64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue
        .write_buffer(&uniform_buf, 0, bytemuck::cast_slice(&mvp));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("readback-mvp"),
        layout: resources.camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });

    let icon = create_icon_pipeline(device, resources.icon_pipeline_layout, shader, fmt);
    let text = create_text_pipeline(device, resources.text_pipeline_layout, shader, fmt);
    let pipelines = ReadbackPipelines {
        quad,
        textured,
        density,
        line,
        oriented_quad,
        polygon,
        icon,
        text,
    };
    let (read_buf, _view, _texture) =
        render_target_readback(engine, w, h, padded, &bind_group, &pipelines);
    read_buf
}

/// Draws the live batch list with `pipelines` and the camera `bind_group` into a fresh
/// `w` × `h` target, copies it into a mappable buffer and submits; returns the buffer, the view
/// and the target.
fn render_target_readback(
    engine: &RenderEngine,
    w: u32,
    h: u32,
    padded: u32,
    bind_group: &wgpu::BindGroup,
    pipelines: &ReadbackPipelines,
) -> (wgpu::Buffer, wgpu::TextureView, wgpu::Texture) {
    let gpu = engine.diagnostic_device();
    let scene = engine.diagnostic_scene();
    let device = gpu.device;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("readback-target"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
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
    let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback-buffer"),
        size: u64::from(padded) * u64::from(h),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("readback"),
    });
    // The readback draws the SAME batch list through the same renderer entry point, with its own
    // offscreen pipelines and its own camera bind group: swapping the two tables is all it takes.
    // The live path refills the engine's persistent tables; this one deliberately does not.
    // Writing the offscreen pipelines and camera bind group into the engine's tables would leave
    // the next real frame bound to an offscreen target. It runs once per probe rather than 60
    // times a second, which is why two local allocations here are not the persistent-batch-list
    // case. The scene view is read-only, which makes that structural rather than a matter of
    // discipline.
    let mut pipeline_slots = Vec::new();
    pipeline_table(
        &pipelines.quad,
        &pipelines.textured,
        &pipelines.density,
        &pipelines.line,
        &pipelines.oriented_quad,
        &pipelines.polygon,
        &pipelines.icon,
        &pipelines.text,
        None,
        &mut pipeline_slots,
    );
    let mut bind_groups = Vec::new();
    scene.fill_bind_group_table(bind_group, &mut bind_groups);
    let mvp = scene.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
    let packet = gpu_frame::frame::FramePacket {
        camera: render_primitives::frame::camera::CameraUniform::new(mvp),
        clear: scene.clear_color,
        batches: scene.batches,
        text: &[],
        indirect: &[],
        pipelines: &pipeline_slots,
        bind_groups: &bind_groups,
        camera_bind: packet_bindings::BIND_CAMERA,
        unit_quad: scene.unit_quad_buffer,
    };

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("readback"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(scene.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        gpu_frame::draw::encode::encode(&mut pass, &packet);
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read_buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit(Some(encoder.finish()));
    (read_buf, view, texture)
}

/// Reads one pixel of the engine's live scene, redrawn offscreen at the canvas size; resolves to
/// `{"x", "y", "backend", "rgba": [r, g, b, a]}`, or rejects when the pixel is outside the canvas.
pub fn readback_rgba(engine: &RenderEngine, x_px: u32, y_px: u32) -> js_sys::Promise {
    let gpu = engine.diagnostic_device();
    let w = gpu.surface_width;
    let h = gpu.surface_height;
    if x_px >= w || y_px >= h {
        return js_sys::Promise::reject(&JsValue::from_str("readback: pixel out of bounds"));
    }
    let padded = padded_bytes_per_row(w);
    let read_buf = encode_scene_readback(engine, w, h, padded);
    let offset = u64::from(y_px * padded + x_px * 4);
    let device = gpu.device.clone();
    let backend = gpu.backend_kind.to_owned();
    wasm_bindgen_futures::future_to_promise(async move {
        let rgba = map_read_4(&device, &read_buf, offset)
            .await
            .map_err(|e| JsValue::from_str(&e))?;
        Ok(JsValue::from_str(&format!(
            "{{\"x\":{},\"y\":{},\"backend\":\"{}\",\"rgba\":[{},{},{},{}]}}",
            x_px, y_px, backend, rgba[0], rgba[1], rgba[2], rgba[3],
        )))
    })
}
