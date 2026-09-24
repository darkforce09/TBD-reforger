# Streaming memory accounting

Two records of what map streaming holds for the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map: the memory budget, a
ledger of the world assets' bytes that also chooses the satellite basemap's finest affordable
level, and the world residency's counters as one JSON snapshot.

## Contents

```text
apps/website/map-engine/src/streaming/memory/
├── budget/       the per-session memory budget: asset rows, peaks, measured growth, satellite floor
├── mod.rs        the module tree
└── residency.rs  `WorldResidency::stats_json`: the residency's counters as one JSON object
```

## How it works

`budget/` keeps one thread-local ledger per page: seven asset rows (DEM, hillshade, satellite,
world, forest, labels, water) of declared, peak and measured bytes against a ceiling of 1,536 MiB
unless `?memBudgetMb` or `window.__memBudgetMb` sets another. The streaming host declares and
measures its loads there, the satellite loader claims its level floor there, each declared
change or floor claim republishes `window.__t9386`, and the debug HUD reads its tail once a
second.

`residency.rs` renders the residency's counters as one JSON object of 18 keys: resident, pinned,
applied and drawn chunks; apply frames with the last and longest apply time and the frames over
budget; pinned building instances; the spatial index size; the in-flight count and whether the
pin has settled; the exact tree count and the heatmap state; the buffers revision and the glyph
and fill recompose counts; and the known-empty chunks. The world loader merges the revision and
recompose counts into the page's asset statistics after each upload decision. Both modules need
the `streaming` feature; the budget's browser reads have native stand-ins, so its tests run
natively.

## Public surface

- `budget`: the accounting calls of the streaming host, the floor claim and budget figures of the
  satellite loader, and `hud_suffix` for the Mission Creator's debug HUD.
- `WorldResidency::stats_json`: for the world loader's statistics merge and the residency tests.

## Boundaries

- Depends on: `crate::streaming::scheduler::state::WorldResidency`, whose counters the scheduler
  and the buffer composers keep, for `stats_json`; for the budget, nothing else of the crate, and
  `web-sys`, `js-sys` and `wasm-bindgen` on wasm32.
- Used by:
  - `crate::streaming::host` and `crate::world::terrain::satellite::quadtree` (the budget);
  - `crate::streaming::loaders::world_loader` (`stats_json`), and the tests under
    `apps/website/map-engine/src/streaming/loaders/tests/` and
    `apps/website/map-engine/src/streaming/scheduler/residency/`;
  - the Mission Creator's frame pump, which shows the HUD tail
    (`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`).
- Rules:
  - `stats_json` is written by hand with `format!`, so it must stay valid JSON and keep its key
    names, which `merge_residency_stats` in `crate::streaming::bridge::statistics` and the tests
    read: `class_r_chunks_draw_matches_draw_ids_len` parses it and reads `chunks_draw`, and
    `parsed_empty_chunk_is_known_empty_and_not_refetched` reads `known_empty_count`;
  - the budget allocates and frees nothing: a load declares its bytes or measures its growth
    against its own asset row, and the satellite floor is the only choice it makes.
