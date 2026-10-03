# Interior line-of-sight tests

Unit tests of the compound walk, its verdicts and the floor wash.

## Contents

```text
crates/line_of_sight/interior_line_of_sight/src/tests/
├── compound_walk_tests.rs  doors, glass, foliage, trunks, shell verdicts, compound drawings, compound wash
└── floor_wash_tests.rs     grid and disc, room and slab visibility, per-level washes, radius cap, sliced job
```

## Boundaries

- Depends on: `compound_walk`, `sight_line_evaluation` and `floor_wash` in the crate, the
  compound and blueprint model of `building_interiors` and its `test_fixtures`, and the cuboid
  scenes of `spatial_indexes::test_fixtures`.
- Used by: `cargo test -p interior_line_of_sight`; each file is mounted from the module it tests
  with a `#[path]` attribute.
- Rules: the tests build their meshes in memory and need no asset.
