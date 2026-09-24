# Frame vocabulary

The types a caller uses to describe one frame to the graphics engine: a camera matrix, a sorted
list of draw batches, glyph runs, indirect draws, and the pipelines and bind groups they index.
Every field names geometry or a GPU handle, never a thing in the world, and the module also holds
the swapchain present step, the damage flag and the cell-atlas GPU handles.

## Contents

```text
apps/website/graphics-engine/src/frame/
├── atlas.rs    `TextAtlasGpu`, `GlyphAtlasGpu` and their creators: texture, uniforms, bind group
├── batch.rs    `DrawBatch`, `DrawPayload` by vertex layout, and `IndirectDraw`
├── buffers.rs  `InstanceBuffer`, `IndexedMesh` and `VertexStream`: buffers and counts
├── camera.rs   `CameraUniform`: the 64-byte clip-from-local matrix bound at group 0
├── damage.rs   `RenderDamage`: whether a frame needs submitting at all
├── ids.rs      `LaneId`, `PipelineId` and `BindGroupId`: opaque keys the caller assigns
├── mod.rs      the module tree; re-exports the packet, batch, buffer, id and atlas types
├── packet.rs   `FramePacket`: one frame's draw list; `upsert` and `remove` keep a list sorted
├── present.rs  `acquire` and `submit`: swapchain image, timestamp resolve, present
├── tests/      unit tests for the damage decision
└── text.rs     `TextRun`: a packed glyph run with its lane, atlas and pipeline
```

## How it works

The caller keeps a list of `DrawBatch`es sorted by `LaneId`; `packet::upsert` inserts or replaces
a lane's batch without breaking the order, and `packet::remove` drops it. A batch carries its own
`PipelineId` and a `DrawPayload` named for its vertex layout (`Quads`, `OrientedQuads`, `Sprites`,
`TexturedRect`, `Lines`, `Indexed`, `SpritesWithText`, `Text`), so the encoder binds what the batch
names instead of deciding from the lane. Each frame the caller wraps the list in a `FramePacket`
with the camera, the clear colour, its `TextRun`s and `IndirectDraw`s, the pipeline table, a sparse
bind-group table (`None` while an atlas is still loading) and the unit-quad buffer, and passes it
to `crate::draw::encode::encode`. The lane value is the caller's paint order and this crate only
compares it.

`present.rs` splits a frame into two calls because the caller's own encoding sits between them.
`acquire` gets the next swapchain image, reconfigures the surface once on `Outdated` or `Lost`,
and returns `Skip` on `Timeout` or `Occluded`; `submit` optionally resolves a timestamp query pair
into a readback buffer, submits the command buffer and presents. `frame_ms_ema` smooths the CPU
frame time.

`RenderDamage` decides whether a render call does any GPU work: it submits while `dirty` or
`continuous` is set, and `after_submit` clears `dirty` unless the loop runs continuously. It
starts dirty, so the first frame always draws.

`atlas.rs` uploads a cell atlas as an RGBA8 texture, checking that the byte length is width times
height times 4, and builds its uniform buffer and bind group. `create_text_atlas` writes the
16-byte text uniforms from `crate::text::pack`; `create_glyph_atlas` takes the caller's packed
uniform block unread, because its UV table is the caller's cell layout.

`camera.rs`, `damage.rs` and `ids.rs` compile natively; every other file names `wgpu` types and
compiles for WebAssembly only.

## Boundaries

- Depends on: `bytemuck`; `wgpu` in the WebAssembly build; `crate::text::pack` for the text
  uniform block.
- Used by: `crate::draw` (`encode.rs`, `lines.rs` and `polygons.rs` build and read these types);
  and `website-map-engine`, which names this module in one file,
  `apps/website/map-engine/src/frame/mod.rs`, and re-exports `damage`, `packet`, `present`,
  `CameraUniform`, the three ids, the batch group, the buffer group and the atlas and text group
  to the rest of its crate.
- Rules: fields and variants name geometry and GPU handles only, and payload variants are named
  for their vertex layout; the renderer never re-ranks lanes (`FramePacket::batches_sorted` in
  debug builds); a clean frame skips its submit and a continuous one always submits
  (`class_r_clean_second_frame_skips`, `class_r_continuous_always_submits`); only
  `apps/website/map-engine/src/frame/mod.rs` may name `website_graphics_engine::frame`, with a
  pinned count of 8 references (`cargo xtask verify engine-layers`, rule 3a).
