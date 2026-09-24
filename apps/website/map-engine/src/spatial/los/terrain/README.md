# Line of sight over terrain

Line of sight over the bare ground of the elevation model (DEM): the elevation profile along a
sight line, and the viewshed, a raster of which cells around an observer are visible, in dead
ground, or unknown, computed in one call or in budgeted steps.

## Contents

```text
apps/website/map-engine/src/spatial/los/terrain/
├── march.rs      the radial march: the ray and step schedule and the horizon rule for one sample
├── mod.rs        the module tree
├── overlay.rs    `viewshed_upload` and `viewshed_clear`, which show a raster as the viewshed lane
├── sampler.rs    `sample_segment`, the ground elevation profile along a segment
├── scheduler.rs  `ViewshedJob`, the viewshed computed a ray at a time under a time budget
└── viewshed.rs   `Visibility`, the `Viewshed` raster and its grid, the cell cap, `compute_viewshed`
```

## How it works

The profile and the viewshed take the DEM's `DemManifest` and an injected elevation sampler
`elev_at(x, y) -> Option<f64>`, in world metres of the map frame (x east, y north), so the same
code runs over any grid. A point outside the manifest's coverage, or one the sampler cannot answer,
is never given an invented height: `sample_segment` leaves it out of the profile, and the viewshed
marks it `Unknown` unless another ray already saw it.

`compute_viewshed` sizes the raster with `viewshed_grid`: square cells of `cell_m` (8 m when the
value is not positive), out to `radius_m` (`VIEWSHED_DEFAULT_RADIUS_M`, 2000 m, when not positive),
clipped to the DEM. The march then casts rays out from the observer's eye (its ground plus
`eye_height_m`), spaced half a cell apart at the rim and stepping half a cell, the 2× oversample of
`march_schedule`. Along each ray a sample is `Visible` when the slope from the eye down or up to
its ground is at least the steepest slope met so far on that ray, and `Hidden` otherwise; a cell
once `Visible` stays so. The observer's own cell is `Visible`; an observer with no ground makes
every cell `Unknown`.

`ViewshedJob` marches the same rays in the same order, a whole ray at a time between budget checks,
so its raster equals the synchronous one; `generation` is the caller's cancel token and `cancel`
retires a job in place. In the browser, `crate::editing::tools::line_of_sight` packs a finished
raster into RGBA rows, and `viewshed_upload` shows them as the `Viewshed` texture lane over the
raster's world rectangle; its rows must be at least four bytes a texel and 256-byte aligned.

## Boundaries

- Depends on: `crate::world::terrain::dem` (`DemManifest`, `in_coverage`); for `overlay.rs`,
  `crate::frame::engine` (`RenderEngine`), `crate::overlay::lanes` (`LaneRole::Viewshed`),
  `crate::world::scene` (the world-to-scene rectangle), `crate::world::terrain::satellite`
  (`TexLane`), `wgpu` and `wasm-bindgen`.
- Used by: `crate::editing::tools::line_of_sight`, whose terrain survey computes the profile and
  the viewshed and whose verdict, projection, palette and texture read them;
  `crate::editing::tools::viewshed_scheduler`, whose terrain lane runs a `ViewshedJob`;
  `crate::spatial::los::interior`, whose wash reuses `Visibility` and `ViewshedCapRefused`;
  `crate::world::terrain::dem::sample`, which re-exports the profile and viewshed items and holds
  their tests; and the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s input
  handlers (`apps/website/frontend/src/v2/apps/editor/input/`) and canvas mount
  (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`) and the debug
  building viewer (`apps/website/frontend/src/v2/apps/debug/building_viewer/`), which upload and
  clear the viewshed lane.
- Rules: `overlay.rs` compiles only for wasm32 with the `render` feature, and nothing else here
  touches the GPU or the browser; a raster over `MAX_VIEWSHED_CELLS` (300 000) cells is refused:
  `ViewshedJob::new` returns the `ViewshedCapRefused` naming the cap and the measured count, and
  `compute_viewshed` returns an empty raster, while the 2000 m / 8 m default (251 001 cells) passes
  (`over_cap_viewshed_is_refused_with_a_message`); the sliced raster is identical to the
  synchronous one (`sliced_viewshed_is_bit_identical_to_the_sync_path`); an observer off coverage
  yields an all-`Unknown` raster (`viewshed_observer_off_coverage_is_all_unknown`); all in
  `apps/website/map-engine/src/world/terrain/dem/sample/tests/cases_1.rs`.
