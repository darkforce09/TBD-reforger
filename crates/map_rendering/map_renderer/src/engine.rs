//! **Role:** `RenderEngine` — the canvas's GPU context, the shader, layouts and pipelines, the
//! camera, the typed layers and the persistent batch list — and `EngineHandle`, the slot a host
//! keeps it in.
//! **Position:** the map renderer's state; `boot.rs` creates it, the other modules of the crate add
//! its methods, the Mission Creator and the debug benches hold it in an `EngineHandle`.
//! **Signals & state:** every lifetime-scoped GPU handle, plus `batches: Vec<DrawBatch>`, which is
//! the frame packet's backing store.
//! **Invariants:** `batches` is persistent on purpose: it is cleared and refilled, never
//! reallocated per frame, and the packet's pipeline and bind-group tables are engine fields for
//! the same reason.
//!
//! The world facts the engine opens on (`EVERON_BOUNDS`, `INITIAL_TARGET`, `INITIAL_ZOOM`,
//! `ANCHOR`) are `map_coordinates::terrain_frames`'; `CLEAR_COLOR` is here because it is a render
//! target's clear value, not a fact about Everon. The world layers (buildings, forest, terrain
//! textures, the terrain line of sight overlay) and the symbology layers (slot symbology, glyph
//! atlas, icon lane cull) are fields the engine lends its lanes to.

use camera_math::ortho::state::OrthoCamera;
use gpu_device::{GpuContext, GpuTimer};
use gpu_frame::frame::{DrawBatch, TextAtlasGpu};
use render_primitives::draw::instances::QuadInstance;
use render_primitives::frame::ids::LaneId;
use renderer_core::frame_hook::FrameHooks;
use renderer_core::render_stats::RenderStats;
use symbology_layers_gpu::glyph_atlas_gpu::GlyphAtlasGpu;
use symbology_layers_gpu::icon_cull_gpu::IconCullGpu;
use symbology_layers_gpu::slot_symbology::SlotSymbologyGpu;
use world_layers_gpu::building_layer::BuildingLayerGpu;
use world_layers_gpu::forest_layer::ForestLayerGpu;
use world_layers_gpu::terrain_line_of_sight_overlay::TerrainLineOfSightOverlayGpu;
use world_layers_gpu::terrain_texture_layer::TerrainTextureLayerGpu;
use world_layers_gpu::textured_lane::TexLane;

/// Background clear — (51, 68, 85, 255)/255. The f64→f32→unorm8 chain error (< 1.2e-7) is four
/// orders of magnitude under the unorm8 rounding margin (1/510 ≈ 2e-3), so readback bytes are
/// forced exactly; the readback self-checks compare against it.
pub const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 51.0 / 255.0,
    g: 68.0 / 255.0,
    b: 85.0 / 255.0,
    a: 1.0,
};

/// A mounted engine, or an empty slot during initialization and teardown: the one spelling of the
/// engine slot that every holder, the Mission Creator and the frame pump included, shares.
pub type EngineHandle = std::rc::Rc<std::cell::RefCell<Option<RenderEngine>>>;

/// The render engine — owns the canvas's GPU context, the camera, the typed layers and the batch
/// list. Created via [`RenderEngine::create`]; its Rust host holds it in an [`EngineHandle`] and
/// drops it exactly once.
pub struct RenderEngine {
    /// The canvas's GPU: instance, surface, adapter facts, device, queue and surface
    /// configuration.
    pub(crate) gpu: GpuContext,

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

    /// The glyph atlas the world icon lanes sample.
    pub(crate) glyph_atlas: GlyphAtlasGpu,

    /// Text atlas.
    pub(crate) text_atlas: Option<TextAtlasGpu>,

    /// Text labels drawn.
    pub(crate) text_labels_drawn: u32,

    /// Town labels drawn.
    pub(crate) town_labels_drawn: u32,

    /// Road labels drawn.
    pub(crate) road_labels_drawn: u32,

    /// The building footprints, outlines and fence strips.
    pub(crate) buildings: BuildingLayerGpu,

    /// The forest density lane, the forest lane settings and the forest counters.
    pub(crate) forest: ForestLayerGpu,

    /// The satellite basemap and hillshade texture lanes.
    pub(crate) terrain_textures: TerrainTextureLayerGpu,

    /// The viewshed lane of the terrain line of sight overlay.
    pub(crate) terrain_line_of_sight_overlay: TerrainLineOfSightOverlayGpu,

    /// The slot symbology: the slot atlas, the slot bridge and the pooled sprite lanes.
    pub(crate) slot_symbology: SlotSymbologyGpu,

    /// Batches.
    pub(crate) batches: Vec<DrawBatch>,

    /// The frame packet's pipeline table — nine slots, refilled by `encode.rs`.
    ///
    /// Fixed-size and scene-independent, so it is not the expensive half of the refill rule —
    /// `batches` above is. It is a field because `encode.rs` is the module that sets the
    /// packet-building pattern, and a per-frame `Vec::with_capacity` inside it would be the one
    /// allocation the pattern exists to avoid.
    pub(crate) frame_pipelines: Vec<wgpu::RenderPipeline>,

    /// The frame packet's sparse bind-group table — `bindings::BIND_SLOTS` slots, refilled by
    /// `encode.rs`. Persistent for the same reason as `frame_pipelines`.
    pub(crate) frame_bind_groups: Vec<Option<wgpu::BindGroup>>,

    /// The texture record of every live `DrawPayload::TexturedRect` lane, keyed by the lane whose
    /// batch points at it: a batch carries a `BindGroupId`, not a texture, so the handle to
    /// destroy and the mode, tile and byte counters live in the world layers' `TexLane` beside it.
    pub(crate) tex_lanes: Vec<(LaneId, TexLane)>,

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

    /// Icon lane uploads.
    pub(crate) icon_lane_uploads: u64,

    /// Polygon lane uploads.
    pub(crate) polygon_lane_uploads: u64,

    /// Strip lane uploads.
    pub(crate) strip_lane_uploads: u64,

    /// Text label uploads.
    pub(crate) text_label_uploads: u64,

    /// The frame and lane counters: the CPU cost of the last submitted frame and its moving
    /// average, whether the last frame was submitted, and the sea, landcover, contour and road
    /// lanes' polygon and segment counts, keyed by lane.
    pub(crate) render_stats: RenderStats,

    /// Timer.
    pub(crate) timer: Option<GpuTimer>,

    /// Damage.
    pub(crate) damage: render_primitives::frame::damage::RenderDamage,

    /// The compute frustum cull of the icon lanes.
    pub(crate) icon_cull: IconCullGpu,

    /// The layers' per-frame callbacks, run with the engine lent to them.
    pub(crate) frame_hooks: FrameHooks<RenderEngine>,
}
