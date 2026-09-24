# Building blueprint items under one path

`mod.rs` re-exports the building blueprint's items from the parent folder under one path (the
blueprint, level and feature types, the footprint grids, the line-of-sight result types and the
2D helpers) and mounts the blueprint unit tests
(`apps/website/map-engine/src/world/architecture/blueprint/tests/model.rs`) as `model::tests`,
whose fixtures other test modules of the crate share.

## Contents

```text
apps/website/map-engine/src/world/architecture/blueprint/model/
└── mod.rs  the module tree; re-exports the blueprint types, LOS results and 2D helpers
```

## Boundaries

- Depends on: `structure`, `footprint`, `attribution_1` and `geometry` in
  `crate::world::architecture::blueprint`.
- Used by: tests only. The blueprint unit tests import every item through `model::*`, and the
  fixtures `room_blueprint` (a two-level 6 m × 6 m room with a window per floor and an open door),
  `room_sidecar` (its collision mesh) and `slab` under `model::tests` serve the section tests in
  `apps/website/map-engine/src/world/architecture/section/tests/` and the wash tests in
  `apps/website/map-engine/src/spatial/los/interior/tests/wash.rs`. No production code imports
  this path; callers name the item's own module.
- Rules: the module compiles only with the `io` feature and defines no item of its own; the test
  module is `pub(crate)`, which is what lets the other test modules reach its fixtures.
