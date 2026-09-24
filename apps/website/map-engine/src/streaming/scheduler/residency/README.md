# World residency re-exports and tests

The scheduler's `residency` module: one path that re-exports the world residency type, its
ingest-outcome and residency-event types and its tuning constants, and the unit tests that drive
a residency through viewports, ingests, evictions and buffer rebuilds.

## Contents

```text
apps/website/map-engine/src/streaming/scheduler/residency/
├── mod.rs            the module tree; re-exports `WorldResidency`, its result types and constants
├── t151_11_3_tests/  unit tests for the ingest budget and the building lane toggles
├── t152_3_tests/     unit tests for glyphs, badges and strips on the committed Everon export
└── tests/            unit tests for the residency lifecycle, eviction, draw set and tree heatmap
```

## How it works

`mod.rs` defines no item of its own. It re-exports `WorldResidency`, `IngestOutcome` and
`ResidencyEvent` from `crate::streaming::scheduler::state`; `LRU_MIN_CHUNKS`, `FETCH_FAILURE_CAP`
and `DRAW_CULL_MARGIN_M` from `crate::streaming::scheduler::viewport`; `APPLY_BUDGET_MS` from
`crate::streaming::scheduler::budget`; and `BUILDING_MIN_ZOOM` from
`crate::streaming::buffers::revision`. In test builds it also imports three prefab readers from
`crate::world::environment::buildings` and declares the three test modules, which take all of it
through `use super::*`.

| Test module | What it drives | Data |
|---|---|---|
| `tests/` | the lifecycle: requested set, pin key, known-empty chunks, retry cap, LRU eviction, apply-frame accounting, picking, draw set, tree heatmap, glyph memo | synthetic gzip chunks on a 25 × 25 grid of 512 m cells |
| `t151_11_3_tests/` | the ingest budget, the buildings toggle, the building zoom gate | one synthetic building |
| `t152_3_tests/` | the glyph lookup, atlas keys, badges, landmarks, fill de-emphasis, strips | the export under `assets_v2/terrains/everon/` and the glyphs under `assets_v2/glyphs/` |

## Public surface

- `WorldResidency`, `IngestOutcome`, `ResidencyEvent`, `LRU_MIN_CHUNKS`, `FETCH_FAILURE_CAP`,
  `DRAW_CULL_MARGIN_M`, `APPLY_BUDGET_MS` and `BUILDING_MIN_ZOOM` under
  `crate::streaming::scheduler::residency`: nothing outside the folder names this path; callers
  import each item from its owning module.

## Boundaries

- Depends on: `crate::streaming::scheduler` (`state`, `viewport`, `budget`) and
  `crate::streaming::buffers::revision` for the re-exported items;
  `crate::world::environment::buildings` for the prefab readers the tests share; in the tests,
  `crate::overlay` (the class gates and `INSTANCE_BUDGET`), `crate::world::environment::vegetation`
  (the tree counts and the density grid), `flate2` and `serde_json`.
- Used by: nothing outside the folder names `scheduler::residency`;
  `apps/website/map-engine/src/streaming/scheduler/mod.rs` declares the module.
- Rules:
  - the module holds re-exports only, so every re-exported type and constant keeps one
    definition, in its owning module;
  - the tests in `tests/` hold the residency lifecycle:
    - a viewport requests exactly the chunk-math set (`requests_exactly_the_chunk_math_set`); a
      zoom below the building band, or an unchanged pin whose chunks are in flight, requests
      nothing (`skip_below_building_band_and_unchanged_set`); and an unchanged pin requests its
      missing chunks again once the in-flight marks are cleared
      (`clear_inflight_allows_same_key_rerequest`);
    - a parsed empty chunk is known-empty and never requested again
      (`parsed_empty_chunk_is_known_empty_and_not_refetched`); a malformed chunk is retried up to
      `FETCH_FAILURE_CAP` and then cached as empty (`shape_mismatch_retries_to_cap_then_caches`),
      and a new pin key resets the count (`fetch_failures_reset_on_new_pin_key`);
    - residency stays within `LRU_MIN_CHUNKS` or three times the pinned count, whichever is
      larger, and never evicts a pinned chunk (`lru_caps_and_never_evicts_pinned`);
    - the draw set is the strict viewport's chunks inside the pinned set, which the preload margin
      makes larger (`class_s_draw_set_equals_strict_reference`);
    - trees switch to the heatmap above `INSTANCE_BUDGET` visible trees and back below 85 % of it
      (`class_r_heatmap_swap_and_full_pack`, `class_r_heatmap_hysteresis`), and no zoom step
      leaves the forest blank (`property_never_blank_zoom_ladder`);
    - an identical viewport rebuilds nothing, while a zoom change or a new chunk does
      (`compose_memo_stable_then_bumps`, `compose_memo_invalidates_on_new_chunk`);
  - the crate's tests need every feature (`map_engine_tests_require_all_features` in
    `apps/website/map-engine/src/tests/feature_gate_tripwire.rs`), and `cargo xtask mk wasm-ci`
    runs them with `--all-features`.
