# Terrain line of sight

The `terrain_line_of_sight` crate: line of sight over the bare ground of a terrain's elevation
model. It samples the ground's elevation profile along a sight line and computes the viewshed
around an observer, the raster of cells the observer's eye sees, cannot see, or cannot judge, in
one call or a ray at a time under a time budget. It holds no browser or GPU code; the map engine
uploads a finished raster as the viewshed lane.

## Contents

```text
crates/line_of_sight/terrain_line_of_sight/
├── Cargo.toml  the package: `terrain_elevation`, `thiserror`, layout tier 3
└── src/        the elevation profile, the radial march, the viewshed and its sliced job
```

## How it works

Every call takes the elevation model's `DemManifest` and an injected sampler
`elev_at(x, y) -> Option<f64>` in world metres of the map frame (x east, y north), so the same
code runs over any grid. A point outside the manifest's coverage, or one the sampler cannot
answer, is never given an invented height: `sample_segment` leaves it out of the profile, and the
viewshed marks it `Unknown` unless another ray already saw it.

`compute_viewshed` sizes the raster with `viewshed_grid`: square cells of `cell_m` (8 m when the
value is not positive), out to `radius_m` (`VIEWSHED_DEFAULT_RADIUS_M`, 2000 m, when not
positive), clipped to the elevation model. Rays leave the observer's eye (its ground plus
`eye_height_m`) half a cell apart at the rim and step half a cell. Along each ray a sample is
`Visible` when the slope from the eye to its ground is at least the steepest slope met so far on
that ray, and `Hidden` otherwise; a cell once `Visible` stays so. `ViewshedJob` marches the same
rays in the same order, a whole ray between budget checks, so its raster equals the one-call one.

## Getting started

Run from the repository root:

```bash
cargo test -p terrain_line_of_sight   # profile, viewshed, sliced job, cell cap
```

## Public surface

- `elevation_profile`: `sample_segment` and `ProfileSample`.
- `viewshed`: `compute_viewshed`, `viewshed_grid`, `Viewshed`, `ViewshedGrid`, `ViewshedParams`,
  `Visibility`, `ViewshedCapRefused`, `MAX_VIEWSHED_CELLS`, `VIEWSHED_DEFAULT_RADIUS_M`.
- `viewshed_job`: `ViewshedJob` (`new`, `step`, `raster`, `into_raster`, `progress`, `cancel`).
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `terrain_elevation` (`DemManifest`, `in_coverage`; the samplers in the tests) and
  `thiserror`.
- Used by:
  - `interior_line_of_sight`, whose floor wash reuses `Visibility` and `ViewshedCapRefused`;
  - `world_line_of_sight`;
  - the map engine (`legacy/map_engine`): its line-of-sight tool and visibility scheduler under
    `crates/mission_editing/map_editing_tools/src/`;
  - the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s input handlers,
    the debug building viewer and the mortar map picker in `apps/frontend/`.
- Rules: a raster over `MAX_VIEWSHED_CELLS` (300 000) cells is refused, `ViewshedJob::new`
  returning the `ViewshedCapRefused` that names the cap and the measured count and
  `compute_viewshed` an empty raster, while the 2000 m / 8 m default (251 001 cells) passes
  (`over_cap_viewshed_is_refused_with_a_message`); the sliced raster equals the one-call raster
  (`sliced_viewshed_is_bit_identical_to_the_sync_path`); line of sight tier 3
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Line of sight crates](/crates/line_of_sight/README.md) — the three layers and how they share
  their vocabulary.
