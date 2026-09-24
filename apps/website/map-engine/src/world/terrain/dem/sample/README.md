# Elevation sampling and terrain sight items under one path

`mod.rs` re-exports the elevation model's manifest and sampling items and the terrain
line-of-sight items of `crate::spatial::los::terrain` (the segment sampler, the viewshed and its
sliced job) under one path, and mounts their unit tests, which exercise the two together.

## Contents

```text
apps/website/map-engine/src/world/terrain/dem/sample/
├── mod.rs  the module tree; re-exports the DEM sampling and terrain line-of-sight items
└── tests/  unit tests for elevation sampling, segment profiles and viewsheds
```

## Boundaries

- Depends on: `manifest` and `sampling` in `crate::world::terrain::dem`; `sampler`, `viewshed`
  and `scheduler` in `crate::spatial::los::terrain`.
- Used by: nothing outside the folder; callers import the same items from their own modules.
- Rules: the module defines no item of its own. Its tests hold the sampling rules: a stored 0
  reads exactly the minimum height and 65 535 the maximum (`zero_is_exact_min`,
  `full_scale_is_max_within_epsilon` in `tests/cases_1.rs`); the world rectangle's corners map to
  the first and last pixel, mirrored when the manifest flips an axis (`world_to_pixel_endpoints`,
  `world_to_pixel_axis_flip`); a point off the raster samples as `None`
  (`sample_elevation_out_of_bounds_is_none`); a segment profile keeps both endpoints and drops the
  samples off the coverage (`segment_includes_both_endpoints_and_respects_step`,
  `segment_drops_off_coverage_samples`); the sliced viewshed job matches the one-call viewshed bit
  for bit and can be cancelled (`sliced_viewshed_is_bit_identical_to_the_sync_path`,
  `viewshed_job_cancels_mid_disc`); a viewshed over the cell cap is refused with a message while the
  default 2000 m radius at 8 m cells is not (`over_cap_viewshed_is_refused_with_a_message`).
