//! Role: frame 1.
//! Position: `diagnostics/bench` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::diagnostics::readback::scene::readback_sleep_ms;
use crate::diagnostics::timing::gpu::now_ms;
use crate::diagnostics::timing::gpu::perf_now_ms;
use crate::frame::bindings;
use crate::frame::engine::RenderEngine;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;

use crate::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use crate::world::scene::ANCHOR;
use wasm_bindgen::prelude::*;
use website_graphics_engine::layout::CHUNK_CAPACITY;

#[wasm_bindgen]
impl RenderEngine {
    /// Render bench.
    pub fn render_bench(&mut self, n: u32) -> js_sys::Promise {
        let n = n.clamp(1, 20_000);
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench-target"),
            size: wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);

        let mut cpu_ms: Vec<f64> = Vec::with_capacity(n as usize);
        let mut submit_ms: Vec<f64> = Vec::with_capacity(n as usize);
        let t_start = perf_now_ms();
        for _ in 0..n {
            let f0 = perf_now_ms();
            self.queue
                .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&mvp));
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("bench-frame"),
                });
            self.encode_main_pass(&mut encoder, &view, false);
            let cmd = encoder.finish();
            let f1 = perf_now_ms();
            self.queue.submit(Some(cmd));
            let f2 = perf_now_ms();
            cpu_ms.push(f1 - f0);
            submit_ms.push(f2 - f1);
        }
        let submit_wall_ms = perf_now_ms() - t_start;
        let submit_avg = submit_ms.iter().sum::<f64>() / submit_ms.len() as f64;
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let done = std::sync::Arc::clone(&done);
            self.queue.on_submitted_work_done(move || {
                done.store(true, std::sync::atomic::Ordering::SeqCst);
            });
        }
        let mut sorted = cpu_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let avg = cpu_ms.iter().sum::<f64>() / cpu_ms.len() as f64;
        let p95 = sorted[((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1)];
        let max = sorted[sorted.len() - 1];

        let cpu_fps_equiv = 1000.0 / avg.max(0.0001);
        wasm_bindgen_futures::future_to_promise(async move {
            let drain_start = perf_now_ms();
            while !done.load(std::sync::atomic::Ordering::SeqCst)
                && perf_now_ms() - drain_start < 3_000.0
            {
                readback_sleep_ms(4).await;
            }
            let drained = done.load(std::sync::atomic::Ordering::SeqCst);
            let total_wall_ms = perf_now_ms() - t_start;
            Ok(JsValue::from_str(&format!(
                "{{\"n\":{n},\"submit_wall_ms\":{submit_wall_ms:.3},\"total_wall_ms\":{total_wall_ms:.3},\"cpu_avg_ms\":{avg:.4},\"cpu_p95_ms\":{p95:.4},\"cpu_max_ms\":{max:.4},\"submit_avg_ms\":{submit_avg:.4},\"fps_equiv\":{cpu_fps_equiv:.1},\"drained\":{drained}}}"
            )))
        })
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Stream `n` deterministic stress quads into the chunked pool (plan §S4d). Integer accounting: `stats().instances` afterwards equals exactly `n`.
    pub fn seed_stress(&mut self, n: u32, seed: u32) {
        self.clear_stress();

        let calibration = self
            .batches
            .pop()
            .expect("calibration batch always present");
        let seed = u64::from(seed);
        let mut remaining = n as usize;
        let mut chunk_idx: u32 = 0;
        let mut gen_ms = 0.0;
        let mut upload_ms = 0.0;
        while remaining > 0 {
            let count = remaining.min(CHUNK_CAPACITY);
            let g0 = now_ms();
            crate::world::scene::stress_chunk_into(chunk_idx, count, seed, &mut self.staging);
            gen_ms += now_ms() - g0;

            let u0 = now_ms();
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("stress-chunk"),
                size: (count * 32) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue
                .write_buffer(&buffer, 0, bytemuck::cast_slice(&self.staging));
            upload_ms += now_ms() - u0;

            #[allow(clippy::cast_possible_truncation)]
            self.batches.push(DrawBatch {
                lane: lane_id(LaneRole::Stress),
                visible: true,
                pipeline: bindings::PIPE_QUAD,
                payload: DrawPayload::Quads(InstanceBuffer::whole(buffer, 32, count as u32)),
            });
            self.stress_instances += count as u64;
            self.staging_peak_bytes = self
                .staging_peak_bytes
                .max(self.staging.capacity() as u64 * 32);
            remaining -= count;
            chunk_idx += 1;
        }
        self.batches.push(calibration);
        self.gen_ms = gen_ms;
        self.upload_ms = upload_ms;
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// Drop the stress pool (buffers destroyed eagerly).
    pub fn clear_stress(&mut self) {
        let calibration = self
            .batches
            .pop()
            .expect("calibration batch always present");
        for batch in self.batches.drain(..) {
            let lane = batch.lane;
            match batch.payload {
                DrawPayload::Quads(i) | DrawPayload::OrientedQuads(i) => i.buffer.destroy(),
                DrawPayload::Sprites { instances, .. } => {
                    if !Self::is_pooled_icon_lane(lane) {
                        instances.buffer.destroy();
                    }
                }
                DrawPayload::SpritesWithText { sprites, text, .. } => {
                    sprites.buffer.destroy();
                    if let Some(run) = text {
                        run.glyphs.buffer.destroy();
                    }
                }

                // T-0xx Phase 1D: a textured batch no longer carries its texture — the handle
                // is in `tex_lanes`, drained below, so the same textures are destroyed.
                DrawPayload::TexturedRect { .. } => {}
                DrawPayload::Lines(v) => v.vertices.destroy(),
                DrawPayload::Text(run) => run.glyphs.buffer.destroy(),
                DrawPayload::Indexed(m) => {
                    m.vertices.destroy();
                    m.indices.destroy();
                }
            }
        }
        for (_, tex) in self.tex_lanes.drain(..) {
            tex.texture.destroy();
        }
        self.batches.push(calibration);
        self.lane_pool.clear();
        self.stress_instances = 0;
        self.gen_ms = 0.0;
        self.upload_ms = 0.0;
    }
}
