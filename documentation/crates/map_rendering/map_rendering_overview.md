**Status:** live

# Map rendering overview

The map rendering crates draw the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map. `map_renderer` holds `RenderEngine`, which owns the GPU of the map canvas, the camera and the
persistent list of draw batches; `symbology_layers_gpu` and `world_layers_gpu` are the typed GPU
layers the engine holds as fields, one for the [mission](/documentation/glossary/g_to_m.md#mission)'s
symbology and one for the streamed world; `map_render_diagnostics` measures the engine from
outside it. They sit above the GPU device and frame crates, which
never name a map concept, and below the Mission Creator, which creates the engine and drives it.
This overview covers the crates, the path from a mounted canvas to a drawn frame and the open
work; the code READMEs it links hold the exact detail.

## Where it lives

- Code: [`crates/map_rendering/`](/crates/map_rendering/README.md), with
  [`map_renderer`](/crates/map_rendering/map_renderer/README.md),
  [`symbology_layers_gpu`](/crates/map_rendering/symbology_layers_gpu/README.md) and
  [`world_layers_gpu`](/crates/map_rendering/world_layers_gpu/README.md); the render diagnostics
  (readback self-checks, the frame benchmark, the stress pool) are
  [`map_render_diagnostics`](/crates/map_rendering/map_render_diagnostics/README.md), which
  reads the engine through `map_renderer::diagnostic_accessors`.
- Entry:
  - the Mission Creator creates the engine (`RenderEngine::create` through `create_engine` in
    `crates/frontend/foundation/frontend_map_view/src/engine_mount.rs`), starts the frame pump on its
    `EngineHandle` (`crates/frontend/foundation/frontend_map_view/src/frame_pump.rs`) and starts streaming
    through `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/world_assets.rs`;
  - the debug benches under `crates/frontend/workspaces/debug_benches/src/` (the building viewer and the
    world line of sight bench) create their own engine the same way.
- Related features: [map streaming](/documentation/crates/streaming/map_streaming.md), whose host
  and loaders write through the engine's asset sink; the
  [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) for the GPU
  device and frame crates it draws with; the
  [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) for
  the app around it.

## Behaviour

### Crates

| Crate | What it does | Tier | Deeper doc |
|---|---|---|---|
| `map_renderer` | `RenderEngine`: the GPU context, shader, layouts and pipelines, camera, batch list, lane sinks, upload belts, frame path, statistics and asset sink | 5 | [README](/crates/map_rendering/map_renderer/src/README.md) |
| `symbology_layers_gpu` | the slot symbology (slot atlas, binds, selection, drag, clusters), the glyph atlas, the icon lane cull, the world icon lanes and the lane preferences | 4 | [README](/crates/map_rendering/symbology_layers_gpu/README.md) |
| `world_layers_gpu` | the building, forest density, satellite and hillshade texture and terrain line of sight overlay layers, and their textured lane record | 3 | [README](/crates/map_rendering/world_layers_gpu/README.md) |
| `map_render_diagnostics` | the readback self-checks of the calibration quads and each pipeline, the one-pixel scene readback, the frame benchmark and the stress pool, as functions over the engine's diagnostic views | 6 | [README](/crates/map_rendering/map_render_diagnostics/src/README.md) |

A typed layer owns its GPU state (atlases, uniform blocks, pooled buffers, compute passes) and
writes its lanes through `renderer_core`'s `LaneSink`; the engine lends it a lane sink borrowed
apart from the rest of the engine for each call, so a layer never names the renderer. Browser code
(the canvas, the GPU, browser bitmaps) compiles only for wasm32; the error types, the surface size
policy, the statistics report, the calibration scene, the icon uniform layout and the textured quad
rules build and test natively.

### From a mounted canvas to a drawn frame

1. The Mission Creator sizes the canvas's backing store and creates the engine with
   `RenderEngine::create(canvas, force_webgl)`: the engine creates `gpu_device`'s `GpuContext`
   with the device label `map-engine-render` and timestamp queries when the adapter offers them
   (WebGPU with a WebGL2 fallback, or WebGL2 alone when forced), then builds its shader, layouts,
   pipelines, the compute cull on WebGPU, the typed layers and the calibration batch. On success
   the host sizes the surface, bounds and places the camera, hides the calibration quads, turns
   frame timing and continuous rendering off (the engine is damage-driven), uploads the slot icon
   atlas, registers the render context and binds the document's slots and vehicles to their
   symbology lanes.
2. It starts `gpu_frame`'s `RafPump` on the engine slot; each animation frame calls `render`,
   which returns at once unless something marked the frame damaged, and then `poll`.
3. It starts streaming for the terrain the document names: the map host fetches the terrain
   manifest, the elevation model, hillshade and satellite basemap, then the world objects,
   forest, water and labels, and writes each into the engine through `map_streaming_model`'s
   `MapAssetSink`, which the engine implements by forwarding to its belts and its typed layers.
   [Map streaming](/documentation/crates/streaming/map_streaming.md) follows this in detail.
4. Each upload puts its lane's batch into the engine's sorted batch list through the lane sink and
   marks the frame damaged; each camera move marks it damaged too and runs the frame hooks (the
   slot symbology re-derives its zoom uniform and cluster gate). The next `render` acquires a
   swapchain image through the GPU context, encodes one `FramePacket` over the persistent list and
   hands it to `gpu_frame`'s encoder in one call.
5. Every edit runs through the [editing layer](/documentation/crates/mission_editing/editing_layer.md)
   and ends in the host's post-change hook, which rebinds the affected symbology lanes through
   `RenderEngine::with_symbology`; the next frame draws them.

The render loop's steps (damage check, camera uniform, surface acquire, compute cull, packet
encode, submit) are in the [map renderer source README](/crates/map_rendering/map_renderer/src/README.md#how-it-works).

### Known discrepancies

- `CLAUDE.md` lists a top-level `symbology/` with NATO MIL-STD-2525 symbols; the symbology is the
  `unit_symbology` crate, drawn by `symbology_layers_gpu`, and implements no MIL-STD-2525 set
  (`crates/map_overlay/unit_symbology/README.md`).

## Data

- Browser globals the Mission Creator publishes for the page and the gates:
  `window.__selfChecks` (the render diagnostics' readback self-checks and `readback_rgba`),
  `window.__editorBench` (the stress seeding, the compute-cull switches and counters and the debug
  HUD) and `window.__wgpuSlotStats` (`RenderEngine::slot_stats_json`), plus the statistics JSON
  of `RenderEngine::stats`, whose keys the HUD and the browser gates read by name.
- The served map data the loaders turn into uploads: [map streaming](/documentation/crates/streaming/map_streaming.md#data).

## Design

The rendering crates are one layer of a one-way stack: the frontend uses the map renderer, the
map renderer uses the typed layers, the renderer contracts and the GPU device and frame crates, and
no GPU crate names a map concept. The
[crate boundary rules](/documentation/standards/crate_boundary_rules.md) state the four
frame-path rules; the damage pins in `crates/map_rendering/map_renderer/src/tests/` hold rules 1
and 3 (a frame is submitted only when damaged, and nothing the size of the scene is rebuilt per
frame), and the crate-tier law holds the layering and keeps `wgpu` in the rendering and GPU crates.

Where the built crates differ from that target:

- Several map constants are fixed to Everon's 12,800 m square: the opening camera bounds, the
  grid, the basemap extent and the forest density grid, while Arland is 4,096 m
  (`assets/terrains/terrain-registry.json`).

## Open work

- Fix map basemap switch back never restoring the satellite imagery (ticket `fix-map-basemap-switch`
  in `ttm`): switching the basemap from `map` back to `satellite` shows the imagery again.
- Check whether empty uploads leave stale height and road labels (ticket
  `check-whether-empty-uploads` in `ttm`): an empty height or road label upload removes the old
  labels, as the town labels do.
- Derive map grid, basemap, peaks and forest from terrain size (ticket `derive-map-grid-basemap` in
  `ttm`): the fixed 12,800 m constants give way to the loaded terrain's size.
- Check map render bench stress helpers and calibration hide (ticket `check-map-render-bench` in
  `ttm`): `seed_stress` and `clear_stress` stop destroying the map lanes, or go.
- Add guards for map binary formats and doll shader layout (ticket `add-guards-map-binary` in
  `ttm`): the density decoder checks its version, and the doll's region and layout constants get
  tests.
- Lint map engine render code in the wasm32 clippy step (ticket `lint-map-engine-render` in `ttm`):
  `wasm-ci`'s wasm32 clippy compiles the render code, not the default tier alone.
- Remove dead map engine code, facades and duplicated constants (ticket `remove-dead-map-engine` in
  `ttm`), Rewrite stale map engine comments outside mission data (ticket
  `rewrite-stale-map-engine-comments` in `ttm`) and Rename unclear map engine modules and numbered
  file splits (ticket `rename-unclear-map-engine` in `ttm`): cleanup across the former map engine
  code, including the placeholder field docs of `RenderEngine`.
- Rename ticket ids out of code names and UI strings (ticket `rename-ticket-ids-out` in `ttm`): the
  test names and messages that carry ticket ids get subject names.
- Engine and wasm performance (ticket `engine-wasm-performance` in `ttm`): pooled lane buffers,
  measured chunk uploads, GPU culling for every icon lane, frame-sliced viewsheds and a wasm memory
  guard.

Open work of the former map engine outside the rendering crates, kept here until each subject's
crate has a feature doc:

- Add @contract tags to map-engine compiled mission document structs (ticket `add-contract-tags-map`
  in `ttm`): the AST, world, io and descriptor models that project `contracts` schemas gain
  `@contract` tags the citations gate resolves.
- Check world line-of-sight sidecar eviction ignoring recency (ticket `check-world-line-sight` in
  `ttm`): the line-of-sight occluder's sidecar cap evicts the least recently used sidecar, as the
  chunk residency does, instead of the lowest path.
- Fix map object instance schema naming a nonexistent pod file (ticket `fix-map-object-instance` in
  `ttm`): the schema cites `io/pod/instance.rs`.
- Decide whether ignored map engine inputs are intended (ticket `decide-whether-ignored-map` in
  `ttm`): the hillshade slope scale, the satellite `Retry-After` and the archived blueprints' door
  and furniture fields are used or documented as ignored.

## Decisions

- Typed layers own their GPU state and write through a lane sink: the engine holds them as fields
  and lends each call a sink borrowed apart from itself, so the layer crates never name the
  renderer and the renderer never reaches into a layer's buffers.
- One GPU bootstrap: the map renderer and the Arsenal paper doll renderer both create their GPU
  through `gpu_device`'s `GpuContext`, differing only in the device label, the timestamp request
  and their own resize policy (the map rounds `css × dpr` and refuses a non-positive size).
- The renderer is damage-driven: the frame pump calls `render` every animation frame, and a frame
  nothing damaged costs a flag check, no swapchain acquire.
- The frontend's errors from the engine are the crate's typed `Error`, whose message starts with
  a stable code (`canvas-zero-size`, `resize-nonpositive`, …); nothing is exported to
  JavaScript, and the gates reach the engine only through the globals the Mission Creator
  publishes.
