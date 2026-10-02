# Frame vocabulary, GPU-free part

The frame types that hold no GPU handle: the opaque keys a frame's batches carry, the damage flag
that decides whether a frame submits at all, and the camera uniform bound at group 0. The graphics
engine's frame packet, batches and buffers are built from them.

## Contents

```text
crates/graphics/render_primitives/src/frame/
├── camera.rs  `CameraUniform`: the 64-byte clip-from-local matrix bound at group 0
├── damage.rs  `RenderDamage` and `FrameDecision`: whether a frame needs submitting at all
├── ids.rs     `LaneId`, `PipelineId` and `BindGroupId`: opaque keys the caller assigns
├── mod.rs     the module tree
└── tests/     unit tests for the damage decision
```

## How it works

A `LaneId` is the caller's paint order: the renderer sorts by it and never interprets it.
`PipelineId` and `BindGroupId` index the pipeline and bind-group tables of the graphics engine's
frame packet, so the encoder binds what a batch names instead of deciding from the lane.

`RenderDamage` decides whether a render call does any GPU work: `begin_frame` submits while
`dirty` or `continuous` is set, and `after_submit` clears `dirty` unless the loop runs
continuously. It starts dirty, so the first frame always draws.

`CameraUniform` carries sixteen floats composed by the caller in column-major order; its default
is the identity, so a packet built before the caller has a camera still submits.

## Boundaries

- Depends on: `bytemuck`.
- Used by: the graphics engine (`legacy/graphics_engine/src/frame/`), whose batches, glyph runs,
  packet and encoder carry these types and whose `frame` module forwards `damage`,
  `CameraUniform` and the three ids to the map engine.
- Rules: fields name geometry and opaque keys only; a clean frame skips its submit and a
  continuous one always submits (`class_r_clean_second_frame_skips`,
  `class_r_continuous_always_submits`).
