# Streaming bridge to the page

What crosses between the streaming layer and the page that embeds it: the preference readers the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) supplies, the boot progress the
loaders report, the asset counters published at `window.__mapAssets`, and the world-layer
toggles the residency applies.

## Contents

```text
apps/website/map-engine/src/streaming/bridge/
├── host_preferences.rs  `HostPreferences`: the page's world-layer, basemap and render readers
├── mod.rs               the module tree
├── preferences.rs       `WorldLayerPrefs`: the twelve world-layer switches and their defaults
├── progress.rs          boot progress events and segments, the Range split and ordered reassembly
├── statistics.rs        `MapAssetsBridge`: asset and upload counters for `window.__mapAssets`
└── toggles.rs           the residency's world-layer toggles and the visibility they derive
```

## How it works

`preferences`, `host_preferences` and `progress` compile in every build of the streaming module;
`toggles` needs the `streaming` feature, and `statistics` wasm32 with `render`.

- **Preferences in.** `HostPreferences` holds three `fn` pointers (`world_layers`, `basemap`,
  `render`), so loaders read the current setting at each use, after an await included.
  `WorldLayerPrefs` holds twelve serialised switches (`townLabels` and `roadNames` in camelCase),
  all on by default except props.
- **Progress out.** Loaders report `BootEvent`s against four `BootSeg`ments
  ([mission](/documentation_v2/glossary.md#mission), terrain, satellite, world): `Budget` sets a
  segment's byte total from a `content-length` or the satellite index, `Files` declares a file
  count, `Done` counts units landed and `Finish` closes it. `STREAM_REPORT_BYTES` (512 KiB)
  batches a streamed body's reports; `split_range` cuts a tile into inclusive `Range` spans of
  `SAT_CHUNK_BYTES` (4 MiB), fetched `SAT_FETCH_CONCURRENCY` (4) at a time, and `Ordered` puts
  completions back in request order.
- **Statistics out.** `MapAssetsBridge` merges the render engine's `stats()` and the residency's
  `stats_json()` counters and installs them as `window.__mapAssets`, one bridge per map host.
- **Toggles.** `WorldResidency` takes the atlas key order, the trees, props, buildings, fences and
  airfield toggles and the airfield box from the runways; each setter returns early when nothing
  changed and rebuilds only the buffers its toggle feeds.

## Boundaries

- Depends on: `crate::streaming::scheduler`, `crate::overlay::lod`,
  `crate::world::environment::buildings::footprint` and `crate::world::terrain::roads::airfield`
  for the toggles; `crate::frame::engine::RenderEngine` for `publish_engine`; `serde`,
  `serde_json`, `wasm-bindgen`, `js-sys` and `web-sys`.
- Used by: `crate::streaming::host`, `crate::streaming::loaders` and the DEM, water, label,
  vegetation and satellite loaders of `crate::world`; the Mission Creator in
  `apps/website/frontend/src/v2/apps/editor/` (its world-asset bridge builds the
  `HostPreferences`, its boot machine and preference store re-export the progress types and
  `WorldLayerPrefs`, and its hydrate reports the mission segment); the editor smoke tests in
  `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`, which read
  `window.__mapAssets`.
- Rules:
  - `split_range` spans are inclusive, contiguous and cover the tile exactly, and `Ordered`
    refuses an out-of-range index or an unfilled one rather than shifting the run
    (`split_range_covers_the_tile_exactly_contiguously_and_in_order`,
    `a_dropped_completion_fails_instead_of_shifting_the_run` and
    `an_out_of_range_slot_is_refused_rather_than_dropped` in
    `apps/website/frontend/src/v2/apps/editor/tests/t628_boot_progress.rs`);
  - `strips_visible` follows the toggles and the zoom only, never the buffer contents, so an empty
    strip buffer mid-hydration uploads as visible instead of blanking the lane;
  - the `window.__mapAssets` keys are the ones the editor smoke tests assert on
    (`tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/fullmap.rs`).
