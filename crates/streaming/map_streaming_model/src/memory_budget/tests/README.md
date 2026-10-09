# Memory budget tests

Unit tests for the memory budget's ledger and its satellite floor walk, run natively against the
Everon satellite mip ladder: how a request is judged and recorded, how peaks and measured growth
are kept, and how far the floor rises.

## Contents

```text
crates/streaming/map_streaming_model/src/memory_budget/tests/
├── ledger_floor_walk_and_hud_cases.rs  decisions, reservations, peaks, the floor walk and measured growth
└── mod.rs                              the module tree; the fixtures: the Everon mip ladder, budget and DEM sizes
```

## Boundaries

- Depends on: the parent module's surface through `use super::*` (`Ledger`, `Asset`, `Decision`,
  `LevelBytes`, `floor_for_budget`, `satellite_resident_bytes`, `budget_bytes_from_settings`,
  `DEFAULT_BUDGET_MB`, `MIB`).
- Used by: nothing outside the folder; `src/memory_budget/mod.rs` compiles it only in test builds
  (`#[cfg(test)] mod tests;`).
- Rules:
  - `decide` answers `Ok` when the bytes fit what is left, `Degrade` when they would fit only an
    empty budget and `Refuse` when they exceed the whole budget
    (`decide_separates_shrink_from_never`), and `reserve` records bytes on `Ok` alone
    (`reserve_records_only_on_ok`);
  - a peak is a per-asset high-water mark that no release lowers, and releasing more than is held
    saturates at zero (`peaks_are_per_asset_high_water_marks`);
  - the floor walk starts at the level the GPU limit chose, moves one level coarser per rejected
    level, never returns a finer level, and loads the coarsest level when none fits
    (`the_floor_rises_one_level_per_degrade`, `the_walk_never_reaches_past_the_ladder`,
    `the_floor_never_rises_above_the_gpu_limit_choice`);
  - measured heap growth accumulates beside the declared bytes and never counts toward them
    (`growth_is_tracked_beside_the_declared_bytes_and_never_inside_them`).
