# Streaming memory accounting

The memory budget of what map streaming holds for the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map: a ledger of the world
assets' bytes that also chooses the satellite basemap's finest affordable level. The world
residency's counters are rendered as JSON beside the draw buffers they read, in
`crate::streaming::buffers`.

## Contents

```text
legacy/map_engine/src/streaming/memory/
├── budget/  the per-session memory budget: asset rows, peaks, measured growth, satellite floor
└── mod.rs   the module tree
```

## How it works

`budget/` keeps one thread-local ledger per page: seven asset rows (DEM, hillshade, satellite,
world, forest, labels, water) of declared, peak and measured bytes against a ceiling of 1,536 MiB
unless `?memBudgetMb` or `window.__memBudgetMb` sets another. The streaming host declares and
measures its loads there, the satellite loader claims its level floor there, each declared
change or floor claim republishes `window.__t9386`, and the debug HUD reads its tail once a
second.

The module needs the `streaming` feature; the budget's browser reads have native stand-ins, so
its tests run natively.

## Public surface

- `budget`: the accounting calls of the streaming host, the floor claim and budget figures of the
  satellite loader, and `hud_suffix` for the Mission Creator's debug HUD.

## Boundaries

- Depends on: nothing else of the crate; `web-sys`, `js-sys` and `wasm-bindgen` on wasm32.
- Used by:
  - `crate::streaming::host` and `crate::world::terrain::satellite::quadtree` (the budget);
  - the Mission Creator's frame pump, which shows the HUD tail
    (`apps/frontend/src/workspaces/editor/bridge/viewport.rs`).
- Rules:
  - the budget allocates and frees nothing: a load declares its bytes or measures its growth
    against its own asset row, and the satellite floor is the only choice it makes.
