# Lane sinks

The render engine's lanes as `renderer_core` lane sinks: the engine's own `LaneSink`, which the
lane-role adapters and the upload belts write through, and the two borrows of its lane state that
a typed layer writes through while the layer is itself borrowed from the engine.

## Contents

```text
crates/map_rendering/map_renderer/src/lane_sinks/
├── engine_lane_sink.rs  `RenderEngine` as `LaneSink<TexLane>`: lane batches, texture records, damage, the layer context
├── mod.rs               the module tree
├── textured_lanes.rs    `TexturedLanes`: the lanes borrowed apart for a world layer that writes textured lanes
└── untextured_lanes.rs  `UntexturedLanes`: the lanes borrowed apart for a layer that writes untextured lanes only
```

## How it works

Every sink writes the same state, the engine's sorted batch list, its texture records and its
damage flag, under the same rules: an upsert replaces the lane's batch in lane order (through
`gpu_frame::frame::packet`) and drops its old texture record, a removal drops both, and every
change that alters what is drawn marks the frame damaged; removing an empty lane does not.
`UntexturedLanes` takes `Infallible` as its texture record, so no textured lane can be written or
read through it; `TexturedLanes` wraps it and keeps the world layers' `TexLane` beside the batch.
Each sink's layer context lends the device, queue, surface format, packet tables and render
statistics.

## Boundaries

- Depends on: `renderer_core` (`LaneSink`, `LayerContext`, `RenderStats`), `gpu_frame` (the batch
  and the ordered packet list), `render_primitives` (`LaneId`, `RenderDamage`) and
  `world_layers_gpu` (`TexLane`).
- Used by: `lifecycle.rs` and the upload belts (the engine's own sink), and `typed_layers/`, which
  builds the borrowed sinks beside the layers it lends them to.
- Rules: every change that alters what is drawn marks the frame damaged.
