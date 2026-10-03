**Status:** live

# Graphics engine overview

`graphics_engine` is the web platform's renderer: a `wgpu` library that draws what it is
told and knows no map concept. It defines the frame vocabulary a caller describes a frame in, and
supplies the pipelines, the vertex uploads, the encoder, GPU sprite culling, the swapchain steps
and the animation-frame loop. The GPU-free half it builds on (instance layouts, geometry and
triangulation, glyph packing, frame ids, damage tracking and the WGSL shader) is the
[`render_primitives`](/crates/graphics/render_primitives/README.md) crate. Its one caller is the map engine, which draws the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map with it.

## Where it lives

- Code: [`legacy/graphics_engine/`](/legacy/graphics_engine/README.md), whose README
  gives the commands, the dependencies and the public surface;
  [`src/`](/legacy/graphics_engine/src/README.md), whose README gives the six modules and
  the frame flow; [`crates/graphics/render_primitives/`](/crates/graphics/render_primitives/README.md)
  for the GPU-free building blocks.
- Entry: the map engine's `RenderEngine` (`legacy/map_engine/src/frame/`), which creates the
  device and surface, builds its pipelines with `pipeline::create_map_shader` and the pipeline
  constructors, keeps the sorted batch list, and implements `r#loop::FrameTarget` so that
  `RafPump` drives it (`legacy/map_engine/src/frame/pump.rs`).
- Related features: the [map engine overview](/documentation/legacy/map_engine/map_engine_overview.md)
  and the [engine boundary rules](/documentation/standards/engine_boundary_rules.md) that keep
  this crate free of map concepts.

## Behaviour

### Modules

| Module | What it holds | README |
|---|---|---|
| `frame` | the GPU frame vocabulary (`FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`, `TextRun`, the buffer types), the cell atlases and the swapchain `present` step | [frame](/legacy/graphics_engine/src/frame/README.md) |
| `draw` | vertex uploads, `encode::encode` and the compute cull in `draw::cull` | [draw](/legacy/graphics_engine/src/draw/README.md), [cull](/legacy/graphics_engine/src/draw/cull/README.md) |
| `pipeline` | `create_map_shader` and ten pipeline constructors | [pipeline](/legacy/graphics_engine/src/pipeline/README.md) |
| `render_primitives::shaders` (crate) | `SHADER_WGSL`, the one WGSL program every pipeline and the compute cull use | [shaders](/crates/graphics/render_primitives/src/shaders/README.md) |
| `render_primitives::text` (crate) | the ASCII atlas bake, glyph metrics, layout, decluttering and sprite packing | [text](/crates/graphics/render_primitives/src/text/README.md) |
| `render_primitives::draw` and `frame` (crate) | triangulation, composition, the grid, instance layouts, the cull oracle, the ids, damage tracking and the camera uniform | [draw](/crates/graphics/render_primitives/src/draw/README.md), [frame](/crates/graphics/render_primitives/src/frame/README.md) |
| `device` | `LanePool` and `ReadbackLane`: persistent lane buffers and the readback guard | [device](/legacy/graphics_engine/src/device/README.md) |
| `loop` (`r#loop`) | `FrameTarget` and `RafPump`, the animation-frame loop | [loop](/legacy/graphics_engine/src/loop/README.md) |

`lib.rs` declares every module without a gate; each file that names a GPU or browser type gates
itself on `wasm32`. The CPU half (triangulation, composition, instance layouts, text, damage and
the cull oracle) lives in `render_primitives`, whose tests run natively; the native build of this
crate keeps the pump's `tick`, the buffer pools and the compute cull's source check.

### One frame

1. `RafPump` calls the target's `render_frame` on each animation frame, then polls the device. A
   target still borrowed from a previous frame skips the frame rather than panicking; a disposed
   flag stops the loop.
2. The caller asks `RenderDamage` whether to submit: it submits while the frame is dirty or
   continuous rendering is on, and it starts dirty, so the first frame always draws.
3. `present::acquire` gets the next swapchain image. `Timeout` and `Occluded` skip the frame; an
   outdated or lost surface is reconfigured once.
4. The caller wraps its sorted batch list in a `FramePacket` with the camera, the clear colour,
   the text runs, the indirect draws, the pipeline table and a sparse bind-group table, and hands
   it to `draw::encode::encode` once. The encoder draws in lane order, merging indirect draws and
   glyph runs by lane, skips an invisible batch or one whose pipeline or bind group is missing,
   and binds by payload shape alone.
5. `present::submit` resolves the timestamp query when the caller samples one, submits and
   presents; `after_submit` clears the damage unless the loop runs continuously.

### Sprite culling

On WebGPU the caller can cull sprite lanes on the GPU: `draw::cull::compute` packs each lane's
20-byte sprites into 32-byte storage records, dispatches `cs_icon_cull` in workgroups of 64, and
writes the survivor count straight into the `draw_indirect` arguments, so the draw needs no CPU
round trip. `render_primitives::draw::cull::oracle` is the CPU reference the tests and the debug readout
compare against.

### Conventions

Positions are anchor-relative metres, cast to f32 after the caller's anchor is subtracted, so
numbers stay small at every zoom; colours are linear 0 to 1 on a non-sRGB target; bind group 0 is
the camera, 1 a textured quad's texture, 2 an atlas. No pipeline has a depth or stencil state, and
each draws single-sampled.

### Known discrepancies

- `CLAUDE.md:168-171` lists `gpu_context/`, `render_passes/` and `frame_loop/`; the crate's
  modules are `device`, `draw`, `frame`, `loop` and `pipeline`
  (`legacy/graphics_engine/src/lib.rs`), with `render_primitives` beside them, and adapter, device, queue and surface creation
  live in the map engine (`legacy/map_engine/src/frame/boot.rs`).
- Declared names such as `BuildingInstance`, `create_building_pipeline`,
  `create_forest_density_pipeline` and `create_map_shader`, and the `slot-icon-lane` buffer label
  (`src/device/buffers/pool.rs:123`), name map concepts in a crate `CLAUDE.md` law 6 says knows
  none; rule 2 of the gate checks five nouns and lets them through.

## Data

The crate reads no file, environment variable or feature flag, and makes no network call. Its
inputs are the caller's values: geometry and instances in the byte layouts of `render_primitives`, RGBA8
atlas pixels, the camera matrix, and the `wgpu::Device` and `wgpu::Queue` the caller created. The
byte layouts are the shared binary contract with the map engine's upload belts; changing one is a
change to both crates.

## Design

- A renderer with no domain: the caller decides what to draw, in which order and with which
  pipeline, and keys each batch on an opaque `LaneId` whose value is only compared; lane roles and
  paint order live in the `map_draw_lanes` crate's `lane_roles` module.
- Damage-driven: nothing is submitted unless something changed or continuous rendering is on,
  and the packet is borrowed, not rebuilt, each frame
  ([engine boundary rules §2C](/documentation/standards/engine_boundary_rules.md#2c-the-packet-boundary)).
- Design target: the pure renderer of the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md#3-the-graphics-engine-a-pure-renderer).
  Differences: `RenderEngine`, which holds the device, surface and every GPU resource, lives in
  the map engine, so the map engine still names `device`, `pipeline` and `r#loop` at five pinned
  sites (rule 3b); and some declared names carry map nouns (the known discrepancies above).

## Open work

- [T-1070 — Remove map nouns from graphics engine names; widen rule 2](/.ai/tickets/T-1070.toml)
  (idea, no plan): the map-named items get geometry names (instanced box, density raster,
  textured quad, contrast mode) and rule 2 checks the wider noun list.
- [T-1078 — Remove unused text parameters and fix stale graphics engine comments](/.ai/tickets/T-1078.toml)
  (idea, no plan): the ignored `_tint` and `char_m`, the 28-entry UV comment, the dead
  `crate::scene` link and the placeholder module headers go.
- [T-1055 — Fix engine-layers gate omitting the map engine editing module](/.ai/tickets/T-1055.toml)
  (idea, no plan): among its fixes, rules 4 and 7 gain the map engine's `editing` module.
- [T-938 — Engine and wasm performance](/documentation/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation/tickets/plans/t-938_plan.md)): pooled lane buffers and GPU
  culling for every icon lane.
- [T-1039 — Fix world line-of-sight bench frame pump running after unmount](/.ai/tickets/T-1039.toml)
  (idea, no plan): the debug bench sets the pump's disposed flag on unmount, so `RafPump` stops.

## Decisions

- The frame vocabulary lives here and the map engine re-exports it: the renderer defines the
  shape of a frame, and the map engine's whole graphics interface is one list in its
  `frame/mod.rs`.
- One call per frame: `encode` takes the whole packet, so the crate boundary costs one call, not
  one per batch.
- Browser code is selected by the `wasm32` target, not by a feature: the crate has no features.
  The CPU half is its own crate, `render_primitives`, which builds and tests on every target.
- The shared byte layouts are one crate (`render_primitives`): the binary contract between the
  crates reads in one place, and the map engine's belts import it directly rather than through
  its `frame/`.
