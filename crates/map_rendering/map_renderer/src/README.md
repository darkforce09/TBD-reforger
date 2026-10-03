# Map renderer source

The source of `map_renderer`: `RenderEngine`, which owns the canvas's GPU context, the camera and
the persistent list of draw batches, the modules that add its methods, the lane sinks and typed
layer doors it lends its lanes through, its upload belts and the crate's error type. Each frame it
hands the batch list to `gpu_frame`'s encoder as a frame packet, and only when something changed.

## Contents

```text
crates/map_rendering/map_renderer/src/
├── asset_sink.rs            `RenderEngine` as `map_streaming_model`'s `MapViewport` and `MapAssetSink` for the streaming host and loaders
├── bindings.rs              the lane-aware binding policy: the bind table size
├── boot.rs                  `RenderEngine::create`: the GPU context, then shader, layouts, pipelines, buffers, layers, first batch
├── calibration_scene.rs     the two known calibration quads of the engine's first batch
├── cull.rs                  the compute-cull switch and counters, and the indirect draws of the sprite lanes
├── diagnostic_accessors.rs  the views the render diagnostics read the engine through: device, pipeline resources, scene, stress pool; `disable_frame_timing`
├── encode.rs                the packet's pipeline and bind-group tables, and `encode_main_pass`
├── engine.rs                `RenderEngine`: GPU context, pipelines, camera, typed layers, batch list, counters, frame hooks; `EngineHandle`; `CLEAR_COLOR`
├── engine_statistics.rs     `stats()`, the engine's statistics report as JSON, and the vector-lane counts
├── error.rs                 `Error` and `Result`: why the engine refused a create, resize, render or upload
├── lane_sinks/              the engine's lanes as lane sinks: its own, and the two borrows the typed layers write through
├── lib.rs                   the crate root: module header, `mod` lines, the engine and error re-exports, the source pins
├── lifecycle.rs             `render`, damage, the lane-role adapters, the clear colour, calibration, `poll`
├── prelude.rs               the engine, its slot and the error
├── pump.rs                  `RenderEngine` as the frame pump's `FrameTarget`
├── surface_size.rs          the surface size policy: `round(css·dpr)` device pixels, a non-positive size refused
├── tests/                   the damage-driven frame path and lane bind source pins, the statistics JSON, the calibration bytes, the surface size and the errors
├── typed_layers/            the doors the symbology and world typed layers are lent the engine's lanes through
├── upload/                  the belts that upload a lane's geometry and put its batch in the list
└── viewport.rs              the camera entry points: resize, view, pan, zoom, bounds, camera changed
```

## How it works

A host creates the engine with `RenderEngine::create(canvas, force_webgl)` and drives it with
`gpu_frame`'s `RafPump`, which calls `render` and then `poll` on every animation frame:

```text
RafPump tick ──▶ render()
                  ├─ damage.begin_frame(): clean, not continuous ──▶ return, no acquire
                  ├─ write the camera uniform: wgpu_clip_matrix at the world anchor
                  ├─ self.gpu.acquire(): timeout or occluded skips, damage stays set;
                  │    lost or outdated reconfigures the surface and tries once more
                  ├─ encode_main_pass
                  │    ├─ compute cull over the visible world rectangle (WebGPU)
                  │    ├─ refill the pipeline and bind-group tables, collect indirect draws
                  │    └─ FramePacket { batches: &self.batches, … } ──▶ draw::encode::encode
                  ├─ present::submit, with the timestamp resolve when the timer samples
                  └─ damage.after_submit(); CPU frame time and its moving average
             ──▶ poll(): drain pending map_async callbacks
```

`boot.rs` creates the canvas's `gpu_device::GpuContext` with the device label
`map-engine-render` and timestamp queries when the adapter offers them: WebGL2 alone when forced,
otherwise WebGPU with WebGL2 as the fallback, the adapter's full texture resolution, a non-sRGB
surface format and FIFO presentation. It then builds the shader module, the layouts, eight
pipelines (plus the storage-buffer icon pipeline and the compute cull on WebGPU, never on WebGL2),
the samplers, the unit quad, the calibration quads and the camera, which opens on
`map_coordinates::terrain_frames::INITIAL_TARGET` within `EVERON_BOUNDS`. A failure is
`Error::Gpu` with the context's code: `canvas-zero-size`, `create-surface`, `no-adapter`,
`no-device`, `srgb-only-surface` or `surface-unsupported-by-adapter`. `resize` sizes the camera in
CSS pixels and the surface at `round(css × dpr)` device pixels (`surface_size.rs`), and refuses a
non-positive argument with `resize-nonpositive`.

The batch list stays sorted by lane id (`map_draw_lanes::lane_roles::lane_id`, whose order is
paint order). `lane_sinks/engine_lane_sink.rs` implements `renderer_core`'s `LaneSink` over it,
keyed by the opaque `LaneId`: `upsert_lane_batch` adds or replaces a lane's batch and marks the
frame damaged, and `remove_lane_batch` drops it and marks the frame damaged when there was one to
drop. The role-keyed `upsert_lane`, `remove_lane` and `tex_lane` in `lifecycle.rs` map a
`LaneRole` onto its lane id and write or read through the sink; the upload belts go through them.

The typed layers are engine fields. The world layers of `world_layers_gpu` (`BuildingLayerGpu`,
`ForestLayerGpu`, `TerrainTextureLayerGpu`, `TerrainLineOfSightOverlayGpu`) are lent the engine's
lanes as a `TexturedLanes` borrowed beside them by `typed_layers/world_layers.rs` (with the strip
upload counter the fence strips share); the asset sink forwards the world loaders' writes to them,
and `RenderEngine::with_terrain_line_of_sight_overlay` is the door the Mission Creator uploads and
clears the viewshed wash through. The symbology layers of `symbology_layers_gpu`
(`SlotSymbologyGpu`, `GlyphAtlasGpu`, `IconCullGpu`) are lent the engine's lanes as an
`UntexturedLanes` by `typed_layers/symbology_layers.rs`, with the camera, the shared text atlas
and the counters they report, and `RenderEngine::with_symbology` is the one door the Mission
Creator drives the slot symbology through. The engine's `FrameHooks` run on every camera change
(`RenderEngine::on_camera_changed`): the slot symbology's hook re-derives its zoom uniform and
cluster gate.

A few callers flip a batch's visibility in place instead (`hide_calibration` here, the lane
preferences in `symbology_layers_gpu`), and the render diagnostics' stress pool pushes and drains
the list through `diagnostic_accessors`. A textured lane keeps its texture beside the list in
`tex_lanes`, keyed by lane, because a batch carries only a bind-group id: `world_layers_gpu`'s
`textured_quad.rs` picks a textured lane's pipeline and `renderer_core::packet_bindings::tex_bind_id`
its texture slot (the symbology layers' `sprite_atlas_for` maps a sprite lane to its atlas), so the
GPU frame crate never learns what a lane means. The engine's first batch draws the two quads of
`calibration_scene.rs`. `cull.rs` turns the compute cull's per-lane output into indirect draws for
the nine sprite lanes it culls.

The engine keeps its frame and lane counters in a `renderer_core::render_stats::RenderStats`:
`render` records each frame into it (the CPU time from `time_source::monotonic_ms`, its moving
average, or a skipped frame), and `set_vector_stat` records the sea, landcover, contour and road
counts by lane and the vector forest counts into the forest layer, which keeps every forest
counter; the building layer keeps the building upload count and the world chunks drawn.
`stats()` (`engine_statistics.rs`) reports the backend, the stress instance and chunk counts, GPU
and staging bytes, the generation, upload and GPU frame times (`gpu_frame_ms` is `null` unless the
timestamp timer has a sample), the basemap mode, tiles and bytes, the world buildings' instance and
outline counts, the instance count of each sprite lane (trees, props, badges,
[slots](/documentation/glossary/n_to_z.md#slot), the slot drag, clusters and vehicles, taken from
the compute cull when it runs), the vector-lane counts, atlas bytes, the upload counters, the
compute-cull counters, and the CPU render time of the last frame with its moving average, as one
flat JSON object written with `renderer_core::stats_json::StatsJson`.

## Boundaries

- Depends on: the crates the [crate README](/crates/map_rendering/map_renderer/README.md) lists.
- Used by: the map engine's `frame` shims, and through them the Mission Creator, the debug benches
  and the render diagnostics.
- Rules: every module that names a GPU or browser type is `wasm32` only; the native build keeps the
  error type, and the tests add the surface size policy, the statistics report and the calibration
  scene. `BIND_SLOTS` must index the widest lane id, which a compile-time assert in `bindings.rs`
  checks against `map_draw_lanes::lane_roles::ALL_LANES`. The damage pins read `lifecycle.rs`,
  `encode.rs`, `engine.rs` and `lane_sinks/engine_lane_sink.rs` by path, and the lane bind pins
  read `upload/hairlines.rs` and the symbology layers' slot symbology files, so moving or renaming
  any of them breaks the pins.
