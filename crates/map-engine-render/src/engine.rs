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
use std::collections::HashSet;
use std::rc::Rc;

use map_engine_core::camera::OrthoCamera;
use map_engine_core::slots_gpu::{
    self, DragGpuPhase, SLOT_ICON_STRIDE, classify_drag_transition, hide_slot_row_patch,
    pack_cluster_instances, pack_drag_overlay, pack_selection_only, pack_slot_instances,
    selected_mask,
};
use wasm_bindgen::prelude::*;

use crate::lanes;
use crate::scene::{self, ANCHOR, CHUNK_CAPACITY, QuadInstance, UNIT_QUAD};

/// T-151.7.3 — slot/selection/drag/cluster GPU policy state on the engine (not in TS).
#[derive(Default)]
struct SlotGpuBridge {
    atlas_ready: bool,
    last_ids: Vec<String>,
    last_xy: Vec<f32>,
    selected_ids: HashSet<String>,
    last_cluster_mode: bool,
    drag_active: bool,
    drag_ids: Vec<String>,
    slots_lane_selection_only: bool,
}

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
/// The render pipeline a batch draws through (T-151.1 L1). `QuadInstanced` is the T-151.0
/// stress/calibration spine; `TexturedQuad` is the W1 basemap + hillshade raster; `Polyline` is
/// the W1 procedural grid. The variant for a batch is derived from its [`BatchPayload`] via
/// [`BatchPayload::kind`], keeping the enum and the payload in lockstep.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PipelineKind {
    QuadInstanced,
    TexturedQuad,
    Polyline,
    /// W3 world-building fill — rotated OBB quads (`scene::BuildingInstance`, `vs_building`).
    BuildingQuad,
    /// W4 triangulated polygon fills (sea, landcover, forest, marquee, road strips).
    PolygonFill,
    /// W5 atlas-sampled icon instances (`scene::IconInstance`, 20 B).
    IconInstanced,
}

/// The 3-way map style's satellite-field opacity mode + the derived basemap render mode, surfaced
/// in `stats().basemap_mode`. Mirrors `useTerrainBasemapLayer.ts` `BasemapRenderMode`.
#[derive(Clone, Copy)]
enum BasemapMode {
    Unified,
    Pyramid,
    Single,
    Hillshade,
}

impl BasemapMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unified => "unified",
            Self::Pyramid => "pyramid",
            Self::Single => "single-bitmap",
            Self::Hillshade => "hillshade",
        }
    }

    fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Pyramid,
            2 => Self::Single,
            3 => Self::Hillshade,
            _ => Self::Unified,
        }
    }
}

// T-151.11.1: `LaneRole` / `lane_order` / `lane_role_from_u32` moved to the pure
// `crate::draw_order` module so the ordering contract is natively unit-tested
// (this module is wasm32-gated — see lib.rs).
use crate::draw_order::{LaneRole, lane_order, lane_role_from_u32};

/// A textured-quad lane (basemap or hillshade): one GPU texture (mip chain for unified, single
/// level for pyramid/hillshade) sampled trilinearly over a 1-instance world-rect quad; `color`
/// carries the opacity tint. `mode`/`tiles`/`bytes` feed the additive `stats()` fields.
struct TexLane {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    instances: wgpu::Buffer,
    mode: BasemapMode,
    tiles: u32,
    bytes: u64,
}

/// A polyline lane (the grid / contours / outlines): a `LineList` vertex buffer of
/// [`lanes::LineVertex`].
struct LineLane {
    verts: wgpu::Buffer,
    count: u32,
}

/// A polygon-fill lane (sea / landcover / forest / marquee / road strips): indexed triangle list
/// of `LineVertex`-compatible verts (pos + color, 24 B).
struct PolyLane {
    verts: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    /// Stats: polygon/segment count reported by the uploader (not triangle count).
    #[allow(dead_code)]
    item_count: u32,
}

/// Icon uniform buffer size: 28×vec4 UV (448) + drag_delta/px_to_m/pad (16) = 464 B.
const ICON_UNIFORM_BYTES: u64 = 464;

/// Shared glyph atlas GPU state (T-151.5 / T-151.6): texture + uniform (UV + params) + bind group.
struct GlyphAtlasGpu {
    texture: wgpu::Texture,
    /// Writable uniform: UV[28] + drag_delta.xy + px_to_m + pad.
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bytes: u64,
}

/// Slot atlas (T-151.6): ring+disc texture with separate base/drag bind groups so drag_delta
/// does not move the base slot lane (write-before-pass; illegal mid-pass).
struct SlotAtlasGpu {
    texture: wgpu::Texture,
    base_uniform_buf: wgpu::Buffer,
    drag_uniform_buf: wgpu::Buffer,
    base_bind_group: wgpu::BindGroup,
    drag_bind_group: wgpu::BindGroup,
    bytes: u64,
    px_to_m: f32,
    drag_delta: [f32; 2],
}

/// A texture being assembled between `tex_layer_begin` and `tex_layer_commit` — blocks/tiles are
/// uploaded incrementally, then finalized into a [`TexLane`].
struct PendingTex {
    texture: wgpu::Texture,
    world_min: [f64; 2],
    world_max: [f64; 2],
    mode: BasemapMode,
    tiles: u32,
    bytes: u64,
}

/// The per-batch payload. `Instanced` is the T-151.0 QuadInstance stream (stress/calibration);
/// `Textured`/`Lines` are the W1 lanes. `kind()` maps each to its [`PipelineKind`].
enum BatchPayload {
    Instanced {
        instances: wgpu::Buffer,
        count: u32,
    },
    Textured(TexLane),
    Lines(LineLane),
    /// W3 world-building fill: `scene::BuildingInstance` stream (40 B), drawn `draw(0..4, 0..count)`.
    BuildingInstanced {
        instances: wgpu::Buffer,
        count: u32,
    },
    /// W4 polygon fill / wide polyline strips (indexed triangle list).
    Polygon(PolyLane),
    /// W5 atlas icon instances (`scene::IconInstance`, 20 B each).
    IconInstanced {
        instances: wgpu::Buffer,
        count: u32,
    },
}

impl BatchPayload {
    fn kind(&self) -> PipelineKind {
        match self {
            Self::Instanced { .. } => PipelineKind::QuadInstanced,
            Self::Textured(_) => PipelineKind::TexturedQuad,
            Self::Lines(_) => PipelineKind::Polyline,
            Self::BuildingInstanced { .. } => PipelineKind::BuildingQuad,
            Self::Polygon(_) => PipelineKind::PolygonFill,
            Self::IconInstanced { .. } => PipelineKind::IconInstanced,
        }
    }
}

/// One entry in the ordered draw list (T-151.0 L7 → T-151.1 L1): its role (draw order + lane
/// identity), whether it draws this frame, and its pipeline-specific payload.
struct Batch {
    role: LaneRole,
    visible: bool,
    payload: BatchPayload,
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

/// The W1 textured-quad pipeline (basemap + hillshade — T-151.1 L1). Same two vertex streams as
/// [`create_quad_pipeline`] (unit quad + `QuadInstance`), but samples `group(1)` texture/sampler
/// (`vs_textured`/`fs_textured`) and **alpha-blends** (`op·src + (1−op)·dst`, non-premultiplied)
/// so the `color` tint's alpha is the satOpacity / hillshadeOpacity. `layout` must carry
/// `[camera-uniform (group 0), texture+sampler (group 1)]`.
pub(crate) fn create_textured_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("textured-quad"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_textured"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x4],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_textured"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

/// The W1 polyline pipeline (the grid — T-151.1 L1). One per-vertex stream of `lanes::LineVertex`
/// (`@location(0) pos`, `@location(1) color`) drawn as a `LineList` (native 1 px), alpha-blended
/// (the grid palette carries alpha). `layout` is the camera-uniform-only layout (group 0).
pub(crate) fn create_line_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("polyline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_line"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_line"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// W5 icon-instanced pipeline — unit quad + 20 B `IconInstance`, samples group-2 atlas.
pub(crate) fn create_icon_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("icon-instanced"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_icon"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 8,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Sint16,
                            offset: 12,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint16,
                            offset: 14,
                            shader_location: 4,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 16,
                            shader_location: 5,
                        },
                    ],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_icon"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

/// Icon pipeline for compute-culled VERTEX|STORAGE instances (32 B `IconStorage` stride).
pub(crate) fn create_icon_pipeline_storage32(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("icon-instanced-storage32"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_icon"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32,
                            offset: 8,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Sint32,
                            offset: 12,
                            shader_location: 3,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 16,
                            shader_location: 4,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Uint32,
                            offset: 20,
                            shader_location: 5,
                        },
                    ],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_icon"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

/// W4 polygon-fill pipeline — same vertex layout as the hairline polyline (`LineVertex` 24 B)
/// but drawn as an **indexed triangle list** with alpha blending (sea/landcover/forest/roads).
pub(crate) fn create_polygon_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("polygon-fill"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_line"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_line"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// The W3 world-building fill pipeline (rotated OBB quads — T-151.3 L8). Unit-quad stream 0 +
/// `scene::BuildingInstance` stream 1 (40 B: center, half, basis, color), `vs_building`/
/// `fs_building`, **alpha-blended** (Deck's semi-transparent footprint fill). `layout` is the
/// camera-uniform-only layout (group 0), same as the quad/line pipelines.
pub(crate) fn create_building_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("world-building"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_building"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 8,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                },
                // scene::BuildingInstance: center, half, basis (cos,sin), color — 40 B.
                wgpu::VertexBufferLayout {
                    array_stride: 40,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Float32x4],
                },
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_building"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

/// Compute-culled tree draw handles for **in-order** emission inside [`draw_batches`]
/// (T-151.11.1 / audit X-01): the `draw_indirect` fires at the `WorldTrees` order slot, not
/// after the whole list — trees must never paint over slots/grid/marquee.
struct IndirectTrees<'a> {
    pipeline: &'a wgpu::RenderPipeline,
    atlas_bind: &'a wgpu::BindGroup,
    instances: &'a wgpu::Buffer,
    indirect: &'a wgpu::Buffer,
}

/// Draw the ordered batch list into `pass` (shared by the live `render()` and the offscreen
/// readback path — T-151.1 L1/L10). Group 0 (the camera mvp) is compatible across all pipeline
/// layouts, so it is bound once. The pipelines are passed in because the live path uses the
/// surface-format pipelines and the readback path rebuilds them at `Rgba8Unorm`.
/// `indirect_trees` (WebGPU compute-cull path) is emitted exactly once, in `WorldTrees` order.
#[allow(clippy::too_many_arguments)]
fn draw_batches<'a>(
    batches: &'a [Batch],
    pass: &mut wgpu::RenderPass<'a>,
    bind_group: &'a wgpu::BindGroup,
    unit_quad_buf: &'a wgpu::Buffer,
    quad_pipeline: &'a wgpu::RenderPipeline,
    textured_pipeline: &'a wgpu::RenderPipeline,
    line_pipeline: &'a wgpu::RenderPipeline,
    building_pipeline: &'a wgpu::RenderPipeline,
    polygon_pipeline: &'a wgpu::RenderPipeline,
    icon_pipeline: &'a wgpu::RenderPipeline,
    glyph_atlas_bind: Option<&'a wgpu::BindGroup>,
    slot_base_bind: Option<&'a wgpu::BindGroup>,
    slot_drag_bind: Option<&'a wgpu::BindGroup>,
    indirect_trees: Option<IndirectTrees<'a>>,
) {
    let mut trees_emitted = false;
    let emit_trees = |pass: &mut wgpu::RenderPass<'a>, emitted: &mut bool| {
        if *emitted {
            return;
        }
        *emitted = true;
        if let Some(t) = &indirect_trees {
            pass.set_pipeline(t.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_bind_group(2, t.atlas_bind, &[]);
            pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
            pass.set_vertex_buffer(1, t.instances.slice(..));
            pass.draw_indirect(t.indirect, 0);
        }
    };
    // group 0 (camera mvp) is set after each `set_pipeline` — its layout is identical across all
    // pipelines, but binding it per-batch (pipeline → groups → buffers → draw) is the always-valid
    // order on both the WebGPU and WebGL2 backends.
    for batch in batches {
        // The compute-culled tree lane has no Batch entry (upload removes it on the compute
        // path); slot its indirect draw in the moment we pass the WorldTrees order position.
        if lane_order(batch.role) > lane_order(LaneRole::WorldTrees) {
            emit_trees(pass, &mut trees_emitted);
        }
        if !batch.visible {
            continue;
        }
        match &batch.payload {
            BatchPayload::Instanced { instances, count } => {
                pass.set_pipeline(quad_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.draw(0..4, 0..*count);
            }
            BatchPayload::Textured(l) => {
                pass.set_pipeline(textured_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_bind_group(1, &l.bind_group, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, l.instances.slice(..));
                pass.draw(0..4, 0..1);
            }
            BatchPayload::Lines(l) => {
                pass.set_pipeline(line_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, l.verts.slice(..));
                pass.draw(0..l.count, 0..1);
            }
            BatchPayload::BuildingInstanced { instances, count } => {
                pass.set_pipeline(building_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.draw(0..4, 0..*count);
            }
            BatchPayload::Polygon(l) => {
                pass.set_pipeline(polygon_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_vertex_buffer(0, l.verts.slice(..));
                pass.set_index_buffer(l.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..l.index_count, 0, 0..1);
            }
            BatchPayload::IconInstanced { instances, count } => {
                let atlas_bg = match batch.role {
                    LaneRole::SlotDrag => slot_drag_bind,
                    LaneRole::Slots | LaneRole::Clusters => slot_base_bind,
                    _ => glyph_atlas_bind,
                };
                let Some(atlas_bg) = atlas_bg else {
                    continue;
                };
                pass.set_pipeline(icon_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_bind_group(2, atlas_bg, &[]);
                pass.set_vertex_buffer(0, unit_quad_buf.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                pass.draw(0..4, 0..*count);
            }
        }
    }
    // No batch ordered after WorldTrees (or an empty list): emit the culled trees at the tail.
    emit_trees(pass, &mut trees_emitted);
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
    /// W1 textured/line pipeline stack (T-151.1). Additive — the T-151.0 `surface_pipeline` +
    /// `self_check` quad path are untouched.
    tex_bind_group_layout: wgpu::BindGroupLayout,
    textured_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    /// W3 world-building fill pipeline (rotated OBB quads). Additive — the outline reuses
    /// `line_pipeline` unchanged.
    building_pipeline: wgpu::RenderPipeline,
    /// W4 polygon-fill / wide-polyline strip pipeline (indexed triangle list).
    polygon_pipeline: wgpu::RenderPipeline,
    /// W5 icon-instanced pipeline (atlas group 2).
    icon_pipeline: wgpu::RenderPipeline,
    /// T-151.8.1: 32 B storage-stride icon pipeline for compute-culled draw_indirect (WebGPU).
    icon_pipeline_storage32: Option<wgpu::RenderPipeline>,
    icon_bind_group_layout: wgpu::BindGroupLayout,
    icon_pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    /// Nearest sampler for crisp glyph atlas pixels.
    icon_sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub(crate) unit_quad_buf: wgpu::Buffer,
    pub(crate) calibration_buf: wgpu::Buffer,
    camera: OrthoCamera,
    /// Shared glyph atlas (None until `upload_glyph_atlas`).
    glyph_atlas: Option<GlyphAtlasGpu>,
    /// Dedicated slot/cluster atlas (None until `upload_slot_atlas` / `ensure_slot_atlas`).
    slot_atlas: Option<SlotAtlasGpu>,
    /// T-151.7.3 slot GPU policy (selection/drag/cluster) — TS is dumb UI only.
    slot_bridge: SlotGpuBridge,
    /// Ordered draw list (T-151.0 L7 → T-151.1 L1): editor lanes (basemap → hillshade → grid,
    /// `lane_order`) or the spike batches (stress chunks then the calibration batch last).
    batches: Vec<Batch>,
    /// Textures being assembled between `tex_layer_begin` and `tex_layer_commit` (index = role:
    /// 0 basemap, 1 hillshade).
    pending: [Option<PendingTex>; 2],
    /// Editor clear color — defaults to [`CLEAR_COLOR`]; the map-style paper tint underlay
    /// (T-151.1 L8) sets it via `set_clear_color`. The spike never changes it.
    clear_color: wgpu::Color,
    stress_instances: u64,
    staging: Vec<QuadInstance>,
    staging_peak_bytes: u64,
    gen_ms: f64,
    upload_ms: f64,
    uniform_bytes_last_frame: u32,
    /// Resident chunks contributing to the current world-building lanes (additive `stats()` field;
    /// set by `upload_world_buildings`, cleared by `clear_world_buildings`).
    world_chunks_drawn: u32,
    /// W4 additive stats counters (set by upload APIs; L14).
    sea_polygons: u32,
    landcover_polygons: u32,
    contour_segments: u32,
    road_segments: u32,
    forest_polygons: u32,
    forest_outline_segments: u32,
    timer: Option<GpuTimer>,
    /// T-151.8 damage-driven render skip.
    damage: crate::damage::RenderDamage,
    /// Last `render()` submitted GPU work (Class R idle gate).
    submitted_last_frame: bool,
    /// Density heatmap visible flag (stats).
    density_heatmap: bool,
    /// T-151.8.1 WebGPU compute cull (None on WebGL2).
    icon_cull: Option<crate::icon_cull_gpu::IconComputeCull>,
    /// Last tree-glyph 20 B upload (CPU oracle + compute src).
    tree_icons_20: Vec<u8>,
    /// When true (WebGPU), WorldTrees draw via compute cull + draw_indirect.
    compute_cull_trees: bool,
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

        // W1 textured/line pipeline stack (T-151.1). `bind_group_layout` (group 0, camera uniform)
        // is reused unchanged; the textured pipeline adds group 1 (sampled texture + sampler). The
        // line pipeline needs only the camera uniform, so it shares `pipeline_layout`.
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
        let line_pipeline = create_line_pipeline(&device, &pipeline_layout, &shader, format);
        // W3 world-building fill pipeline (group-0 camera only, like the quad/line pipelines).
        let building_pipeline =
            create_building_pipeline(&device, &pipeline_layout, &shader, format);
        // W4 polygon fill (same group-0 camera layout; indexed triangle list).
        let polygon_pipeline = create_polygon_pipeline(&device, &pipeline_layout, &shader, format);
        // W5 icon atlas: group 0 camera + group 2 (tex + samp + UV uniform).
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
                            // 28 × vec4 UV + drag_delta/px_to_m/pad = 464 B (T-151.6)
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
        let (icon_pipeline_storage32, icon_cull) = if !is_gl {
            let p32 =
                create_icon_pipeline_storage32(&device, &icon_pipeline_layout, &shader, format);
            let cull = crate::icon_cull_gpu::IconComputeCull::create(&device, &shader);
            (Some(p32), Some(cull))
        } else {
            (None, None)
        };
        // Trilinear + clamp-to-edge — the unified-satellite sampler contract (`satelliteUnified.ts`).
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
        // create (device dims are a placeholder until then). Bounds default to Everon so the
        // spike page clamps sensibly with zero calls; the editor MUST call `set_camera_bounds`
        // with its terrain dims right after create (T-151.11.2 / audit X-02 — Arland is 4096²).
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
            role: LaneRole::Calibration,
            visible: true,
            payload: BatchPayload::Instanced {
                instances: calibration_buf.clone(),
                count: 2,
            },
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
            tex_bind_group_layout,
            textured_pipeline,
            line_pipeline,
            building_pipeline,
            polygon_pipeline,
            icon_pipeline,
            icon_pipeline_storage32,
            icon_bind_group_layout,
            icon_pipeline_layout,
            sampler,
            icon_sampler,
            uniform_buf,
            bind_group,
            unit_quad_buf,
            calibration_buf,
            camera,
            glyph_atlas: None,
            slot_atlas: None,
            slot_bridge: SlotGpuBridge::default(),
            batches: vec![calibration_batch],
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
            contour_segments: 0,
            road_segments: 0,
            forest_polygons: 0,
            forest_outline_segments: 0,
            timer,
            damage: crate::damage::RenderDamage::new(),
            submitted_last_frame: false,
            density_heatmap: false,
            icon_cull,
            tree_icons_20: Vec::new(),
            compute_cull_trees: !is_gl,
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
        self.damage.mark();
        Ok(())
    }

    /// Set the full view state (clamped like the editor's view-state layer).
    pub fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        self.camera.set_view(target_x, target_y, zoom);
        self.damage.mark();
    }

    /// Drag-pan by CSS-pixel deltas (content follows cursor). Live caller: the spike page's
    /// pointer pan (`WgpuCanvas.tsx`); the editor pans via `set_view` (audit X-05 corrected —
    /// this is not dead code).
    pub fn pan(&mut self, dx_px: f64, dy_px: f64) {
        self.camera.pan(dx_px, dy_px);
        self.damage.mark();
    }

    /// Set the camera's target clamp rect to the mounted terrain (T-151.11.2 / audit X-02).
    /// The editor calls this with `[0, 0, terrain.width, terrain.height]` right after create —
    /// the create-time default is Everon 12,800² and is WRONG for Arland (4,096²). The TS
    /// `clampViewState` mirror stays as the synchronous backstop; this keeps the engine-side
    /// clamp (the SoT for `set_view`/`zoom_at`) truthful on every terrain.
    pub fn set_camera_bounds(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.camera.set_bounds(min_x, min_y, max_x, max_y);
        self.damage.mark();
    }

    /// Cursor-anchored zoom (clamped to the view-state band).
    pub fn zoom_at(&mut self, dz: f64, cursor_x_px: f64, cursor_y_px: f64) {
        self.camera.zoom_at(dz, cursor_x_px, cursor_y_px);
        self.damage.mark();
    }

    /// T-151.8 — mark the next `render()` as needing a GPU submit.
    pub fn mark_dirty(&mut self) {
        self.damage.mark();
    }

    /// T-151.8 — HUD continuous render (every rAF submits). Default false.
    pub fn set_continuous_render(&mut self, on: bool) {
        self.damage.set_continuous(on);
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn submitted_last_frame(&self) -> bool {
        self.submitted_last_frame
    }

    /// T-151.8.1 — WebGPU compute cull active (false on WebGL2).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_enabled(&self) -> bool {
        self.compute_cull_trees && self.icon_cull.is_some()
    }

    /// Class R CPU oracle count for the last encode_cull frustum.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_cpu_count(&self) -> u32 {
        self.icon_cull
            .as_ref()
            .map(|c| c.last_cpu_count)
            .unwrap_or(0)
    }

    /// The GPU-side cull counter (T-151.11.4 / X-03): the REAL async-mapped value once
    /// `compute_cull_gpu_sampled` is true; until the first sample it mirrors the CPU oracle.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_gpu_count(&self) -> u32 {
        self.icon_cull
            .as_ref()
            .map(|c| c.gpu_count_for_stats())
            .unwrap_or(0)
    }

    /// True once at least one real GPU counter readback has landed.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn compute_cull_gpu_sampled(&self) -> bool {
        self.icon_cull.as_ref().is_some_and(|c| c.gpu_sampled.get())
    }

    /// Pure CPU compact of current tree icons against a world-meter frustum (Class R harness).
    /// Returns surviving instance count. Frustum is WORLD meters (converted to anchor-relative).
    #[must_use]
    pub fn compute_cull_cpu_count_for_frustum(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> u32 {
        let frustum = [
            min_x - ANCHOR[0],
            min_y - ANCHOR[1],
            max_x - ANCHOR[0],
            max_y - ANCHOR[1],
        ];
        crate::compute_cull::count_icons_in_frustum(&self.tree_icons_20, frustum)
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

    // T-151.11.2 (audit X-05): `unproject_xy` deleted — the FE unprojects through frozen
    // `OrthoCameraJs` snapshots (gesture semantics require a camera frozen at gesture start;
    // a live engine unproject would feedback-loop during pan). `viewportFromEngine` deleted too.

    /// Render one frame to the canvas. Steady-state CPU→GPU traffic is exactly the 64-byte
    /// mvp uniform (the navigation invariant); instance data is static in GPU memory.
    /// T-151.8: when `!dirty && !continuous`, skip surface acquire / encode / submit.
    pub fn render(&mut self) -> Result<(), JsError> {
        if !self.damage.begin_frame().submit {
            self.uniform_bytes_last_frame = 0;
            self.submitted_last_frame = false;
            return Ok(());
        }

        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&mvp));
        // Steady-state: 64 B mvp. During T-061 drag the SlotDrag lane is live → +16 B delta uniform
        // (written by `set_slot_drag_delta`; counted here so stats survive the frame).
        let drag_live = self
            .batches
            .iter()
            .any(|b| b.role == LaneRole::SlotDrag && b.visible);
        self.uniform_bytes_last_frame = if drag_live { 64 + 16 } else { 64 };

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            Cst::Timeout | Cst::Occluded => {
                // Surface not ready — keep dirty so the next rAF retries.
                self.submitted_last_frame = false;
                return Ok(());
            }
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

        // T-151.8.1: WebGPU tree instance cull before the color pass.
        let do_compute_trees = self.compute_cull_trees
            && self.icon_cull.is_some()
            && self.icon_pipeline_storage32.is_some()
            && self.glyph_atlas.is_some()
            && !self.tree_icons_20.is_empty();
        if do_compute_trees {
            let world = self.camera.visible_world_rect();
            // Icon buffers are anchor-relative; frustum must match.
            let frustum = [
                world[0] - ANCHOR[0],
                world[1] - ANCHOR[1],
                world[2] - ANCHOR[0],
                world[3] - ANCHOR[1],
            ];
            if let Some(cull) = &mut self.icon_cull {
                cull.encode_cull(&mut encoder, &self.device, &self.queue, frustum);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
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
            // Iterate the ordered draw list (T-151.1 L1): editor lanes basemap → hillshade → grid
            // (calibration hidden), or the spike's stress pool then calibration (on top). Group 0
            // is bound once inside `draw_batches`; the pipeline is switched per payload.
            let glyph_bg = self.glyph_atlas.as_ref().map(|a| &a.bind_group);
            let slot_base = self.slot_atlas.as_ref().map(|a| &a.base_bind_group);
            let slot_drag = self.slot_atlas.as_ref().map(|a| &a.drag_bind_group);
            // T-151.11.1 (audit X-01): the compute-culled tree draw is emitted INSIDE
            // draw_batches at the WorldTrees order slot — never on top of slots/grid/marquee.
            let indirect_trees = if do_compute_trees {
                match (
                    self.icon_cull.as_ref(),
                    self.icon_pipeline_storage32.as_ref(),
                    self.glyph_atlas.as_ref(),
                ) {
                    (Some(cull), Some(pipe32), Some(atlas)) => {
                        cull.dst_buf.as_ref().map(|dst| IndirectTrees {
                            pipeline: pipe32,
                            atlas_bind: &atlas.bind_group,
                            instances: dst,
                            indirect: &cull.indirect_buf,
                        })
                    }
                    _ => None,
                }
            } else {
                None
            };
            draw_batches(
                &self.batches,
                &mut pass,
                &self.bind_group,
                &self.unit_quad_buf,
                &self.surface_pipeline,
                &self.textured_pipeline,
                &self.line_pipeline,
                &self.building_pipeline,
                &self.polygon_pipeline,
                &self.icon_pipeline,
                glyph_bg,
                slot_base,
                slot_drag,
                indirect_trees,
            );
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
        // T-151.11.4 (X-03): map the real GPU cull counter for this frame (in-flight guarded).
        if do_compute_trees && let Some(cull) = &self.icon_cull {
            cull.kick_readback();
        }
        self.damage.after_submit();
        self.submitted_last_frame = true;
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
                role: LaneRole::Stress,
                visible: true,
                payload: BatchPayload::Instanced {
                    instances: buffer,
                    count: count as u32,
                },
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
            match batch.payload {
                BatchPayload::Instanced { instances, .. }
                | BatchPayload::BuildingInstanced { instances, .. }
                | BatchPayload::IconInstanced { instances, .. } => instances.destroy(),
                BatchPayload::Textured(l) => l.texture.destroy(),
                BatchPayload::Lines(l) => l.verts.destroy(),
                BatchPayload::Polygon(l) => {
                    l.verts.destroy();
                    l.indices.destroy();
                }
            }
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
        // Stress batches are the `Stress`-role QuadInstanced entries (excludes the calibration
        // batch and the W1 lanes). `payload.kind()` keeps the PipelineKind variants live (L1).
        // On the spike (batches = stress… + calibration) these numbers are byte-identical to the
        // T-151.0 `batches.len()-1` / `batches[..n]` derivation.
        let stress_count = self
            .batches
            .iter()
            .filter(|b| {
                matches!(b.payload.kind(), PipelineKind::QuadInstanced)
                    && b.role == LaneRole::Stress
            })
            .count();
        let stress_bytes: u64 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::Stress)
            .map(|b| match &b.payload {
                BatchPayload::Instanced { count, .. } => u64::from(*count) * 32,
                _ => 0,
            })
            .sum();
        let gpu_bytes = stress_bytes + 64 /* uniform */ + 32 /* unit quad */ + 64 /* calibration */;
        // W1 additive fields (L12): the satellite lane's mode/tiles + total lane texture bytes.
        let satellite = self.batches.iter().find_map(|b| match &b.payload {
            BatchPayload::Textured(l) if b.role == LaneRole::Satellite => Some(l),
            _ => None,
        });
        let basemap_mode = satellite.map_or("none", |l| l.mode.as_str());
        let basemap_tiles = satellite.map_or(0, |l| l.tiles);
        let basemap_bytes: u64 = self
            .batches
            .iter()
            .filter_map(|b| match &b.payload {
                BatchPayload::Textured(l)
                    if matches!(b.role, LaneRole::Satellite | LaneRole::Hillshade) =>
                {
                    Some(l.bytes)
                }
                _ => None,
            })
            .sum();
        let gpu_frame_ms = match &self.timer {
            Some(t) if t.has_sample.get() => format!("{:.3}", t.last_ms.get()),
            _ => "null".to_owned(),
        };
        // W3 additive fields (L15) — appended after the T-151.0/1 keys, whose positions/values are
        // untouched. `world_building_instances` = the drawn OBB fill count (filtered from batches);
        // `world_building_outline_vertices` = the LineList vertex count; `world_chunks_drawn` = the
        // resident chunks contributing (from `upload_world_buildings`).
        let world_building_instances: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::WorldBuildings)
            .map(|b| match &b.payload {
                BatchPayload::BuildingInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
        let world_building_outline_vertices: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::WorldBuildingsOutline)
            .map(|b| match &b.payload {
                BatchPayload::Lines(l) => l.count,
                _ => 0,
            })
            .sum();
        // W5 additive glyph stats (L10) — prior keys untouched.
        let tree_glyphs: u32 = if self.compute_cull_trees && self.icon_cull.is_some() {
            self.icon_cull
                .as_ref()
                .map(|c| c.last_cpu_count)
                .unwrap_or(0)
        } else {
            self.batches
                .iter()
                .filter(|b| b.role == LaneRole::WorldTrees)
                .map(|b| match &b.payload {
                    BatchPayload::IconInstanced { count, .. } => *count,
                    _ => 0,
                })
                .sum()
        };
        let prop_glyphs: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::WorldProps)
            .map(|b| match &b.payload {
                BatchPayload::IconInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
        let badge_glyphs: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::WorldBadges)
            .map(|b| match &b.payload {
                BatchPayload::IconInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
        let atlas_bytes = self.glyph_atlas.as_ref().map_or(0, |a| a.bytes)
            + self.slot_atlas.as_ref().map_or(0, |a| a.bytes);
        // W6 additive slot stats — prior keys untouched.
        let slot_instances: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::Slots)
            .map(|b| match &b.payload {
                BatchPayload::IconInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
        let slot_drag_instances: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::SlotDrag)
            .map(|b| match &b.payload {
                BatchPayload::IconInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
        let cluster_instances: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::Clusters)
            .map(|b| match &b.payload {
                BatchPayload::IconInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
        format!(
            concat!(
                "{{\"backend\":\"{}\",\"instances\":{},\"chunks\":{},\"gpu_bytes\":{},",
                "\"staging_peak_bytes\":{},\"gen_ms\":{:.1},\"upload_ms\":{:.1},",
                "\"uniform_bytes_last_frame\":{},\"gpu_frame_ms\":{},",
                "\"basemap_mode\":\"{}\",\"basemap_tiles\":{},\"basemap_bytes\":{},",
                "\"world_building_instances\":{},\"world_building_outline_vertices\":{},",
                "\"world_chunks_drawn\":{},",
                "\"sea_polygons\":{},\"landcover_polygons\":{},\"contour_segments\":{},",
                "\"road_segments\":{},\"forest_polygons\":{},\"forest_outline_segments\":{},",
                "\"tree_glyphs\":{},\"prop_glyphs\":{},\"badge_glyphs\":{},\"atlas_bytes\":{},",
                "\"slot_instances\":{},\"slot_drag_instances\":{},\"cluster_instances\":{},",
                "\"submitted_last_frame\":{},\"density_heatmap\":{},",
                "\"compute_cull\":{},\"compute_cull_cpu_count\":{},\"compute_cull_gpu_count\":{},",
                "\"compute_cull_gpu_sampled\":{}}}"
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
            basemap_mode,
            basemap_tiles,
            basemap_bytes,
            world_building_instances,
            world_building_outline_vertices,
            self.world_chunks_drawn,
            self.sea_polygons,
            self.landcover_polygons,
            self.contour_segments,
            self.road_segments,
            self.forest_polygons,
            self.forest_outline_segments,
            tree_glyphs,
            prop_glyphs,
            badge_glyphs,
            atlas_bytes,
            slot_instances,
            slot_drag_instances,
            cluster_instances,
            self.submitted_last_frame,
            self.density_heatmap,
            self.compute_cull_trees && self.icon_cull.is_some(),
            self.icon_cull
                .as_ref()
                .map(|c| c.last_cpu_count)
                .unwrap_or(0),
            self.icon_cull
                .as_ref()
                .map(|c| c.gpu_count_for_stats())
                .unwrap_or(0),
            self.icon_cull.as_ref().is_some_and(|c| c.gpu_sampled.get()),
        )
    }
}

/// 256-byte-aligned row pitch for a `copy_texture_to_buffer` readback of a `width`-px RGBA row.
fn padded_bytes_per_row(width: u32) -> u32 {
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded.div_ceil(align) * align
}

/// Yield to the browser macrotask queue so the GL fence / map callback can progress (the wasm
/// readback has no blocking poll — mirror of `probe::sleep_ms`).
async fn readback_sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Map `read_buf`, read the 4 bytes at `offset`, unmap, and return them — a bounded poll/yield
/// loop (no blocking poll on wasm). Shared by `readback_rgba` and `texture_self_check`.
async fn map_read_4(
    device: &wgpu::Device,
    read_buf: &wgpu::Buffer,
    offset: u64,
) -> Result<[u8; 4], String> {
    let done = Rc::new(Cell::new(0u8)); // 0 pending, 1 ok, 2 error
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
        readback_sleep_ms(4).await;
        ticks += 1;
        if ticks > 2000 {
            return Err("readback-map-timeout".to_owned());
        }
    }
    if done.get() == 2 {
        return Err("readback-map-failed".to_owned());
    }
    let out: [u8; 4] = {
        let data = read_buf.slice(..).get_mapped_range();
        let b = offset as usize;
        data[b..b + 4].try_into().expect("4 bytes")
    };
    read_buf.unmap();
    Ok(out)
}

// ── W1 lane management (private helpers) ──────────────────────────────────────────────────────
impl RenderEngine {
    /// Update the W4 additive stats counter for a vector lane role.
    fn set_vector_stat(&mut self, role: LaneRole, n: u32) {
        match role {
            LaneRole::Sea => self.sea_polygons = n,
            LaneRole::Landcover => self.landcover_polygons = n,
            LaneRole::Contours => self.contour_segments = n,
            LaneRole::Roads => self.road_segments = n,
            LaneRole::RoadsCasing => {}
            LaneRole::ForestFill => self.forest_polygons = n,
            LaneRole::ForestOutline => self.forest_outline_segments = n,
            _ => {}
        }
    }

    /// Replace-or-insert a lane by role, keeping the fixed W1 draw order (`lane_order`). Removing
    /// the old same-role batch drops its GPU texture/buffer (freed on `Drop`).
    fn upsert_lane(&mut self, role: LaneRole, batch: Batch) {
        self.remove_lane(role);
        let pos = self
            .batches
            .iter()
            .position(|b| lane_order(b.role) > lane_order(role))
            .unwrap_or(self.batches.len());
        self.batches.insert(pos, batch);
        self.damage.mark();
    }

    /// Drop every batch of `role` (its GPU resources free on `Drop`).
    fn remove_lane(&mut self, role: LaneRole) {
        let had = self.batches.iter().any(|b| b.role == role);
        self.batches.retain(|b| b.role != role);
        if had {
            self.damage.mark();
        }
    }

    /// Encode the live scene into an offscreen `Rgba8Unorm` target at the current camera, copy it
    /// to a mappable buffer, and submit — the synchronous half of `readback_rgba` (the `&self`
    /// borrow ends before the returned buffer is mapped, so `render()` can keep running).
    fn encode_scene_readback(&self, w: u32, h: u32, padded: u32) -> wgpu::Buffer {
        let fmt = wgpu::TextureFormat::Rgba8Unorm;
        let quad = create_quad_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);
        let tex_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("readback-textured"),
                bind_group_layouts: &[
                    Some(&self.bind_group_layout),
                    Some(&self.tex_bind_group_layout),
                ],
                immediate_size: 0,
            });
        let textured = create_textured_pipeline(&self.device, &tex_layout, &self.shader, fmt);
        let line = create_line_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);
        let building =
            create_building_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);
        let polygon =
            create_polygon_pipeline(&self.device, &self.pipeline_layout, &self.shader, fmt);

        let mvp = self.camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-mvp"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&uniform_buf, 0, bytemuck::cast_slice(&mvp));
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("readback-mvp"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let icon =
            create_icon_pipeline(&self.device, &self.icon_pipeline_layout, &self.shader, fmt);
        let (read_buf, _view, _texture) = self.render_target_readback(
            w,
            h,
            padded,
            &bind_group,
            &quad,
            &textured,
            &line,
            &building,
            &polygon,
            &icon,
        );
        read_buf
    }

    /// Shared offscreen render + copy-to-buffer: draw `self.batches` with the given (readback-format)
    /// pipelines into a fresh `Rgba8Unorm` target and copy it into a mappable buffer.
    #[allow(clippy::too_many_arguments)]
    fn render_target_readback(
        &self,
        w: u32,
        h: u32,
        padded: u32,
        bind_group: &wgpu::BindGroup,
        quad: &wgpu::RenderPipeline,
        textured: &wgpu::RenderPipeline,
        line: &wgpu::RenderPipeline,
        building: &wgpu::RenderPipeline,
        polygon: &wgpu::RenderPipeline,
        icon: &wgpu::RenderPipeline,
    ) -> (wgpu::Buffer, wgpu::TextureView, wgpu::Texture) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("readback-target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
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
        let read_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-buffer"),
            size: u64::from(padded) * u64::from(h),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("readback"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let glyph_bg = self.glyph_atlas.as_ref().map(|a| &a.bind_group);
            let slot_base = self.slot_atlas.as_ref().map(|a| &a.base_bind_group);
            let slot_drag = self.slot_atlas.as_ref().map(|a| &a.drag_bind_group);
            // Readback path runs no compute encode; culled trees are absent here on WebGPU
            // (pre-11.1 behavior, unchanged) — probes never assert the tree lane.
            draw_batches(
                &self.batches,
                &mut pass,
                bind_group,
                &self.unit_quad_buf,
                quad,
                textured,
                line,
                building,
                polygon,
                icon,
                glyph_bg,
                slot_base,
                slot_drag,
                None,
            );
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        (read_buf, view, texture)
    }
}

// ── W1 lane API (wasm-bindgen) ────────────────────────────────────────────────────────────────
#[wasm_bindgen]
impl RenderEngine {
    /// Allocate a basemap-lane texture (role 0 = basemap, 1 = hillshade) covering the world rect
    /// `[min_x, min_y]..[max_x, max_y]`, sized `tex_w × tex_h` with `mip_count` levels, tagged with
    /// the render `mode` (0 unified, 1 pyramid, 2 single, 3 hillshade). Blocks are then uploaded via
    /// `tex_layer_write_*` and finalized with `tex_layer_commit` (T-151.1 L3).
    ///
    /// # Errors
    /// Bad role or zero dimensions.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_layer_begin(
        &mut self,
        role: u32,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tex_w: u32,
        tex_h: u32,
        mip_count: u32,
        mode: u32,
    ) -> Result<(), JsError> {
        let idx = role as usize;
        if idx > 1 {
            return Err(JsError::new(
                "tex_layer: role must be 0 (basemap) or 1 (hillshade)",
            ));
        }
        if tex_w == 0 || tex_h == 0 || mip_count == 0 {
            return Err(JsError::new("tex_layer: zero texture dimensions"));
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("basemap-lane"),
            size: wgpu::Extent3d {
                width: tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Non-sRGB rgba8: stores the decoded WebP bytes verbatim (matches `satelliteUnified.ts`
            // `rgba8unorm`), so a readback is byte-exact against the source texels.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        // Full mip chain ≈ base × 4/3; a single-level texture is exactly base.
        let base_bytes = u64::from(tex_w) * u64::from(tex_h) * 4;
        let bytes = if mip_count > 1 {
            base_bytes * 4 / 3
        } else {
            base_bytes
        };
        self.pending[idx] = Some(PendingTex {
            texture,
            world_min: [min_x, min_y],
            world_max: [max_x, max_y],
            mode: BasemapMode::from_u32(mode),
            tiles: 0,
            bytes,
        });
        Ok(())
    }

    /// Upload one decoded block/tile (a `web_sys::ImageBitmap`) into the pending texture via
    /// `copy_external_image_to_texture` — the WebGPU fast path (T-151.1 L3). Not available on the
    /// WebGL2 backend; JS routes to `tex_layer_write_rgba` there (checks `engine.backend()`).
    ///
    /// # Errors
    /// `tex_layer_begin` not called for this role.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_layer_write_bitmap(
        &mut self,
        role: u32,
        mip: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        bmp: web_sys::ImageBitmap,
    ) -> Result<(), JsError> {
        let idx = role as usize;
        let queue = &self.queue;
        let pending = self
            .pending
            .get_mut(idx)
            .and_then(|p| p.as_mut())
            .ok_or_else(|| JsError::new("tex_layer_write_bitmap: begin not called"))?;
        queue.copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(bmp),
                origin: wgpu::Origin2d::ZERO,
                flip_y: false,
            },
            wgpu::CopyExternalImageDestInfo {
                texture: &pending.texture,
                mip_level: mip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        pending.tiles += 1;
        Ok(())
    }

    /// Upload one block/tile as raw RGBA8 bytes via `write_texture` — the universal path (WebGL2
    /// fallback + hillshade — T-151.1 L3/L6).
    ///
    /// # Errors
    /// `tex_layer_begin` not called, or `rgba.len() != w*h*4`.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_layer_write_rgba(
        &mut self,
        role: u32,
        mip: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) -> Result<(), JsError> {
        let idx = role as usize;
        if rgba.len() != (w as usize) * (h as usize) * 4 {
            return Err(JsError::new("tex_layer_write_rgba: byte length != w*h*4"));
        }
        let queue = &self.queue;
        let pending = self
            .pending
            .get_mut(idx)
            .and_then(|p| p.as_mut())
            .ok_or_else(|| JsError::new("tex_layer_write_rgba: begin not called"))?;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &pending.texture,
                mip_level: mip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        pending.tiles += 1;
        Ok(())
    }

    /// Finalize the pending texture for `role` into a drawn lane: a 1-instance world-rect quad
    /// tinted `[1,1,1,opacity]` over the uploaded texture, upserted into the draw list in W1 order.
    ///
    /// # Errors
    /// `tex_layer_begin` not called for this role.
    pub fn tex_layer_commit(
        &mut self,
        role: u32,
        opacity: f32,
        visible: bool,
    ) -> Result<(), JsError> {
        let idx = role as usize;
        let pending = self
            .pending
            .get_mut(idx)
            .and_then(Option::take)
            .ok_or_else(|| JsError::new("tex_layer_commit: begin not called"))?;
        let rect = lanes::world_rect_rel(pending.world_min, pending.world_max);
        let inst = QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)],
        };
        use wgpu::util::DeviceExt;
        let instances = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tex-lane-quad"),
                contents: bytemuck::cast_slice(&[inst]),
                // COPY_DST so `set_lane_opacity` can re-tint in place (no texture rebuild — L6).
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let view = pending
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tex-lane"),
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let lane = TexLane {
            texture: pending.texture,
            bind_group,
            instances,
            mode: pending.mode,
            tiles: pending.tiles,
            bytes: pending.bytes,
        };
        let role_enum = if idx == 0 {
            LaneRole::Satellite
        } else {
            LaneRole::Hillshade
        };
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Textured(lane),
            },
        );
        Ok(())
    }

    /// Drop the lane for `role` (and any half-uploaded pending texture) — e.g. a unified→pyramid
    /// re-resolve, or hillshade toggled off.
    pub fn tex_layer_clear(&mut self, role: u32) {
        let idx = role as usize;
        if idx > 1 {
            return;
        }
        self.pending[idx] = None;
        let role_enum = if idx == 0 {
            LaneRole::Satellite
        } else {
            LaneRole::Hillshade
        };
        self.remove_lane(role_enum);
    }

    /// Build the procedural grid lane for a `width × height` terrain (T-151.1 L7). `over_hillshade`
    /// selects the boosted palette; `visible` toggles it (kept in the draw list like Deck).
    pub fn set_grid(&mut self, width: f64, height: f64, over_hillshade: bool, visible: bool) {
        let verts = lanes::grid_lines(width, height, over_hillshade);
        if verts.is_empty() {
            self.remove_lane(LaneRole::Grid);
            return;
        }
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grid-lines"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let lane = LineLane {
            verts: buf,
            count: verts.len() as u32,
        };
        self.upsert_lane(
            LaneRole::Grid,
            Batch {
                role: LaneRole::Grid,
                visible,
                payload: BatchPayload::Lines(lane),
            },
        );
    }

    /// Replace the `world-buildings` fill lane (T-151.3 W3). `fill` is `WorldResidency`'s flat
    /// output — 10 f32 per instance in WORLD meters `[x, y, hx, hy, cos, sin, r, g, b, a]`; the
    /// center is converted to anchor-relative here (the single anchor source of truth). `chunk_count`
    /// is the resident chunks contributing (`stats().world_chunks_drawn`). Empty → drop the lane.
    pub fn upload_world_buildings(&mut self, fill: &[f32], chunk_count: u32, visible: bool) {
        const STRIDE: usize = 10;
        // T-151.4.1: empty + visible → sticky (keep prior lane). Mid-hydration empty uploads
        // used to wipe town buildings; the loader also guards, but belt-and-suspenders here.
        // Callers that truly want to clear must pass visible=false (or clear_world_buildings).
        if fill.is_empty() {
            if !visible {
                self.remove_lane(LaneRole::WorldBuildings);
                self.world_chunks_drawn = 0;
            }
            return;
        }
        if !fill.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldBuildings);
            self.world_chunks_drawn = chunk_count;
            return;
        }
        let mut instances = Vec::with_capacity(fill.len() / STRIDE);
        for c in fill.chunks_exact(STRIDE) {
            instances.push(scene::BuildingInstance {
                center: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                half: [c[2], c[3]],
                basis: [c[4], c[5]],
                color: [c[6], c[7], c[8], c[9]],
            });
        }
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-buildings"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = instances.len() as u32;
        self.world_chunks_drawn = chunk_count;
        self.upsert_lane(
            LaneRole::WorldBuildings,
            Batch {
                role: LaneRole::WorldBuildings,
                visible,
                payload: BatchPayload::BuildingInstanced {
                    instances: buf,
                    count,
                },
            },
        );
    }

    /// Replace the `world-buildings-outline` lane (T-151.3 W3). `lines` is `WorldResidency`'s flat
    /// `LineList` output — 6 f32 per vertex in WORLD meters `[x, y, r, g, b, a]`; positions are
    /// converted to anchor-relative here. Empty → drop the lane.
    pub fn upload_world_building_outlines(&mut self, lines: &[f32], visible: bool) {
        const STRIDE: usize = 6;
        if lines.is_empty() || !lines.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldBuildingsOutline);
            return;
        }
        let mut verts = Vec::with_capacity(lines.len() / STRIDE);
        for c in lines.chunks_exact(STRIDE) {
            verts.push(lanes::LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-buildings-outline"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = verts.len() as u32;
        self.upsert_lane(
            LaneRole::WorldBuildingsOutline,
            Batch {
                role: LaneRole::WorldBuildingsOutline,
                visible,
                payload: BatchPayload::Lines(LineLane { verts: buf, count }),
            },
        );
    }

    /// Replace the `world-fences` strip lane (T-152.4). `packed` is flat `[x,y,r,g,b,a]…` triangle-list
    /// verts in WORLD meters (same layout as road strips).
    pub fn upload_world_fence_strips(&mut self, packed: &[f32], item_count: u32, visible: bool) {
        const STRIDE: usize = 6;
        if packed.is_empty() {
            if !visible {
                self.remove_lane(LaneRole::WorldFences);
            }
            return;
        }
        if !packed.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldFences);
            return;
        }
        let n_verts = packed.len() / STRIDE;
        let mut verts = Vec::with_capacity(n_verts);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(lanes::LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        let indices: Vec<u32> = (0..n_verts as u32).collect();
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-fences"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world-fences-indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let index_count = indices.len() as u32;
        self.upsert_lane(
            LaneRole::WorldFences,
            Batch {
                role: LaneRole::WorldFences,
                visible,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count,
                    item_count,
                }),
            },
        );
    }

    /// Drop both world-building lanes (gate closed below the building band, or terrain switch).
    pub fn clear_world_buildings(&mut self) {
        self.remove_lane(LaneRole::WorldBuildings);
        self.remove_lane(LaneRole::WorldBuildingsOutline);
        self.remove_lane(LaneRole::WorldFences);
        self.world_chunks_drawn = 0;
    }

    // ── W5 glyph atlas + icon lanes ───────────────────────────────────────────────────────────

    /// Upload the world glyph atlas once (T-151.5 L1–L3).
    /// `rgba` = packed RGBA8 top-row-first; `uv` = 28×4 f32 (u0,v0,u1,v1) in key order.
    ///
    /// # Errors
    /// Returns `JsError` when UV count ≠ 28 or rgba length ≠ w·h·4.
    pub fn upload_glyph_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<(), JsError> {
        use scene::ATLAS_GLYPH_COUNT;
        if uv.len() != ATLAS_GLYPH_COUNT * 4 {
            return Err(JsError::new(&format!(
                "glyph-atlas-uv-count: expected {}, got {}",
                ATLAS_GLYPH_COUNT * 4,
                uv.len()
            )));
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap_or(0);
        if rgba.len() != expected {
            return Err(JsError::new("glyph-atlas-rgba-size"));
        }
        use wgpu::util::DeviceExt;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph-atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        // UV[28] (448 B) + drag_delta=0, px_to_m=1, pad (16 B) = 464 B.
        let mut u_bytes = vec![0u8; ICON_UNIFORM_BYTES as usize];
        for (i, v) in uv.iter().enumerate() {
            let off = i * 4;
            if off + 4 <= 448 {
                u_bytes[off..off + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        // px_to_m = 1.0 at offset 456 (after drag_delta at 448)
        u_bytes[456..460].copy_from_slice(&1.0_f32.to_le_bytes());
        let uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glyph-icon-uniforms"),
                contents: &u_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph-atlas"),
            layout: &self.icon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.icon_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        if let Some(old) = self.glyph_atlas.take() {
            old.texture.destroy();
            old.uniform_buf.destroy();
        }
        self.glyph_atlas = Some(GlyphAtlasGpu {
            texture,
            uniform_buf,
            bind_group,
            bytes: expected as u64,
        });
        Ok(())
    }

    /// Upload packed 20 B icon instances for trees (0), props (1), or badges (2).
    /// Positions are WORLD meters; converted to anchor-relative here. Empty + visible → sticky.
    /// T-151.8.1: on WebGPU, tree lane feeds compute cull (`VERTEX|STORAGE` + `draw_indirect`);
    /// WebGL2 keeps the direct IconInstanced path (chunk granularity).
    pub fn upload_icon_lane(&mut self, kind: u32, bytes: &[u8], visible: bool) {
        let role = match kind {
            0 => LaneRole::WorldTrees,
            1 => LaneRole::WorldProps,
            2 => LaneRole::WorldBadges,
            _ => return,
        };
        const STRIDE: usize = 20;
        if bytes.is_empty() {
            if role == LaneRole::WorldTrees {
                self.tree_icons_20.clear();
                if let Some(cull) = &mut self.icon_cull {
                    cull.upload_icons(&self.device, &self.queue, &[]);
                }
            }
            if !visible {
                self.remove_lane(role);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(role);
            return;
        }
        // Convert world pos → anchor-relative in a scratch buffer.
        let mut converted = bytes.to_vec();
        for chunk in converted.chunks_exact_mut(STRIDE) {
            let x = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let ax = (f64::from(x) - ANCHOR[0]) as f32;
            let ay = (f64::from(y) - ANCHOR[1]) as f32;
            chunk[0..4].copy_from_slice(&ax.to_le_bytes());
            chunk[4..8].copy_from_slice(&ay.to_le_bytes());
        }

        // WebGPU trees: compute-cull path (no direct IconInstanced lane).
        if role == LaneRole::WorldTrees && self.compute_cull_trees && self.icon_cull.is_some() {
            self.tree_icons_20 = converted;
            if let Some(cull) = &mut self.icon_cull {
                cull.upload_icons(&self.device, &self.queue, &self.tree_icons_20);
            }
            self.remove_lane(LaneRole::WorldTrees);
            self.damage.mark();
            return;
        }

        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("icon-lane"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = (converted.len() / STRIDE) as u32;
        self.upsert_lane(
            role,
            Batch {
                role,
                visible,
                payload: BatchPayload::IconInstanced {
                    instances: buf,
                    count,
                },
            },
        );
    }

    /// Drop all three glyph icon lanes.
    pub fn clear_icon_lanes(&mut self) {
        self.remove_lane(LaneRole::WorldTrees);
        self.remove_lane(LaneRole::WorldProps);
        self.remove_lane(LaneRole::WorldBadges);
    }

    /// T-151.8 — upload exact-count density grid (LE u32 bytes) as a world-bounds heatmap quad.
    /// Class R count grid lives in residency; GPU samples an RGBA8 visual encoding of those counts.
    /// `world_w`/`world_h` = terrain meters (Everon 12800). Empty / invisible → drop lane.
    pub fn upload_density_grid(
        &mut self,
        counts_le: &[u8],
        width: u32,
        height: u32,
        world_w: f64,
        world_h: f64,
        visible: bool,
    ) {
        self.density_heatmap = visible && width > 0 && height > 0 && !counts_le.is_empty();
        if !self.density_heatmap {
            self.remove_lane(LaneRole::DensityHeat);
            return;
        }
        let Some(rgba) = crate::density_heat::density_counts_to_rgba(counts_le, width, height)
        else {
            self.remove_lane(LaneRole::DensityHeat);
            self.density_heatmap = false;
            return;
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("density-heat"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("density-heat"),
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let rect = lanes::world_rect_rel([0.0, 0.0], [world_w, world_h]);
        let inst = QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [1.0, 1.0, 1.0, 0.85],
        };
        use wgpu::util::DeviceExt;
        let instances = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("density-heat-quad"),
                contents: bytemuck::cast_slice(&[inst]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let lane = TexLane {
            texture,
            bind_group,
            instances,
            mode: BasemapMode::Unified,
            tiles: 0,
            bytes: rgba.len() as u64,
        };
        self.upsert_lane(
            LaneRole::DensityHeat,
            Batch {
                role: LaneRole::DensityHeat,
                visible: true,
                payload: BatchPayload::Textured(lane),
            },
        );
    }

    // ── W6 mission slot / cluster icon lanes ─────────────────────────────────────────────────

    /// Build IconUniforms bytes: UV floats + drag_delta + px_to_m + pad.
    fn pack_icon_uniforms(uv: &[f32], drag_dx: f32, drag_dy: f32, px_to_m: f32) -> Vec<u8> {
        let mut u_bytes = vec![0u8; ICON_UNIFORM_BYTES as usize];
        for (i, v) in uv.iter().enumerate() {
            let off = i * 4;
            if off + 4 <= 448 {
                u_bytes[off..off + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        u_bytes[448..452].copy_from_slice(&drag_dx.to_le_bytes());
        u_bytes[452..456].copy_from_slice(&drag_dy.to_le_bytes());
        u_bytes[456..460].copy_from_slice(&px_to_m.to_le_bytes());
        u_bytes
    }

    /// Convert world-meter icon instances (20 B) to anchor-relative in place.
    fn convert_icon_world_to_anchor(bytes: &mut [u8]) {
        const STRIDE: usize = 20;
        for chunk in bytes.chunks_exact_mut(STRIDE) {
            let x = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            let ax = (f64::from(x) - ANCHOR[0]) as f32;
            let ay = (f64::from(y) - ANCHOR[1]) as f32;
            chunk[0..4].copy_from_slice(&ax.to_le_bytes());
            chunk[4..8].copy_from_slice(&ay.to_le_bytes());
        }
    }

    // ── T-151.7.3 high-level slot GPU bridge (public wasm surface) ───────────────────────────

    /// Upload dedicated slot/cluster atlas once (ring + disc). Replaces low-level
    /// `upload_slot_atlas` as the TS entry point.
    pub fn ensure_slot_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<(), JsError> {
        self.upload_slot_atlas(rgba, width, height, uv)?;
        self.slot_bridge.atlas_ready = true;
        self.sync_slot_zoom_uniform();
        Ok(())
    }

    /// Bind SoA snapshot from MissionDoc (called via wasm free fn `bind_mission_doc`).
    /// Does **not** retain the doc — only caches ids + xy until the next bind.
    pub fn slots_bind_soa(&mut self, ids: Vec<String>, xy: &[f32]) {
        self.slot_bridge.last_ids = ids;
        self.slot_bridge.last_xy = xy.to_vec();
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        self.slot_bridge.last_cluster_mode = slots_gpu::cluster_mode(n, zoom);
        if self.slot_bridge.drag_active && !self.slot_bridge.drag_ids.is_empty() {
            let dx = self
                .slot_atlas
                .as_ref()
                .map(|a| a.drag_delta[0])
                .unwrap_or(0.0);
            let dy = self
                .slot_atlas
                .as_ref()
                .map(|a| a.drag_delta[1])
                .unwrap_or(0.0);
            self.start_slot_drag_overlay(dx, dy);
        } else {
            self.rematerialize_slot_lane();
        }
    }

    /// Selection ids → full rematerialize (T-151.7.2). Empty = clear tint.
    pub fn set_selection(&mut self, ids: Vec<String>) {
        self.slot_bridge.selected_ids = ids.into_iter().collect();
        if !self.slot_bridge.atlas_ready {
            return;
        }
        // Invariant: dragActive only while drag_ids non-empty.
        if self.slot_bridge.drag_active && self.slot_bridge.drag_ids.is_empty() {
            self.clear_slot_drag_internal();
            return;
        }
        if self.slot_bridge.drag_active {
            return; // drag overlay owns tint
        }
        self.rematerialize_slot_lane();
    }

    /// Drag ids + world-meter delta. Empty ids = clear (T-151.7.1 start/delta/end).
    pub fn set_drag(&mut self, ids: Vec<String>, dx: f32, dy: f32) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let had = !self.slot_bridge.drag_ids.is_empty();
        let has = !ids.is_empty();
        let ids_changed = self.slot_bridge.drag_ids != ids;
        // TS only calls when store changed; same ids while dragging ⇒ delta update.
        let delta_changed = had && has && !ids_changed;
        let phase = classify_drag_transition(had, has, ids_changed, delta_changed);
        match phase {
            DragGpuPhase::Idle => {}
            DragGpuPhase::End => {
                self.slot_bridge.drag_ids.clear();
                self.clear_slot_drag_internal();
            }
            DragGpuPhase::Delta => {
                if self.slot_bridge.drag_active {
                    self.set_slot_drag_delta(dx, dy);
                }
            }
            DragGpuPhase::Start | DragGpuPhase::Restart => {
                self.slot_bridge.drag_ids = ids;
                self.slot_bridge.drag_active = true;
                self.start_slot_drag_overlay(dx, dy);
            }
        }
    }

    /// Camera moved: px_to_m + cluster gate re-eval (zoom is engine SoT).
    pub fn on_camera_changed(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        self.sync_slot_zoom_uniform();
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = slots_gpu::cluster_mode(n, zoom);
        if cm != self.slot_bridge.last_cluster_mode {
            self.slot_bridge.last_cluster_mode = cm;
            if !cm {
                self.upload_cluster_lane(&[], false);
            }
            // Markers re-fed by TS when cluster_mode; still rematerialize slot lane.
            if !self.slot_bridge.drag_active {
                self.rematerialize_slot_lane();
            }
        }
    }

    /// Cluster disc markers from FE supercluster (not ported this slice).
    pub fn set_cluster_markers(&mut self, xs: &[f64], ys: &[f64], counts: &[u32]) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = slots_gpu::cluster_mode(n, zoom);
        self.slot_bridge.last_cluster_mode = cm;
        if !cm {
            self.upload_cluster_lane(&[], false);
            if !self.slot_bridge.drag_active {
                self.rematerialize_slot_lane();
            }
            return;
        }
        let bytes = pack_cluster_instances(xs, ys, counts);
        self.upload_cluster_lane(&bytes, !bytes.is_empty());
        if !self.slot_bridge.drag_active {
            self.rematerialize_slot_lane();
        }
    }

    /// Whether cluster mode is active for the cached SoA + current zoom.
    #[wasm_bindgen(js_name = cluster_mode)]
    pub fn slots_cluster_mode(&self) -> bool {
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        slots_gpu::cluster_mode(n, self.zoom())
    }

    /// Debug JSON for `window.__wgpuSlotStats` (engine stats + bridge flags).
    pub fn slot_stats_json(&self) -> String {
        let stats = self.stats();
        let trimmed = stats.trim_end_matches('}');
        format!(
            "{trimmed},\"slot_len\":{},\"cluster_mode\":{},\"slots_lane_selection_only\":{},\"drag_active\":{},\"atlas_ready\":{}}}",
            self.slot_bridge.last_ids.len(),
            if self.slot_bridge.last_cluster_mode {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.slots_lane_selection_only {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.drag_active {
                "true"
            } else {
                "false"
            },
            if self.slot_bridge.atlas_ready {
                "true"
            } else {
                "false"
            },
        )
    }

    /// Drop slot/drag/cluster lanes and reset bridge SoA cache (keeps atlas).
    pub fn clear_slots(&mut self) {
        self.clear_slot_lanes();
        let atlas_ready = self.slot_bridge.atlas_ready;
        self.slot_bridge = SlotGpuBridge {
            atlas_ready,
            ..SlotGpuBridge::default()
        };
    }

    // ── Internal slot lane helpers (not wasm-exported) ───────────────────────────────────────

    /// Upload dedicated slot/cluster atlas (ring + disc). `uv` is 2..28 glyphs × 4 floats
    /// (minU,minV,maxU,maxV); padded to 28. Does **not** touch the world-glyphs atlas.
    fn upload_slot_atlas(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        uv: &[f32],
    ) -> Result<(), JsError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap_or(0);
        if rgba.len() != expected {
            return Err(JsError::new("slot-atlas-rgba-size"));
        }
        use wgpu::util::DeviceExt;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("slot-atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        // Default open zoom −2 → px_to_m = 4
        let px_to_m = 4.0_f32;
        let base_bytes = Self::pack_icon_uniforms(uv, 0.0, 0.0, px_to_m);
        let drag_bytes = Self::pack_icon_uniforms(uv, 0.0, 0.0, px_to_m);
        let base_uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slot-atlas-base-u"),
                contents: &base_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let drag_uniform_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slot-atlas-drag-u"),
                contents: &drag_bytes,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let base_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slot-atlas-base"),
            layout: &self.icon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.icon_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_uniform_buf.as_entire_binding(),
                },
            ],
        });
        let drag_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("slot-atlas-drag"),
            layout: &self.icon_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.icon_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: drag_uniform_buf.as_entire_binding(),
                },
            ],
        });
        if let Some(old) = self.slot_atlas.take() {
            old.texture.destroy();
            old.base_uniform_buf.destroy();
            old.drag_uniform_buf.destroy();
        }
        self.slot_atlas = Some(SlotAtlasGpu {
            texture,
            base_uniform_buf,
            drag_uniform_buf,
            base_bind_group,
            drag_bind_group,
            bytes: expected as u64,
            px_to_m,
            drag_delta: [0.0, 0.0],
        });
        Ok(())
    }

    fn sync_slot_zoom_uniform(&mut self) {
        let px = slots_gpu::px_to_m_at_zoom(self.zoom());
        self.set_slot_px_to_m(px);
    }

    /// Full rematerialize from cached SoA + selection (T-151.7.2). Never OOB patch.
    fn rematerialize_slot_lane(&mut self) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let mask = selected_mask(&self.slot_bridge.last_ids, &self.slot_bridge.selected_ids);
        let zoom = self.zoom();
        #[allow(clippy::cast_possible_truncation)]
        let n = self.slot_bridge.last_ids.len() as u32;
        let cm = slots_gpu::cluster_mode(n, zoom);
        self.slot_bridge.last_cluster_mode = cm;
        if cm {
            let bytes = pack_selection_only(&self.slot_bridge.last_xy, &mask);
            let vis = !bytes.is_empty();
            self.upload_slot_lane(&bytes, vis);
            self.slot_bridge.slots_lane_selection_only = true;
        } else {
            let bytes = pack_slot_instances(&self.slot_bridge.last_xy, &mask);
            let vis = !self.slot_bridge.last_ids.is_empty();
            self.upload_slot_lane(&bytes, vis);
            self.slot_bridge.slots_lane_selection_only = false;
        }
    }

    fn start_slot_drag_overlay(&mut self, dx: f32, dy: f32) {
        let drag_ids = self.slot_bridge.drag_ids.clone();
        if drag_ids.is_empty() {
            self.clear_slot_drag_internal();
            return;
        }
        self.slot_bridge.drag_active = true;
        let (overlay, rows) = pack_drag_overlay(
            &drag_ids,
            &self.slot_bridge.last_ids,
            &self.slot_bridge.last_xy,
        );
        let count = rows.len();
        self.upload_slot_drag_lane(&overlay, count > 0);
        // Hide base rows only on full-n detail lane (not selection-only short lane).
        if !self.slot_bridge.slots_lane_selection_only {
            let hide = hide_slot_row_patch();
            for row in rows {
                #[allow(clippy::cast_possible_truncation)]
                let off = (row * SLOT_ICON_STRIDE + 8) as u32;
                self.patch_slot_lane(off, &hide);
            }
        }
        self.set_slot_drag_delta(dx, dy);
    }

    fn clear_slot_drag_internal(&mut self) {
        self.slot_bridge.drag_active = false;
        self.slot_bridge.drag_ids.clear();
        self.clear_slot_drag_lane();
        self.rematerialize_slot_lane();
    }

    /// Update slot atlas `px_to_m = 2^(-zoom)` on both base + drag uniforms (no instance re-upload).
    fn set_slot_px_to_m(&mut self, px_to_m: f32) {
        let Some(atlas) = self.slot_atlas.as_mut() else {
            return;
        };
        if (atlas.px_to_m - px_to_m).abs() < 1e-9 {
            return;
        }
        atlas.px_to_m = px_to_m;
        // Write only the px_to_m word at offset 456 (4 B) — counted in uniform_bytes_last_frame
        // only when paired with drag writes; zoom updates are free-ish.
        let bytes = px_to_m.to_le_bytes();
        self.queue
            .write_buffer(&atlas.base_uniform_buf, 456, &bytes);
        self.queue
            .write_buffer(&atlas.drag_uniform_buf, 456, &bytes);
    }

    /// Set SlotDrag drag delta (world meters). Writes 8 B drag_delta + leaves px_to_m.
    /// Gate: during drag, `uniform_bytes_last_frame` becomes 64 + 16 (mvp + delta params).
    fn set_slot_drag_delta(&mut self, dx: f32, dy: f32) {
        let Some(atlas) = self.slot_atlas.as_mut() else {
            return;
        };
        atlas.drag_delta = [dx, dy];
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&dx.to_le_bytes());
        bytes[4..8].copy_from_slice(&dy.to_le_bytes());
        bytes[8..12].copy_from_slice(&atlas.px_to_m.to_le_bytes());
        // pad remains 0
        self.queue
            .write_buffer(&atlas.drag_uniform_buf, 448, &bytes);
        // Account for drag uniform write on next frame's stats (render resets to 64 first).
        self.uniform_bytes_last_frame = 64 + 16;
    }

    /// Full upload of slot ring instances (20 B each, world meters). `VERTEX|COPY_DST`.
    fn upload_slot_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Slots, bytes, visible);
    }

    /// Sub-row dirty patch into the Slots lane buffer (T-151.11.2 / audit X-06).
    /// Contract: patches never start on a 20 B stride boundary — the only caller is the drag
    /// hide-patch at `row·20 + 8` (12 B of size/yaw/glyph/tint; positions are untouched, so no
    /// world→anchor conversion applies). The old full-row conversion heuristic was unreachable
    /// and is deleted; full-row updates go through `upload_slot_lane` (rematerialize).
    fn patch_slot_lane(&mut self, byte_offset: u32, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        debug_assert!(
            !byte_offset.is_multiple_of(20),
            "patch_slot_lane is sub-row only (offset {byte_offset} is a stride boundary); \
             use upload_slot_lane for full rows"
        );
        let Some(batch) = self.batches.iter().find(|b| b.role == LaneRole::Slots) else {
            return;
        };
        let BatchPayload::IconInstanced { instances, count } = &batch.payload else {
            return;
        };
        let end = byte_offset as u64 + bytes.len() as u64;
        if end > u64::from(*count) * 20 {
            return;
        }
        self.queue
            .write_buffer(instances, u64::from(byte_offset), bytes);
    }

    /// Upload SlotDrag overlay instances (world meters). Empty → drop lane.
    fn upload_slot_drag_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::SlotDrag, bytes, visible);
    }

    /// Clear the drag overlay lane.
    fn clear_slot_drag_lane(&mut self) {
        self.remove_lane(LaneRole::SlotDrag);
        if let Some(atlas) = self.slot_atlas.as_mut() {
            atlas.drag_delta = [0.0, 0.0];
            let mut bytes = [0u8; 16];
            bytes[8..12].copy_from_slice(&atlas.px_to_m.to_le_bytes());
            self.queue
                .write_buffer(&atlas.drag_uniform_buf, 448, &bytes);
        }
    }

    /// Upload cluster disc instances (world meters). Empty → drop lane.
    fn upload_cluster_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Clusters, bytes, visible);
    }

    /// Drop slots + drag + cluster lanes.
    fn clear_slot_lanes(&mut self) {
        self.remove_lane(LaneRole::Slots);
        self.remove_lane(LaneRole::SlotDrag);
        self.remove_lane(LaneRole::Clusters);
    }

    fn upload_slot_role_lane(&mut self, role: LaneRole, bytes: &[u8], visible: bool) {
        const STRIDE: usize = 20;
        if bytes.is_empty() {
            if !visible {
                self.remove_lane(role);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(role);
            return;
        }
        let mut converted = bytes.to_vec();
        Self::convert_icon_world_to_anchor(&mut converted);
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slot-icon-lane"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = (converted.len() / STRIDE) as u32;
        self.upsert_lane(
            role,
            Batch {
                role,
                visible,
                payload: BatchPayload::IconInstanced {
                    instances: buf,
                    count,
                },
            },
        );
    }

    // ── W4 vector lane uploads ────────────────────────────────────────────────────────────────

    /// Upload a pre-triangulated polygon mesh (T-151.4 L1).
    /// `positions` = interleaved world-meter `[x,y]…`, `colors` = normalized f32 `[r,g,b,a]…`
    /// (same length as positions/2 * 4), `indices` = triangle-list u32.
    /// `role`: 0 sea, 1 landcover, 5 forest_fill, 7 marquee.
    /// `item_count` feeds `stats()` (polygon count). Empty → drop the lane.
    pub fn upload_polygon_mesh(
        &mut self,
        role: u32,
        positions: &[f32],
        colors: &[f32],
        indices: &[u32],
        item_count: u32,
        visible: bool,
    ) {
        let Some(role_enum) = lane_role_from_u32(role) else {
            return;
        };
        let n_verts = positions.len() / 2;
        if indices.is_empty()
            || n_verts == 0
            || colors.len() < n_verts * 4
            || !positions.len().is_multiple_of(2)
        {
            self.remove_lane(role_enum);
            self.set_vector_stat(role_enum, 0);
            return;
        }
        let mut verts = Vec::with_capacity(n_verts);
        for i in 0..n_verts {
            verts.push(lanes::LineVertex {
                pos: [
                    (f64::from(positions[i * 2]) - ANCHOR[0]) as f32,
                    (f64::from(positions[i * 2 + 1]) - ANCHOR[1]) as f32,
                ],
                color: [
                    colors[i * 4],
                    colors[i * 4 + 1],
                    colors[i * 4 + 2],
                    colors[i * 4 + 3],
                ],
            });
        }
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("polygon-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("polygon-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let index_count = indices.len() as u32;
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count,
                    item_count,
                }),
            },
        );
    }

    /// Upload a wide-polyline strip already expanded to a triangle list as flat
    /// `[x,y,r,g,b,a]…` world-meter verts (6 f32/vert, 3 verts/tri). Used for road casing/
    /// centerline (T-151.4 L2). `role`: 3 roads_casing, 4 roads.
    pub fn upload_strip_tris(&mut self, role: u32, packed: &[f32], item_count: u32, visible: bool) {
        const STRIDE: usize = 6;
        let Some(role_enum) = lane_role_from_u32(role) else {
            return;
        };
        if packed.is_empty() || !packed.len().is_multiple_of(STRIDE) {
            self.remove_lane(role_enum);
            self.set_vector_stat(role_enum, 0);
            return;
        }
        let n_verts = packed.len() / STRIDE;
        let mut verts = Vec::with_capacity(n_verts);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(lanes::LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        // Sequential index buffer (already a triangle list).
        let indices: Vec<u32> = (0..n_verts as u32).collect();
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("strip-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("strip-indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let index_count = indices.len() as u32;
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count,
                    item_count,
                }),
            },
        );
    }

    /// Upload hairline LineList segments: flat `[x,y,r,g,b,a]…` (6 f32/vert, 2 verts/segment).
    /// `role`: 2 contours, 6 forest_outline.
    pub fn upload_hairline_segments(
        &mut self,
        role: u32,
        packed: &[f32],
        item_count: u32,
        visible: bool,
    ) {
        const STRIDE: usize = 6;
        let Some(role_enum) = lane_role_from_u32(role) else {
            return;
        };
        if packed.is_empty() || !packed.len().is_multiple_of(STRIDE) {
            self.remove_lane(role_enum);
            self.set_vector_stat(role_enum, 0);
            return;
        }
        let mut verts = Vec::with_capacity(packed.len() / STRIDE);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(lanes::LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        use wgpu::util::DeviceExt;
        let buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("hairline-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        #[allow(clippy::cast_possible_truncation)]
        let count = verts.len() as u32;
        self.set_vector_stat(role_enum, item_count);
        self.upsert_lane(
            role_enum,
            Batch {
                role: role_enum,
                visible,
                payload: BatchPayload::Lines(LineLane { verts: buf, count }),
            },
        );
    }

    /// Upload a world-meter axis-aligned marquee rect (T-151.4 L12; T-151.11.1 Deck parity —
    /// fill `[173,198,255,40]` + 1 px border `[173,198,255,200]`, the exact
    /// `useSelectionLayer.ts` oracle values; audit P-02).
    pub fn upload_marquee(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        visible: bool,
    ) {
        if !visible || min_x >= max_x || min_y >= max_y {
            self.remove_lane(LaneRole::Marquee);
            self.remove_lane(LaneRole::MarqueeOutline);
            return;
        }
        // Deck FILL: Aegis primary at α 40/255.
        let c = [173.0 / 255.0, 198.0 / 255.0, 1.0, 40.0 / 255.0];
        let corners = [
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ];
        let mut verts = Vec::with_capacity(4);
        for p in corners {
            verts.push(lanes::LineVertex {
                pos: [(p[0] - ANCHOR[0]) as f32, (p[1] - ANCHOR[1]) as f32],
                color: c,
            });
        }
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        use wgpu::util::DeviceExt;
        let vbuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        self.upsert_lane(
            LaneRole::Marquee,
            Batch {
                role: LaneRole::Marquee,
                visible: true,
                payload: BatchPayload::Polygon(PolyLane {
                    verts: vbuf,
                    indices: ibuf,
                    index_count: 6,
                    item_count: 1,
                }),
            },
        );
        // Deck LINE: 1 px hairline ring, α 200/255 (T-151.11.1 / P-02).
        let oc = [173.0 / 255.0, 198.0 / 255.0, 1.0, 200.0 / 255.0];
        let ring = [
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ];
        let mut outline = Vec::with_capacity(8);
        for e in 0..4 {
            let a = ring[e];
            let b = ring[(e + 1) % 4];
            outline.push(lanes::LineVertex {
                pos: [(a[0] - ANCHOR[0]) as f32, (a[1] - ANCHOR[1]) as f32],
                color: oc,
            });
            outline.push(lanes::LineVertex {
                pos: [(b[0] - ANCHOR[0]) as f32, (b[1] - ANCHOR[1]) as f32],
                color: oc,
            });
        }
        let obuf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-outline"),
                contents: bytemuck::cast_slice(&outline),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.upsert_lane(
            LaneRole::MarqueeOutline,
            Batch {
                role: LaneRole::MarqueeOutline,
                visible: true,
                payload: BatchPayload::Lines(LineLane {
                    verts: obuf,
                    count: 8,
                }),
            },
        );
    }

    /// Drop a W4 vector lane by role id (see `upload_polygon_mesh`). Role 7 (marquee) drops the
    /// border lane with the fill.
    pub fn clear_vector_lane(&mut self, role: u32) {
        if let Some(r) = lane_role_from_u32(role) {
            self.remove_lane(r);
            if r == LaneRole::Marquee {
                self.remove_lane(LaneRole::MarqueeOutline);
            }
            self.set_vector_stat(r, 0);
        }
    }

    /// The device's `maxTextureDimension2D` — JS `pickBaseLevel` uses it to choose the unified
    /// satellite base mip that fits this GPU (T-151.1 L4).
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// Re-tint a committed lane's opacity in place + toggle its visibility (role 0 basemap,
    /// 1 hillshade) — no texture rebuild (L6 cheap-memo / hybrid-style dim). The tint alpha is
    /// `color[3]` at byte offset 16 of the 1-instance quad buffer.
    pub fn set_lane_opacity(&mut self, role: u32, opacity: f32, visible: bool) {
        let want = if role == 0 {
            LaneRole::Satellite
        } else {
            LaneRole::Hillshade
        };
        let color = [1.0f32, 1.0, 1.0, opacity.clamp(0.0, 1.0)];
        let target = self.batches.iter_mut().find_map(|b| {
            if b.role == want {
                b.visible = visible;
                if let BatchPayload::Textured(l) = &b.payload {
                    return Some(l.instances.clone());
                }
            }
            None
        });
        if let Some(buf) = target {
            self.queue
                .write_buffer(&buf, 16, bytemuck::cast_slice(&color));
        }
    }

    /// Set the frame clear color (0..1 linear RGB) — the editor's map-style paper-tint underlay
    /// (T-151.1 L8). The default and the spike page stay at [`CLEAR_COLOR`].
    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 };
    }

    /// Hide the calibration batch (editor mount only — T-151.1 L1). The spike page never calls this,
    /// so its `self_check` + 20M stress scene are unchanged.
    pub fn hide_calibration(&mut self) {
        for b in &mut self.batches {
            if b.role == LaneRole::Calibration {
                b.visible = false;
            }
        }
    }

    /// Read back one texel `[r,g,b,a]` of the live scene at the **current** camera (T-151.1 L10) —
    /// resolves to JSON `{"x","y","backend","rgba":[…]}`. JS frames each corner via `set_view`
    /// first so the sampled pixel lands at a texel center (byte-exact, per the probe.rs margin
    /// argument). The `&self` borrow ends when this returns; the async map holds only cloned handles.
    pub fn readback_rgba(&self, x_px: u32, y_px: u32) -> js_sys::Promise {
        let w = self.config.width;
        let h = self.config.height;
        if x_px >= w || y_px >= h {
            return js_sys::Promise::reject(&JsValue::from_str("readback: pixel out of bounds"));
        }
        let padded = padded_bytes_per_row(w);
        let read_buf = self.encode_scene_readback(w, h, padded);
        let offset = u64::from(y_px * padded + x_px * 4);
        let device = self.device.clone();
        let backend = self.backend_kind.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            let rgba = map_read_4(&device, &read_buf, offset)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(JsValue::from_str(&format!(
                "{{\"x\":{},\"y\":{},\"backend\":\"{}\",\"rgba\":[{},{},{},{}]}}",
                x_px, y_px, backend, rgba[0], rgba[1], rgba[2], rgba[3],
            )))
        })
    }

    /// Byte-exact GPU self-check for the textured pipeline (T-151.1 L10): render a synthetic 2×2
    /// texture (NW red, NE green, SW blue, SE white) over a full-target quad at the fixed 800×600
    /// probe camera, read back the three corners, and assert them exactly. The NW corner reading
    /// **red** (not blue = Y-flip, not green = X-flip) proves the north-up UV. Resolves to JSON.
    pub fn texture_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let quad_layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let tex_bgl = self.tex_bind_group_layout.clone();
        let sampler = self.sampler.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;

            // 2×2 texture, row-major: row0 = [NW red, NE green], row1 = [SW blue, SE white].
            let texels: [u8; 16] = [
                255, 0, 0, 255, 0, 255, 0, 255, // row 0 (north)
                0, 0, 255, 255, 255, 255, 255, 255, // row 1 (south)
            ];
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tex-self-check"),
                size: wgpu::Extent3d {
                    width: 2,
                    height: 2,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                texture.as_image_copy(),
                &texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: Some(2),
                },
                wgpu::Extent3d {
                    width: 2,
                    height: 2,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Fixed probe camera (800×600, zoom 0, target = ANCHOR): the quad covers ±[400,300].
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tex-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tex-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });
            let tex_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tex-self-check-tex"),
                layout: &tex_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            let inst = QuadInstance {
                min: [-400.0, -300.0],
                max: [400.0, 300.0],
                color: [1.0, 1.0, 1.0, 1.0],
            };
            let inst_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tex-self-check-quad"),
                contents: bytemuck::cast_slice(&[inst]),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let tex_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("tex-self-check"),
                bind_group_layouts: &[Some(&cam_bgl), Some(&tex_bgl)],
                immediate_size: 0,
            });
            let pipeline = create_textured_pipeline(&device, &tex_layout, &shader, fmt);
            let _ = &quad_layout; // camera-only layout kept for parity with the live path

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tex-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let tview = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tex-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tex-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("tex-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_bind_group(1, &tex_bind, &[]);
                pass.set_vertex_buffer(0, unit_quad.slice(..));
                pass.set_vertex_buffer(1, inst_buf.slice(..));
                pass.draw(0..4, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            // Probes: NW (100,100) = red, NE (700,100) = green, SW (100,500) = blue.
            let probes: [(u32, u32, [u8; 4], &str); 3] = [
                (
                    100,
                    100,
                    [255, 0, 0, 255],
                    "NW (north-up proof: red, not blue/green)",
                ),
                (700, 100, [0, 255, 0, 255], "NE"),
                (100, 500, [0, 0, 255, 255], "SW"),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                let pass = got == expect;
                all_pass &= pass;
                json.push(format!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}",
                    px, py, expect[0], expect[1], expect[2], expect[3],
                    got[0], got[1], got[2], got[3], pass, label,
                ));
            }
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }

    /// Byte-exact GPU self-check for the W3 building fill pipeline (T-151.3 L14, Class GPU-R).
    /// Draws one synthetic OBB (center = ANCHOR, half [40,20], rot 37°, **fill α = 1.0**) over
    /// `CLEAR_COLOR` at the fixed 800×600 probe camera (rel `(dx,dy)` → pixel `(400+dx, 300−dy)`),
    /// then reads back three texels and asserts them exactly:
    /// - **(400,300)** center = `[38,38,44,255]` (FILL_DEFAULT rgb): α=1 collapses `ALPHA_BLENDING`
    ///   to `src·1 + dst·0` (exact), and `k/255` round-trips unorm8 (error `< 0.5`); the centroid is
    ///   ≥20 px interior for any rotation, so this is rotation-invariant and byte-exact.
    /// - **(460,300)** exterior = `CLEAR_COLOR [51,68,85,255]` (60 px > the `√(40²+20²)=44.7 px`
    ///   corner reach → outside for any rotation).
    /// - **(425,310)** = rel `(25,−10)`: inside the **+37°** OBB but outside the **−37°** OBB (both
    ///   ≥2 px from every edge → no rasterization ambiguity), so a wrong-sign shader reads clear here
    ///   → proves the rotation handedness matches `obb::obb_corners`. Resolves to JSON.
    pub fn world_building_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let layout = self.pipeline_layout.clone(); // camera-only (group 0), like the live pipeline
        let cam_bgl = self.bind_group_layout.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;

            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bld-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bld-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            let rad = (37.0_f64 * std::f64::consts::PI) / 180.0;
            let inst = scene::BuildingInstance {
                center: [0.0, 0.0],
                half: [40.0, 20.0],
                basis: [rad.cos() as f32, rad.sin() as f32],
                color: [38.0 / 255.0, 38.0 / 255.0, 44.0 / 255.0, 1.0],
            };
            let inst_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bld-self-check-inst"),
                contents: bytemuck::cast_slice(core::slice::from_ref(&inst)),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let pipeline = create_building_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bld-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let tview = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bld-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bld-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bld-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, unit_quad.slice(..));
                pass.set_vertex_buffer(1, inst_buf.slice(..));
                pass.draw(0..4, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            let probes: [(u32, u32, [u8; 4], &str); 3] = [
                (
                    400,
                    300,
                    [38, 38, 44, 255],
                    "center fill byte-exact (FILL_DEFAULT rgb)",
                ),
                (460, 300, [51, 68, 85, 255], "exterior = CLEAR_COLOR"),
                (
                    425,
                    310,
                    [38, 38, 44, 255],
                    "orientation +37 (inside +37, outside -37)",
                ),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                let pass = got == expect;
                all_pass &= pass;
                json.push(format!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}",
                    px, py, expect[0], expect[1], expect[2], expect[3],
                    got[0], got[1], got[2], got[3], pass, label,
                ));
            }
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }

    /// GPU-R sea-band fill probe (T-151.4 L11): draw a full-target quad tinted with the ≤0 m
    /// sea colour `[72,118,160,255]` via the polygon pipeline; center texel must be byte-exact.
    pub fn sea_band_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sea-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sea-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            // Full-viewport quad in anchor-relative meters covering the camera (zoom 0, 800×600).
            let half_w = f64::from(PW) * 0.5;
            let half_h = f64::from(PH) * 0.5;
            let sea = [72.0 / 255.0, 118.0 / 255.0, 160.0 / 255.0, 1.0];
            let corners = [
                [-half_w as f32, -half_h as f32],
                [half_w as f32, -half_h as f32],
                [half_w as f32, half_h as f32],
                [-half_w as f32, half_h as f32],
            ];
            let mut verts = Vec::with_capacity(4);
            for p in corners {
                verts.push(lanes::LineVertex { pos: p, color: sea });
            }
            let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
            let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sea-self-check-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sea-self-check-idx"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let pipeline = create_polygon_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sea-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let tview = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sea-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sea-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("sea-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..6, 0, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            let probes: [(u32, u32, [u8; 4], &str); 2] = [
                (
                    400,
                    300,
                    [72, 118, 160, 255],
                    "center = sea <=0m band colour",
                ),
                (50, 50, [72, 118, 160, 255], "corner interior still sea"),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                let pass = got == expect;
                all_pass &= pass;
                json.push(format!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}",
                    px, py, expect[0], expect[1], expect[2], expect[3],
                    got[0], got[1], got[2], got[3], pass, label,
                ));
            }
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }

    /// GPU-R road centerline probe (T-151.4 L11): horizontal strip of known colour through
    /// the screen centre; the centerline pixel is byte-exact.
    pub fn road_centerline_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("road-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("road-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            // Horizontal road strip: width 20 m, colour highway grey [200,200,200,255].
            // At zoom 0, 20 m = 20 px; strip spans full camera width at y=0.
            let half_w = f64::from(PW) * 0.5;
            let half_h = 10.0_f64; // half of 20 m width
            let road = [200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 1.0];
            let corners = [
                [-half_w as f32, -half_h as f32],
                [half_w as f32, -half_h as f32],
                [half_w as f32, half_h as f32],
                [-half_w as f32, half_h as f32],
            ];
            let mut verts = Vec::with_capacity(4);
            for p in corners {
                verts.push(lanes::LineVertex {
                    pos: p,
                    color: road,
                });
            }
            let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
            let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("road-self-check-verts"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("road-self-check-idx"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let pipeline = create_polygon_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("road-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let tview = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("road-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("road-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("road-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..6, 0, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            let probes: [(u32, u32, [u8; 4], &str); 2] = [
                (
                    400,
                    300,
                    [200, 200, 200, 255],
                    "centerline pixel = highway grey",
                ),
                (400, 50, [51, 68, 85, 255], "far exterior = CLEAR_COLOR"),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                let pass = got == expect;
                all_pass &= pass;
                json.push(format!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}",
                    px, py, expect[0], expect[1], expect[2], expect[3],
                    got[0], got[1], got[2], got[3], pass, label,
                ));
            }
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }

    /// GPU-R tree glyph probe (T-151.5 L11): synthetic solid white 1×1 atlas + one icon at
    /// ANCHOR with forest-green tint `[74,122,50,255]` and size 40 m → center texel matches
    /// tint (α=1 solid), exterior remains CLEAR_COLOR.
    pub fn tree_glyph_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let icon_bgl = self.icon_bind_group_layout.clone();
        let icon_layout = self.icon_pipeline_layout.clone();
        let unit_quad = self.unit_quad_buf.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tree-glyph-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tree-glyph-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            // 1×1 solid white atlas + full UV for glyph 0.
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tree-glyph-self-check-atlas"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                tex.as_image_copy(),
                &[255u8, 255, 255, 255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            let mut u_bytes = vec![0u8; ICON_UNIFORM_BYTES as usize];
            // uv[0] = (0,0,1,1); px_to_m = 1.0 at offset 456
            for (i, v) in [0.0f32, 0.0, 1.0, 1.0].iter().enumerate() {
                u_bytes[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            u_bytes[456..460].copy_from_slice(&1.0_f32.to_le_bytes());
            let uv_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tree-glyph-self-check-uv"),
                contents: &u_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let samp = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("tree-glyph-self-check-samp"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..wgpu::SamplerDescriptor::default()
            });
            let tview = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let atlas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tree-glyph-self-check-atlas"),
                layout: &icon_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tview),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&samp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uv_buf.as_entire_binding(),
                    },
                ],
            });

            // Forest green tint DEFAULT_GLYPH_RGBA = [74,122,50,255]
            let tint = 74u32 | (122u32 << 8) | (50u32 << 16) | (255u32 << 24);
            let inst = scene::IconInstance {
                pos: [0.0, 0.0],
                size: 40.0,
                yaw: 0,
                glyph: 0,
                tint,
            };
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("tree-glyph-self-check-inst"),
                contents: bytemuck::bytes_of(&inst),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let pipeline = create_icon_pipeline(&device, &icon_layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("tree-glyph-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("tree-glyph-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tree-glyph-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("tree-glyph-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_bind_group(2, &atlas_bg, &[]);
                pass.set_vertex_buffer(0, unit_quad.slice(..));
                pass.set_vertex_buffer(1, ibuf.slice(..));
                pass.draw(0..4, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            // Center (400,300) at zoom 0: 40 m icon → 40 px; interior solid green tint.
            // Exterior (400,50): well outside 20 px half-size.
            let probes: &[(u32, u32, [u8; 4], &str)] = &[
                (400, 300, [74, 122, 50, 255], "glyph center = forest tint"),
                (400, 50, [51, 68, 85, 255], "far exterior = CLEAR_COLOR"),
            ];
            let mut json = Vec::with_capacity(probes.len());
            let mut all_pass = true;
            for (px, py, expect, label) in probes {
                let offset = u64::from(py * padded + px * 4);
                let got = map_read_4(&device, &read_buf, offset)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                // Center: require nonzero α + RGB class match (exact for solid white×tint).
                let pass = if *label == "glyph center = forest tint" {
                    got[3] > 0 && got[0] == expect[0] && got[1] == expect[1] && got[2] == expect[2]
                } else {
                    got == *expect
                };
                all_pass &= pass;
                json.push(format!(
                    "{{\"px\":{},\"py\":{},\"expect\":[{},{},{},{}],\"got\":[{},{},{},{}],\"pass\":{},\"label\":\"{}\"}}",
                    px, py, expect[0], expect[1], expect[2], expect[3],
                    got[0], got[1], got[2], got[3], pass, label,
                ));
            }
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// GPU-R-adv marquee probe (T-151.11.1 / audit P-02): draws the Deck-parity marquee
    /// (fill `[173,198,255,40]` + 1 px border `[173,198,255,200]`) over `CLEAR_COLOR` at the
    /// fixed 800×600 probe camera with rect rel `[-100,-100]…[100,100]` (pixel edges x∈[300,500],
    /// y∈[200,400] — integer-aligned like the calibration scene) and reads back:
    /// - **(400,300)** interior = `blend(fill, clear)`,
    /// - **(300,300)** or **(299,300)** border column = `blend(border, blend(fill, clear))`
    ///   (either pixel accepted — a native 1 px line centered on an integer column may rasterize
    ///   to the left or right pixel; both are checked),
    /// - **(600,300)** exterior = `CLEAR_COLOR` byte-exact.
    ///
    /// Non-α-1 blends round through the GPU's float pipeline, so interior/border assert
    /// **±1 per channel** against the f64-computed expectation (documented GPU-R-adv, unlike the
    /// α=1 byte-exact checks). Resolves to JSON.
    pub fn marquee_self_check(&self) -> js_sys::Promise {
        const PW: u32 = 800;
        const PH: u32 = 600;
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();
        let layout = self.pipeline_layout.clone();
        let cam_bgl = self.bind_group_layout.clone();
        let backend = self.backend_kind.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            use wgpu::util::DeviceExt;
            let fmt = wgpu::TextureFormat::Rgba8Unorm;
            let camera = OrthoCamera::new(f64::from(PW), f64::from(PH), ANCHOR[0], ANCHOR[1], 0.0);
            let mvp = camera.wgpu_clip_matrix(ANCHOR[0], ANCHOR[1]);
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("marquee-self-check-mvp"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&mvp));
            let cam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("marquee-self-check-mvp"),
                layout: &cam_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            // Geometry: the exact upload_marquee construction, anchor-relative.
            let fill_c = [173.0_f32 / 255.0, 198.0 / 255.0, 1.0, 40.0 / 255.0];
            let line_c = [173.0_f32 / 255.0, 198.0 / 255.0, 1.0, 200.0 / 255.0];
            let (x0, y0, x1, y1) = (-100.0_f32, -100.0, 100.0, 100.0);
            let fill_verts = [
                lanes::LineVertex {
                    pos: [x0, y0],
                    color: fill_c,
                },
                lanes::LineVertex {
                    pos: [x1, y0],
                    color: fill_c,
                },
                lanes::LineVertex {
                    pos: [x1, y1],
                    color: fill_c,
                },
                lanes::LineVertex {
                    pos: [x0, y1],
                    color: fill_c,
                },
            ];
            let fill_idx: [u32; 6] = [0, 1, 2, 0, 2, 3];
            let ring = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
            let mut line_verts = Vec::with_capacity(8);
            for e in 0..4 {
                let a = ring[e];
                let b = ring[(e + 1) % 4];
                line_verts.push(lanes::LineVertex {
                    pos: a,
                    color: line_c,
                });
                line_verts.push(lanes::LineVertex {
                    pos: b,
                    color: line_c,
                });
            }
            let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-self-check-fill"),
                contents: bytemuck::cast_slice(&fill_verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-self-check-idx"),
                contents: bytemuck::cast_slice(&fill_idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let lbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marquee-self-check-line"),
                contents: bytemuck::cast_slice(&line_verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let polygon = create_polygon_pipeline(&device, &layout, &shader, fmt);
            let line = create_line_pipeline(&device, &layout, &shader, fmt);

            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("marquee-self-check-target"),
                size: wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let tview = target.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = padded_bytes_per_row(PW);
            let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("marquee-self-check-read"),
                size: u64::from(padded) * u64::from(PH),
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("marquee-self-check"),
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("marquee-self-check"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&polygon);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..6, 0, 0..1);
                pass.set_pipeline(&line);
                pass.set_bind_group(0, &cam_bind, &[]);
                pass.set_vertex_buffer(0, lbuf.slice(..));
                pass.draw(0..8, 0..1);
            }
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read_buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(PH),
                    },
                },
                wgpu::Extent3d {
                    width: PW,
                    height: PH,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));

            // f64 expectations through the exact blend chain (src·α + dst·(1−α), unorm8 round).
            let clear = [51.0_f64, 68.0, 85.0];
            let blend = |src: [f64; 3], alpha: f64, dst: [f64; 3]| -> [f64; 3] {
                [
                    src[0] * alpha + dst[0] * (1.0 - alpha),
                    src[1] * alpha + dst[1] * (1.0 - alpha),
                    src[2] * alpha + dst[2] * (1.0 - alpha),
                ]
            };
            let prim = [173.0_f64, 198.0, 255.0];
            let interior_f = blend(prim, 40.0 / 255.0, clear);
            // The 1 px border on the integer column x=300 rasterizes to pixel 299 (over CLEAR)
            // or pixel 300 (over the fill) depending on the line rule — both composites prove
            // the border color+alpha, so both are accepted (at either column).
            let border_over_fill_f = blend(prim, 200.0 / 255.0, interior_f);
            let border_over_clear_f = blend(prim, 200.0 / 255.0, clear);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let to_u8 = |v: [f64; 3]| -> [u8; 3] {
                [
                    v[0].round().clamp(0.0, 255.0) as u8,
                    v[1].round().clamp(0.0, 255.0) as u8,
                    v[2].round().clamp(0.0, 255.0) as u8,
                ]
            };
            let interior_e = to_u8(interior_f);
            let border_fill_e = to_u8(border_over_fill_f);
            let border_clear_e = to_u8(border_over_clear_f);
            let within = |got: [u8; 4], expect: [u8; 3]| -> bool {
                (0..3).all(|i| got[i].abs_diff(expect[i]) <= 1)
            };

            let read = |px: u32, py: u32| {
                let offset = u64::from(py * padded + px * 4);
                (px, py, offset)
            };
            let mut json = Vec::new();
            let mut all_pass = true;

            // Interior ±1.
            let (px, py, off) = read(400, 300);
            let got = map_read_4(&device, &read_buf, off)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            let pass = within(got, interior_e);
            all_pass &= pass;
            json.push(format!(
                "{{\"px\":{px},\"py\":{py},\"expect\":[{},{},{}],\"got\":[{},{},{},{}],\"tol\":1,\"pass\":{pass},\"label\":\"fill interior (adv ±1)\"}}",
                interior_e[0], interior_e[1], interior_e[2], got[0], got[1], got[2], got[3],
            ));

            // Border column: accept pixel 300 or 299, composited over fill OR clear.
            let mut border_pass = false;
            let mut border_got = [0u8; 4];
            for bx in [300u32, 299] {
                let (_, _, off) = read(bx, 300);
                let got = map_read_4(&device, &read_buf, off)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                if within(got, border_fill_e) || within(got, border_clear_e) {
                    border_pass = true;
                    border_got = got;
                    break;
                }
                border_got = got;
            }
            all_pass &= border_pass;
            json.push(format!(
                "{{\"px\":\"300|299\",\"py\":300,\"expect\":\"[{},{},{}] over fill | [{},{},{}] over clear\",\"got\":[{},{},{},{}],\"tol\":1,\"pass\":{border_pass},\"label\":\"border column (adv ±1)\"}}",
                border_fill_e[0], border_fill_e[1], border_fill_e[2],
                border_clear_e[0], border_clear_e[1], border_clear_e[2],
                border_got[0], border_got[1], border_got[2], border_got[3],
            ));

            // Exterior byte-exact clear.
            let (px, py, off) = read(600, 300);
            let got = map_read_4(&device, &read_buf, off)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            let pass = got == [51, 68, 85, 255];
            all_pass &= pass;
            json.push(format!(
                "{{\"px\":{px},\"py\":{py},\"expect\":[51,68,85,255],\"got\":[{},{},{},{}],\"pass\":{pass},\"label\":\"exterior = CLEAR_COLOR (byte-exact)\"}}",
                got[0], got[1], got[2], got[3],
            ));

            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{}\",\"probes\":[{}],\"pass\":{}}}",
                backend,
                json.join(","),
                all_pass,
            )))
        })
    }
}

#[wasm_bindgen]
impl RenderEngine {
    /// T-151.11.4 (audit X-03) — CPU==GPU compute-cull equality proof, self-contained:
    /// seeds a deterministic 512-icon field (LCG, the house constants), runs the compute cull
    /// against a pinned frustum on a THROWAWAY `IconComputeCull` (engine state untouched),
    /// awaits the real counter readback, and asserts `gpu == cpu` — both sides now share the
    /// f32 frustum domain, so equality is exact, not approximate.
    /// Resolves `{"backend","cpu","gpu","pass"}`; `{"skipped":true}` on WebGL2 (no compute).
    pub fn compute_cull_self_check(&self) -> js_sys::Promise {
        let backend = self.backend_kind.clone();
        if self.backend_kind == "webgl2" {
            return js_sys::Promise::resolve(&JsValue::from_str(&format!(
                "{{\"backend\":\"{backend}\",\"skipped\":true,\"pass\":true}}"
            )));
        }
        let device = self.device.clone();
        let queue = self.queue.clone();
        let shader = self.shader.clone();

        wasm_bindgen_futures::future_to_promise(async move {
            // Deterministic icon field: 512 icons over anchor-relative [-6400, 6400)².
            let mut src20 = Vec::with_capacity(512 * 20);
            let mut s: u32 = 0xC0FF_EE11;
            let mut unit = || {
                s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (s >> 8) as f32 / 16_777_216.0
            };
            for _ in 0..512 {
                let x = unit() * 12_800.0 - 6_400.0;
                let y = unit() * 12_800.0 - 6_400.0;
                let size = 2.0 + unit() * 14.0;
                src20.extend_from_slice(&x.to_le_bytes());
                src20.extend_from_slice(&y.to_le_bytes());
                src20.extend_from_slice(&size.to_le_bytes());
                src20.extend_from_slice(&0_i16.to_le_bytes());
                src20.extend_from_slice(&0_u16.to_le_bytes());
                src20.extend_from_slice(&0xFF00_FF00_u32.to_le_bytes());
            }
            let frustum = [-1_234.5_f64, -987.25, 2_345.75, 1_876.5];

            let mut cull = crate::icon_cull_gpu::IconComputeCull::create(&device, &shader);
            cull.upload_icons(&device, &queue, &src20);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("cull-self-check"),
            });
            cull.encode_cull(&mut encoder, &device, &queue, frustum);
            queue.submit(Some(encoder.finish()));
            let cpu = cull.last_cpu_count;

            // Map the counter readback directly (bounded poll/yield, like map_read_4).
            let done = Rc::new(Cell::new(0u8));
            {
                let done = done.clone();
                cull.readback_buf
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |res| {
                        done.set(if res.is_ok() { 1 } else { 2 });
                    });
            }
            let mut ticks = 0;
            while done.get() == 0 {
                let _ = device.poll(wgpu::PollType::Poll);
                readback_sleep_ms(4).await;
                ticks += 1;
                if ticks > 2000 {
                    return Err(JsValue::from_str("cull-self-check: readback timeout"));
                }
            }
            if done.get() == 2 {
                return Err(JsValue::from_str("cull-self-check: readback map failed"));
            }
            let gpu = {
                let data = cull.readback_buf.slice(..).get_mapped_range();
                u32::from_le_bytes(data[0..4].try_into().expect("4 bytes"))
            };
            cull.readback_buf.unmap();

            let pass = gpu == cpu && cpu > 0 && cpu < 512;
            Ok(JsValue::from_str(&format!(
                "{{\"backend\":\"{backend}\",\"cpu\":{cpu},\"gpu\":{gpu},\"pass\":{pass}}}"
            )))
        })
    }
}
