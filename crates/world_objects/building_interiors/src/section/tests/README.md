# Section tests

Unit tests of the section cuts and the y-interval index, each declared by its module through
`#[path]`.

## Contents

```text
crates/world_objects/building_interiors/src/section/tests/
├── cutter_tests.rs  plan cuts, height fields, the stairwell, the roof field and level drawings
└── index_tests.rs   the indexed cut against brute force, BVH enclosure, visits and sparse memory
```

## Boundaries

- Depends on: `crate::section`, the synthetic room in `crate::test_fixtures`, and
  `spatial_indexes`' cuboid scenes and triangle BVH.
- Used by: `cargo test -p building_interiors`.
- Rules: the cases keep their assertions; `index_tests.rs` reads the FarmHouse sidecar
  `assets/terrains/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.bvh`.
