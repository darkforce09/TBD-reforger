//! Role: boot — everything `RenderEngine::create` has to stand up before a frame exists.
//! Position: `frame` in the map engine.
//! Signals & state: the adapter, device, queue, surface config, and every pipeline, layout,
//! sampler and static buffer the engine holds for its lifetime.
//! Invariants: this runs once per canvas. Nothing here may run per frame.
//!
//! T-0xx Phase 2B.1: this is `core/context/device_2.rs`, with the 29 lines of
//! `core/context/device_1.rs` folded in below — two of that file's callers were this one and
//! itself, which is not a module. The pipeline and shader-module construction it calls into
//! belongs to `website-graphics-engine`; what is left here is the *order* in which a map
//! engine asks for them.

use crate::camera::ortho::state::OrthoCamera;
use crate::diagnostics::timing::gpu::GpuTimer;
use crate::frame::bindings;
use crate::frame::engine::CLEAR_COLOR;
use crate::frame::engine::RenderEngine;
use crate::overlay::lanes::LaneRole;
use crate::overlay::lanes::lane_id;
use crate::world::scene::EVERON_BOUNDS;
use crate::world::scene::INITIAL_TARGET;
use crate::world::scene::INITIAL_ZOOM;

use crate::frame::lifecycle::TEXT_UNIFORM_BYTES;
use crate::frame::pipelines::building::create_building_pipeline;
use crate::frame::pipelines::create_map_shader;
use crate::frame::pipelines::icon::create_icon_pipeline;
use crate::frame::pipelines::icon::create_icon_pipeline_storage32;
use crate::frame::pipelines::quad::create_quad_pipeline;
use crate::frame::pipelines::text::create_text_pipeline;
use crate::frame::pipelines::textured::create_forest_density_pipeline;
use crate::frame::pipelines::textured::create_textured_pipeline;
use crate::frame::pipelines::vector::create_line_pipeline;
use crate::frame::pipelines::vector::create_polygon_pipeline;
use crate::overlay::symbology::instances::bridge_1::SlotGpuBridge;
use crate::overlay::symbology::instances::lanes::ICON_UNIFORM_BYTES;
use wasm_bindgen::prelude::*;
use website_graphics_engine::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use website_graphics_engine::layout::UNIT_QUAD;

#[wasm_bindgen]
impl RenderEngine {
    /// Async constructor. `canvas.width/height` must already hold the device-pixel backing size (JS owns the canvas element; see `deviceSize` in `WgpuCanvas.tsx`).
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

        let info = adapter.get_info();
        let is_gl = info.backend == wgpu::Backend::Gl;
        let backend_kind = if is_gl { "webgl2" } else { "webgpu" }.to_owned();

        let base_limits = if is_gl {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        let want_timestamps = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);

        let adapter_max_texture_dimension_2d = adapter.limits().max_texture_dimension_2d;
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

        // T-0xx Phase 2B (Kind A): compiling the shader module is GPU resource creation, so
        // it moved to `website-graphics-engine` beside the pipeline constructors that consume
        // it. The WGSL source never leaves the crate that owns it.
        let shader = create_map_shader(&device);
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

        let tex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("basemap-texture"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let textured_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("textured-quad"),
                bind_group_layouts: &[Some(&bind_group_layout), Some(&tex_bind_group_layout)],
                immediate_size: 0,
            });
        let textured_pipeline =
            create_textured_pipeline(&device, &textured_pipeline_layout, &shader, format);
        let forest_density_pipeline =
            create_forest_density_pipeline(&device, &textured_pipeline_layout, &shader, format);
        let line_pipeline = create_line_pipeline(&device, &pipeline_layout, &shader, format);

        let building_pipeline =
            create_building_pipeline(&device, &pipeline_layout, &shader, format);

        let polygon_pipeline = create_polygon_pipeline(&device, &pipeline_layout, &shader, format);

        let icon_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("glyph-atlas"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,

                            min_binding_size: wgpu::BufferSize::new(ICON_UNIFORM_BYTES),
                        },
                        count: None,
                    },
                ],
            });
        let icon_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("icon-instanced"),
            bind_group_layouts: &[
                Some(&bind_group_layout),
                None,
                Some(&icon_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let icon_pipeline = create_icon_pipeline(&device, &icon_pipeline_layout, &shader, format);
        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("text-atlas"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(TEXT_UNIFORM_BYTES),
                        },
                        count: None,
                    },
                ],
            });
        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text-instanced"),
            bind_group_layouts: &[
                Some(&bind_group_layout),
                None,
                Some(&text_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let text_pipeline = create_text_pipeline(&device, &text_pipeline_layout, &shader, format);
        let (icon_pipeline_storage32, icon_cull) = if !is_gl {
            let p32 =
                create_icon_pipeline_storage32(&device, &icon_pipeline_layout, &shader, format);
            let cull = crate::frame::compute::IconComputeCull::create(&device, &shader);
            (Some(p32), Some(cull))
        } else {
            (None, None)
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("basemap-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let icon_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("glyph-atlas-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let density_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("forest-density-linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });

        use wgpu::util::DeviceExt;
        let unit_quad_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("unit-quad"),
            contents: bytemuck::cast_slice(&UNIT_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let calibration = crate::world::scene::calibration_instances();
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

        let calibration_batch = DrawBatch {
            lane: lane_id(LaneRole::Calibration),
            visible: true,
            pipeline: bindings::PIPE_QUAD,
            payload: DrawPayload::Quads(InstanceBuffer::whole(calibration_buf.clone(), 32, 2)),
        };

        let timer = want_timestamps.then(|| GpuTimer::new(&device, &queue));

        Ok(Self {
            device,
            queue,
            surface,
            config,
            backend_kind,
            adapter_max_texture_dimension_2d,
            shader,
            pipeline_layout,
            bind_group_layout,
            surface_pipeline,
            tex_bind_group_layout,
            textured_pipeline,
            forest_density_pipeline,
            line_pipeline,
            building_pipeline,
            polygon_pipeline,
            icon_pipeline,
            text_pipeline,
            icon_pipeline_storage32,
            icon_bind_group_layout,
            text_bind_group_layout,
            icon_pipeline_layout,
            text_pipeline_layout,
            sampler,
            icon_sampler,
            density_sampler,
            uniform_buf,
            bind_group,
            unit_quad_buf,
            calibration_buf,
            camera,
            glyph_atlas: None,
            text_atlas: None,
            text_labels_drawn: 0,
            town_labels_drawn: 0,
            road_labels_drawn: 0,
            slot_atlas: None,
            slot_bridge: SlotGpuBridge::default(),
            batches: vec![calibration_batch],
            tex_lanes: Vec::new(),
            pending: [None, None],
            clear_color: CLEAR_COLOR,
            stress_instances: 0,
            staging: Vec::new(),
            staging_peak_bytes: 0,
            gen_ms: 0.0,
            upload_ms: 0.0,
            uniform_bytes_last_frame: 0,
            world_chunks_drawn: 0,
            sea_polygons: 0,
            landcover_polygons: 0,
            forest_density_w: 0,
            forest_density_h: 0,
            forest_bins_ok: 0,
            forest_outline_segments_stored: 0,
            forest_mode: String::new(),
            contour_segments: 0,
            road_segments: 0,
            forest_polygons: 0,
            forest_outline_segments: 0,
            icon_lane_uploads: 0,
            polygon_lane_uploads: 0,
            strip_lane_uploads: 0,
            building_uploads: 0,
            text_label_uploads: 0,
            render_cpu_ms_last: 0.0,
            render_cpu_ms_ema: 0.0,
            timer,
            damage: crate::frame::damage::RenderDamage::new(),
            submitted_last_frame: false,
            icon_cull,
            tree_icons_20: Vec::new(),
            compute_cull_trees: !is_gl,
            lane_pool: crate::frame::buffers::pool::LanePool::new(),
        })
    }
}

// ─────────── from `core/context/device_1.rs` (T-0xx Phase 2B.1) ───────────

/// Start.
#[wasm_bindgen(start)]
pub(crate) fn start() {
    console_error_panic_hook::set_once();
}

/// Web display.
#[derive(Debug)]
pub(crate) struct WebDisplay;

impl wgpu::rwh::HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        Ok(wgpu::rwh::DisplayHandle::web())
    }
}

/// Instance descriptor.
pub(crate) fn instance_descriptor(backends: wgpu::Backends) -> wgpu::InstanceDescriptor {
    let mut desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(WebDisplay));
    desc.backends = backends;
    desc
}
