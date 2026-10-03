# Terrain line-of-sight tests

Unit tests of the terrain line of sight: the elevation profile along a segment, the one-call
viewshed and the viewshed computed in budgeted steps.

## Contents

```text
crates/line_of_sight/terrain_line_of_sight/src/tests/
├── elevation_profile_tests.rs  `sample_segment`: distances, endpoints, step, sampler order, coverage
├── viewshed_fixtures.rs        the uniform manifest and the viewshed parameters the other two files share
├── viewshed_job_tests.rs       `ViewshedJob`: identical to the one-call raster at every budget, cancellation
└── viewshed_tests.rs           `compute_viewshed`: classification, dead ground, eye anchor, cell cap
```

## Boundaries

- Depends on: `elevation_profile`, `viewshed` and `viewshed_job` in the crate, and the elevation
  model's `DemManifest` and samplers in `terrain_elevation`.
- Used by: `cargo test -p terrain_line_of_sight`; each test file is mounted from the module it
  tests with a `#[path]` attribute, the fixtures from `lib.rs`.
- Rules: the tests use synthetic elevation samplers and need no asset or GPU;
  `viewshed_perf_default_radius_is_reported` prints the timing of the 2000 m / 8 m default
  viewshed and asserts only its size.
