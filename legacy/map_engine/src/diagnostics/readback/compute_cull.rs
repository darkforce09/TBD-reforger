//! Role: compute cull.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::diagnostics::readback::scene::readback_sleep_ms;
use crate::frame::engine::RenderEngine;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl RenderEngine {
    /// Compute cull self check.
    pub fn compute_cull_self_check(&self) -> js_sys::Promise {
        let backend = self.backend_kind.clone();
        if self.backend_kind == "webgl2" {
            return js_sys::Promise::resolve(&JsValue::from_str(&format!(
                "{{\"backend\":\"{backend}\",\"skipped\":true,\"pass\":true}}"
            )));
        }
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            let mut src20 = Vec::with_capacity(512 * 20);
            let mut s: u32 = 0xC0FF_EE11;
            let mut unit = || {
                s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (s >> 8) as f32 / 16_777_216.0
            };
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

            let mut cull = crate::frame::compute::IconComputeCull::create(&device, &shader);
            let cpu = crate::frame::oracle::count_icons_in_frustum(&src20, frustum);
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
}
