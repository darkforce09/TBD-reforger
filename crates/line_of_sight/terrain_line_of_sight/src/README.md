# Terrain line of sight source

The source of `terrain_line_of_sight`: the elevation profile, the radial march, the viewshed and
its sliced job, the error and the crate root that declares them.

## Contents

```text
crates/line_of_sight/terrain_line_of_sight/src/
├── elevation_profile.rs  `sample_segment`, the ground elevation profile along a segment
├── error.rs              `Error` and `Result`: a `ViewshedCapRefused` behind one type
├── lib.rs                the crate root: module header, `mod` lines and re-exports
├── prelude.rs            the names most readers import
├── radial_march.rs       the ray and step schedule and the horizon rule for one sample (private)
├── tests/                unit tests for the profile, the viewshed and the sliced job
├── viewshed.rs           `Visibility`, the `Viewshed` raster and its grid, the cell cap, `compute_viewshed`
└── viewshed_job.rs       `ViewshedJob`, the viewshed computed a ray at a time under a time budget
```

## How it works

`viewshed.rs` declares the raster and its request and computes it in one call through
`radial_march.rs`; `viewshed_job.rs` marches the same rays through the same march in slices.
`elevation_profile.rs` stands alone. Each module mounts its tests from `tests/`; the shared
viewshed fixtures are mounted from `lib.rs`.

## Boundaries

- Depends on: `terrain_elevation` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: only `radial_march.rs` is private; nothing here touches a browser or a GPU.
