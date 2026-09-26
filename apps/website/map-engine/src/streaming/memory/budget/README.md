# Memory budget

A per-session ledger of the bytes the map's world assets hold in WebAssembly memory, kept against
one ceiling: it chooses the finest satellite basemap level that fits, and reports the held, peak
and measured bytes to the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s debug
HUD and to the page.

## Contents

```text
apps/website/map-engine/src/streaming/memory/budget/
├── accounting.rs  the live ledger's calls: hold, set, release, heap marks, satellite floor claim
├── ledger.rs      `Ledger` bookkeeping: decide, reserve, hold, release, peaks, growth, the floor
├── mod.rs         the module tree; re-exports the budget's calls, holds the thread-local ledger
├── model.rs       the assets, decisions, rows and `Ledger` type; `MIB` and `DEFAULT_BUDGET_MB`
├── platform.rs    the configured budget (`?memBudgetMb`, `window.__memBudgetMb`) and the heap size
├── satellite.rs   satellite level costs and the floor walk down the mip ladder
├── stats.rs       the HUD tail and the `window.__t9386` snapshot
└── t938_6/        unit tests for the ledger, the floor walk and the HUD tail
```

## How it works

```text
host bootstrap and terrain load          satellite basemap loader
  hold / set_held / release                claim_satellite_floor(levels, GPU-limit level)
  heap_mark ... observe_since                floor_for_budget: coarser until decide says Ok
        │                                    reserve(Satellite), set_satellite_floor
        ▼                                          ▼
      thread-local LEDGER, budget = configured_budget_bytes()
        │ publish() on hold, set, release, claim   │ hud_suffix()
        ▼                                          ▼
      window.__t9386                             debug HUD tail
```

The live ledger is a thread-local `Ledger` built on first use from `configured_budget_bytes()`: the
`memBudgetMb` query parameter in MiB, else a positive `window.__memBudgetMb`, else
`DEFAULT_BUDGET_MB` (1,536 MiB); a native build always takes the default and measures no heap.

Each of the seven assets in `Asset::ALL` (DEM, hillshade, satellite, world, forest, labels, water)
owns one row of three figures: `held`, the declared bytes not yet released; `peak`, its
high-water mark, which no release lowers; and `growth`, the linear memory measured between a
`heap_mark` and `observe_since` while the asset loaded, kept beside `held` and never counted in it.
The ledger allocates and frees nothing; loaders declare what they hold.

`decide` answers `Ok` when the bytes fit beside everything held, `Degrade` when they would fit
only an empty budget, and `Refuse` when they exceed the whole budget; `reserve` records bytes on
`Ok` alone, `hold` records unconditionally, `set_held` replaces a forecast with the real size, and
`release` saturates at zero. The one choice the budget makes is the satellite floor: a level costs
every level from it down held at once, decoded RGBA plus compressed bodies
(`satellite_resident_bytes`), and the walk starts at the level the GPU limit chose, moves one
level coarser per level `decide` turns down, loads the coarsest when none fits, reserves the
chosen cost for the satellite and records the floor and how many levels it rose. The HUD tail
reads `· mem <held>/<budget>MB · sat L<floor> (+<raised>)` and is empty while nothing is held and
no floor is chosen.

## Public surface

- `hold`, `set_held`, `release`, `heap_mark`, `observe_since` and `publish`: the accounting calls
  of the streaming host's bootstrap and terrain load.
- `claim_satellite_floor`, `with_ledger`, `LevelBytes` and `MIB`: the satellite basemap loader's
  floor claim and its budget warnings.
- `hud_suffix`: the debug HUD tail the Mission Creator's frame pump appends.
- `Ledger`, `Asset`, `Decision`, `Entry`, `FloorWalk`, `floor_for_budget`,
  `satellite_resident_bytes`, `configured_budget_bytes`, `heap_bytes` and `DEFAULT_BUDGET_MB`: the
  model the calls above work on.

## Boundaries

- Depends on: nothing else of the crate; `web-sys`, `js-sys` and `wasm-bindgen` on wasm32 for the
  page's query string, the window globals and the linear memory size.
- Used by:
  - `crate::streaming::host`, whose bootstrap and terrain load declare the DEM and hillshade bytes,
    measure the world, forest, water and label growth, and re-export `hud_suffix` as
    `memory_hud_suffix`;
  - `crate::world::terrain::satellite::quadtree`, which claims the floor at boot and names the
    budget's share of a downscale in its warning;
  - the Mission Creator's frame pump, which shows the HUD tail
    (`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`).
- Rules:
  - a request that would fit an empty budget is told to shrink, never refused, and a reservation
    records nothing unless it fits; measured growth never enters `held`; the walk never returns a
    level finer than the GPU limit's choice; the default budget is 1,536 MiB (the tests in
    `t938_6/` hold each);
  - an added asset extends `Asset::ALL`, `Asset::index`, `Asset::name` and the ledger's seven-row
    entry array together (`every_asset_indexes_its_own_row`);
  - the wasm32 and native builds define `configured_budget_bytes`, `heap_bytes` and `publish` side
    by side, so the ledger's tests run natively, as `cargo xtask mk wasm-ci` runs them.
