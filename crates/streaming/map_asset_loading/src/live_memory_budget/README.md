# Memory budget

The live memory budget of one page: the thread-local ledger of the bytes the map's world assets
hold in WebAssembly memory, the accounting calls the map host and the satellite loader make on it,
the page's budget settings and heap size, and the snapshot and HUD tail it reports to the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s debug HUD and to the page.
The ledger itself, its rows and the satellite floor walk are the pure model in
`crates/streaming/map_streaming_model/src/memory_budget/`.

## Contents

```text
crates/streaming/map_asset_loading/src/live_memory_budget/
├── accounting.rs          the live ledger's calls: hold, set, release, heap marks, satellite floor claim
├── mod.rs                 the module tree; re-exports the budget's calls, holds the thread-local ledger
├── platform.rs            the configured budget (`?memBudgetMb`, `window.__memBudgetMb`) and the heap size
└── published_snapshot.rs  the live HUD tail and the `window.__t9386` snapshot
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

The live ledger is a thread-local `map_streaming_model::memory_budget::Ledger` built on first use
from `configured_budget_bytes()`, which reads the `memBudgetMb` query parameter and the
`window.__memBudgetMb` global and resolves them with `budget_bytes_from_settings` (1,536 MiB
without a positive setting); a native build always takes the default and measures no heap.
`heap_mark` and `observe_since` measure the linear memory an asset claimed while it loaded and
record it as that asset's growth; `claim_satellite_floor` walks the satellite mip ladder against
the live ledger and reserves the chosen level's cost. Every declared change and floor claim
republishes `window.__t9386`.

## Public surface

- `hold`, `set_held`, `release`, `heap_mark`, `observe_since` and `publish`: the accounting calls
  of the map host's bootstrap and terrain load.
- `claim_satellite_floor` and `with_ledger`: the satellite basemap loader's floor claim and its
  budget warnings.
- `hud_suffix`: the debug HUD tail the Mission Creator's frame pump appends.
- `configured_budget_bytes` and `heap_bytes`: the page readers.

## Boundaries

- Depends on: `map_streaming_model::memory_budget` (the ledger, the rows, the floor walk, the
  settings resolution); `web-sys`, `js-sys` and `wasm-bindgen` on wasm32 for the page's query
  string, the window globals and the linear memory size.
- Used by:
  - the map host of `map_streaming_host`, whose bootstrap and terrain load declare the DEM and
    hillshade bytes and measure the world, forest, water and label growth;
  - `crate::terrain::satellite_quadtree`, which claims the floor at boot and names the budget's
    share of a downscale in its warning;
  - the Mission Creator's frame pump, which shows the HUD tail
    (`crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/viewport.rs`).
- Rules:
  - the wasm32 and native builds define `configured_budget_bytes`, `heap_bytes` and `publish` side
    by side, so the model's tests and this folder's test run natively
    (`a_native_build_takes_the_default_budget_and_measures_no_linear_memory`).
