# Blueprint tests

Unit tests of the blueprint model and its sight-line attribution, declared by `blueprint/mod.rs`
through `#[path]`.

## Contents

```text
crates/world_objects/building_interiors/src/blueprint/tests/
└── sight_line_tests.rs  the FarmHouse JSON parse, band clipping, and every attribution rule over the synthetic room
```

## Boundaries

- Depends on: `crate::blueprint`, the synthetic room in `crate::test_fixtures`, and
  `spatial_indexes`' cuboid scenes.
- Used by: `cargo test -p building_interiors`.
- Rules: the cases keep their assertions; the FarmHouse case reads
  `assets/terrains/everon/prefabs/buildings/FarmHouse_E_1L01.json`.
