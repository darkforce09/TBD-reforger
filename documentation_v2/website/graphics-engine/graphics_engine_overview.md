**Status:** live

# Graphics engine overview

`website-graphics-engine` is the web platform's renderer: a `wgpu` library that draws what it is
told and knows no map concept. It defines the frame vocabulary a caller describes a frame in, and
supplies the pipelines, the WGSL shader, geometry and text packing, sprite culling, the swapchain
steps and the animation-frame loop. Its one caller is the map engine, which draws the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map with it.

## Where it lives

- Code: [`apps/website/graphics-engine/`](/apps/website/graphics-engine/README.md), whose README
  gives the commands, the dependencies and the public surface;
  [`src/`](/apps/website/graphics-engine/src/README.md), whose README gives the eight modules and
  the frame flow.
- Entry: the map engine's `RenderEngine` (`apps/website/map-engine/src/frame/`), which creates the
  device and surface, builds its pipelines with `pipeline::create_map_shader` and the pipeline
  constructors, keeps the sorted batch list, and implements `r#loop::FrameTarget` so that
  `RafPump` drives it (`apps/website/map-engine/src/frame/pump.rs`).
- Related features: the [map engine overview](/documentation_v2/website/map-engine/map_engine_overview.md)
  and the [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) that keep
  this crate free of map concepts.

## Behaviour

### Modules

| Module | What it holds | README |
|---|---|---|
| `frame` | the frame vocabulary (`FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`, `TextRun`, the buffer types, `LaneId`, `PipelineId`, `BindGroupId`, `CameraUniform`), `RenderDamage`, the cell atlases and the swapchain `present` step | [frame](/apps/website/graphics-engine/src/frame/README.md) |
| `draw` | triangulation, mesh and hairline composition, the grid, instance layouts, vertex uploads, `encode::encode`, and the sprite cull pair in `draw::cull` | [draw](/apps/website/graphics-engine/src/draw/README.md), [cull](/apps/website/graphics-engine/src/draw/cull/README.md) |
| `pipeline` | `create_map_shader` and ten pipeline constructors | [pipeline](/apps/website/graphics-engine/src/pipeline/README.md) |
| `shaders` | `SHADER_WGSL`, the one WGSL program every pipeline and the compute cull use | [shaders](/apps/website/graphics-engine/src/shaders/README.md) |
| `text` | the ASCII atlas bake, glyph metrics, layout, decluttering and sprite packing | [text](/apps/website/graphics-engine/src/text/README.md) |
| `layout` | every shared byte layout, re-exported as one list | [layout](/apps/website/graphics-engine/src/layout/README.md) |
| `device` | `LanePool` and `ReadbackLane`: persistent lane buffers and the readback guard | [device](/apps/website/graphics-engine/src/device/README.md) |
| `loop` (`r#loop`) | `FrameTarget` and `RafPump`, the animation-frame loop | [loop](/apps/website/graphics-engine/src/loop/README.md) |

`lib.rs` declares every module without a gate; each file that names a GPU or browser type gates
itself on `wasm32`. A native build keeps the CPU half (triangulation, composition, instance
layouts, text, damage, the cull oracle and the pump's `tick`), which the native tests cover.

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
round trip. `draw::cull::oracle` is the CPU reference the tests and the debug readout compare
against.

### Conventions

Positions are anchor-relative metres, cast to f32 after the caller's anchor is subtracted, so
numbers stay small at every zoom; colours are linear 0 to 1 on a non-sRGB target; bind group 0 is
the camera, 1 a textured quad's texture, 2 an atlas. No pipeline has a depth or stencil state, and
each draws single-sampled.

### Known discrepancies

- `CLAUDE.md:168-171` lists `gpu_context/`, `render_passes/` and `frame_loop/`; the crate's
  modules are `device`, `draw`, `frame`, `layout`, `loop`, `pipeline`, `shaders` and `text`
  (`apps/website/graphics-engine/src/lib.rs`), and adapter, device, queue and surface creation
  live in the map engine (`apps/website/map-engine/src/frame/boot.rs`).
- Declared names such as `BuildingInstance`, `create_building_pipeline`,
  `create_forest_density_pipeline` and `create_map_shader`, and the `slot-icon-lane` buffer label
  (`src/device/buffers/pool.rs:123`), name map concepts in a crate `CLAUDE.md` law 6 says knows
  none; rule 2 of the gate checks five nouns and lets them through.
- `src/draw/instances.rs:62` describes a 28-entry UV table; the shader's table has 32 entries.
- `src/draw/cull/oracle.rs:11` links a `crate::scene` that does not exist.

## Data

The crate reads no file, environment variable or feature flag, and makes no network call. Its
inputs are the caller's values: geometry and instances in the byte layouts `layout` lists, RGBA8
atlas pixels, the camera matrix, and the `wgpu::Device` and `wgpu::Queue` the caller created. The
byte layouts are the shared binary contract with the map engine's upload belts; changing one is a
change to both crates.

## Design

- A renderer with no domain: the caller decides what to draw, in which order and with which
  pipeline, and keys each batch on an opaque `LaneId` whose value is only compared; lane roles and
  paint order live in the map engine's `overlay/lanes.rs`.
- Damage-driven: nothing is submitted unless something changed or continuous rendering is on,
  and the packet is borrowed, not rebuilt, each frame
  ([engine boundary rules §2C](/documentation_v2/standards/engine_boundary_rules.md#2c-the-packet-boundary)).
- Design target: the pure renderer of the
  [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md#3-the-graphics-engine-a-pure-renderer).
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
  (idea, no plan): among its fixes, rule 3b stops matching `text::gpu`, a module this crate does
  not have.
- [T-938 — Engine and wasm performance](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-938_plan.md)): pooled lane buffers and GPU
  culling for every icon lane.
- [T-1039 — Fix world line-of-sight bench frame pump running after unmount](/.ai/tickets/T-1039.toml)
  (idea, no plan): the debug bench sets the pump's disposed flag on unmount, so `RafPump` stops.

## Decisions

- The frame vocabulary lives here and the map engine re-exports it: the renderer defines the
  shape of a frame, and the map engine's whole graphics interface is one list in its
  `frame/mod.rs`.
- One call per frame: `encode` takes the whole packet, so the crate boundary costs one call, not
  one per batch.
- Browser code is selected by the `wasm32` target, not by a feature: the crate has no features,
  and a native build compiles and tests the CPU half.
- The shared byte layouts are one list (`layout`): the binary contract between the crates reads
  in one place, and the map engine's belts import it directly rather than through its `frame/`.
