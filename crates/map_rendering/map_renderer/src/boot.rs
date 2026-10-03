//! **Role:** boot — everything `RenderEngine::create` stands up before a frame exists: the
//! canvas's GPU context, then the shader module, layouts, pipelines, samplers, static buffers,
//! camera, typed layers and the first batch.
//! **Position:** the map renderer; the Mission Creator and the debug benches call
//! `RenderEngine::create` once per canvas.
//! **Signals & state:** the GPU context and every pipeline, layout, sampler and static buffer the
//! engine holds for its lifetime.
//! **Invariants:** this runs once per canvas and nothing here runs per frame. The GPU context is
//! `gpu_device`'s, created with the device label `map-engine-render` and timestamp queries when
//! the adapter has them; the compute cull and its storage-buffer icon pipeline exist only on
//! WebGPU, never on WebGL2. The pipeline and shader-module constructors are `gpu_frame`'s; what is
//! here is the order in which the map renderer asks for them.

use crate::bindings;
use crate::calibration_scene::calibration_instances;
use crate::engine::{CLEAR_COLOR, RenderEngine};
use crate::error::Result;
use crate::typed_layers::symbology_layers::SlotSymbologyCameraSync;
use camera_math::ortho::state::OrthoCamera;
use gpu_device::{GpuContext, GpuTimer};
use gpu_frame::draw::cull::compute::IconComputeCull;
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use gpu_frame::pipeline::create_render_shader;
use gpu_frame::pipeline::icon::{create_icon_pipeline, create_icon_pipeline_storage32};
use gpu_frame::pipeline::oriented_quad::create_oriented_quad_pipeline;
use gpu_frame::pipeline::quad::create_quad_pipeline;
use gpu_frame::pipeline::text::create_text_pipeline;
use gpu_frame::pipeline::textured::{create_density_pipeline, create_textured_pipeline};
use gpu_frame::pipeline::vector::{create_line_pipeline, create_polygon_pipeline};
use map_coordinates::terrain_frames::{EVERON_BOUNDS, INITIAL_TARGET, INITIAL_ZOOM};
use map_draw_lanes::lane_roles::{LaneRole, lane_id};
use render_primitives::draw::instances::UNIT_QUAD;
use render_primitives::text::pack::TEXT_UNIFORM_BYTES;
use renderer_core::frame_hook::FrameHooks;
use renderer_core::packet_bindings;
use renderer_core::render_stats::RenderStats;
use symbology_layers_gpu::glyph_atlas_gpu::GlyphAtlasGpu;
use symbology_layers_gpu::icon_cull_gpu::IconCullGpu;
use symbology_layers_gpu::icon_uniforms::ICON_UNIFORM_BYTES;
use symbology_layers_gpu::slot_symbology::SlotSymbologyGpu;
use world_layers_gpu::building_layer::BuildingLayerGpu;
use world_layers_gpu::forest_layer::ForestLayerGpu;
use world_layers_gpu::terrain_line_of_sight_overlay::TerrainLineOfSightOverlayGpu;
use world_layers_gpu::terrain_texture_layer::TerrainTextureLayerGpu;

/// The device label the map canvas's GPU context is created with.
const DEVICE_LABEL: &str = "map-engine-render";

impl RenderEngine {
    /// Create the engine on `canvas`, whose `width` and `height` already hold the device-pixel
    /// backing size the host set. `force_webgl` skips WebGPU detection and uses WebGL2 alone.
    ///
    /// # Errors
    /// [`crate::Error::Gpu`] carrying the GPU context's refusal (`canvas-zero-size`,
    /// `create-surface`, `no-adapter`, `no-device`, `srgb-only-surface` or
    /// `surface-unsupported-by-adapter`).
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        force_webgl: bool,
    ) -> Result<RenderEngine> {
        let gpu = GpuContext::create(canvas, force_webgl, DEVICE_LABEL, true).await?;
        let device = gpu.device();
        let queue = gpu.queue();
        let format = gpu.surface_format();
        let is_gl = gpu.is_gl();
        let (device_w, device_h) = gpu.surface_size();

        let shader = create_render_shader(device);
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
        let surface_pipeline = create_quad_pipeline(device, &pipeline_layout, &shader, format);

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
            create_textured_pipeline(device, &textured_pipeline_layout, &shader, format);
        let forest_density_pipeline =
            create_density_pipeline(device, &textured_pipeline_layout, &shader, format);
        let line_pipeline = create_line_pipeline(device, &pipeline_layout, &shader, format);

        let building_pipeline =
            create_oriented_quad_pipeline(device, &pipeline_layout, &shader, format);

        let polygon_pipeline = create_polygon_pipeline(device, &pipeline_layout, &shader, format);

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
        let icon_pipeline = create_icon_pipeline(device, &icon_pipeline_layout, &shader, format);
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
        let text_pipeline = create_text_pipeline(device, &text_pipeline_layout, &shader, format);
        let (icon_pipeline_storage32, icon_cull) = if !is_gl {
            let p32 =
                create_icon_pipeline_storage32(device, &icon_pipeline_layout, &shader, format);
            let cull = IconComputeCull::create(device, &shader);
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
        let calibration = calibration_instances();
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
            pipeline: packet_bindings::PIPE_QUAD,
            payload: DrawPayload::Quads(InstanceBuffer::whole(calibration_buf.clone(), 32, 2)),
        };

        let timer = gpu
            .timestamps_enabled()
            .then(|| GpuTimer::new(device, queue));
        let icon_cull = IconCullGpu::new(icon_cull, icon_pipeline_storage32.is_some(), !is_gl);
        let glyph_atlas = GlyphAtlasGpu::new(icon_bind_group_layout.clone(), icon_sampler.clone());
        let slot_symbology =
            SlotSymbologyGpu::new(icon_bind_group_layout.clone(), icon_sampler.clone());
        let terrain_textures =
            TerrainTextureLayerGpu::new(tex_bind_group_layout.clone(), sampler.clone());
        let forest = ForestLayerGpu::new(tex_bind_group_layout.clone(), density_sampler.clone());
        let terrain_line_of_sight_overlay =
            TerrainLineOfSightOverlayGpu::new(tex_bind_group_layout.clone(), density_sampler);
        let mut frame_hooks = FrameHooks::new();
        frame_hooks.register(Box::new(SlotSymbologyCameraSync));

        Ok(Self {
            gpu,
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
            uniform_buf,
            bind_group,
            unit_quad_buf,
            calibration_buf,
            camera,
            glyph_atlas,
            text_atlas: None,
            text_labels_drawn: 0,
            town_labels_drawn: 0,
            road_labels_drawn: 0,
            buildings: BuildingLayerGpu::new(),
            forest,
            terrain_textures,
            terrain_line_of_sight_overlay,
            slot_symbology,
            batches: vec![calibration_batch],
            // Sized once here so the first frame is the only one that can allocate them;
            // `encode_main_pass` clears and refills them and never reallocates.
            frame_pipelines: Vec::with_capacity(packet_bindings::PIPELINE_SLOTS),
            frame_bind_groups: vec![None; bindings::BIND_SLOTS],
            tex_lanes: Vec::new(),
            clear_color: CLEAR_COLOR,
            stress_instances: 0,
            staging: Vec::new(),
            staging_peak_bytes: 0,
            gen_ms: 0.0,
            upload_ms: 0.0,
            uniform_bytes_last_frame: 0,
            icon_lane_uploads: 0,
            polygon_lane_uploads: 0,
            strip_lane_uploads: 0,
            text_label_uploads: 0,
            render_stats: RenderStats::new(),
            timer,
            damage: render_primitives::frame::damage::RenderDamage::new(),
            icon_cull,
            frame_hooks,
        })
    }
}
