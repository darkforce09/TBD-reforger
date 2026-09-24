# Ingest budget and building lane tests

Unit tests that drive a `WorldResidency` holding one resident building: the per-frame ingest
budget, the buildings toggle that empties and refills the building buffers, and the zoom gate
below which buildings stop drawing.

## Contents

```text
apps/website/map-engine/src/streaming/scheduler/residency/t151_11_3_tests/
├── cases_1.rs  the cases: ingest budget, buildings toggle, building zoom gate
└── mod.rs      the module tree; the fixture: a residency with one building ingested
```

## Boundaries

- Depends on: the parent module's re-exports through `use super::*` (`WorldResidency`,
  `APPLY_BUDGET_MS`); the residency's manifest, prefab, viewport, ingest, budget, buffer and
  toggle methods; `flate2` to gzip the fixture chunk.
- Used by: nothing outside the folder;
  `apps/website/map-engine/src/streaming/scheduler/residency/mod.rs` compiles it only in test
  builds (`#[cfg(test)] mod t151_11_3_tests;`).
- Rules:
  - the ingest budget reads exhausted only inside an open frame, from `APPLY_BUDGET_MS` (4 ms)
    after its start, and closing a frame that ran longer counts one frame over budget
    (`ingest_budget_policy_is_core_owned`);
  - turning buildings off empties the building fill and outline buffers, and turning them back on
    refills them (`buildings_toggle_hides_and_restores_whole_lane`);
  - a viewport below the building zoom gate (`BUILDING_MIN_ZOOM`, -2.5) hides the buildings
    (`buildings_visible_respects_zoom_gate`).
