//! Role: scene.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::frame::bindings;
use crate::frame::encode::bind_group_table;
use crate::frame::encode::pipeline_table;
use crate::frame::engine::RenderEngine;
use crate::frame::pipelines::building::create_building_pipeline;
use crate::frame::pipelines::icon::create_icon_pipeline;
use crate::frame::pipelines::quad::create_quad_pipeline;
use crate::frame::pipelines::text::create_text_pipeline;
use crate::frame::pipelines::textured::create_forest_density_pipeline;
use crate::frame::pipelines::textured::create_textured_pipeline;
use crate::frame::pipelines::vector::create_line_pipeline;
use crate::frame::pipelines::vector::create_polygon_pipeline;
use crate::world::scene::ANCHOR;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Padded bytes per row.
pub(crate) fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(align) * align
}

/// Readback sleep ms.
pub(crate) async fn readback_sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Map read 4.
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

impl RenderEngine {
    /// Poll.
    pub fn poll(&self) {
        let _ = self.device.poll(wgpu::PollType::Poll);
    }
}

impl RenderEngine {
    /// Disable frame timing.
    pub fn disable_frame_timing(&mut self) {
        self.timer = None;
    }
}

impl RenderEngine {
    /// Encode scene readback.
    pub(crate) fn encode_scene_readback(&self, w: u32, h: u32, padded: u32) -> wgpu::Buffer {
        let fmt = wgpu::TextureFormat::Rgba8Unorm;
        let quad = create_quad_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);
        let tex_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("readback-textured"),
                bind_group_layouts: &[
                    Some(&self.bind_group_layout),
                    Some(&self.tex_bind_group_layout),
                ],
                immediate_size: 0,
            });
        let textured = create_textured_pipeline(&self.device, &tex_layout, &self.shader, fmt);
        let forest_density =
            create_forest_density_pipeline(&self.device, &tex_layout, &self.shader, fmt);
        let line = create_line_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);
        let building =
            create_building_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);
        let polygon =
            create_polygon_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);

        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-mvp"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniform_buf, 0, bytemuck::cast_slice(&mvp));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("readback-mvp"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let icon =
            create_icon_pipeline(&self.device, &self.icon_pipeline_layout, &self.shader, fmt);
        let text =
            create_text_pipeline(&self.device, &self.text_pipeline_layout, &self.shader, fmt);
        let (read_buf, _view, _texture) = self.render_target_readback(
            w,
            h,
            padded,
            &bind_group,
            &quad,
            &textured,
            &forest_density,
            &line,
            &building,
            &polygon,
            &icon,
            &text,
        );
        read_buf
    }
}

impl RenderEngine {
    /// Render target readback.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_target_readback(
        &self,
        w: u32,
        h: u32,
        padded: u32,
        bind_group: &wgpu::BindGroup,
        quad: &wgpu::RenderPipeline,
        textured: &wgpu::RenderPipeline,
        forest_density: &wgpu::RenderPipeline,
        line: &wgpu::RenderPipeline,
        building: &wgpu::RenderPipeline,
        polygon: &wgpu::RenderPipeline,
        icon: &wgpu::RenderPipeline,
        text: &wgpu::RenderPipeline,
    ) -> (wgpu::Buffer, wgpu::TextureView, wgpu::Texture) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
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
        let read_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-buffer"),
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        // T-0xx Phase 1D: the readback path draws the SAME batch list through the same
        // renderer entry point, with its own offscreen pipelines and its own camera bind
        // group. That it can do so by swapping two tables is the point of the packet.
        // T-0xx Phase 2C §R1: the live path refills `RenderEngine`'s persistent tables; this
        // one deliberately does not. The probe draws with its OWN offscreen pipelines and its
        // OWN camera bind group, so writing them into the engine's tables would leave the next
        // real frame bound to an offscreen target. It also runs once per probe rather than 60
        // times a second, which is why two local allocations here are not the rule-1 case.
        // `&self` makes that structural rather than a matter of discipline.
        let mut pipelines = Vec::new();
        pipeline_table(
            quad,
            textured,
            forest_density,
            line,
            building,
            polygon,
            icon,
            text,
            None,
            &mut pipelines,
        );
        let mut bind_groups = Vec::new();
        bind_group_table(
            bind_group,
            self.glyph_atlas.as_ref(),
            self.text_atlas.as_ref(),
            self.slot_atlas.as_ref(),
            &self.tex_lanes,
            &mut bind_groups,
        );
        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        let packet = crate::frame::FramePacket {
            camera: crate::frame::CameraUniform::new(mvp),
            clear: self.clear_color,
            batches: &self.batches,
            text: &[],
            indirect: &[],
            pipelines: &pipelines,
            bind_groups: &bind_groups,
            camera_bind: bindings::BIND_CAMERA,
            unit_quad: &self.unit_quad_buf,
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("readback"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            website_graphics_engine::draw::encode::encode(&mut pass, &packet);
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
        self.queue.submit(Some(encoder.finish()));
        (read_buf, view, texture)
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Readback rgba.
    pub fn readback_rgba(&self, x_px: u32, y_px: u32) -> js_sys::Promise {
        let w = self.config.width;
        let h = self.config.height;
        if x_px >= w || y_px >= h {
            return js_sys::Promise::reject(&JsValue::from_str("readback: pixel out of bounds"));
        }
        let padded = padded_bytes_per_row(w);
        let read_buf = self.encode_scene_readback(w, h, padded);
        let offset = u64::from(y_px * padded + x_px * 4);
        let device = self.device.clone();
        let backend = self.backend_kind.clone();
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
}
