//! **Role:** the views the render diagnostics read the engine through: the device and surface, the
//! pipeline resources an offscreen probe builds its own pipelines from, the live scene a readback
//! redraws, and the stress pool the benchmark fills; plus the benchmark's door into the main pass
//! and the frame timer switch.
//! **Position:** the map renderer's public diagnostic surface; the render diagnostics (the
//! readback self-checks, `readback_rgba`, the benchmark, the stress pool) call these and name no
//! engine field, and the hosts call `disable_frame_timing`.
//! **Signals & state:** none of its own; each view borrows engine fields, shared except in the stress
//! pool, which borrows the batch list, the texture records, the slot lane pool, the staging buffer
//! and the stress counters mutably and apart.
//! **Invariants:** a read-only view never lets a diagnostic write the engine's frame tables, camera
//! uniform or batch list, so a readback can run beside the live render loop; only the stress pool
//! writes, and only the batch list, texture records, lane pool, staging buffer and stress
//! counters.

use crate::encode::bind_group_table;
use crate::engine::RenderEngine;
use camera_math::ortho::state::OrthoCamera;
use gpu_frame::frame::{DrawBatch, TextAtlasGpu};
use render_primitives::draw::instances::QuadInstance;
use render_primitives::frame::ids::LaneId;
use symbology_layers_gpu::glyph_atlas_gpu::GlyphAtlasGpu;
use symbology_layers_gpu::slot_symbology::SlotSymbologyGpu;
use world_layers_gpu::textured_lane::TexLane;

/// The device, queue and canvas surface a diagnostic draws with.
pub struct DiagnosticDevice<'a> {
    /// The engine's device.
    pub device: &'a wgpu::Device,

    /// The engine's queue.
    pub queue: &'a wgpu::Queue,

    /// The canvas surface's texture format.
    pub surface_format: wgpu::TextureFormat,

    /// The canvas surface's width in physical pixels.
    pub surface_width: u32,

    /// The canvas surface's height in physical pixels.
    pub surface_height: u32,

    /// The backend the browser gave the engine: `webgpu` or `webgl2`.
    pub backend_kind: &'a str,
}

/// The shader module, layouts, sampler and shared vertex buffers an offscreen probe builds its
/// own pipelines from.
pub struct DiagnosticPipelineResources<'a> {
    /// The engine's one shader module.
    pub shader: &'a wgpu::ShaderModule,

    /// The pipeline layout of the camera-only pipelines (quad, line, polygon, oriented quad).
    pub quad_pipeline_layout: &'a wgpu::PipelineLayout,

    /// The camera uniform's bind-group layout (group 0 of every pipeline).
    pub camera_bind_group_layout: &'a wgpu::BindGroupLayout,

    /// The texture-and-sampler bind-group layout of the textured pipelines (group 1).
    pub textured_bind_group_layout: &'a wgpu::BindGroupLayout,

    /// The icon atlas and icon uniform bind-group layout.
    pub icon_bind_group_layout: &'a wgpu::BindGroupLayout,

    /// The icon pipeline layout.
    pub icon_pipeline_layout: &'a wgpu::PipelineLayout,

    /// The text atlas and text uniform bind-group layout.
    pub text_bind_group_layout: &'a wgpu::BindGroupLayout,

    /// The text pipeline layout.
    pub text_pipeline_layout: &'a wgpu::PipelineLayout,

    /// The texture sampler of the textured pipelines.
    pub sampler: &'a wgpu::Sampler,

    /// The unit quad every instanced pipeline expands.
    pub unit_quad_buffer: &'a wgpu::Buffer,

    /// The two calibration quads of the engine's first batch.
    pub calibration_buffer: &'a wgpu::Buffer,
}

/// The live scene a readback redraws offscreen: the camera, the clear colour, the persistent batch
/// list and the sources of the packet's bind-group table.
pub struct DiagnosticScene<'a> {
    /// The live camera.
    pub camera: &'a OrthoCamera,

    /// The live clear colour.
    pub clear_color: wgpu::Color,

    /// The persistent batch list, in draw order.
    pub batches: &'a [DrawBatch],

    /// The live camera uniform buffer the main pass binds.
    pub camera_uniform_buffer: &'a wgpu::Buffer,

    /// The unit quad every instanced pipeline expands.
    pub unit_quad_buffer: &'a wgpu::Buffer,

    /// The glyph atlas the world icon lanes sample.
    glyph_atlas: &'a GlyphAtlasGpu,

    /// The shared text atlas, once uploaded.
    text_atlas: Option<&'a TextAtlasGpu>,

    /// The slot symbology, whose atlases the slot lanes sample.
    slot_symbology: &'a SlotSymbologyGpu,

    /// The texture record of every textured lane.
    textured_lanes: &'a [(LaneId, TexLane)],
}

impl DiagnosticScene<'_> {
    /// Fills `out` with the packet's bind-group table as the main pass builds it, with `camera`
    /// in the camera slot in place of the live camera bind group.
    pub fn fill_bind_group_table(
        &self,
        camera: &wgpu::BindGroup,
        out: &mut Vec<Option<wgpu::BindGroup>>,
    ) {
        bind_group_table(
            camera,
            self.glyph_atlas,
            self.text_atlas,
            self.slot_symbology,
            self.textured_lanes,
            out,
        );
    }
}

/// The engine borrowed apart for the stress pool: the batch list and everything a stress batch or
/// its removal touches.
pub struct DiagnosticStressPool<'a> {
    /// The engine's device.
    pub device: &'a wgpu::Device,

    /// The engine's queue.
    pub queue: &'a wgpu::Queue,

    /// The persistent batch list; the calibration batch is its last entry.
    pub batches: &'a mut Vec<DrawBatch>,

    /// The texture record of every textured lane.
    pub textured_lanes: &'a mut Vec<(LaneId, TexLane)>,

    /// The slot symbology, which owns the pooled sprite lanes' buffers.
    pub slot_symbology: &'a mut SlotSymbologyGpu,

    /// The staging buffer a stress chunk is generated into.
    pub staging: &'a mut Vec<QuadInstance>,

    /// The stress quads in the batch list.
    pub stress_instances: &'a mut u64,

    /// The staging buffer's largest capacity in bytes.
    pub staging_peak_bytes: &'a mut u64,

    /// The milliseconds the last seeding spent generating quads.
    pub generation_ms: &'a mut f64,

    /// The milliseconds the last seeding spent uploading quads.
    pub upload_ms: &'a mut f64,
}

impl RenderEngine {
    /// The device, queue and canvas surface, for a diagnostic that draws offscreen.
    #[must_use]
    pub fn diagnostic_device(&self) -> DiagnosticDevice<'_> {
        let (surface_width, surface_height) = self.gpu.surface_size();
        DiagnosticDevice {
            device: self.gpu.device(),
            queue: self.gpu.queue(),
            surface_format: self.gpu.surface_format(),
            surface_width,
            surface_height,
            backend_kind: self.gpu.backend_kind().as_str(),
        }
    }

    /// The shader module, layouts, sampler and shared vertex buffers, for a probe that builds its
    /// own pipelines.
    #[must_use]
    pub fn diagnostic_pipeline_resources(&self) -> DiagnosticPipelineResources<'_> {
        DiagnosticPipelineResources {
            shader: &self.shader,
            quad_pipeline_layout: &self.pipeline_layout,
            camera_bind_group_layout: &self.bind_group_layout,
            textured_bind_group_layout: &self.tex_bind_group_layout,
            icon_bind_group_layout: &self.icon_bind_group_layout,
            icon_pipeline_layout: &self.icon_pipeline_layout,
            text_bind_group_layout: &self.text_bind_group_layout,
            text_pipeline_layout: &self.text_pipeline_layout,
            sampler: &self.sampler,
            unit_quad_buffer: &self.unit_quad_buf,
            calibration_buffer: &self.calibration_buf,
        }
    }

    /// The live scene, for a readback that redraws it offscreen.
    #[must_use]
    pub fn diagnostic_scene(&self) -> DiagnosticScene<'_> {
        DiagnosticScene {
            camera: &self.camera,
            clear_color: self.clear_color,
            batches: &self.batches,
            camera_uniform_buffer: &self.uniform_buf,
            unit_quad_buffer: &self.unit_quad_buf,
            glyph_atlas: &self.glyph_atlas,
            text_atlas: self.text_atlas.as_ref(),
            slot_symbology: &self.slot_symbology,
            textured_lanes: &self.tex_lanes,
        }
    }

    /// The engine borrowed apart for the stress pool.
    pub fn diagnostic_stress_pool(&mut self) -> DiagnosticStressPool<'_> {
        let Self {
            gpu,
            batches,
            tex_lanes,
            slot_symbology,
            staging,
            stress_instances,
            staging_peak_bytes,
            gen_ms,
            upload_ms,
            ..
        } = self;
        DiagnosticStressPool {
            device: gpu.device(),
            queue: gpu.queue(),
            batches,
            textured_lanes: tex_lanes,
            slot_symbology,
            staging,
            stress_instances,
            staging_peak_bytes,
            generation_ms: gen_ms,
            upload_ms,
        }
    }

    /// Encodes the main pass into `view` exactly as a live frame does, without the GPU frame
    /// timer, for the frame benchmark. Returns whether the pass ran the compute cull.
    pub fn encode_untimed_main_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> bool {
        self.encode_main_pass(encoder, view, false)
    }

    /// Turns the GPU frame timer off for the engine's lifetime.
    pub fn disable_frame_timing(&mut self) {
        self.timer = None;
    }
}
