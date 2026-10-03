# World line-of-sight tests

Unit tests of the world occluder and of the chunk walk.

## Contents

```text
crates/line_of_sight/world_line_of_sight/src/tests/
├── chunk_cells_tests.rs  `cells_on_segment` against the brute-force rasteriser that lives only here
└── occluder_tests.rs     box tree, placement, verdicts, coverage, policies, doors, wash, budget, removal
```

## Boundaries

- Depends on: the crate root's public surface, `interior_line_of_sight`'s floor wash,
  `terrain_line_of_sight`'s `Visibility`, `building_interiors`' instance records,
  `prefab_catalog`'s `PrefabRow`, `world_chunks`' `WorldChunk` and `spatial_indexes`' BVH.
- Used by: `cargo test -p world_line_of_sight`; `occluder_tests.rs` is mounted from `lib.rs`,
  `chunk_cells_tests.rs` from `chunk_cells.rs`.
- Rules: the tests build chunks, descriptors and cuboid sidecars in memory and need no asset; the
  random cases run from fixed seeds.
