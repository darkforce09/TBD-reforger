//! `RenderEngine` — the wgpu spine (T-151 plan §S4): one instanced quad pipeline over a
//! `wgpu::SurfaceTarget::Canvas`, WebGPU primary with automatic WebGL2 fallback, driven by
//! the deck-parity `OrthoCamera` from `map-engine-core`.
//!
//! Hard constraints encoded here (see the plan for derivations):
//! - **Backend decision happens BEFORE the canvas is touched.** A canvas permanently commits
//!   to its first `getContext` kind, so same-canvas fallback is impossible; WebGPU
//!   availability is probed off-canvas (`new_instance_with_webgpu_detection` — `navigator.gpu`
//!   presence alone is not sufficient). Retry granularity is a fresh canvas element (JS I7).
//! - **WebGL2 requires the compatible surface before `request_adapter`** (wgpu 29 contract);
//!   the init order below works on both backends.
//! - **Non-sRGB surface format or hard error** (`srgb-only-surface`) — the byte-exact
//!   readback color contract must never silently pass through a transfer function.
//! - **Chunked instance pool**: buffers of ≤ [`CHUNK_CAPACITY`] × 32 B = 64 MiB, legal by
//!   construction under WebGPU's default 256 MiB `maxBufferSize`; upload streams through one
//!   reused staging `Vec` so peak wasm heap is one chunk at any instance count.
//! - **Navigation invariant**: a steady-state frame uploads exactly 64 bytes (the mvp
//!   uniform) — surfaced as `uniform_bytes_last_frame` in [`RenderEngine::stats`].

use std::cell::Cell;
use std::rc::Rc;

use map_engine_core::camera::OrthoCamera;
use wasm_bindgen::prelude::*;

use crate::scene::{self, ANCHOR, CHUNK_CAPACITY, QuadInstance, UNIT_QUAD};

/// Background clear — (51, 68, 85, 255)/255. The f64→f32→unorm8 chain error (< 1.2e-7) is
/// four orders of magnitude under the unorm8 rounding margin (1/510 ≈ 2e-3), so readback
/// bytes are forced exactly (plan §S4 margin argument).
pub(crate) const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 51.0 / 255.0,
    g: 68.0 / 255.0,
    b: 85.0 / 255.0,
    a: 1.0,
};

const INITIAL_TARGET: [f64; 2] = [6400.0, 6400.0];
const INITIAL_ZOOM: f64 = -2.0;
const EVERON_BOUNDS: [f64; 4] = [0.0, 0.0, 12_800.0, 12_800.0];

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// `Math.round` (half toward +∞) — must match the TS `deviceSize` helper bit-for-bit so the
/// canvas backing store and the surface configuration always agree.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// The web "display" for wgpu's display-handle plumbing. The GL (wgpu-core) path requires a
/// display handle from either the instance or the surface call; the safe
/// `SurfaceTarget::Canvas` path passes none at the surface, so the **instance** must carry
/// the browser display — `DisplayHandle::web()` (a unit handle; the WebGPU backend ignores
/// it). Without this, `create_surface` on the WebGL2 fallback fails with
/// `MissingDisplayHandle` (found empirically in V8a; wgpu 29 display-handle rework).
#[derive(Debug)]
struct WebDisplay;

impl wgpu::rwh::HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        Ok(wgpu::rwh::DisplayHandle::web())
    }
}

fn instance_descriptor(backends: wgpu::Backends) -> wgpu::InstanceDescriptor {
    let mut desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(WebDisplay));
    desc.backends = backends;
    desc
}

/// The render engine's draw list is an ordered `Vec<Batch>` (T-151.0 L7) — one entry per
/// instanced draw, iterated in order by `render()`. This is the seam the W1+ layer stack
/// (basemap, world objects, slots) hangs off; this slice ships exactly the pre-batch-list scene
/// (the stress pool, then the calibration scene on top), so the spike gates pass byte-identically.
#[derive(Clone, Copy)]
pub(crate) enum PipelineKind {
    /// The one instanced-quad pipeline ([`create_quad_pipeline`]): per-instance `QuadInstance`
    /// (min/max/color) over the shared unit-quad triangle strip.
    QuadInstanced,
}

/// One instanced draw: a per-instance vertex buffer of [`CHUNK_CAPACITY`]-bounded `QuadInstance`s
/// (≤ 64 MiB), how many instances to draw, and whether it is drawn this frame.
struct Batch {
    kind: PipelineKind,
    instances: wgpu::Buffer,
    count: u32,
    visible: bool,
}

/// GPU frame timing via `TIMESTAMP_QUERY` when the adapter offers it (plan §S4d: fps is a
/// readout, and where possible the GPU pass time is too). One 2-slot query set resolved per
/// frame; readback is async with an in-flight guard so mapping never overlaps.
struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve_buf: wgpu::Buffer,
    read_buf: wgpu::Buffer,
    period_ns: f32,
    last_ms: Rc<Cell<f64>>,
    has_sample: Rc<Cell<bool>>,
    in_flight: Rc<Cell<bool>>,
}

impl GpuTimer {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame-timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp-resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp-read"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            query_set,
            resolve_buf,
            read_buf,
            period_ns: queue.get_timestamp_period(),
            last_ms: Rc::new(Cell::new(0.0)),
            has_sample: Rc::new(Cell::new(false)),
            in_flight: Rc::new(Cell::new(false)),
        }
    }

    fn kick_readback(&self) {
        self.in_flight.set(true);
        let buf = self.read_buf.clone();
        let last = self.last_ms.clone();
        let has = self.has_sample.clone();
        let flag = self.in_flight.clone();
        let period = f64::from(self.period_ns);
        self.read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |res| {
                if res.is_ok() {
                    {
                        let data = buf.slice(..).get_mapped_range();
                        let t0 = u64::from_le_bytes(data[0..8].try_into().expect("8 bytes"));
                        let t1 = u64::from_le_bytes(data[8..16].try_into().expect("8 bytes"));
                        last.set(t1.saturating_sub(t0) as f64 * period / 1.0e6);
                        has.set(true);
                    }
                    buf.unmap();
                }
                flag.set(false);
            });
    }
}

/// Create the one instanced quad pipeline for a given color-target format (the surface
/// format for the live path; `Rgba8Unorm` for the probe path — plan §S4).
pub(crate) fn create_quad_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("quad-instanced"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                // Stream 0: unit quad (triangle strip), per-vertex.
                wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                },
                // Stream 1: QuadInstance { min, max, color } — 32 B, per-instance. Vertex
                // streams (not storage buffers) by design: WebGL2 has zero storage buffers,
                // and WebGPU compute can still write VERTEX|STORAGE buffers for future
                // cull/compaction (plan §20M).
                wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x4],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None, // opaque; draw order defines overlap (no depth attachment)
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// The render engine — owns the GPU device, the canvas surface, the camera, and the
/// instance pools. Created via [`RenderEngine::create`]; freed from JS via `.free()`
/// (effect-local, exactly once — lifecycle invariants I1–I7 in `WgpuCanvas.tsx`).
#[wasm_bindgen]
pub struct RenderEngine {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pub(crate) backend_kind: String,
    pub(crate) shader: wgpu::ShaderModule,
    pub(crate) pipeline_layout: wgpu::PipelineLayout,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
    surface_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub(crate) unit_quad_buf: wgpu::Buffer,
    pub(crate) calibration_buf: wgpu::Buffer,
    camera: OrthoCamera,
    /// Ordered draw list (T-151.0 L7): stress chunks first, the calibration batch always last.
    batches: Vec<Batch>,
    stress_instances: u64,
    staging: Vec<QuadInstance>,
    staging_peak_bytes: u64,
    gen_ms: f64,
    upload_ms: f64,
    uniform_bytes_last_frame: u32,
    timer: Option<GpuTimer>,
}

#[wasm_bindgen]
impl RenderEngine {
    /// Async constructor. `canvas.width/height` must already hold the device-pixel backing
    /// size (JS owns the canvas element; see `deviceSize` in `WgpuCanvas.tsx`).
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        force_webgl: bool,
    ) -> Result<RenderEngine, JsError> {
        let device_w = canvas.width();
        let device_h = canvas.height();
        if device_w == 0 || device_h == 0 {
            return Err(JsError::new(
                "canvas-zero-size: set canvas.width/height before RenderEngine.create",
            ));
        }

        // Backend decision BEFORE the canvas is touched (plan §S4 hard constraint).
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

        // WebGL2 REQUIRES the compatible surface at adapter request time (wgpu 29).
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..wgpu::RequestAdapterOptions::default()
            })
            .await
            .map_err(|e| JsError::new(&format!("no-adapter: {e}")))?;

        let info = adapter.get_info();
        let is_gl = info.backend == wgpu::Backend::Gl;
        let backend_kind = if is_gl { "webgl2" } else { "webgpu" }.to_owned();

        let base_limits = if is_gl {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        let want_timestamps = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("map-engine-render"),
                required_features: if want_timestamps {
                    wgpu::Features::TIMESTAMP_QUERY
                } else {
                    wgpu::Features::empty()
                },
                required_limits: base_limits.using_resolution(adapter.limits()),
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .map_err(|e| JsError::new(&format!("no-device: {e}")))?;

        // Non-sRGB surface format or hard error — the readback color contract.
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
            label: Some("quad-instanced"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad-instanced"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let surface_pipeline = create_quad_pipeline(&device, &pipeline_layout, &shader, format);

        use wgpu::util::DeviceExt;
        let unit_quad_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("unit-quad"),
            contents: bytemuck::cast_slice(&UNIT_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let calibration = scene::calibration_instances();
        let calibration_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("calibration-instances"),
            contents: bytemuck::cast_slice(&calibration),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-mvp"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-mvp"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        // Camera starts at the editor defaults; JS calls resize() with CSS dims right after
        // create (device dims are a placeholder until then).
        let mut camera = OrthoCamera::new(
            f64::from(device_w),
            f64::from(device_h),
            INITIAL_TARGET[0],
            INITIAL_TARGET[1],
            INITIAL_ZOOM,
        );
        camera.set_bounds(
            EVERON_BOUNDS[0],
            EVERON_BOUNDS[1],
            EVERON_BOUNDS[2],
            EVERON_BOUNDS[3],
        );

        // Permanent calibration batch — drawn last (on top of the stress pool), 2 instances (the
        // green square + the red orientation marker; `draw(0..4, 0..2)` in the pre-batch-list
        // engine). Cloning the buffer here keeps `calibration_buf` as a field for
        // `self_check`/`probe.rs`, which read it directly and stay untouched (L7).
        let calibration_batch = Batch {
            kind: PipelineKind::QuadInstanced,
            instances: calibration_buf.clone(),
            count: 2,
            visible: true,
        };

        let timer = want_timestamps.then(|| GpuTimer::new(&device, &queue));

        Ok(Self {
            device,
            queue,
            surface,
            config,
            backend_kind,
            shader,
            pipeline_layout,
            bind_group_layout,
            surface_pipeline,
            uniform_buf,
            bind_group,
            unit_quad_buf,
            calibration_buf,
            camera,
            batches: vec![calibration_batch],
            stress_instances: 0,
            staging: Vec::new(),
            staging_peak_bytes: 0,
            gen_ms: 0.0,
            upload_ms: 0.0,
            uniform_bytes_last_frame: 0,
            timer,
        })
    }

    /// Resize: camera in CSS px; surface at `round(css·dpr)` device px (must equal the
    /// canvas backing size JS just set via `deviceSize` — same rounding function).
    pub fn resize(&mut self, css_w: f64, css_h: f64, dpr: f64) -> Result<(), JsError> {
        if !(css_w > 0.0 && css_h > 0.0 && dpr > 0.0) {
            return Err(JsError::new("resize-nonpositive"));
        }
        self.camera.resize(css_w, css_h);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            self.config.width = (js_round(css_w * dpr).max(1.0)) as u32;
            self.config.height = (js_round(css_h * dpr).max(1.0)) as u32;
        }
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Set the full view state (clamped like the editor's view-state layer).
    pub fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.camera.set_view(target_x, target_y, zoom);
    }

    /// Drag-pan by CSS-pixel deltas (content follows cursor).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.camera.pan(dx_px, dy_px);
    }

    /// Cursor-anchored zoom (clamped to the view-state band).
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        self.camera.zoom_at(dz, cursor_x_px, cursor_y_px);
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target_x(&self) -> f64 {
        self.camera.target_x()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn target_y(&self) -> f64 {
        self.camera.target_y()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.camera.zoom()
    }

    /// `"webgpu"` or `"webgl2"` — the HUD + verify-gate readout.
    #[must_use]
    pub fn backend(&self) -> String {
        self.backend_kind.clone()
    }

    /// Visible world rect `[minX, minY, maxX, maxY]` (deck `getBounds` parity — the future
    /// culling primitive).
    #[must_use]
    pub fn visible_bounds(&self) -> Vec<f64> {
        self.camera.visible_world_rect().to_vec()
    }

    /// Render one frame to the canvas. Steady-state CPU→GPU traffic is exactly the 64-byte
    /// mvp uniform (the navigation invariant); instance data is static in GPU memory.
    pub fn render(&mut self) -> Result<(), JsError> {
        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&mvp));
        self.uniform_bytes_last_frame = 64;

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            Cst::Timeout | Cst::Occluded => return Ok(()), // skip this frame
            Cst::Outdated | Cst::Lost => {
                // Reconfigure + retry once — also self-heals the StrictMode canvas
                // reconfiguration race (plan §S5 I3).
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

        let take_timing = self.timer.as_ref().is_some_and(|t| !t.in_flight.get());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
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
                timestamp_writes: self.timer.as_ref().filter(|_| take_timing).map(|t| {
                    wgpu::RenderPassTimestampWrites {
                        query_set: &t.query_set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.surface_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.unit_quad_buf.slice(..));
            // Iterate the ordered draw list (L7): stress pool first (underneath), calibration
            // batch last (on top) — the pre-batch-list draw order, unchanged. One pipeline this
            // slice, so it is bound once above; the `kind` match is the seam for future pipelines.
            for batch in &self.batches {
                if !batch.visible {
                    continue;
                }
                match batch.kind {
                    PipelineKind::QuadInstanced => {
                        pass.set_vertex_buffer(1, batch.instances.slice(..));
                        pass.draw(0..4, 0..batch.count);
                    }
                }
            }
        }
        if take_timing && let Some(t) = &self.timer {
            encoder.resolve_query_set(&t.query_set, 0..2, &t.resolve_buf, 0);
            encoder.copy_buffer_to_buffer(&t.resolve_buf, 0, &t.read_buf, 0, 16);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        if take_timing && let Some(t) = &self.timer {
            t.kick_readback();
        }
        Ok(())
    }

    /// Stream `n` deterministic stress quads into the chunked pool (plan §S4d). Integer
    /// accounting: `stats().instances` afterwards equals exactly `n`.
    pub fn seed_stress(&mut self, n: u32, seed: u32) {
        self.clear_stress();
        // Insert stress batches before the trailing calibration batch (draw order stress→
        // calibration): pop it, append the freshly-generated chunks, restore it last.
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
            scene::stress_chunk_into(chunk_idx, count, seed, &mut self.staging);
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
            self.batches.push(Batch {
                kind: PipelineKind::QuadInstanced,
                instances: buffer,
                count: count as u32,
                visible: true,
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

    /// Drop the stress pool (buffers destroyed eagerly).
    pub fn clear_stress(&mut self) {
        // Drop every stress batch (all but the trailing calibration), destroying GPU buffers
        // eagerly; the permanent calibration batch is popped aside and restored (its buffer is a
        // clone of the kept `calibration_buf`, so it must not be destroyed).
        let calibration = self
            .batches
            .pop()
            .expect("calibration batch always present");
        for batch in self.batches.drain(..) {
            batch.instances.destroy();
        }
        self.batches.push(calibration);
        self.stress_instances = 0;
        self.gen_ms = 0.0;
        self.upload_ms = 0.0;
    }

    /// Machine-readable engine stats (every performance claim in the verify log is one of
    /// these numbers). `upload_ms` is CPU-side enqueue time for the staging→GPU copies;
    /// `gpu_frame_ms` is present only when `TIMESTAMP_QUERY` is available and sampled.
    #[must_use]
    pub fn stats(&self) -> String {
        // Stress batches are every draw-list entry except the trailing calibration batch (L7).
        let stress_count = self.batches.len() - 1;
        let stress_bytes: u64 = self.batches[..stress_count]
            .iter()
            .map(|b| u64::from(b.count) * 32)
            .sum();
        let gpu_bytes = stress_bytes + 64 /* uniform */ + 32 /* unit quad */ + 64 /* calibration */;
        let gpu_frame_ms = match &self.timer {
            Some(t) if t.has_sample.get() => format!("{:.3}", t.last_ms.get()),
            _ => "null".to_owned(),
        };
        format!(
            concat!(
                "{{\"backend\":\"{}\",\"instances\":{},\"chunks\":{},\"gpu_bytes\":{},",
                "\"staging_peak_bytes\":{},\"gen_ms\":{:.1},\"upload_ms\":{:.1},",
                "\"uniform_bytes_last_frame\":{},\"gpu_frame_ms\":{}}}"
            ),
            self.backend_kind,
            self.stress_instances,
            stress_count,
            gpu_bytes,
            self.staging_peak_bytes,
            self.gen_ms,
            self.upload_ms,
            self.uniform_bytes_last_frame,
            gpu_frame_ms,
        )
    }
}
