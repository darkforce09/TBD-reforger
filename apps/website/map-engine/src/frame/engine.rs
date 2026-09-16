//! Role: `RenderEngine` — the GPU resources, the camera, and the persistent batch list.
//! Position: `frame` in the map engine.
//! Signals & state: every lifetime-scoped GPU handle, plus `batches: Vec<DrawBatch>`, which is
//! the frame packet's backing store.
//! Invariants: `batches` is persistent on purpose — Phase 2C rule 1 is that it is cleared and
//! refilled, never reallocated per frame.
//!
//! T-0xx Phase 2B.1: from `core/context/state.rs`. The three world facts it used to hold —
//! `EVERON_BOUNDS`, `INITIAL_TARGET`, `INITIAL_ZOOM` — went to `world/scene.rs`, beside
//! `ANCHOR`, which had already gone there in Phase 1D. `CLEAR_COLOR` stayed: it is a render
//! target's clear value, not a fact about Everon.

use crate::camera::ortho::state::OrthoCamera;
use crate::diagnostics::timing::gpu::GpuTimer;
use crate::frame::DrawBatch;
use crate::frame::LaneId;
use crate::frame::upload::text::TextAtlasGpu;
use crate::overlay::symbology::atlas::gpu::GlyphAtlasGpu;
use crate::overlay::symbology::instances::bridge_1::SlotAtlasGpu;
use crate::overlay::symbology::instances::bridge_1::SlotGpuBridge;
use crate::world::terrain::satellite::textures::PendingTex;
use crate::world::terrain::satellite::textures::TexLane;
use wasm_bindgen::prelude::*;
use website_graphics_engine::layout::QuadInstance;

/// Background clear — (51, 68, 85, 255)/255. The f64→f32→unorm8 chain error (< 1.2e-7) is four orders of magnitude under the unorm8 rounding margin (1/510 ≈ 2e-3), so readback bytes are forced exactly (plan §S4 margin argument).
pub(crate) const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 51.0 / 255.0,
    g: 68.0 / 255.0,
    b: 85.0 / 255.0,
    a: 1.0,
};

// T-0xx Phase 1D: `PipelineKind` lived here and was derived from `BatchPayload::kind()`. Its
// only reader was `stats()`, counting stress batches; a `DrawBatch` now carries a real
// `PipelineId` chosen by `core/pipeline/bindings.rs`, and `stats()` matches the payload
// variant directly. A second, parallel notion of "which pipeline" is exactly the kind of
// drift the packet boundary exists to prevent, so it is gone rather than moved.

/// Basemap mode.
#[derive(Clone, Copy)]
pub(crate) enum BasemapMode {
    /// Unified.
    Unified,

    /// Pyramid.
    Pyramid,

    /// Single.
    Single,

    /// Hillshade.
    Hillshade,
}

impl BasemapMode {
    /// As str.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unified => "unified",
            Self::Pyramid => "pyramid",
            Self::Single => "single-bitmap",
            Self::Hillshade => "hillshade",
        }
    }
}

impl BasemapMode {
    /// From u32.
    pub(crate) fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Pyramid,
            2 => Self::Single,
            3 => Self::Hillshade,
            _ => Self::Unified,
        }
    }
}

/// The render engine — owns the GPU device, the canvas surface, the camera, and the instance pools. Created via [`RenderEngine::create`]; freed from JS via `.free()` (effect-local, exactly once — lifecycle invariants I1–I7 in `WgpuCanvas.tsx`).
#[wasm_bindgen]
pub struct RenderEngine {
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

    /// Adapter max texture dimension 2d.
    pub(crate) adapter_max_texture_dimension_2d: u32,

    /// Shader.
    pub(crate) shader: wgpu::ShaderModule,

    /// Pipeline layout.
    pub(crate) pipeline_layout: wgpu::PipelineLayout,

    /// Bind group layout.
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,

    /// Surface pipeline.
    pub(crate) surface_pipeline: wgpu::RenderPipeline,

    /// Tex bind group layout.
    pub(crate) tex_bind_group_layout: wgpu::BindGroupLayout,

    /// Textured pipeline.
    pub(crate) textured_pipeline: wgpu::RenderPipeline,

    /// Forest density pipeline.
    pub(crate) forest_density_pipeline: wgpu::RenderPipeline,

    /// Line pipeline.
    pub(crate) line_pipeline: wgpu::RenderPipeline,

    /// Building pipeline.
    pub(crate) building_pipeline: wgpu::RenderPipeline,

    /// Polygon pipeline.
    pub(crate) polygon_pipeline: wgpu::RenderPipeline,

    /// Icon pipeline.
    pub(crate) icon_pipeline: wgpu::RenderPipeline,

    /// Text pipeline.
    pub(crate) text_pipeline: wgpu::RenderPipeline,

    /// Icon pipeline storage32.
    pub(crate) icon_pipeline_storage32: Option<wgpu::RenderPipeline>,

    /// Icon bind group layout.
    pub(crate) icon_bind_group_layout: wgpu::BindGroupLayout,

    /// Text bind group layout.
    pub(crate) text_bind_group_layout: wgpu::BindGroupLayout,

    /// Icon pipeline layout.
    pub(crate) icon_pipeline_layout: wgpu::PipelineLayout,

    /// Text pipeline layout.
    pub(crate) text_pipeline_layout: wgpu::PipelineLayout,

    /// Sampler.
    pub(crate) sampler: wgpu::Sampler,

    /// Icon sampler.
    pub(crate) icon_sampler: wgpu::Sampler,

    /// Density sampler.
    pub(crate) density_sampler: wgpu::Sampler,

    /// Uniform buf.
    pub(crate) uniform_buf: wgpu::Buffer,

    /// Bind group.
    pub(crate) bind_group: wgpu::BindGroup,

    /// Unit quad buf.
    pub(crate) unit_quad_buf: wgpu::Buffer,

    /// Calibration buf.
    pub(crate) calibration_buf: wgpu::Buffer,

    /// Camera.
    pub(crate) camera: OrthoCamera,

    /// Glyph atlas.
    pub(crate) glyph_atlas: Option<GlyphAtlasGpu>,

    /// Text atlas.
    pub(crate) text_atlas: Option<TextAtlasGpu>,

    /// Text labels drawn.
    pub(crate) text_labels_drawn: u32,

    /// Town labels drawn.
    pub(crate) town_labels_drawn: u32,

    /// Road labels drawn.
    pub(crate) road_labels_drawn: u32,

    /// Slot atlas.
    pub(crate) slot_atlas: Option<SlotAtlasGpu>,

    /// Slot bridge.
    pub(crate) slot_bridge: SlotGpuBridge,

    /// Batches.
    pub(crate) batches: Vec<DrawBatch>,

    /// The texture bookkeeping for every live `DrawPayload::TexturedRect` lane.
    ///
    /// T-0xx Phase 1D: a batch carries a `BindGroupId`, not a texture. The handle to destroy,
    /// the basemap mode, the tile count and the byte total are this crate's facts about a
    /// layer, so they stay here, keyed by the lane whose batch points at them.
    pub(crate) tex_lanes: Vec<(LaneId, TexLane)>,

    /// Pending.
    pub(crate) pending: [Option<PendingTex>; 2],

    /// Clear color.
    pub(crate) clear_color: wgpu::Color,

    /// Stress instances.
    pub(crate) stress_instances: u64,

    /// Staging.
    pub(crate) staging: Vec<QuadInstance>,

    /// Staging peak bytes.
    pub(crate) staging_peak_bytes: u64,

    /// Gen ms.
    pub(crate) gen_ms: f64,

    /// Upload ms.
    pub(crate) upload_ms: f64,

    /// Uniform bytes last frame.
    pub(crate) uniform_bytes_last_frame: u32,

    /// World chunks drawn.
    pub(crate) world_chunks_drawn: u32,

    /// Sea polygons.
    pub(crate) sea_polygons: u32,

    /// Landcover polygons.
    pub(crate) landcover_polygons: u32,

    /// Contour segments.
    pub(crate) contour_segments: u32,

    /// Road segments.
    pub(crate) road_segments: u32,

    /// Forest polygons.
    pub(crate) forest_polygons: u32,

    /// Forest outline segments.
    pub(crate) forest_outline_segments: u32,

    /// Forest density w.
    pub(crate) forest_density_w: u32,

    /// Forest density h.
    pub(crate) forest_density_h: u32,

    /// Forest bins ok.
    pub(crate) forest_bins_ok: u32,

    /// Forest outline segments stored.
    pub(crate) forest_outline_segments_stored: u32,

    /// Forest mode.
    pub(crate) forest_mode: String,

    /// Icon lane uploads.
    pub(crate) icon_lane_uploads: u64,

    /// Polygon lane uploads.
    pub(crate) polygon_lane_uploads: u64,

    /// Strip lane uploads.
    pub(crate) strip_lane_uploads: u64,

    /// Building uploads.
    pub(crate) building_uploads: u64,

    /// Text label uploads.
    pub(crate) text_label_uploads: u64,

    /// Render cpu ms last.
    pub(crate) render_cpu_ms_last: f64,

    /// Render cpu ms ema.
    pub(crate) render_cpu_ms_ema: f64,

    /// Timer.
    pub(crate) timer: Option<GpuTimer>,

    /// Damage.
    pub(crate) damage: crate::frame::damage::RenderDamage,

    /// Submitted last frame.
    pub(crate) submitted_last_frame: bool,

    /// Icon cull.
    pub(crate) icon_cull: Option<crate::frame::compute::IconComputeCull>,

    /// Tree icons 20.
    pub(crate) tree_icons_20: Vec<u8>,

    /// Compute cull trees.
    pub(crate) compute_cull_trees: bool,

    /// Lane pool.
    pub(crate) lane_pool: crate::frame::buffers::pool::LanePool,
}
