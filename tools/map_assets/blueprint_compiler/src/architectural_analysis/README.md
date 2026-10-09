# Architectural analysis

The geometry stages of the blueprint compiler: from a building's voxel dump they find the floor
slabs, the walls of each floor band, the walkable floor plates and their outlines, and the roof
heightfield; for the collision meshes they rebuild convex colliders and classify each triangle as
opaque, glass or foliage. They also hold the one march skeleton every voxel dump generator shares.

## Contents

```text
tools/map_assets/blueprint_compiler/src/architectural_analysis/
├── collision_face_pairing.rs  one-sided collision faces paired into solid intervals, for the walls
├── convex_hulls.rs            `hull_triangles`: the triangulated convex hull of a small point cloud
├── floor_plates.rs            `floor_plate`: a level's walkable cells and heights around its slab
├── polygon_rings.rs           `trace`: outer rings and holes of a plan grid, traced on cell edges
├── roof_profiles.rs           `build`: the blueprint's optional roof heightfield, biased low
├── surface_classification.rs  surface kind from a game material or layer preset; `--kind` overrides
├── tests/                     the unit tests of every stage
├── vertical_slabs.rs          `analyze`: slabs, floors, eave, ridge, chimney and the roof slope field
├── wall_extraction/           the `segments` and `grid` wall extractors and the exterior flood fill
└── wall_extraction.rs         `WallAlgorithm`, `BandWalls`, the `--debug-dir` records; re-exports `extract_band_walls`
```

## How it works

The files are modules of `architectural_analysis`, declared in
`tools/map_assets/blueprint_compiler/src/architectural_analysis.rs`; the unit tests are in `tests/`, one file per module.
`blueprint-from-voxels` runs them per dump:

```text
VoxelDump ──slabs::analyze──▶ VerticalScan (floors, eave, ridge, top surface, top slope)
    │
    ├─ per floor band [floor .. next floor or eave]:
    │     walls::extract_band_walls ──▶ walls, exterior flags, masses
    │     plate::floor_plate  ──▶ walkable cells ──rings::trace──▶ footprint + floor polygons
    ├─ attic band [last band .. ridge] when the ridge rises high enough: walls only, no plate
    └─ roof::build ──▶ RoofGrid (in blueprint assembly)
```

`vertical_slabs.rs` histograms the downward entry faces of the `y-` march into slab candidates and
keeps as floors those between the floor filter and the eave. `floor_plates.rs` keeps, per plan
cell, the highest downward entry within a window around the slab, so a mezzanine void has no plate
and stays void. `polygon_rings.rs` stitches the directed edges between covered and uncovered cells
into counter-clockwise outer rings and clockwise holes on the integer lattice, taking the
left-most turn at a vertex where two cells touch diagonally, and drops rings under
`plate_min_ring_area_m2`. `roof_profiles.rs` takes the minimum of each coarse cell's top surfaces
and leaves a cell empty unless its fine block is covered to `roof_min_coverage`: a phantom roof
would block sight lines the engine clears.

`collision_face_pairing.rs` rebuilds solid intervals from the scan runs: Enfusion collision faces
register only when marched into from the front, so it pairs each forward entry face with the
nearest closing face (`pair_faces_consuming`), each closing face used at most once.

`surface_classification.rs` maps a `.gamemat` stem to a surface kind (`glass*` and `plexiglass*`
are glass; `foliage*`, `grass*`, `moss*` and `seaweed*` are foliage; the rest is opaque), reads a
collider's layer preset as a second opinion, and decides from the preset whether a collider stops
a projectile. `convex_hulls.rs` rebuilds the faces of the COLL chunk's convex colliders from their
vertices.

## Public surface

None outside the blueprint compiler: the modules are private to
`tools/map_assets/blueprint_compiler/src/`, and the commands that use them are listed in that
folder's README.

## Boundaries

- Depends on: `tools/map_assets/blueprint_compiler/src/voxel_processing/` (`AnalysisParameters`, `VoxelDump`,
  `VerticalScan`, `PlanGrid`), the face pairing in
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/collision_face_pairing.rs`, and
  `building_interiors::blueprint` (`FloorPolygon`, `RoofGrid`) and
  `spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind`.
- Used by: the blueprint root (`run`, `interpret_one`, `build_bands`); the blueprint assembly in
  `tools/map_assets/blueprint_compiler/src/archive_emission/` (walls, roof, `round_to_two_decimals`) and its prefab
  library (`hull_triangles`); the COLL reader in
  `tools/map_assets/blueprint_compiler/src/mesh_decoding/`, whose `xob-inspect` also classifies
  triangles; the sidecar batch in `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/`;
  the voxel generators in `tools/map_assets/blueprint_compiler/src/voxel_processing/`.
- Rules: ring tracing works on integer lattice coordinates and is deterministic
  (`trace_is_deterministic`), with holes wound clockwise (`donut_has_one_cw_hole`); the roof grid
  biases low and never reaches past the silhouette (`gable_box_grid_min_biases_low`,
  `coverage_ground_filter_and_erosion_guards`); surface kinds come from game materials, never from
  visual `.emat` names (`gamemat_stems_classify`); these tests are in
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/tests/` (`polygon_rings_tests.rs`, `roof_profiles_tests.rs`,
  `surface_classification_tests.rs`).
