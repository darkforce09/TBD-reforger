# Building section cuts

Architectural drawings of a building mesh: the plan section where a horizontal plane cuts its
walls, and the height field of the surfaces below that plane, for each level and for the roof. The
debug building viewer draws floor plans from them.

## Contents

```text
apps/website/map-engine/src/world/architecture/section/
├── cutter.rs  plan section cuts and height fields of a building mesh, per level and for the roof
├── index.rs   the y-interval index over triangles, and the sparse tiled height raster
├── mod.rs     the module tree
└── tests/     unit tests for the cuts, the height fields, the index and the sparse raster
```

## How it works

`building_drawing(bp, occl)` turns each blueprint level into a `LevelSpec` (its index and height
band) over the union of the blueprint's footprint box and the mesh's plan bounds, padded by
`VOID_PAD_M` (1 m), and hands them to `drawing_for`, which needs no blueprint. For each band it
cuts at `CUT_MAIN_M` (1.2 m, eye height) and `CUT_LOW_M` (0.45 m, where sills read) above the
floor, clamped to 60 % and 25 % of a short band. `section_at` intersects the plane with every
near-vertical face (`|n.y| ≤ CUT_MAX_NY`, 0.35) and returns plan segments `[[x, z], [x, z]]`;
`section_at_owned` also tags each segment with its triangle's owner in a flattened compound.
`HeightField::build` rasters, on a `PLAN_CELL_M` (0.2 m) grid, the highest face at or below the
main cut that is not near-vertical (`|n.y| ≥ SURFACE_MIN_NY`, 0.2, so steep pitches still count),
and the roof field is the same raster with no cut. A level's floor is the window `FLOOR_WINDOW_M` (−0.25 m to 0.35 m) around its base, and a
cell with no surface from `LevelDrawing::floor_min_y` up is a void; `through_voids` keeps the
pieces of a lower level's cut that show through those voids, and `PIT_DEPTH_M` (3 m) is how far
below the floor window the viewer's pit shading reaches.

The section cuts find their candidate triangles through `triangles_overlapping_y`, a one-axis BVH
over the triangles' y extents (`YIntervalIndex`), built for each query, which answers exactly what
a scan of every triangle answers. A height field stores its cells in `SparseHeights`: `f32` tiles
of `HEIGHT_TILE` × `HEIGHT_TILE` (16 × 16) cells, allocated on the first write, with NaN for no
surface. A field wider than `MAX_PLAN_DIM` (2048) cells on an axis coarsens its cell to fit.

## Boundaries

- Depends on: `crate::spatial::bvh` (`BvhSidecar`, the vector helpers) and
  `crate::world::architecture::blueprint` (`BuildingBlueprint`, for `building_drawing`).
- Used by: the debug building viewer, interior bench and world line-of-sight bench in
  `apps/website/frontend/src/v2/apps/debug/` (`building_viewer.rs`, `building_viewer/`,
  `building_interior.rs`, `world_los/live.rs`), which draw the cuts, the voids and the height
  ramps; the compound tests in `apps/website/map-engine/src/spatial/los/interior/tests/walker.rs`
  cut a flattened compound.
- Rules: both files compile only with the `io` feature; the indexed cut equals a scan of every
  triangle at four heights on the Everon farmhouse mesh and five synthetic buildings
  (`golden_section_cut_equals_brute_force` in `tests/index_tests.rs`), and every vertex of those
  meshes lies inside the root box of their BVH (`bvh_root_encloses_section_geometry`); a sloped
  face is never cut (`sloped_faces_are_not_cut` in `tests/cutter.rs`); the height field keeps only
  surfaces at or below the cut (`heightfield_clips_at_the_plane_and_sees_the_stairwell`) and the
  roof field is the top surface (`roof_field_is_the_top_surface`); cut heights clamp into short
  bands (`drawing_levels_and_clamps`); a height field with 1 % of its cells written allocates
  under a fiftieth of a dense grid of `Option<f64>` cells (`sparse_heightfield_one_percent_memory`).
