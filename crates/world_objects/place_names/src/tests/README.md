# Place names tests

Unit tests of `place_names`, one file per tested module, each declared by its module through
`#[path]`.

## Contents

```text
crates/world_objects/place_names/src/tests/
├── peaks_tests.rs            peak finding, the spot-height declutter and zoom band, Everon's DEM
├── route_placement_tests.rs  road name placement, declutter, geometry, class codes, the road lane
└── towns_tests.rs            the label JSON parsers, the archive's lanes, the whole-file reader
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`), `terrain_elevation`'s PNG decode,
  `road_network`'s segments and class codec, and `world_file_formats`' labels archive codec.
- Used by: `cargo test -p place_names`.
- Rules: the cases keep their assertions and fixtures; `peaks_tests.rs` reads the Git LFS file
  `assets/terrains/everon/dem/everon-dem-16bit.png` and fails with its cause when the file is
  missing or only a pointer.
