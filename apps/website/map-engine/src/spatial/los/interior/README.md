# Line of sight inside buildings

Traces and visibility rasters through the geometry of one building: whether an observer sees a
target past walls, door leaves, glass panes and foliage, and which cells of each floor an observer
can see.

## Contents

```text
apps/website/map-engine/src/spatial/los/interior/
├── mod.rs     the module tree
├── tests/     unit tests for the walker and the wash
├── walker.rs  observer-to-target traces through a compound building, with blocking and concealment
└── wash.rs    per-floor visibility rasters around an observer, whole or in budgeted batches
```

## How it works

`walker.rs` gives `CompoundBuilding` three answers along a segment in the building's frame:
`trace` lists every crossing of the shell and of each placed instance as a `TraceEvent` sorted by
`t` (0 at the observer, 1 at the target); `blocked` asks whether anything opaque stands on it; and
`evaluate_los` reduces the crossings to a `LosResult`. The first opaque crossing stops the ray and
names the wall, door leaf, frame, furniture or prop it hit; each glass pane adds
`GLASS_CONCEALMENT` (0.05), its two collider faces within `PANE_MERGE_M` counting once; foliage
adds `1 − exp(−FOLIAGE_K · depth)` for the metres of canopy crossed; an open door leaf the ray
passes adds an aperture hit. The concealment is `1 − Π(1 − cᵢ)` over the pass-through hits, and 1
when blocked.

`wash.rs` rasters one floor around an observer: a square grid of `WASH_CELL_M` cells (0.25 m)
covering a disc of `WASH_RADIUS_M` (25 m) by default, rows north first, each cell `Visible` or
`Hidden` by one ray from the observer to a point `WASH_EYE_M` (1 m) above the floor, and `Unknown`
outside the disc. A disc wider than `MAX_WASH_DIM` (2048) cells a side coarsens the cell to fit.
`wash_band` computes a raster in one call from any blocking closure; `WashJob` computes the same
raster in batches of `WASH_BATCH_CELLS` (256) under a time budget, and can be cancelled.

## Boundaries

- Depends on: `crate::spatial::bvh` (surface kinds, traversal hits, the sidecar mesh),
  `crate::spatial::los::terrain::viewshed` (`Visibility`, `ViewshedCapRefused`) and
  `crate::world::architecture` (building blueprints, compound buildings, their instances and rigid
  transforms, and the `LosHit` and `LosResult` types).
- Used by: `crate::spatial::los::world`, whose world occluder reuses the walker's instance trace,
  blocking test, crossing reduction and concealment fold, and whose object tree uses
  `segment_aabb_window`; `crate::editing::tools::viewshed_scheduler`, whose building-wash lane
  runs a `WashJob` in budgeted steps; the debug building viewer
  (`apps/website/frontend/src/v2/apps/debug/building_viewer.rs` and
  `apps/website/frontend/src/v2/apps/debug/building_viewer/`);
  the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s line-of-sight tool
  (`apps/website/frontend/src/v2/apps/editor/input/tools/los_world_wasm.rs`); and the blueprint
  tooling (`tools_v2/developer-tools/src/blueprint/bvh/construction.rs`).
- Rules: both modules compile only with the `io` feature; `wash_cap_check` refuses a wash radius
  above `MAX_WASH_RADIUS_M` (400 m), `WashJob::new` returns the refusal and `wash_band` returns an
  empty raster without casting a ray (`over_cap_wash_radius_is_refused_with_a_message` in
  `tests/wash.rs`); a `WashJob` may pause at any cell, because each cell's verdict depends only on
  its index, the observer, the eye height and the blocking test
  (`sliced_wash_is_bit_identical_to_the_sync_path`); glass and foliage conceal but never block
  (`glass_conceals_five_percent_per_pane_and_never_blocks`,
  `foliage_conceals_by_depth_and_trunks_block` in `tests/walker.rs`).
