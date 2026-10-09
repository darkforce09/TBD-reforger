# Memory budget model

The pure model of the map's memory budget: a ledger of the bytes the world assets hold in
WebAssembly memory, kept against one ceiling, and the walk that chooses the finest satellite
basemap level that fits. The live ledger of `map_asset_loading`
(`crates/streaming/map_asset_loading/src/live_memory_budget/`) keeps one of these per page, reads
the page's settings and heap size, and publishes it.

## Contents

```text
crates/streaming/map_streaming_model/src/memory_budget/
├── budget_settings.rs  `budget_bytes_from_settings`: the query parameter, else the window global, else the default
├── hud_suffix.rs       `Ledger::hud_suffix`: the debug HUD tail
├── ledger.rs           `Ledger` bookkeeping: decide, reserve, hold, release, peaks, growth, the floor
├── mod.rs              the module tree; re-exports the model
├── model.rs            the assets, decisions, rows and `Ledger` type; `MIB` and `DEFAULT_BUDGET_MB`
├── satellite_floor.rs  satellite level costs and the floor walk down the mip ladder
└── tests/              unit tests for the ledger, the floor walk, the HUD tail and the settings
```

## How it works

Each of the seven assets in `Asset::ALL` (DEM, hillshade, satellite, world, forest, labels, water)
owns one row of three figures: `held`, the declared bytes not yet released; `peak`, its
high-water mark, which no release lowers; and `growth`, the linear memory measured while the
asset loaded, kept beside `held` and never counted in it. The ledger allocates and frees nothing;
loaders declare what they hold.

`decide` answers `Ok` when the bytes fit beside everything held, `Degrade` when they would fit
only an empty budget, and `Refuse` when they exceed the whole budget; `reserve` records bytes on
`Ok` alone, `hold` records unconditionally, `set_held` replaces a forecast with the real size, and
`release` saturates at zero. A satellite level costs every level from it down held at once,
decoded RGBA plus compressed bodies (`satellite_resident_bytes`); `floor_for_budget` starts at the
level the GPU limit chose, moves one level coarser per level `decide` turns down, and returns the
coarsest when none fits. The HUD tail reads `· mem <held>/<budget>MB · sat L<floor> (+<raised>)`
and is empty while nothing is held and no floor is chosen. `budget_bytes_from_settings` turns the
`memBudgetMb` query value or the `window.__memBudgetMb` global into bytes, 1,536 MiB without a
positive setting.

## Boundaries

- Depends on: nothing outside the module.
- Used by: the live ledger and the satellite loader of `map_asset_loading`
  (`crates/streaming/map_asset_loading/src/live_memory_budget/`,
  `crates/streaming/map_asset_loading/src/terrain/satellite_quadtree/`).
- Rules:
  - a request that would fit an empty budget is told to shrink, never refused, and a reservation
    records nothing unless it fits; measured growth never enters `held`; the walk never returns a
    level finer than the GPU limit's choice (the tests in `tests/` hold each); the default budget
    is 1,536 MiB;
  - an added asset extends `Asset::ALL`, `Asset::index`, `Asset::name` and the ledger's seven-row
    entry array together.
