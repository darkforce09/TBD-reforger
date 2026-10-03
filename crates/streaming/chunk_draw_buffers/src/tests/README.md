# World residency tests

The unit tests of the world residency: each drives a `WorldResidency` through viewports, ingests,
evictions, toggles and frame closes, and checks the chunk residency and the draw buffers composed
over it together.

## Contents

```text
crates/streaming/chunk_draw_buffers/src/tests/
├── everon_glyphs_and_strips/           glyphs, badges and strips on the committed Everon export
├── ingest_budget_and_building_toggle/  the ingest budget and the building lane toggles
├── mod.rs                              the module tree, compiled only in test builds
├── prefab_lane_parity.rs               the archive and JSON prefab lanes compose the same buffers
└── residency_lifecycle/                the lifecycle, eviction, draw set and tree heatmap
```

## How it works

`mod.rs` declares the four test modules; each imports the items it drives from their owning
modules (`crate::world_residency`, `chunk_scheduler`). The cases
that set the viewport, the zoom or a resident chunk's rows directly do so through the scheduler's
test hooks (its `test_fixtures` feature, enabled from this crate's `[dev-dependencies]`), and reach
the draw buffers through the world residency's fields, which the whole crate sees. A chunk the
cases name by its string becomes a `world_chunks::ChunkId` where a residency call takes one.

| Test module | What it drives | Data |
|---|---|---|
| `residency_lifecycle/` | the lifecycle: requested set, pin key, known-empty chunks, retry cap, LRU eviction, apply-frame accounting, picking, draw set, tree heatmap, glyph memo | synthetic gzip chunks on a 25 × 25 grid of 512 m cells |
| `ingest_budget_and_building_toggle/` | the ingest budget, the buildings toggle, the building zoom gate | one synthetic building |
| `everon_glyphs_and_strips/` | the glyph lookup, atlas keys, badges, landmarks, fill de-emphasis, strips | the export under `assets/terrains/everon/` and the glyphs under `assets/glyphs/` |
| `prefab_lane_parity.rs` | the rkyv and JSON prefab lanes through to every draw buffer and the statistics | `prefab_catalog::test_fixtures` and `assets/glyphs/` |

## Boundaries

- Depends on: the crate root (`WorldResidency`, `DrawBuffers`) and
  `chunk_scheduler` (`IngestOutcome`, the tuning constants, the test hooks); in the
  cases, `map_draw_lanes` and `label_layout::glyph_math` (the class gates, `INSTANCE_BUDGET` and
  the icon keys), `vegetation::canopy` (the tree counts and the density grid), `prefab_catalog`
  (the payload reader, the prefab readers, class codes and fixtures), `render_primitives`
  (`norm`), `road_network` (the strip geometry), `flate2` and `serde_json`.
- Used by: nothing outside the folder; `crates/streaming/chunk_draw_buffers/src/lib.rs` compiles it
  only in test builds (`#[cfg(test)] mod tests;`).
- Rules:
  - the tests in `residency_lifecycle/` hold the residency lifecycle:
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
  - the archive and JSON prefab lanes build equal prefab tables and, through a full viewport pass,
    equal draw buffers, glyph tables and statistics
    (`everon_archive_lane_builds_the_same_residency_as_the_json_lane`);
  - `cargo test -p chunk_draw_buffers` runs every case; the crate has no feature of its own.
