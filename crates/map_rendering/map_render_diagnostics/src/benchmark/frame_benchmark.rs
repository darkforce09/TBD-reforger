//! **Role:** the frame benchmark: `render_bench` times n offscreen frames of the live scene through
//! the engine's untimed main pass.
//! **Position:** `benchmark` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes it as `window.__editorBench`; it reaches the engine only through the views of
//! `map_renderer::diagnostic_accessors` and `encode_untimed_main_pass`.
//! **Signals & state:** writes the live camera uniform and the frame tables through the main pass;
//! the offscreen target is its own.
//! **Invariants:** n is clamped to 1..=20 000; the report waits at most 3 s for the queue to drain
//! and says whether it did.

use crate::readback::scene::readback_sleep_ms;
use map_coordinates::terrain_frames::ANCHOR;
use map_renderer::engine::RenderEngine;
use time_source::monotonic_ms;
use wasm_bindgen::JsValue;

/// Times `n` offscreen frames of the live scene (clamped to 1..=20 000); resolves to the JSON
/// report `{"n", "submit_wall_ms", "total_wall_ms", "cpu_avg_ms", "cpu_p95_ms", "cpu_max_ms",
/// "submit_avg_ms", "fps_equiv", "drained"}`.
pub fn render_bench(engine: &mut RenderEngine, n: u32) -> js_sys::Promise {
    let n = n.clamp(1, 20_000);
    let (device, queue, camera_uniform_buffer, mvp, _target, view) = {
        let gpu = engine.diagnostic_device();
        let scene = engine.diagnostic_scene();
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench-target"),
            size: wgpu::Extent3d {
                width: gpu.surface_width.max(1),
                height: gpu.surface_height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mvp = scene.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        (
            gpu.device.clone(),
            gpu.queue.clone(),
            scene.camera_uniform_buffer.clone(),
            mvp,
            target,
            view,
        )
    };

    let mut cpu_ms: Vec<f64> = Vec::with_capacity(n as usize);
    let mut submit_ms: Vec<f64> = Vec::with_capacity(n as usize);
    let t_start = monotonic_ms();
    for _ in 0..n {
        let f0 = monotonic_ms();
        queue.write_buffer(&camera_uniform_buffer, 0, bytemuck::cast_slice(&mvp));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bench-frame"),
        });
        engine.encode_untimed_main_pass(&mut encoder, &view);
        let cmd = encoder.finish();
        let f1 = monotonic_ms();
        queue.submit(Some(cmd));
        let f2 = monotonic_ms();
        cpu_ms.push(f1 - f0);
        submit_ms.push(f2 - f1);
    }
    let submit_wall_ms = monotonic_ms() - t_start;
    let submit_avg = submit_ms.iter().sum::<f64>() / submit_ms.len() as f64;
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let done = std::sync::Arc::clone(&done);
        queue.on_submitted_work_done(move || {
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
        let drain_start = monotonic_ms();
        while !done.load(std::sync::atomic::Ordering::SeqCst)
            && monotonic_ms() - drain_start < 3_000.0
        {
            readback_sleep_ms(4).await;
        }
        let drained = done.load(std::sync::atomic::Ordering::SeqCst);
        let total_wall_ms = monotonic_ms() - t_start;
        Ok(JsValue::from_str(&format!(
            "{{\"n\":{n},\"submit_wall_ms\":{submit_wall_ms:.3},\"total_wall_ms\":{total_wall_ms:.3},\"cpu_avg_ms\":{avg:.4},\"cpu_p95_ms\":{p95:.4},\"cpu_max_ms\":{max:.4},\"submit_avg_ms\":{submit_avg:.4},\"fps_equiv\":{cpu_fps_equiv:.1},\"drained\":{drained}}}"
        )))
    })
}
