# Terrain line-of-sight tests

Unit tests of the terrain line of sight: the elevation profile along a segment, the one-call
viewshed and the viewshed computed in budgeted steps.

## Contents

```text
legacy/map_engine/src/spatial/los/terrain/tests/
├── sampler_tests.rs      `sample_segment`: distances, endpoints, step, sampler order, coverage
├── scheduler_tests.rs    `ViewshedJob`: identical to the one-call raster at every budget, cancellation
├── viewshed_fixtures.rs  the uniform manifest and the viewshed parameters the other two files share
└── viewshed_tests.rs     `compute_viewshed`: classification, dead ground, eye anchor, cell cap
```

## Boundaries

- Depends on: `sampler`, `viewshed` and `scheduler` in `crate::spatial::los::terrain`, and the
  elevation model's `DemManifest` and `sample_elevation_meters` in `crate::world::terrain::dem`.
- Used by: `cargo test -p map_engine --all-features`; each file is mounted from `../mod.rs` with a
  `#[path]` attribute.
- Rules: the tests use synthetic elevation samplers and need no asset or GPU;
  `viewshed_perf_default_radius_is_reported` prints the timing of the 2000 m / 8 m default
  viewshed and asserts only its size.
