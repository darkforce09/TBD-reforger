# Frame vocabulary

The types a caller uses to describe one frame to the GPU frame crate: a sorted list of draw
batches, glyph runs, indirect draws, and the pipelines and bind groups they index. Every field
names geometry or a GPU handle, never a thing in the world, and the module also holds the
swapchain submit and present step and the cell-atlas GPU handles. The GPU-free part of the vocabulary (the
ids, damage tracking and the camera uniform) is `render_primitives::frame`
(`crates/graphics/render_primitives/src/frame/`).

## Contents

```text
crates/graphics/gpu_frame/src/frame/
├── atlas.rs    `TextAtlasGpu`, `GlyphAtlasGpu` and their creators: texture, uniforms, bind group
├── batch.rs    `DrawBatch`, `DrawPayload` by vertex layout, and `IndirectDraw`
├── buffers.rs  `InstanceBuffer`, `IndexedMesh` and `VertexStream`: buffers and counts
├── mod.rs      the module tree; re-exports the packet, batch, buffer, text run and atlas types
├── packet.rs   `FramePacket`: one frame's draw list; `upsert` and `remove` keep a list sorted
├── present.rs  `submit` and `frame_ms_ema`: timestamp resolve, submit, present, frame cost
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

A frame is two calls because the caller's own encoding sits between them. The first,
`gpu_device`'s acquire (`GpuContext::acquire`), gets the next swapchain image, reconfigures the
surface once on `Outdated` or `Lost`, and returns `Skip` on `Timeout` or `Occluded`; the second,
`present::submit`, optionally resolves a timestamp query pair into a readback buffer, submits the
command buffer and presents. `frame_ms_ema` smooths the CPU
frame time.

`atlas.rs` uploads a cell atlas as an RGBA8 texture, checking that the byte length is width times
height times 4 (`crate::Error::AtlasPixelLength` otherwise), and builds its uniform buffer and bind group. `create_text_atlas` writes the
16-byte text uniforms from `render_primitives::text::pack`; `create_glyph_atlas` takes the
caller's packed uniform block unread, because its UV table is the caller's cell layout.

Every file names `wgpu` types and compiles for WebAssembly only.

## Boundaries

- Depends on: `render_primitives` (the ids, the camera uniform and `text::pack` for the text
  uniform block); `bytemuck`; `wgpu` in the WebAssembly build.
- Used by: `crate::draw` (`encode.rs`, `lines.rs` and `polygons.rs` build and read these types);
  `renderer_core`, whose lane sink and layer context carry its batches; and the map rendering
  crates (`crates/map_rendering/`), which build the packet, present the surface and write the
  glyph atlas.
- Rules: fields and variants name geometry and GPU handles only, and payload variants are named
  for their vertex layout; the renderer never re-ranks lanes (`FramePacket::batches_sorted` in
  debug builds); only the crates the wgpu firewall admits depend on this crate (`cargo xtask
  verify crate-tiers`).
