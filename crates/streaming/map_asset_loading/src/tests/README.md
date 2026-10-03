# map_asset_loading tests

The crate's native unit tests that sit beside no module folder of their own.

## Contents

```text
crates/streaming/map_asset_loading/src/tests/
└── mesh_composition_tests.rs  land cover, contour and hairline meshes, with the road, sea and sea band meshes they are drawn with
```

## Boundaries

- Depends on: `crate::mesh_composition`, `render_primitives`, `road_network`, `terrain_elevation`,
  `terrain_relief` and `water_bodies`.
- Used by: `cargo test -p map_asset_loading`.
- Rules: the tests keep their names and assertions; they run natively.
