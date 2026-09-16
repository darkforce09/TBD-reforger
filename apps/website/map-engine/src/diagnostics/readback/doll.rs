//! Role: doll.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::doll::renderer::lifecycle_1::DollEngine;
use crate::doll::renderer::lifecycle_1::UNIFORM_SIZE;
use crate::doll::renderer::pack::pack_instances;
use crate::doll::renderer::pass::doll_pass;
use crate::doll::renderer::pass::draw_doll;
use crate::doll::renderer::pipeline::create_depth;
use crate::doll::renderer::pipeline::create_doll_pipeline;

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Canonical probe w value.
pub(crate) const PROBE_W: u32 = 800;

/// Canonical probe h value.
pub(crate) const PROBE_H: u32 = 600;

/// Canonical padded bytes per row value.
pub(crate) const PADDED_BYTES_PER_ROW: u32 = 3328;

/// Self check states.
pub(crate) fn self_check_states() -> [u8; 14] {
    let mut s = [crate::doll::scene::instances::STATE_EMPTY; 14];
    let idx = |key: &str| {
        crate::doll::scene::instances::REGION_KEYS
            .iter()
            .position(|k| *k == key)
            .expect("key")
    };
    s[idx("headCover")] = crate::doll::scene::instances::STATE_ACTIVE;
    s[idx("armoredVest")] = crate::doll::scene::instances::STATE_EQUIPPED;
    s[idx("primary")] = crate::doll::scene::instances::STATE_EQUIPPED;
    s
}

/// Unorm8.
pub(crate) fn unorm8(c: [f32; 4]) -> [u8; 4] {
    core::array::from_fn(|i| (f64::from(c[i]) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Project px.
pub(crate) fn project_px(world: [f64; 3]) -> (u32, u32) {
    use crate::camera::math::glmat4::transform_vector;
    let vp =
        crate::camera::orbit::projection::view_proj_gl(0.0, f64::from(PROBE_W), f64::from(PROBE_H));
    let ndc = transform_vector(&vp, [world[0], world[1], world[2], 1.0]);
    let x = ((ndc[0] + 1.0) / 2.0 * f64::from(PROBE_W)).round();
    let y = ((1.0 - ndc[1]) / 2.0 * f64::from(PROBE_H)).round();
    (
        x.clamp(0.0, f64::from(PROBE_W - 1)) as u32,
        y.clamp(0.0, f64::from(PROBE_H - 1)) as u32,
    )
}

/// Sleep ms.
pub(crate) async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[wasm_bindgen]
impl DollEngine {
    /// Byte-exact offscreen self-check (flat colors — the lit path is operator-visual). Probes: background clear; helmet front (ACTIVE); plate front (EQUIPPED — the depth kill-shot: the backpack draws AFTER the plate but sits BEHIND it, so a missing depth test paints this probe backpack-EMPTY); rifle receiver (EQUIPPED, in front of the jacket band); boot front (EMPTY). Resolves to `{"backend","probes":[…],"pass"}`.
    pub fn doll_self_check(&self) -> js_sys::Promise {
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let pipeline_layout = self.pipeline_layout.clone();
        let bind_group_layout = self.bind_group_layout.clone();
        let cube_vbuf = self.cube_vbuf.clone();
        let cube_ibuf = self.cube_ibuf.clone();
        let cube_index_count = self.cube_index_count;
        let cyl_vbuf = self.cyl_vbuf.clone();
        let cyl_ibuf = self.cyl_ibuf.clone();
        let cyl_index_count = self.cyl_index_count;
        let backend = self.backend_kind.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            run_doll_self_check(
                &device,
                &queue,
                &shader,
                &pipeline_layout,
                &bind_group_layout,
                &cube_vbuf,
                &cube_ibuf,
                cube_index_count,
                &cyl_vbuf,
                &cyl_ibuf,
                cyl_index_count,
                &backend,
            )
            .await
            .map(JsValue::from)
            .map_err(|e| JsValue::from_str(&e))
        })
    }
}

/// Run doll self check.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) async fn run_doll_self_check(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    bind_group_layout: &wgpu::BindGroupLayout,
    cube_vbuf: &wgpu::Buffer,
    cube_ibuf: &wgpu::Buffer,
    cube_index_count: u32,
    cyl_vbuf: &wgpu::Buffer,
    cyl_ibuf: &wgpu::Buffer,
    cyl_index_count: u32,
    backend: &str,
) -> Result<String, String> {
    let states = self_check_states();
    let active = unorm8(crate::doll::scene::instances::state_color(
        crate::doll::scene::instances::STATE_ACTIVE,
        false,
    ));
    let equipped = unorm8(crate::doll::scene::instances::state_color(
        crate::doll::scene::instances::STATE_EQUIPPED,
        false,
    ));
    let empty = unorm8(crate::doll::scene::instances::state_color(
        crate::doll::scene::instances::STATE_EMPTY,
        false,
    ));
    let clear = unorm8(core::array::from_fn(|i| {
        crate::doll::scene::instances::CLEAR_COLOR[i] as f32
    }));

    let helmet = project_px([0.0, 1.82, 0.145]);

    let plate = project_px([0.19, 1.40, 0.141]);
    let rifle = project_px([0.0, 1.02, 0.235]);
    let boot = project_px([0.11, 0.08, 0.169]);
    let probes: Vec<(u32, u32, [u8; 4], &str)> = vec![
        (20, 20, clear, "background clear"),
        (helmet.0, helmet.1, active, "helmet front (ACTIVE)"),
        (
            plate.0,
            plate.1,
            equipped,
            "plate front (EQUIPPED, depth kill-shot vs launcher tube)",
        ),
        (
            rifle.0,
            rifle.1,
            equipped,
            "rifle receiver (EQUIPPED, in front of jacket)",
        ),
        (boot.0, boot.1, empty, "boot front (EMPTY)"),
    ];

    let mvp = crate::camera::orbit::projection::view_proj_wgpu(
        0.0,
        f64::from(PROBE_W),
        f64::from(PROBE_H),
    );
    let mut uniform = [0f32; 20];
    uniform[..16].copy_from_slice(&mvp);
    uniform[16] = 1.0;
    let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("doll-probe-uniforms"),
        size: UNIFORM_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buf, 0, bytemuck::cast_slice(&uniform));
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("doll-probe-uniforms"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });

    let streams = pack_instances(&states, -1);
    let inst_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("doll-probe-instances"),
        size: streams.bytes.len() as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&inst_buf, 0, &streams.bytes);

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("doll-probe-target"),
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
    let depth = create_depth(device, PROBE_W, PROBE_H);
    let pipeline = create_doll_pipeline(
        device,
        pipeline_layout,
        shader,
        wgpu::TextureFormat::Rgba8Unorm,
    );

    let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("doll-probe-readback"),
        size: u64::from(PADDED_BYTES_PER_ROW) * u64::from(PROBE_H),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("doll-probe"),
    });
    {
        let mut pass = doll_pass(&mut encoder, &view, &depth);
        draw_doll(
            &mut pass,
            &pipeline,
            &bind_group,
            cube_vbuf,
            cube_ibuf,
            cube_index_count,
            cyl_vbuf,
            cyl_ibuf,
            cyl_index_count,
            &inst_buf,
            streams.n_cube,
            streams.n_cyl,
        );
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
        sleep_ms(4).await;
        ticks += 1;
        if ticks > 2000 {
            return Err("doll-probe-map-timeout".to_owned());
        }
    }
    if done.get() == 2 {
        return Err("doll-probe-map-failed".to_owned());
    }

    let mut probes_json = Vec::with_capacity(probes.len());
    let mut all_pass = true;
    {
        let data = read_buf.slice(..).get_mapped_range();
        for &(px, py, expect, label) in &probes {
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
