# Render engine and frame packet

The map engine's side of drawing: `RenderEngine`, which owns the GPU device, the canvas surface,
the camera and the persistent list of draw batches, and the one module that names the graphics
engine's frame vocabulary for the rest of the crate. Each frame it hands the batch list to
`website-graphics-engine` as a frame packet, and only when something changed.

## Contents

```text
apps/website/map-engine/src/frame/
├── bindings.rs   the pipeline and bind-group ids a lane draws with, and the table sizes
├── boot.rs       `RenderEngine::create`: adapter, device, surface, pipelines, buffers, first batch
├── cull.rs       the compute-cull switch and counters, and the indirect draws of the sprite lanes
├── encode.rs     the packet's pipeline and bind-group tables, and `encode_main_pass`
├── engine.rs     `RenderEngine`: GPU handles, camera, atlases, batch list, counters; `CLEAR_COLOR`
├── lifecycle.rs  `render`, damage, lane upsert and removal, previews, markers, calibration
├── mod.rs        the module tree; re-exports the frame vocabulary, `RenderEngine` and the pump
├── pump.rs       `RenderEngine` as the render loop's `FrameTarget`
├── tests/        unit tests that pin the damage-driven frame path in the source
└── upload/       the belts that upload a lane's geometry and put its batch in the list
```

## How it works

A host creates the engine with `RenderEngine::create(canvas, force_webgl)` and drives it with the
graphics engine's `RafPump`, which calls `render` and then `poll` on every animation frame:

```text
RafPump tick ──▶ render()
                  ├─ damage.begin_frame(): clean, not continuous ──▶ return, no acquire
                  ├─ write the camera uniform: wgpu_clip_matrix at the world anchor
                  ├─ present::acquire: timeout or occluded skips, damage stays set;
                  │    lost or outdated reconfigures the surface and tries once more
                  ├─ encode_main_pass
                  │    ├─ compute cull over the visible world rectangle (WebGPU)
                  │    ├─ refill the pipeline and bind-group tables, collect indirect draws
                  │    └─ FramePacket { batches: &self.batches, … } ──▶ draw::encode::encode
                  ├─ present::submit, with the timestamp resolve when the timer samples
                  └─ damage.after_submit(); CPU frame time and its moving average
             ──▶ poll(): drain pending map_async callbacks
```

`boot.rs` asks for WebGL2 alone when forced, otherwise WebGPU with WebGL2 as the fallback, on a
high-performance adapter. It takes the adapter's full texture resolution, the timestamp feature
when offered, a non-sRGB surface format and FIFO presentation, then builds the shader module, the
layouts, eight pipelines (plus the storage-buffer icon pipeline and the compute cull on WebGPU),
the samplers, the unit quad, the calibration quads and the camera, which opens on
`crate::world::scene::INITIAL_TARGET` within `EVERON_BOUNDS`. A failure rejects with a short code:
`canvas-zero-size`, `create-surface`, `no-adapter`, `no-device`, `srgb-only-surface` or
`surface-unsupported-by-adapter`. The crate's `#[wasm_bindgen(start)]` function, also here,
installs the panic hook that sends Rust panics to the console.

The batch list stays sorted by lane id (`crate::overlay::lanes::lane_id`, whose order is paint
order). `upsert_lane` adds or replaces a lane's batch and marks the frame damaged, and
`remove_lane` drops it and marks the frame damaged when there was one to drop; the upload belts,
the overlay's symbol bridges and the streaming loaders go through them.
A few callers flip a batch's visibility in place instead (`hide_calibration` here, the lane
preferences in `crate::overlay`), and the stress pool in `crate::diagnostics::bench` pushes and
drains the list directly. A textured lane keeps its texture beside the list in `tex_lanes`, keyed
by lane, because a batch carries only a bind-group id: `bindings.rs` maps a lane to its pipeline,
its sprite atlas and its texture slot, so the graphics engine never learns what a lane means.
`cull.rs` turns the compute cull's per-lane output into indirect draws for the nine sprite lanes
it culls.

## Public surface

- `RenderEngine` (at `engine::RenderEngine` and re-exported here) and `EngineHandle`, the shared
  and optional holder a host keeps it in: for the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s canvas, bridge and input
  handlers, the debug benches, and `crate::editing`'s selection gesture.
- `RenderEngine::create`, `render`, `mark_dirty`, `set_continuous_render`, `backend`,
  `set_place_preview`, `clear_place_preview`, `clear_vector_lane` and `hide_calibration`, plus the
  belts of `upload/`: the engine's JavaScript-facing methods; the compute-cull getters and
  `submitted_last_frame` are exported too, with no caller in the repository.
- `RafPump` and `FrameTarget`: the render loop, which the frontend reaches through this crate.
- The frame vocabulary for the crate's own modules: `damage`, `packet`, `present`,
  `CameraUniform`, `LaneId`, `PipelineId`, `BindGroupId`, `DrawBatch`, `DrawPayload`,
  `FramePacket`, `IndirectDraw`, `IndexedMesh`, `InstanceBuffer`, `VertexStream`, the text run and
  atlas types; with them `oracle`, `compute`, `buffers` and `pipelines`.
- `boot::instance_descriptor`, which `crate::doll`'s renderer shares.

## Boundaries

- Depends on: `website-graphics-engine` (the frame vocabulary, `draw`, `layout`, the pipelines and
  the render loop); `crate::camera::ortho` (the camera); `crate::overlay` (lane ids and the symbol
  and marker atlases and bridges); `crate::world` (the anchor, the opening view, the calibration
  quads and the satellite texture lanes); `crate::diagnostics` (the GPU timer, the clock and
  `poll`); and `wgpu`, `wasm-bindgen`, `web-sys` and `console_error_panic_hook`.
- Used by:
  - inside the crate: `crate::camera::viewport`, `crate::overlay`, `crate::world`,
    `crate::streaming`, `crate::spatial` (the viewshed texture), `crate::diagnostics`,
    `crate::doll` and `crate::editing`;
  - the Mission Creator under `apps/website/frontend/src/v2/apps/editor/` (canvas mount, bridge,
    input and tools) and the debug benches under `apps/website/frontend/src/v2/apps/debug/`;
  - the engine-layers gate in `tools_v2/xtask/src/verifications/architecture/`, which pins this
    folder's imports of the graphics engine.
- Rules:
  - `mod.rs` is the only file of the crate that names `website_graphics_engine::frame`, on exactly
    eight lines (rule 3a of `cargo xtask verify engine-layers`), and the graphics engine's GPU
    modules are named only in `mod.rs` (3 sites) and `pump.rs` (2 sites) (rule 3b);
  - the renderer is damage-driven and allocates no batch list per frame: `render` returns before
    acquiring a frame when nothing is damaged, and the packet borrows the engine's own batch list
    and tables (the tests in `tests/damage_discipline.rs`);
  - `BIND_SLOTS` must index the widest lane id, which a compile-time assert in `bindings.rs` checks
    against `crate::overlay::lanes::ALL_LANES`;
  - the overlay's draw-order suites in `apps/website/map-engine/src/overlay/tests/tests/` read
    these files by path, so a moved or renamed file breaks them.

## Related documentation

- [Map engine overview](/documentation_v2/website/map-engine/map_engine_overview.md) — the path
  from a mounted canvas to a drawn frame.
- [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the four
  frame-path rules (§2C) and the frame vocabulary this module alone names (§2C.1, rule 3a).
