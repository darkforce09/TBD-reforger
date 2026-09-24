# Memory budget tests

Unit tests for the memory budget's ledger and its satellite floor walk, run natively against the
Everon satellite mip ladder: what each level costs, how a request is judged and recorded, how
peaks and measured growth are kept, and how far the floor rises under a tight budget.

## Contents

```text
apps/website/map-engine/src/streaming/memory/budget/t938_6/
├── cases_1.rs  the cases: level costs, decisions, reservations, peaks, the floor walk, the HUD
└── mod.rs      the module tree; the fixtures: the Everon mip ladder, budget and DEM sizes
```

## Boundaries

- Depends on: the parent module's surface through `use super::*` (`Ledger`, `Asset`, `Decision`,
  `LevelBytes`, `floor_for_budget`, `satellite_resident_bytes`, `configured_budget_bytes`,
  `heap_bytes`, `DEFAULT_BUDGET_MB`, `MIB`).
- Used by: nothing outside the folder; `apps/website/map-engine/src/streaming/memory/budget/mod.rs`
  compiles it only in test builds (`#[cfg(test)] mod t938_6;`).
- Rules:
  - a satellite base level holds every level from it down at once, decoded RGBA plus compressed
    bodies: 1,026,523,730 bytes from Everon's level 0 and 260,606,070 from level 1, and a base
    past the ladder costs nothing (`the_everon_ladder_costs_what_the_audit_says`);
  - `decide` answers `Ok` when the bytes fit what is left, `Degrade` when they would fit only an
    empty budget and `Refuse` when they exceed the whole budget
    (`decide_separates_shrink_from_never`), and `reserve` records bytes on `Ok` alone
    (`reserve_records_only_on_ok`);
  - a peak is a per-asset high-water mark that no release lowers, and releasing more than is held
    saturates at zero (`peaks_are_per_asset_high_water_marks`);
  - the floor walk starts at the level the GPU limit chose, moves one level coarser per rejected
    level, never returns a finer level, and loads the coarsest level when none fits
    (`the_floor_rises_one_level_per_degrade`, `a_tighter_budget_walks_further_down_the_ladder`,
    `the_walk_never_reaches_past_the_ladder`, `the_floor_never_rises_above_the_gpu_limit_choice`);
  - the HUD tail reads the held bytes against the budget and the floor with its raise count, and
    is empty while nothing is held and no floor is chosen
    (`the_hud_shows_reserved_against_budget_and_the_floor`);
  - measured heap growth accumulates beside the declared bytes and never counts toward them
    (`growth_is_tracked_beside_the_declared_bytes_and_never_inside_them`);
  - the default budget is 1,536 MiB, and a native build measures no linear memory
    (`the_default_budget_is_the_one_the_spec_names`); every asset has its own row
    (`every_asset_indexes_its_own_row`).
