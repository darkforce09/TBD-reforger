# Road network tests

Unit tests of `road_network`, one file per concern, each declared by the module it tests through
`#[path]`.

## Contents

```text
crates/terrain/road_network/src/tests/
├── airfield_apron_tests.rs        the apron fill over synthetic vector grids: flatness, height band, area
├── cartographic_strip_tests.rs    fence, pier and bridge-rail strips along a footprint's long axis
├── export_image_styling_tests.rs  export image styles, layer file round trip, draw order permutation
├── network_tests.rs               segments from the JSON export and the archive, widths, refusals
└── styling_tests.rs               the class table, zoom gates, class signature and strip expansion
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`), `terrain_elevation`'s vector grid,
  `prefab_catalog`'s footprint corners and `world_file_formats`' road archive codec.
- Used by: `cargo test -p road_network`.
- Rules: the cases keep their assertions and fixtures; inputs are built in memory, no file is read.
