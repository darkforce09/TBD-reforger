//! The paper doll's byte-exact offscreen readback self-check.
//!
//! **Role:** [`PaperDollRenderer::self_check`] draws the doll flat-shaded into an 800 × 600
//! `Rgba8Unorm` target with fixed probe states, reads the target back, and compares five probe
//! pixels with the exact bytes the states' colours give: the background clear, the helmet front
//! (active), the plate front (equipped; the depth test's proof, since the backpack draws after the
//! plate but sits behind it), the rifle receiver (equipped, in front of the jacket) and the boot
//! front (empty).
//! **Position:** called by the Arsenal host's `window.__arsenalDoll.doll_self_check` hook, which
//! hands the result to the browser as a promise; the lit path stays operator-visual.
//! **Signals & state:** none of the renderer's: the check builds its own uniform, instance buffer,
//! pipeline, target and readback buffer and shares only the device, queue, program and meshes.
//! **Invariants:** the readback rows are 3328 bytes (800 × 4 padded to 256); the probe colours
//! are the unlit state colours (`params.x = 1` in the uniform), rounded to 8 bits; the readback
//! polls every 4 ms and gives up after 2000 polls.

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

use crate::doll_draw::{DollMeshes, InstanceDraw, doll_pass, draw_doll};
use crate::error::{Error, Result};
use crate::instance_packing::pack_instances;
use crate::pipeline::{DollProgram, create_depth};
use crate::renderer::PaperDollRenderer;
use camera_math::matrix4::transform_vector;
use camera_math::orbit::projection::{view_proj_gl, view_proj_wgpu};
use paper_doll_scene::soldier_parts::{
    CLEAR_COLOR, REGION_COUNT, REGION_KEYS, STATE_ACTIVE, STATE_EMPTY, STATE_EQUIPPED, state_color,
};

/// The probe target's width in pixels.
const PROBE_W: u32 = 800;

/// The probe target's height in pixels.
const PROBE_H: u32 = 600;

/// One readback row: `PROBE_W` × 4 bytes padded to wgpu's 256-byte row alignment.
const PADDED_BYTES_PER_ROW: u32 = 3328;

/// The longest the readback is polled for, in 4 ms polls.
const MAX_MAP_POLLS: u32 = 2000;

/// The GPU objects the self-check draws with, cloned out of the renderer so the check outlives
/// the borrow that started it.
struct SelfCheckInputs {
    device: wgpu::Device,
    queue: wgpu::Queue,
    program: DollProgram,
    meshes: DollMeshes,
    backend: &'static str,
}

impl PaperDollRenderer {
    /// Run the byte-exact offscreen self-check. Resolves to the JSON readout
    /// `{"backend","probes":[{"px","py","expect","got","pass","label"}…],"pass"}`.
    ///
    /// The returned future holds clones of the GPU handles, not the renderer, so the renderer
    /// stays usable while it runs.
    ///
    /// # Errors
    /// [`Error::ProbeMapTimeout`] or [`Error::ProbeMapFailed`] when the readback never maps.
    pub fn self_check(&self) -> impl Future<Output = Result<String>> + 'static {
        let inputs = SelfCheckInputs {
            device: self.gpu.device().clone(),
            queue: self.gpu.queue().clone(),
            program: self.program.clone(),
            meshes: self.meshes.clone(),
            backend: self.backend(),
        };
        async move { run_self_check(&inputs).await }
    }
}

/// The probe states: the helmet active, the plate and the rifle equipped, the rest empty.
fn self_check_states() -> [u8; REGION_COUNT] {
    let mut s = [STATE_EMPTY; REGION_COUNT];
    let idx = |key: &str| REGION_KEYS.iter().position(|k| *k == key).expect("key");
    s[idx("headCover")] = STATE_ACTIVE;
    s[idx("armoredVest")] = STATE_EQUIPPED;
    s[idx("primary")] = STATE_EQUIPPED;
    s
}

/// A linear colour as the four bytes an `Rgba8Unorm` target stores.
fn unorm8(c: [f32; 4]) -> [u8; 4] {
    core::array::from_fn(|i| (f64::from(c[i]) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// The probe target pixel a world point projects to at yaw 0, clamped into the target.
fn project_px(world: [f64; 3]) -> (u32, u32) {
    let vp = view_proj_gl(0.0, f64::from(PROBE_W), f64::from(PROBE_H));
    let ndc = transform_vector(&vp, [world[0], world[1], world[2], 1.0]);
    let x = ((ndc[0] + 1.0) / 2.0 * f64::from(PROBE_W)).round();
    let y = ((1.0 - ndc[1]) / 2.0 * f64::from(PROBE_H)).round();
    (
        x.clamp(0.0, f64::from(PROBE_W - 1)) as u32,
        y.clamp(0.0, f64::from(PROBE_H - 1)) as u32,
    )
}

/// Wait `ms` milliseconds on the browser's timer.
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Draw the probe scene, read it back and judge the probes.
async fn run_self_check(inputs: &SelfCheckInputs) -> Result<String> {
    let SelfCheckInputs {
        device,
        queue,
        program,
        meshes,
        backend,
    } = inputs;
    let states = self_check_states();
    let active = unorm8(state_color(STATE_ACTIVE, false));
    let equipped = unorm8(state_color(STATE_EQUIPPED, false));
    let empty = unorm8(state_color(STATE_EMPTY, false));
    let clear = unorm8(core::array::from_fn(|i| CLEAR_COLOR[i] as f32));

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

    let mvp = view_proj_wgpu(0.0, f64::from(PROBE_W), f64::from(PROBE_H));
    let mut uniform = [0f32; 20];
    uniform[..16].copy_from_slice(&mvp);
    uniform[16] = 1.0;
    let (uniform_buf, bind_group) = program.uniforms(device, "doll-probe-uniforms");
    queue.write_buffer(&uniform_buf, 0, bytemuck::cast_slice(&uniform));

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
    let pipeline = program.pipeline(device, wgpu::TextureFormat::Rgba8Unorm);

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
            meshes,
            &InstanceDraw {
                buffer: &inst_buf,
                cube_instances: streams.cube_instances,
                cylinder_instances: streams.cylinder_instances,
            },
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
    let mut polls = 0;
    while done.get() == 0 {
        let _ = device.poll(wgpu::PollType::Poll);
        sleep_ms(4).await;
        polls += 1;
        if polls > MAX_MAP_POLLS {
            return Err(Error::ProbeMapTimeout);
        }
    }
    if done.get() == 2 {
        return Err(Error::ProbeMapFailed);
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
