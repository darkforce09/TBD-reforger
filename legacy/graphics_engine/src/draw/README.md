# The GPU draw path

The GPU half of the draw path: the uploads of vertices and indices into GPU buffers, the compute
pass that culls sprites, and the encoder that turns a frame packet into render-pass commands. The
CPU geometry it uploads (triangulation, composition, the grid, the instance layouts and the cull
reference) is `render_primitives::draw` (`crates/graphics/render_primitives/src/draw/`).

## Contents

```text
legacy/graphics_engine/src/draw/
├── cull/        sprite frustum culling: the WebGPU compute pass checked against the CPU reference
├── encode.rs    `encode`: one frame packet into one render pass, WebAssembly only
├── lines.rs     `LineList` vertex streams uploaded to the GPU, WebAssembly only
├── mod.rs       the module tree
└── polygons.rs  indexed triangle meshes uploaded to the GPU, WebAssembly only
```

## How it works

Every file here compiles for WebAssembly only. `lines.rs` and `polygons.rs` upload
`render_primitives` vertices into a `VertexStream` or an `IndexedMesh` (from
`crate::frame::buffers`). `encode.rs` takes a `FramePacket` and emits its draws in lane order:
before each batch it emits the indirect draws and glyph runs whose lane sorts lower, skips a batch
marked invisible or whose pipeline or bind group is missing, and binds by payload shape alone.
Group 0 is always the camera, group 1 a textured rect's texture, group 2 an atlas.

```text
rings, points, colours (f64 world metres + anchor)
   │ render_primitives::draw (triangulate / compose / grid / instances)
   ▼
TriMesh, PolyMeshGpu, HairlineGpu, LineVertex, instance bytes
   │ lines / polygons (upload)                     WebAssembly only
   ▼
VertexStream, IndexedMesh, InstanceBuffer ─▶ DrawBatch in a FramePacket ─▶ encode ─▶ pass
```

## Public surface

- `draw::lines` and `draw::polygons`: the uploads.
- `draw::encode::encode`: the render-pass encoder.
- `draw::cull`: the compute pass, described in its own README.

## Boundaries

- Depends on: `crate::frame` (the packet, batch and buffer types that `encode.rs`, `lines.rs` and
  `polygons.rs` use); `render_primitives` (`LineVertex`, `rel`, the frame ids); `bytemuck`; `wgpu`
  in the WebAssembly build.
- Used by: `map_engine`:
  - `legacy/map_engine/src/frame/`: the lane uploads in `upload/` call `lines` and
    `polygons`, `encode.rs` runs `encode::encode`, and `mod.rs` re-exports `cull::compute`;
  - `legacy/map_engine/src/world/environment/buildings/buffers.rs`, whose building buffers upload
    through `lines` and `polygons`;
  - `legacy/map_engine/src/overlay/lanes_prefs.rs`, which uploads the grid lane through `lines`;
  - `legacy/map_engine/src/diagnostics/readback/scene.rs`, which encodes an offscreen
    packet.
- Rules: `encode` asserts, in debug builds, that the packet's batches ascend by lane.
