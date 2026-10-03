# CPU geometry for the draw path

Everything between a caller's coordinates and the bytes a GPU upload takes: triangulation, line
and mesh composition, the procedural grid, the per-instance byte layouts and the CPU reference of
sprite culling. Geometry comes in and geometry goes out; no module asks what a shape represents.

## Contents

```text
crates/graphics/render_primitives/src/draw/
├── compose.rs      coloured triangle meshes and hairline segment lists, ready for upload
├── cull/           the CPU reference of sprite frustum culling
├── geometry.rs     `LineVertex`, the anchor-relative fold `rel`, north-up rect and UV arithmetic
├── grid.rs         `grid_lines`: a 1 km line grid over a rect with 5 km major lines
├── instances.rs    the instance layouts `QuadInstance`, `BuildingInstance` and `IconInstance`
├── mod.rs          the module tree
├── tests/          unit tests for the geometry, grid, instance layouts and triangulation
└── triangulate.rs  ear-clipping of rings with holes into `TriMesh`, with area checks
```

## How it works

A caller works in f64 world metres and hands this module an anchor point with every call.
`geometry::rel` subtracts the anchor before casting to f32, so the numbers stay small enough for
sub-pixel precision at every zoom; the crate keeps no origin of its own. Colours arrive as RGBA8
and leave as linear 0 to 1 floats through `crate::color_normalization`, since the target is not
sRGB.

- `triangulate.rs` turns rings into a `TriMesh` of interleaved positions and triangle indices
  through `earcutr`, and checks the result with `ring_area`, `triangle_area_sum` and
  `area_tolerance`;
- `compose.rs` colours a `TriMesh` into a `PolyMeshGpu` (`mesh_from_tri`) and builds `HairlineGpu`
  segment lists, in one colour (`compose_hairlines`) or two (`compose_two_tone_hairlines`);
- `grid.rs` builds the grid's `LineVertex` list, with a brighter palette when the caller says a
  shaded layer lies beneath;
- `instances.rs` fixes the byte layouts the shaders read: `QuadInstance` (32 bytes),
  `BuildingInstance` (40 bytes, an oriented box), `IconInstance` (20 bytes, an atlas sprite), the
  `UNIT_QUAD` strip every instanced draw expands, `ATLAS_GLYPH_COUNT` (32 cells) and
  `CHUNK_CAPACITY`, the instance count of one 64 MiB buffer.

The graphics engine uploads these results (its `draw/lines.rs` and `draw/polygons.rs` build on
`LineVertex` and `rel`) and encodes the draws.

## Public surface

- `draw::triangulate`: `TriMesh`, `triangulate_simple`, `triangulate_with_holes`,
  `triangulate_ring_buffer`, `triangulate_region_rings` and the area helpers.
- `draw::compose`: `PolyMeshGpu`, `HairlineGpu`, `mesh_from_tri`, `retint_fill_alpha`,
  `compose_hairlines` and `compose_two_tone_hairlines`.
- `draw::geometry`: `LineVertex`, `rel`, `corner_uv`, `pack_offset` and `world_rect_rel`.
- `draw::instances`: the three instance layouts and their constants.
- `draw::grid::grid_lines`: the grid vertices.
- `draw::cull::oracle`: the culling reference, described in its own README.

## Boundaries

- Depends on: `crate::color_normalization`, `crate::shaders` (read by the instance layout tests
  and the cull oracle); `bytemuck` and `earcutr`.
- Used by: `gpu_frame`, whose uploads and compute cull build on it; and the map rendering,
  streaming and overlay crates, which pack instances and geometry with it directly.
- Rules: instance layouts are byte-exact (`icon_instance_layout_is_20_bytes`,
  `building_instance_layout_and_bytes_exact`); the shader's cell table stays `ATLAS_GLYPH_COUNT`
  long (`shader_uv_table_tracks_atlas_glyph_count`); triangulation conserves area
  (`polygon_with_hole_area_conserved`); UVs are north-up (`corner_uv_is_north_up`).
