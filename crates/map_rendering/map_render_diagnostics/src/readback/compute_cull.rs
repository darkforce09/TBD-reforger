//! **Role:** the compute cull readback self-check: the GPU icon cull against the CPU oracle over 512
//! seeded icons.
//! **Position:** `readback` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes it as `window.__selfChecks.compute_cull_self_check`; it reads the engine only through
//! `map_renderer::diagnostic_accessors`.
//! **Signals & state:** none; each run builds and drops its own cull and buffers.
//! **Invariants:** the icon field and the frustum are seeded constants, independent of the live view;
//! the GPU count equals the CPU oracle's and falls strictly between 0 and 512.

use crate::readback::scene::readback_sleep_ms;
use deterministic_random::LinearCongruential32;
use map_renderer::engine::RenderEngine;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsValue;

/// Culls 512 seeded icons against a fixed rectangle on the GPU and compares the count with the CPU
/// oracle's. Resolves to `{"backend", "cpu", "gpu", "pass"}`, or `{"backend", "skipped": true,
/// "pass": true}` on WebGL2, which has no compute cull.
pub fn compute_cull_self_check(engine: &RenderEngine) -> js_sys::Promise {
    let gpu = engine.diagnostic_device();
    let resources = engine.diagnostic_pipeline_resources();
    let backend = gpu.backend_kind.to_owned();
    if gpu.backend_kind == "webgl2" {
        return js_sys::Promise::resolve(&JsValue::from_str(&format!(
            "{{\"backend\":\"{backend}\",\"skipped\":true,\"pass\":true}}"
        )));
    }
    let device = gpu.device.clone();
    let queue = gpu.queue.clone();
    let shader = resources.shader.clone();

    wasm_bindgen_futures::future_to_promise(async move {
        let mut src20 = Vec::with_capacity(512 * 20);
        let mut draws = LinearCongruential32::new(0xC0FF_EE11);
        let mut unit = || draws.next_unit();
        for _ in 0..512 {
            let x = unit() * 12_800.0 - 6_400.0;
            let y = unit() * 12_800.0 - 6_400.0;
            let size = 2.0 + unit() * 14.0;
            src20.extend_from_slice(&x.to_le_bytes());
            src20.extend_from_slice(&y.to_le_bytes());
            src20.extend_from_slice(&size.to_le_bytes());
            src20.extend_from_slice(&0_i16.to_le_bytes());
            src20.extend_from_slice(&0_u16.to_le_bytes());
            src20.extend_from_slice(&0xFF00_FF00_u32.to_le_bytes());
        }
        let frustum = [-1_234.5_f64, -987.25, 2_345.75, 1_876.5];

        let mut cull = gpu_frame::draw::cull::compute::IconComputeCull::create(&device, &shader);
        let cpu = render_primitives::draw::cull::oracle::count_icons_in_frustum(&src20, frustum);
        cull.upload_icons(&device, &queue, &src20);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("cull-self-check"),
        });
        cull.encode_cull(&mut encoder, &device, &queue, frustum);
        queue.submit(Some(encoder.finish()));
        let readback = cull
            .readback_buf(0)
            .expect("self-check lane uploaded")
            .clone();

        let done = Rc::new(Cell::new(0u8));
        {
            let done = done.clone();
            readback
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
                return Err(JsValue::from_str("cull-self-check: readback timeout"));
            }
        }
        if done.get() == 2 {
            return Err(JsValue::from_str("cull-self-check: readback map failed"));
        }
        let gpu = {
            let data = readback.slice(..).get_mapped_range();
            u32::from_le_bytes(data[0..4].try_into().expect("4 bytes"))
        };
        readback.unmap();

        let pass = gpu == cpu && cpu > 0 && cpu < 512;
        Ok(JsValue::from_str(&format!(
            "{{\"backend\":\"{backend}\",\"cpu\":{cpu},\"gpu\":{gpu},\"pass\":{pass}}}"
        )))
    })
}
