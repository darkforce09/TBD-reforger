//! T-154 — `DollEngine`: the arsenal doll's own wgpu engine on its own canvas (second engine
//! instance beside the map `RenderEngine`; no shared state — see the multi-instance audit in
//! the T-154 spec). First 3D pass in the engine: perspective camera, `Depth32Float`
//! attachment, one instanced pipeline over unit cube/cylinder meshes. ALL policy (scene,
//! camera, colors, picking) lives in `map_engine_core::doll` — this module is the GPU shell.
//!
//! Lifecycle mirrors `RenderEngine`: JS sizes `canvas.width/height` BEFORE `create`, calls
//! `resize(css_w, css_h, dpr)` on layout changes, drives `render()` from rAF (damage-driven:
//! idle frames no-op), and `.free()`s exactly once on unmount.

use map_engine_core::doll;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::doll_pack::pack_instances;
use crate::engine::instance_descriptor;

const INSTANCE_STRIDE: u64 = crate::doll_pack::INSTANCE_STRIDE as u64;
const UNIFORM_SIZE: u64 = 80; // mvp (64) + params vec4 (16)

fn create_doll_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("doll3d"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_doll"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                },
                wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_STRIDE,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4,
                        6 => Float32x4
                    ],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_doll"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None, // opaque scene — state colors are deliberately alpha-1
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        // cull_mode None: 23 instances; winding-proof beats a micro-optimization.
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("doll-depth"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// The doll engine — owns its GPU device, canvas surface, camera yaw, and region states.
/// Created via [`DollEngine::create`]; freed from JS via `.free()` exactly once.
#[wasm_bindgen]
pub struct DollEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    backend_kind: String,
    pipeline: wgpu::RenderPipeline,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buf: wgpu::Buffer,
    cube_vbuf: wgpu::Buffer,
    cube_ibuf: wgpu::Buffer,
    cube_index_count: u32,
    cyl_vbuf: wgpu::Buffer,
    cyl_ibuf: wgpu::Buffer,
    cyl_index_count: u32,
    inst_buf: wgpu::Buffer,
    n_cube: u32,
    n_cyl: u32,
    depth: wgpu::TextureView,
    css_w: f64,
    css_h: f64,
    yaw: f64,
    states: [u8; 14],
    dirty: bool,
    continuous: bool,
}

#[wasm_bindgen]
impl DollEngine {
    /// Async constructor. `canvas.width/height` must already hold the device-pixel backing
    /// size (JS owns the element; same contract as `RenderEngine.create`).
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        force_webgl: bool,
    ) -> Result<DollEngine, JsError> {
        let device_w = canvas.width();
        let device_h = canvas.height();
        if device_w == 0 || device_h == 0 {
            return Err(JsError::new(
                "canvas-zero-size: set canvas.width/height before DollEngine.create",
            ));
        }

        let instance = if force_webgl {
            wgpu::Instance::new(instance_descriptor(wgpu::Backends::GL))
        } else {
            wgpu::util::new_instance_with_webgpu_detection(instance_descriptor(
                wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ))
            .await
        };
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsError::new(&format!("create-surface: {e}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..wgpu::RequestAdapterOptions::default()
            })
            .await
            .map_err(|e| JsError::new(&format!("no-adapter: {e}")))?;
        let is_gl = adapter.get_info().backend == wgpu::Backend::Gl;
        let backend_kind = if is_gl { "webgl2" } else { "webgpu" }.to_owned();
        let base_limits = if is_gl {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("doll-engine"),
                required_features: wgpu::Features::empty(),
                required_limits: base_limits.using_resolution(adapter.limits()),
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .map_err(|e| JsError::new(&format!("no-device: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .ok_or_else(|| JsError::new("srgb-only-surface: no non-sRGB surface format"))?;
        let mut config = surface
            .get_default_config(&adapter, device_w, device_h)
            .ok_or_else(|| JsError::new("surface-unsupported-by-adapter"))?;
        config.format = format;
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("doll3d"),
            source: wgpu::ShaderSource::Wgsl(include_str!("doll.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("doll-uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(UNIFORM_SIZE),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("doll3d"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = create_doll_pipeline(&device, &pipeline_layout, &shader, format);

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("doll-uniforms"),
            size: UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("doll-uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let make_mesh = |label: &str, verts: &[f32], idx: &[u16]| {
            let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (verts.len() * 4) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vbuf, 0, bytemuck::cast_slice(verts));
            let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (idx.len() * 2) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ibuf, 0, bytemuck::cast_slice(idx));
            (vbuf, ibuf, u32::try_from(idx.len()).expect("index count"))
        };
        let (cube_v, cube_i) = doll::mesh_cube();
        let (cube_vbuf, cube_ibuf, cube_index_count) = make_mesh("doll-cube", &cube_v, &cube_i);
        let (cyl_v, cyl_i) = doll::mesh_cylinder(16);
        let (cyl_vbuf, cyl_ibuf, cyl_index_count) = make_mesh("doll-cyl", &cyl_v, &cyl_i);

        let states = [doll::STATE_EMPTY; 14];
        let streams = pack_instances(&states);
        let inst_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("doll-instances"),
            size: streams.bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&inst_buf, 0, &streams.bytes);

        let depth = create_depth(&device, device_w, device_h);

        Ok(DollEngine {
            device,
            queue,
            surface,
            config,
            backend_kind,
            pipeline,
            shader,
            pipeline_layout,
            bind_group,
            bind_group_layout,
            uniform_buf,
            cube_vbuf,
            cube_ibuf,
            cube_index_count,
            cyl_vbuf,
            cyl_ibuf,
            cyl_index_count,
            inst_buf,
            n_cube: streams.n_cube,
            n_cyl: streams.n_cyl,
            depth,
            css_w: f64::from(device_w),
            css_h: f64::from(device_h),
            yaw: 0.0,
            states,
            dirty: true,
            continuous: false,
        })
    }

    #[must_use]
    pub fn backend(&self) -> String {
        self.backend_kind.clone()
    }

    /// Reconfigure for a new CSS size + device pixel ratio (recreates the depth buffer).
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) {
        let w = (css_w * dpr).round().max(1.0) as u32;
        let h = (css_h * dpr).round().max(1.0) as u32;
        self.css_w = css_w.max(1.0);
        self.css_h = css_h.max(1.0);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth = create_depth(&self.device, w, h);
        self.dirty = true;
    }

    /// Orbit the doll: horizontal drag delta in CSS px → yaw.
    pub fn rotate(&mut self, dx_px: f64) {
        self.yaw += dx_px * 0.012;
        self.dirty = true;
    }

    /// Push the 14 region states (RAIL order; values 0=empty, 1=equipped, 2=active).
    pub fn set_states(&mut self, states: &[u8]) -> Result<(), JsError> {
        if states.len() != 14 {
            return Err(JsError::new("set_states expects exactly 14 region bytes"));
        }
        self.states.copy_from_slice(states);
        let streams = pack_instances(&self.states);
        self.queue.write_buffer(&self.inst_buf, 0, &streams.bytes);
        self.dirty = true;
        Ok(())
    }

    /// Nearest clickable region under a CSS pixel, or -1 (pure core math — same matrices
    /// the GPU draws with).
    #[must_use]
    pub fn pick_region(&self, x_css: f64, y_css: f64) -> i32 {
        doll::pick(self.yaw, self.css_w, self.css_h, x_css, y_css)
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// DEV escape hatch (FPS HUD parity with the map engine).
    pub fn set_continuous_render(&mut self, on: bool) {
        self.continuous = on;
    }

    /// Damage-driven frame: no-ops (no surface acquire) when nothing changed.
    pub fn render(&mut self) -> Result<(), JsError> {
        if !self.dirty && !self.continuous {
            return Ok(());
        }
        let mvp = doll::view_proj_wgpu(self.yaw, self.css_w, self.css_h);
        let mut uniform = [0f32; 20];
        uniform[..16].copy_from_slice(&mvp);
        // params = 0 → lit path.
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&uniform));

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            Cst::Timeout | Cst::Occluded => {
                // Surface not ready - keep dirty so the next rAF retries.
                return Ok(());
            }
            Cst::Outdated | Cst::Lost => {
                // Reconfigure + retry once (same self-heal as the map engine).
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    Cst::Success(f) | Cst::Suboptimal(f) => f,
                    other => {
                        return Err(JsError::new(&format!(
                            "surface-acquire-after-reconfigure: {other:?}"
                        )));
                    }
                }
            }
            other => return Err(JsError::new(&format!("surface-acquire: {other:?}"))),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("doll"),
            });
        {
            let mut pass = doll_pass(&mut encoder, &view, &self.depth);
            draw_doll(
                &mut pass,
                &self.pipeline,
                &self.bind_group,
                &self.cube_vbuf,
                &self.cube_ibuf,
                self.cube_index_count,
                &self.cyl_vbuf,
                &self.cyl_ibuf,
                self.cyl_index_count,
                &self.inst_buf,
                self.n_cube,
                self.n_cyl,
            );
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.dirty = false;
        Ok(())
    }
}

fn doll_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    color: &'a wgpu::TextureView,
    depth: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("doll"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: doll::CLEAR_COLOR[0],
                    g: doll::CLEAR_COLOR[1],
                    b: doll::CLEAR_COLOR[2],
                    a: doll::CLEAR_COLOR[3],
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_doll(
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    cube_vbuf: &wgpu::Buffer,
    cube_ibuf: &wgpu::Buffer,
    cube_index_count: u32,
    cyl_vbuf: &wgpu::Buffer,
    cyl_ibuf: &wgpu::Buffer,
    cyl_index_count: u32,
    inst_buf: &wgpu::Buffer,
    n_cube: u32,
    n_cyl: u32,
) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    // Cubes: instances [0, n_cube) at buffer offset 0.
    pass.set_vertex_buffer(0, cube_vbuf.slice(..));
    pass.set_vertex_buffer(1, inst_buf.slice(..u64::from(n_cube) * INSTANCE_STRIDE));
    pass.set_index_buffer(cube_ibuf.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..cube_index_count, 0, 0..n_cube);
    // Cylinders: buffer-offset slice instead of first_instance (absent on WebGL2).
    if n_cyl > 0 {
        pass.set_vertex_buffer(0, cyl_vbuf.slice(..));
        pass.set_vertex_buffer(1, inst_buf.slice(u64::from(n_cube) * INSTANCE_STRIDE..));
        pass.set_index_buffer(cyl_ibuf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..cyl_index_count, 0, 0..n_cyl);
    }
}

// ── self-check (offscreen, byte-exact — probe.rs pattern) ─────────────────────

const PROBE_W: u32 = 800;
const PROBE_H: u32 = 600;
const PADDED_BYTES_PER_ROW: u32 = 3328; // align(800·4, 256)

/// Fixed self-check states: helmet ACTIVE, plate + rifle EQUIPPED, rest EMPTY.
fn self_check_states() -> [u8; 14] {
    let mut s = [doll::STATE_EMPTY; 14];
    let idx = |key: &str| {
        doll::REGION_KEYS
            .iter()
            .position(|k| *k == key)
            .expect("key")
    };
    s[idx("headCover")] = doll::STATE_ACTIVE;
    s[idx("armoredVest")] = doll::STATE_EQUIPPED;
    s[idx("primary")] = doll::STATE_EQUIPPED;
    s
}

fn unorm8(c: [f32; 4]) -> [u8; 4] {
    core::array::from_fn(|i| (f64::from(c[i]) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// Project a world point to probe pixel coordinates at the fixed self-check camera.
fn project_px(world: [f64; 3]) -> (u32, u32) {
    use map_engine_core::camera::glmat4::transform_vector;
    let vp = doll::view_proj_gl(0.0, f64::from(PROBE_W), f64::from(PROBE_H));
    let ndc = transform_vector(&vp, [world[0], world[1], world[2], 1.0]);
    let x = ((ndc[0] + 1.0) / 2.0 * f64::from(PROBE_W)).round();
    let y = ((1.0 - ndc[1]) / 2.0 * f64::from(PROBE_H)).round();
    (
        x.clamp(0.0, f64::from(PROBE_W - 1)) as u32,
        y.clamp(0.0, f64::from(PROBE_H - 1)) as u32,
    )
}

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
impl DollEngine {
    /// Byte-exact offscreen self-check (flat colors — the lit path is operator-visual).
    /// Probes: background clear; helmet front (ACTIVE); plate front (EQUIPPED — the depth
    /// kill-shot: the backpack draws AFTER the plate but sits BEHIND it, so a missing depth
    /// test paints this probe backpack-EMPTY); rifle receiver (EQUIPPED, in front of the
    /// jacket band); boot front (EMPTY). Resolves to `{"backend","probes":[…],"pass"}`.
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn run_doll_self_check(
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
    let active = unorm8(doll::state_color(doll::STATE_ACTIVE));
    let equipped = unorm8(doll::state_color(doll::STATE_EQUIPPED));
    let empty = unorm8(doll::state_color(doll::STATE_EMPTY));
    let clear = unorm8(core::array::from_fn(|i| doll::CLEAR_COLOR[i] as f32));

    // (world point or screen px, expected bytes, label)
    let helmet = project_px([0.0, 1.82, 0.145]);
    // Plate-only strip: x beyond the chest rig's half-width (0.15) but inside the plate's
    // (0.23), at a height the launcher tube (the LAST-drawn instance, z = -0.31) crosses
    // behind - a missing depth test paints this probe tube-EMPTY.
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

    // Flat-shaded uniform at the fixed camera.
    let mvp = doll::view_proj_wgpu(0.0, f64::from(PROBE_W), f64::from(PROBE_H));
    let mut uniform = [0f32; 20];
    uniform[..16].copy_from_slice(&mvp);
    uniform[16] = 1.0; // params.x = flat
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

    let streams = pack_instances(&states);
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
