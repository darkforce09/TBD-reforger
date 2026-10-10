**Status:** live

# GPU rendering overview

The graphics crates are the web platform's renderer: `wgpu` libraries that draw what they are
told and know no map concept. [`gpu_device`](/crates/graphics/gpu_device/README.md) stands up the
GPU of a browser canvas and keeps the long-lived buffer bookkeeping;
[`gpu_frame`](/crates/graphics/gpu_frame/README.md) defines the frame vocabulary a caller
describes a frame in and supplies the pipelines, the vertex uploads, the encoder, GPU sprite
culling, the swapchain submit and the animation-frame loop. The GPU-free half they build on
(instance layouts, geometry and triangulation, glyph packing, frame ids, damage tracking and the
WGSL shader) is [`render_primitives`](/crates/graphics/render_primitives/README.md). Their one
callers are the map renderer, which draws the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map with them, and the
Arsenal's paper doll renderer.

## Where it lives

- Code: [`crates/graphics/gpu_device/`](/crates/graphics/gpu_device/README.md) and
  [`crates/graphics/gpu_frame/`](/crates/graphics/gpu_frame/README.md), whose READMEs give the
  commands, the dependencies and the public surface; the
  [`gpu_frame` source](/crates/graphics/gpu_frame/src/README.md), whose README draws the frame
  flow; [`crates/graphics/render_primitives/`](/crates/graphics/render_primitives/README.md) for
  the GPU-free building blocks.
- Entry: the map renderer's `RenderEngine` (`crates/map_rendering/map_renderer/`), which creates
  the device and surface through `GpuContext`, builds its pipelines with
  `pipeline::create_render_shader` and the pipeline constructors, keeps the sorted batch list, and
  implements `FrameTarget` so that `RafPump` drives it
  (`crates/map_rendering/map_renderer/src/pump.rs`); the paper doll renderer
  (`crates/paper_doll/paper_doll_renderer/`) creates its GPU through the same `GpuContext`.
- Related features: the [map rendering overview](/documentation/crates/map_rendering/map_rendering_overview.md)
  and the [crate boundary rules](/documentation/standards/crate_boundary_rules.md) that keep
  these crates free of map concepts.

## Behaviour

### Crates and modules

| Crate and module | What it holds | README |
|---|---|---|
| `gpu_device::context` | `GpuContext` (create, resize, acquire, adapter facts), `acquire_frame`, `WebDisplay`, `instance_descriptor` and the surface policy | [context](/crates/graphics/gpu_device/src/context/README.md) |
| `gpu_device::buffers` | `LanePool` and `ReadbackLane`: persistent lane buffers and the readback guard | [buffers](/crates/graphics/gpu_device/src/buffers/README.md) |
| `gpu_device::timing` | `GpuTimer`: the timestamp queries of one render pass | [timing](/crates/graphics/gpu_device/src/timing/README.md) |
| `gpu_frame::frame` | the GPU frame vocabulary (`FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`, `TextRun`, the buffer types), the cell atlases and the swapchain `present::submit` | [frame](/crates/graphics/gpu_frame/src/frame/README.md) |
| `gpu_frame::draw` | vertex uploads, `encode::encode` and the compute cull in `draw::cull` | [draw](/crates/graphics/gpu_frame/src/draw/README.md), [cull](/crates/graphics/gpu_frame/src/draw/cull/README.md) |
| `gpu_frame::pipeline` | `create_render_shader` and ten pipeline constructors | [pipeline](/crates/graphics/gpu_frame/src/pipeline/README.md) |
| `gpu_frame::frame_pump` | `FrameTarget` and `RafPump`, the animation-frame loop | [frame pump](/crates/graphics/gpu_frame/src/frame_pump/README.md) |
| `renderer_core` | `LaneSink`, `LayerContext`, `FrameHook`/`FrameHooks`, `RenderStats`, `StatsJson` and the frame packet's binding ids (`packet_bindings`) | [renderer core source](/crates/graphics/renderer_core/src/README.md) |
| `render_primitives::shaders` | `SHADER_WGSL`, the one WGSL program every pipeline and the compute cull use | [shaders](/crates/graphics/render_primitives/src/shaders/README.md) |
| `render_primitives::text` | the ASCII atlas bake, glyph metrics, layout, decluttering and sprite packing | [text](/crates/graphics/render_primitives/src/text/README.md) |
| `render_primitives::draw` and `frame` | triangulation, composition, the grid, instance layouts, the cull oracle, the ids, damage tracking and the camera uniform | [draw](/crates/graphics/render_primitives/src/draw/README.md), [frame](/crates/graphics/render_primitives/src/frame/README.md) |

Each crate's `lib.rs` declares every module without a gate; each file that names a GPU or
browser type gates itself on `wasm32`. The native builds keep the pump's `tick`, the buffer
pools, the readback guard, the surface policy and the compute cull's source check, which the
native tests cover.

### One frame

1. `RafPump` calls the target's `render_frame` on each animation frame, then polls the device. A
   target still borrowed from a previous frame skips the frame rather than panicking; a disposed
   flag stops the loop.
2. The caller asks `RenderDamage` whether to submit: it submits while the frame is dirty or
   continuous rendering is on, and it starts dirty, so the first frame always draws.
3. The acquire (`gpu_device::acquire_frame`, or `GpuContext::acquire`) gets the next swapchain
   image. `Timeout` and `Occluded` skip the frame; an outdated or lost surface is reconfigured
   once.
4. The caller wraps its sorted batch list in a `FramePacket` with the camera, the clear colour,
   the text runs, the indirect draws, the pipeline table and a sparse bind-group table, and hands
   it to `draw::encode::encode` once. The encoder draws in lane order, merging indirect draws and
   glyph runs by lane, skips an invisible batch or one whose pipeline or bind group is missing,
   and binds by payload shape alone.
5. `present::submit` resolves the timestamp query of the `GpuTimer` when the caller samples one,
   submits and presents; `after_submit` clears the damage unless the loop runs continuously.

### The GPU context

`GpuContext::create(canvas, force_webgl, label, want_timestamps)` is the one bootstrap of a
canvas: WebGPU detection with a WebGL2 fallback (or WebGL2 alone when forced), a high-performance
adapter, a device under the caller's label with timestamp queries when wanted and supported and,
on WebGL2, the downlevel limits raised to the adapter's resolution limits, and a surface
configured with the first non-sRGB format and `Fifo`. It keeps the backend kind, the adapter's
limits (the largest 2D texture side among them) and whether timestamps are on. `resize` takes
device pixels after the caller's own rounding and clamping and refuses a zero side. Every failure
is a `gpu_device::Error` whose message starts with a stable code.

### Sprite culling

On WebGPU the caller can cull sprite lanes on the GPU: `draw::cull::compute` packs each lane's
20-byte sprites into 32-byte storage records, dispatches `cs_icon_cull` in workgroups of 64, and
writes the survivor count straight into the `draw_indirect` arguments, so the draw needs no CPU
round trip. `render_primitives::draw::cull::oracle` is the CPU reference the tests and the debug
readout compare against.

### Conventions

Positions are anchor-relative metres, cast to f32 after the caller's anchor is subtracted, so
numbers stay small at every zoom; colours are linear 0 to 1 on a non-sRGB target; bind group 0 is
the camera, 1 a textured quad's texture, 2 an atlas. No pipeline has a depth or stencil state, and
each draws single-sampled.

### Known discrepancies

- `render_primitives`' `BuildingInstance` and the WGSL entry points `vs_building`,
  `fs_building` and `fs_forest_density` (`crates/graphics/render_primitives/src/shaders/shader.wgsl`)
  still name map concepts in crates that know none; the no-map-noun rule checks five nouns and lets
  them through.

## Data

The crates read no file, environment variable or feature flag, and make no network call. Their
inputs are the caller's values: the canvas, geometry and instances in the byte layouts of
`render_primitives`, RGBA8 atlas pixels and the camera matrix. The byte layouts are the shared
binary contract with the map crates' upload belts; changing one is a change to both sides.

## Design

- A renderer with no domain: the caller decides what to draw, in which order and with which
  pipeline, and keys each batch on an opaque `LaneId` whose value is only compared; lane roles and
  paint order live in the `map_draw_lanes` crate's `lane_roles` module.
- One bootstrap: device creation, surface configuration, resize and acquire live once, in
  `gpu_device`, below the frame code that draws with them; `gpu_frame` takes the device and queue
  as arguments and never creates them.
- Damage-driven: nothing is submitted unless something changed or continuous rendering is on,
  and the packet is borrowed, not rebuilt, each frame
  ([crate boundary rules §2C](/documentation/standards/crate_boundary_rules.md#2c-the-packet-boundary)).
- Design target: the pure renderer of the
  [crate boundary rules](/documentation/standards/crate_boundary_rules.md#3-the-graphics-category-a-pure-renderer).
  Differences: `RenderEngine`, which holds the device, surface and every GPU resource, lives in
  the map renderer (`crates/map_rendering/map_renderer/`); and some declared names carry map
  nouns (the known discrepancies above).

## Open work

- Remove map nouns from graphics engine names; widen rule 2 (ticket `remove-map-nouns-graphics` in
  `ttm`): the map-named items get geometry names (instanced box, density raster, textured quad,
  contrast mode) and rule 2 checks the wider noun list.
- Remove unused text parameters and fix stale graphics engine comments (ticket
  `remove-unused-text-parameters` in `ttm`): the ignored `_tint` and `char_m`, the 28-entry UV
  comment and the placeholder module headers go.
- Engine and wasm performance (ticket `engine-wasm-performance` in `ttm`): pooled lane buffers and
  GPU culling for every icon lane.
- Fix world line-of-sight bench frame pump running after unmount (ticket `fix-world-line-sight` in
  `ttm`): the debug bench sets the pump's disposed flag on unmount, so `RafPump` stops.

## Decisions

- The frame vocabulary lives in `gpu_frame`: the graphics crates define the shape of a frame, and
  the map renderer and its typed layers import it directly.
- One call per frame: `encode` takes the whole packet, so the crate boundary costs one call, not
  one per batch.
- The acquire belongs to the device crate and the submit to the frame crate: `GpuContext` owns
  the surface the acquire reconfigures, and the frame crate stays a consumer of the device below
  it.
- Browser code is selected by the `wasm32` target, not by a feature: the crates have no features.
  The CPU half is its own crate, `render_primitives`, which builds and tests on every target.
