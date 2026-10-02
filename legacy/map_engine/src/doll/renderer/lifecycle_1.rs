//! Role: lifecycle 1.
//! Position: `doll/renderer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::doll::renderer::pack::pack_instances;
use crate::doll::renderer::pipeline::create_depth;
use crate::doll::renderer::pipeline::create_doll_pipeline;
use crate::frame::boot::instance_descriptor;

use wasm_bindgen::prelude::*;

/// Canonical instance stride value.
pub(crate) const INSTANCE_STRIDE: u64 = crate::doll::renderer::pack::INSTANCE_STRIDE as u64;

/// Canonical uniform size value.
pub(crate) const UNIFORM_SIZE: u64 = 80;

/// The doll engine — owns its GPU device, canvas surface, camera yaw, and region states. Created via [`DollEngine::create`]; freed from JS via `.free()` exactly once.
#[wasm_bindgen]
pub struct DollEngine {
    /// Device.
    pub(crate) device: wgpu::Device,

    /// Queue.
    pub(crate) queue: wgpu::Queue,

    /// Surface.
    pub(crate) surface: wgpu::Surface<'static>,

    /// Config.
    pub(crate) config: wgpu::SurfaceConfiguration,

    /// Backend kind.
    pub(crate) backend_kind: String,

    /// Pipeline.
    pub(crate) pipeline: wgpu::RenderPipeline,

    /// Shader.
    pub(crate) shader: wgpu::ShaderModule,

    /// Pipeline layout.
    pub(crate) pipeline_layout: wgpu::PipelineLayout,

    /// Bind group.
    pub(crate) bind_group: wgpu::BindGroup,

    /// Bind group layout.
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,

    /// Uniform buf.
    pub(crate) uniform_buf: wgpu::Buffer,

    /// Cube vbuf.
    pub(crate) cube_vbuf: wgpu::Buffer,

    /// Cube ibuf.
    pub(crate) cube_ibuf: wgpu::Buffer,

    /// Cube index count.
    pub(crate) cube_index_count: u32,

    /// Cyl vbuf.
    pub(crate) cyl_vbuf: wgpu::Buffer,

    /// Cyl ibuf.
    pub(crate) cyl_ibuf: wgpu::Buffer,

    /// Cyl index count.
    pub(crate) cyl_index_count: u32,

    /// Inst buf.
    pub(crate) inst_buf: wgpu::Buffer,

    /// N cube.
    pub(crate) n_cube: u32,

    /// N cyl.
    pub(crate) n_cyl: u32,

    /// Depth.
    pub(crate) depth: wgpu::TextureView,

    /// Css w.
    pub(crate) css_w: f64,

    /// Css h.
    pub(crate) css_h: f64,

    /// Yaw.
    pub(crate) yaw: f64,

    /// States.
    pub(crate) states: [u8; 14],

    /// Hover.
    pub(crate) hover: i32,

    /// Dirty.
    pub(crate) dirty: bool,

    /// Continuous.
    pub(crate) continuous: bool,
}

#[wasm_bindgen]
impl DollEngine {
    /// Async constructor. `canvas.width/height` must already hold the device-pixel backing size (JS owns the element; same contract as `RenderEngine.create`).
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
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/doll.wgsl").into()),
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
        let (cube_v, cube_i) = crate::doll::scene::mesh::mesh_cube();
        let (cube_vbuf, cube_ibuf, cube_index_count) = make_mesh("doll-cube", &cube_v, &cube_i);
        let (cyl_v, cyl_i) = crate::doll::scene::mesh::mesh_cylinder(16);
        let (cyl_vbuf, cyl_ibuf, cyl_index_count) = make_mesh("doll-cyl", &cyl_v, &cyl_i);

        let states = [crate::doll::scene::instances::STATE_EMPTY; 14];
        let streams = pack_instances(&states, -1);
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
            hover: -1,
            dirty: true,
            continuous: false,
        })
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// Backend.
    #[must_use]
    pub fn backend(&self) -> String {
        self.backend_kind.clone()
    }
}

#[wasm_bindgen]
impl DollEngine {
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
}

#[wasm_bindgen]
impl DollEngine {
    /// Rotate.
    pub fn rotate(&mut self, dx_px: f64) {
        self.yaw -= dx_px * 0.012;
        self.dirty = true;
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// Hover highlight: region index or -1. No-op when unchanged; repacks the tiny instance buffer through the lifted palette.
    pub fn set_hover(&mut self, region: i32) {
        if region == self.hover {
            return;
        }
        self.hover = region;
        let streams = pack_instances(&self.states, self.hover);
        self.queue.write_buffer(&self.inst_buf, 0, &streams.bytes);
        self.dirty = true;
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// The region's callout anchor in CSS px, `[x, y]` — empty when hidden/behind.
    #[must_use]
    pub fn anchor_px(&self, region: i32) -> Vec<f64> {
        crate::doll::interaction::picking::anchor_px(self.yaw, self.css_w, self.css_h, region)
            .map_or_else(Vec::new, |(x, y)| vec![x, y])
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// Push the 14 region states (RAIL order; values 0=empty, 1=equipped, 2=active).
    pub fn set_states(&mut self, states: &[u8]) -> Result<(), JsError> {
        if states.len() != 14 {
            return Err(JsError::new("set_states expects exactly 14 region bytes"));
        }
        self.states.copy_from_slice(states);
        let streams = pack_instances(&self.states, self.hover);
        self.queue.write_buffer(&self.inst_buf, 0, &streams.bytes);
        self.dirty = true;
        Ok(())
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// Nearest clickable region under a CSS pixel, or -1 (pure core math — same matrices the GPU draws with).
    #[must_use]
    pub fn pick_region(&self, x_css: f64, y_css: f64) -> i32 {
        crate::doll::interaction::picking::pick(self.yaw, self.css_w, self.css_h, x_css, y_css)
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// Mark dirty.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[wasm_bindgen]
impl DollEngine {
    /// DEV escape hatch (FPS HUD parity with the map engine).
    pub fn set_continuous_render(&mut self, on: bool) {
        self.continuous = on;
    }
}
