# Map streaming model sources

The modules of the map streaming model: the world-layer switches, the host preferences and boot
scope, the boot progress and Range planning, the memory budget model and the asset sink contract
the map loaders write the renderer through.

## Contents

```text
crates/streaming/map_streaming_model/src/
├── asset_sink/                 `MapAssetSink`, `MapViewport`, the shared sink handle and the payloads
├── boot_progress.rs            `BootEvent`, `BootSeg`, `ProgressFn`, `split_range`, `Ordered`, the fetch sizes
├── error.rs                    `Error` and `Result`: a sink write the renderer refused
├── host_preferences.rs         `BootstrapScope`, `HostPreferences`, `RenderPreferences`
├── lib.rs                      the crate root: the module tree
├── memory_budget/              the ledger, the asset rows, the satellite floor walk, the HUD tail
├── prelude.rs                  the common names for `use map_streaming_model::prelude::*;`
├── tests/                      tests of the world-layer switches
└── world_layer_preferences.rs  `WorldLayerPrefs`: the twelve world-layer switches and their defaults
```

## How it works

- **Preferences in.** `HostPreferences` holds the boot's `BootstrapScope` (`Full`: every layer;
  `TerrainAndImagery`: manifest, DEM with its full-resolution raster kept, hillshade, satellite,
  basemap tiles and grid, and no world objects, forest, water or labels) and three `fn` pointers
  (`world_layers`, `basemap`, `render`), so the loaders read the current setting at each use, after
  an await included. `WorldLayerPrefs` holds twelve serialised switches (`townLabels` and
  `roadNames` in camelCase), all on by default except props.
- **Progress out.** The loaders report `BootEvent`s against four `BootSeg`ments
  ([mission](/documentation/glossary/g_to_m.md#mission), terrain, satellite, world): `Budget` sets a
  segment's byte total, `Files` declares a file count before the fetch, `Done` counts units landed
  and `Finish` closes it. `STREAM_REPORT_BYTES` (512 KiB) batches a streamed body's reports;
  `split_range` cuts a tile into inclusive `Range` spans of `SAT_CHUNK_BYTES` (4 MiB), fetched
  `SAT_FETCH_CONCURRENCY` (4) at a time, and `Ordered` puts completions back in request order.
- **Budget.** `memory_budget` is the pure ledger the map engine's live, page-wide ledger wraps.
- **Sink.** `asset_sink` is the only way the loaders reach the renderer.

## Boundaries

- Depends on: `render_primitives`, `serde`, `thiserror`.
- Used by: the map engine's streaming host and loaders, its render engine (the sink impl) and,
  through the map engine's re-exports, the frontend.
- Rules:
  - `split_range` spans are inclusive, contiguous and cover the tile exactly, and `Ordered`
    refuses an out-of-range index or an unfilled one rather than shifting the run (the cases in
    `crates/frontend/workspaces/mission_creator_workspace/src/tests/t628_boot_progress.rs`);
  - the stored world-layer keys are pinned by `the_label_switches_are_stored_in_camel_case`.
