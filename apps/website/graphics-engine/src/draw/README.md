# Geometry assembly and the draw path

Everything between a caller's coordinates and a GPU draw call: triangulation, line and mesh
composition, the per-instance byte layouts, the uploads into vertex and index buffers, sprite
culling, and the encoder that turns a frame packet into render-pass commands. Geometry comes in
and geometry goes out; no module asks what a shape represents.

## Contents

```text
apps/website/graphics-engine/src/draw/
├── compose.rs      coloured triangle meshes and hairline segment lists, ready for upload
├── cull/           sprite frustum culling: the CPU reference and its WebGPU compute twin
├── encode.rs       `encode`: one frame packet into one render pass, WebAssembly only
├── geometry.rs     `LineVertex`, the anchor-relative fold, north-up rect and UV arithmetic
├── grid.rs         `grid_lines`: a 1 km line grid over a rect with 5 km major lines
├── instances.rs    the instance layouts `QuadInstance`, `BuildingInstance` and `IconInstance`
├── lines.rs        `LineList` vertex streams uploaded to the GPU, WebAssembly only
├── mod.rs          the module tree
├── polygons.rs     indexed triangle meshes uploaded to the GPU, WebAssembly only
├── tests/          unit tests for the geometry, grid, instance layouts and triangulation
└── triangulate.rs  ear-clipping of rings with holes into `TriMesh`, with area checks
```

## How it works

A caller works in f64 world metres and hands this module an anchor point with every call.
`geometry.rs` subtracts the anchor before casting to f32, so the numbers stay small enough for
sub-pixel precision at every zoom; the crate keeps no origin of its own. Colours arrive as RGBA8
and leave as linear 0 to 1 floats, since the target is not sRGB.

The CPU half runs natively and is what the native tests cover:

- `triangulate.rs` turns rings into a `TriMesh` of interleaved positions and triangle indices
  through `earcutr`, and checks the result with `ring_area`, `triangle_area_sum` and
  `area_tolerance`;
- `compose.rs` colours a `TriMesh` into a `PolyMeshGpu` (`mesh_from_tri`) and builds `HairlineGpu`
  segment lists, in one colour (`compose_hairlines`) or two (`compose_two_tone_hairlines`);
- `grid.rs` builds the grid's `LineVertex` list, with a brighter palette when the caller says a
  shaded layer lies beneath;
- `instances.rs` fixes the byte layouts the shaders read: `QuadInstance` (32 bytes),
  `BuildingInstance` (40 bytes, an oriented quad), `IconInstance` (20 bytes, an atlas sprite), the
  `UNIT_QUAD` strip every instanced draw expands, `ATLAS_GLYPH_COUNT` (32 cells) and
  `CHUNK_CAPACITY`, the instance count of one 64 MiB buffer.

The GPU half compiles for WebAssembly only. `lines.rs` and `polygons.rs` upload those vertices
into a `VertexStream` or an `IndexedMesh` (from `crate::frame::buffers`). `encode.rs` takes a
`FramePacket` and emits its draws in lane order: before each batch it emits the indirect draws
and glyph runs whose lane sorts lower, skips a batch marked invisible or whose pipeline or bind
group is missing, and binds by payload shape alone. Group 0 is always the camera, group 1 a
textured rect's texture, group 2 an atlas.

```text
rings, points, colours (f64 world metres + anchor)
   │ triangulate / compose / grid / instances      native and WebAssembly
   ▼
TriMesh, PolyMeshGpu, HairlineGpu, LineVertex, instance bytes
   │ lines / polygons (upload)                     WebAssembly only
   ▼
VertexStream, IndexedMesh, InstanceBuffer ─▶ DrawBatch in a FramePacket ─▶ encode ─▶ pass
```

## Public surface

- `draw::triangulate`: `TriMesh`, `triangulate_simple`, `triangulate_with_holes`,
  `triangulate_ring_buffer`, `triangulate_region_rings` and the area helpers, re-exported by the
  map engine's `world::mesh`.
- `draw::compose`: `PolyMeshGpu`, `HairlineGpu`, `mesh_from_tri`, `retint_fill_alpha`,
  `u8_rgba_to_f32`, `compose_hairlines` and `compose_two_tone_hairlines`.
- `draw::geometry`: `LineVertex`, `corner_uv`, `pack_offset` and `world_rect_rel`.
- `draw::instances`: the three instance layouts and their constants, also listed in
  `crate::layout`.
- `draw::grid::grid_lines`, `draw::lines` and `draw::polygons`: grid vertices and the uploads.
- `draw::encode::encode`: the render-pass encoder.
- `draw::cull`: the culling pair, described in its own README.

## Boundaries

- Depends on: `crate::frame` (the packet, batch, buffer and id types that `encode.rs`, `lines.rs`
  and `polygons.rs` use); `bytemuck` and `earcutr`; `wgpu` in the WebAssembly build.
- Used by: `website-map-engine`:
  - `apps/website/map-engine/src/frame/`: the lane uploads in `upload/` call `lines` and
    `polygons`, `encode.rs` runs `encode::encode`, and `mod.rs` re-exports `cull::oracle` and
    `cull::compute`;
  - `apps/website/map-engine/src/world/`: `mesh.rs` re-exports `triangulate` and the `compose`
    types, `scene.rs` uses `geometry` and `QuadInstance`, and the building buffers upload through
    `lines` and `polygons`;
  - `apps/website/map-engine/src/overlay/lanes_prefs.rs`, which builds the grid lane with
    `grid::grid_lines`;
  - `apps/website/map-engine/src/diagnostics/readback/scene.rs`, which encodes an offscreen
    packet;
  - through the map engine's re-exports, the frontend's debug building viewer
    (`apps/website/frontend/src/v2/apps/debug/building_viewer/geom.rs`), which triangulates
    footprints.
- Rules: instance layouts are byte-exact (`icon_instance_layout_is_20_bytes`,
  `building_instance_layout_and_bytes_exact`); the shader's cell table stays `ATLAS_GLYPH_COUNT`
  long (`shader_uv_table_tracks_atlas_glyph_count`); triangulation conserves area
  (`polygon_with_hole_area_conserved`); UVs are north-up (`corner_uv_is_north_up`); `encode`
  asserts, in debug builds, that the packet's batches ascend by lane.
