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

use crate::lanes;
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

/// A batch's role — governs the fixed W1 draw order (basemap → hillshade → grid) via
/// [`lane_order`] and lets the editor find/replace a lane in place on LOD / opacity change.
/// `Stress`/`Calibration` are the T-151.0 spike batches (never mixed with the editor lanes).
#[derive(Clone, Copy, PartialEq)]
enum LaneRole {
    Stress,
    Calibration,
    Satellite,
    /// W4 sea underlay (after basemap, before hillshade).
    Sea,
    Hillshade,
    /// W4 land-cover hulls.
    Landcover,
    Contours,
    RoadsCasing,
    Roads,
    /// W3 world-building OBB fills (`world-buildings`).
    WorldBuildings,
    /// W3 world-building outline casing (`world-buildings-outline`).
    WorldBuildingsOutline,
    ForestFill,
    ForestOutline,
    /// W5 tree + vegetation glyphs.
    WorldTrees,
    /// W5 prop + rockLarge glyphs.
    WorldProps,
    /// W5 building badges.
    WorldBadges,
    /// W6 mission slot rings.
    Slots,
    /// W6 drag-preview overlay (T-061).
    SlotDrag,
    /// W6 cluster discs (T-065).
    Clusters,
    Grid,
    /// Optional selection marquee (on top of grid).
    Marquee,
}

/// Draw-order key (T-151.6 L2): … badges → slots → slot-drag → clusters → grid → marquee.
/// Spike batches sort first, never interleaved.
fn lane_order(role: LaneRole) -> u8 {
    match role {
        LaneRole::Stress | LaneRole::Calibration => 0,
        LaneRole::Satellite => 1,
        LaneRole::Sea => 2,
        LaneRole::Hillshade => 3,
        LaneRole::Landcover => 4,
        LaneRole::Contours => 5,
        LaneRole::RoadsCasing => 6,
        LaneRole::Roads => 7,
        LaneRole::WorldBuildings => 8,
        LaneRole::WorldBuildingsOutline => 9,
        LaneRole::ForestFill => 10,
        LaneRole::ForestOutline => 11,
        LaneRole::WorldTrees => 12,
        LaneRole::WorldProps => 13,
        LaneRole::WorldBadges => 14,
        LaneRole::Slots => 15,
        LaneRole::SlotDrag => 16,
        LaneRole::Clusters => 17,
        LaneRole::Grid => 18,
        LaneRole::Marquee => 19,
    }
}

/// Map a public role u32 (upload API) → [`LaneRole`]. Returns `None` for unknown ids.
fn lane_role_from_u32(role: u32) -> Option<LaneRole> {
    Some(match role {
        0 => LaneRole::Sea,
        1 => LaneRole::Landcover,
        2 => LaneRole::Contours,
        3 => LaneRole::RoadsCasing,
        4 => LaneRole::Roads,
        5 => LaneRole::ForestFill,
        6 => LaneRole::ForestOutline,
        7 => LaneRole::Marquee,
        _ => return None,
    })
}

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

/// Draw the ordered batch list into `pass` (shared by the live `render()` and the offscreen
/// readback path — T-151.1 L1/L10). Group 0 (the camera mvp) is compatible across all pipeline
/// layouts, so it is bound once. The pipelines are passed in because the live path uses the
/// surface-format pipelines and the readback path rebuilds them at `Rgba8Unorm`.
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
) {
    // group 0 (camera mvp) is set after each `set_pipeline` — its layout is identical across all
    // pipelines, but binding it per-batch (pipeline → groups → buffers → draw) is the always-valid
    // order on both the WebGPU and WebGL2 backends.
    for batch in batches {
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
    /// Dedicated slot/cluster atlas (None until `upload_slot_atlas`).
    slot_atlas: Option<SlotAtlasGpu>,
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
        let tree_glyphs: u32 = self
            .batches
            .iter()
            .filter(|b| b.role == LaneRole::WorldTrees)
            .map(|b| match &b.payload {
                BatchPayload::IconInstanced { count, .. } => *count,
                _ => 0,
            })
            .sum();
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
                "\"slot_instances\":{},\"slot_drag_instances\":{},\"cluster_instances\":{}}}"
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
    }

    /// Drop every batch of `role` (its GPU resources free on `Drop`).
    fn remove_lane(&mut self, role: LaneRole) {
        self.batches.retain(|b| b.role != role);
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

    /// Drop both world-building lanes (gate closed below the building band, or terrain switch).
    pub fn clear_world_buildings(&mut self) {
        self.remove_lane(LaneRole::WorldBuildings);
        self.remove_lane(LaneRole::WorldBuildingsOutline);
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
    pub fn upload_icon_lane(&mut self, kind: u32, bytes: &[u8], visible: bool) {
        let role = match kind {
            0 => LaneRole::WorldTrees,
            1 => LaneRole::WorldProps,
            2 => LaneRole::WorldBadges,
            _ => return,
        };
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

    /// Upload dedicated slot/cluster atlas (ring + disc). `uv` is 2..28 glyphs × 4 floats
    /// (minU,minV,maxU,maxV); padded to 28. Does **not** touch the world-glyphs atlas.
    pub fn upload_slot_atlas(
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

    /// Update slot atlas `px_to_m = 2^(-zoom)` on both base + drag uniforms (no instance re-upload).
    pub fn set_slot_px_to_m(&mut self, px_to_m: f32) {
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
    pub fn set_slot_drag_delta(&mut self, dx: f32, dy: f32) {
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
    pub fn upload_slot_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Slots, bytes, visible);
    }

    /// Dirty-range patch into the Slots lane buffer (byte offset must be 20-aligned).
    pub fn patch_slot_lane(&mut self, byte_offset: u32, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
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
        // Convert world→anchor if this looks like a full instance or pos prefix.
        // Callers pass already-anchor-relative for size/tint-only patches at offset+8.
        // For full 20 B rows starting at stride boundaries with world coords, convert.
        let mut converted = bytes.to_vec();
        if byte_offset.is_multiple_of(20) && converted.len().is_multiple_of(20) {
            // Heuristic: if |pos| > 6400 something could be world — always convert full rows
            // as world meters (SoA contract). Size/tint-only patches use non-zero offset within row.
            Self::convert_icon_world_to_anchor(&mut converted);
        }
        self.queue
            .write_buffer(instances, u64::from(byte_offset), &converted);
    }

    /// Upload SlotDrag overlay instances (world meters). Empty → drop lane.
    pub fn upload_slot_drag_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::SlotDrag, bytes, visible);
    }

    /// Clear the drag overlay lane.
    pub fn clear_slot_drag_lane(&mut self) {
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
    pub fn upload_cluster_lane(&mut self, bytes: &[u8], visible: bool) {
        self.upload_slot_role_lane(LaneRole::Clusters, bytes, visible);
    }

    /// Drop slots + drag + cluster lanes.
    pub fn clear_slot_lanes(&mut self) {
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

    /// Upload a world-meter axis-aligned marquee rect (T-151.4 L12).
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
            return;
        }
        // Aegis primary tint α≈0.24
        let c = [173.0 / 255.0, 198.0 / 255.0, 1.0, 60.0 / 255.0];
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
    }

    /// Drop a W4 vector lane by role id (see `upload_polygon_mesh`).
    pub fn clear_vector_lane(&mut self, role: u32) {
        if let Some(r) = lane_role_from_u32(role) {
            self.remove_lane(r);
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
