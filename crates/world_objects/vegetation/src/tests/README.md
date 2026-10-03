# Vegetation tests

Unit tests of `vegetation`, one file per module, each declared by its module through `#[path]`.

## Contents

```text
crates/world_objects/vegetation/src/tests/
├── canopy_tests.rs   exact and visible tree counts, the heatmap switch, the density grid's texel sum
├── density_tests.rs  the island stitch without seams, north as row 0, the island dimensions
├── mass_tests.rs     the marching-squares outline and fill and the fill's opacity ladder
└── regions_tests.rs  regions from the JSON goldens and Everon's export, the archive round trip
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`), `world_chunks`' chunk record,
  `prefab_catalog`'s class codes and payload decoding, and `world_file_formats`' forest archive.
- Used by: `cargo test -p vegetation`.
- Rules: the cases keep their assertions and fixtures; `regions_tests.rs` reads the map goldens in
  `contracts/fixtures/map/` and Everon's `assets/terrains/everon/objects/forest-regions.json.gz`.
