# Line of sight

Every visibility question the map engine answers, at three scales: over the bare ground of the
elevation model, through the objects placed on the streamed terrain, and inside one building. The
three are separate layers, each with its own data structure and cost, and are not merged.

## Contents

```text
apps/website/map-engine/src/spatial/los/
├── interior/  inside one building: traces through its meshes and per-floor visibility rasters
├── mod.rs     the module tree
├── terrain/   over the elevation model: sight-line profiles and viewsheds
└── world/     through every placed object of the streamed world, chunk by chunk
```

## How it works

| Layer | Answers | Traces through | Feature |
|---|---|---|---|
| `terrain/` | a profile along a line; a viewshed raster | a radial march over the DEM | `world` |
| `world/` | a verdict with its crossings and coverage | per-chunk box trees, then meshes | `streaming` |
| `interior/` | a verdict in one building; floor rasters | the building's own meshes | `io` |

The layers share their vocabulary rather than their work. The terrain viewshed's `Visibility`
(visible, hidden, unknown) and `ViewshedCapRefused` (the cap, its limit and the measured value)
serve the building wash too. The world occluder traces each expanded prefab with the interior
walker and reduces the crossings the same way: an opaque surface stops the ray, glass and foliage
add concealment. The world occluder's `blocked_fn` has the shape of blocking test the interior's
`wash_band` takes, so a floor-style raster can be washed over the streamed world's objects
(`the_blocked_closure_drives_a_wash_over_two_chunks` in `world/tests/occluder.rs`). The
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s line-of-sight tool instead
refines the terrain viewshed with its own budgeted pass in `crate::editing::tools::line_of_sight`,
asking the world occluder cell by cell.

Both rasters come in two forms: one call (`compute_viewshed`, `wash_band`), or a job that does the
same work in slices under a time budget (`ViewshedJob` a ray at a time, `WashJob` in batches of
cells) and produces the identical raster. A job carries the caller's `generation` token and can be
cancelled; `crate::editing::tools::viewshed_scheduler` runs them.

## Public surface

- `terrain`: `viewshed::{compute_viewshed, Viewshed, ViewshedParams, Visibility,
  ViewshedCapRefused}`, `scheduler::ViewshedJob`, `sampler::sample_segment`, and the browser's
  `viewshed_upload` and `viewshed_clear` on `RenderEngine`.
- `interior`: `CompoundBuilding::{trace, blocked, evaluate_los}` from `walker`, and
  `wash::{wash_band, WashJob, WashParams, LevelWash, level_wash, level_washes}` with the compound
  forms.
- `world`: `WorldOccluder`, `WorldLos`, `WorldVerdict`, `BlockPolicy`, `map_to_engine`, and the
  `descriptor` data model.

## Boundaries

- Depends on: `crate::spatial::bvh` (meshes, trees, surface kinds); `crate::world::terrain::dem`,
  `crate::world::architecture` and `crate::world::environment::buildings`; `crate::streaming` for
  the world layer's chunks; `crate::io::archives`; and, for the browser upload only,
  `crate::frame`, `crate::overlay` and `wgpu`.
- Used by:
  - `crate::editing::tools::line_of_sight` and `crate::editing::tools::viewshed_scheduler`, the
    Mission Creator's line-of-sight tool and the scheduler of its visibility jobs;
  - `crate::streaming`, whose occluder loader and host queries own and lend the world occluder;
  - `crate::world::terrain::dem::sample`, which re-exports the terrain profile and viewshed items;
  - the Mission Creator's input handlers in `apps/website/frontend/src/v2/apps/editor/input/`, and
    the debug benches in `apps/website/frontend/src/v2/apps/debug/`;
  - the blueprint tooling and the map checks in `tools_v2/developer-tools/src/`.
- Rules: the three layers stay three modules; a request over a cap is refused with one
  `ViewshedCapRefused` that names the cap and the measured value
  (`over_cap_viewshed_is_refused_with_a_message` in
  `apps/website/map-engine/src/world/terrain/dem/sample/tests/cases_1.rs`,
  `over_cap_wash_radius_is_refused_with_a_message` in `interior/tests/wash.rs`); a sliced job's
  raster equals the one-call raster (`sliced_viewshed_is_bit_identical_to_the_sync_path`,
  `sliced_wash_is_bit_identical_to_the_sync_path`).
