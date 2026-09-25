# Map streaming

Everything that gets a terrain's served map data into the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map and keeps it there:
fetching the files under `/map-assets`, deciding which world chunks stay resident, composing their
draw buffers, accounting for the memory they hold, and reporting progress and statistics to the
page. It hands finished data to the render engine and depends on no UI crate.

## Contents

```text
apps/website/map-engine/src/streaming/
├── bridge/     what crosses to the page: preference readers, boot progress, statistics, toggles
├── buffers/    the draw set and the packed icon, strip and building buffers of resident chunks
├── host/       the browser entry point: the boot sequence, settle refreshes and map queries
├── loaders/    the served-file parsers, the fetch helpers, and the world and occluder loaders
├── memory/     the memory budget ledger and the residency's statistics
├── mod.rs      the module tree
└── scheduler/  the world chunk residency: chunk math, pin, eviction, ingest budget, picking
```

## How it works

```text
Mission Creator
   │ bootstrap, settles, queries; preference readers and a progress callback
   ▼
host/        boot sequence, settle passes, camera, place and occluder queries
   │ also drives the DEM, satellite, water, forest and label loaders of crate::world
   ▼
loaders/     fetch /map-assets/<terrain>/ and parse it; the world and occluder loaders
   │ missing chunk ids out, parsed chunks in
   ▼
scheduler/   WorldResidency: pin, in-flight marks, failure cap, LRU eviction, ingest budget
   │
   ▼
buffers/     draw set; icon, strip and building buffers ─> render engine uploads (crate::frame)

bridge/      the preference and progress types; statistics back to the page (window.__mapAssets)
memory/      the budget ledger the loads report to; the residency's statistics
```

The page mounts a render engine, registers it with the host's render context and calls
`bootstrap` with its preference readers and a progress callback. The host fetches the terrain
manifest, loads the DEM with its hillshade and the satellite basemap at once (the satellite's
finest level chosen under the memory budget), then the world objects, forest, water and labels,
and runs viewport passes until nothing more arrives. Each camera settle runs passes again: the
world loader asks the residency which chunks the viewport pins, fetches the missing ones twelve
at a time as `TBDC` binaries or gzip JSON, ingests up to 24 a pass, and uploads the rebuilt
buffers when their revision changes, while the occluder loader mirrors the resident chunks into
the line-of-sight occluder.

`WorldResidency`, defined in `scheduler/`, is extended across the children: its loads and ingest
in `loaders/`, its buffer composers in `buffers/`, its layer toggles in `bridge/` and its
statistics in `memory/`. The module compiles with the `io` feature, where `bridge/` provides its
preference and progress types; `buffers/`, `loaders/`, `memory/` and `scheduler/` need
`streaming`; `host/`, the fetch helpers and the two browser loaders need wasm32 with `render`. So
the parsers, the residency, the buffers and the budget also run natively, as the developer tools
and the crate's tests use them.

## Public surface

- `host`: `bootstrap`, the host and DEM grid handles, `RENDER_CTX`, the settle and viewport calls,
  the hillshade, grid, basemap and world-layer calls, and the camera, place-name, water and
  occluder queries, for the Mission Creator.
- `bridge`: the preference types (`HostPreferences`, `RenderPreferences`, `WorldLayerPrefs`) and
  the progress types (`BootEvent`, `BootSeg`, `ProgressFn`) for the Mission Creator; progress,
  the Range helpers and statistics for the loaders in `crate::world`.
- `loaders`: the chunk and manifest parsers, `WorldStore`, `bytes_to_json`, the fetch helpers and
  `OccluderHost`, for `crate::world`, `crate::spatial::los::world`, the debug world line-of-sight
  bench and the developer tools.
- `scheduler`: `WorldResidency` and the chunk math, for `crate::spatial::los::world`,
  `crate::world`, the debug bench and the developer tools.
- `buffers::revision`: `BUILDING_MIN_ZOOM` and `norm`, for
  `crate::world::environment::buildings::footprint`.
- `memory::budget`: the accounting calls and the satellite floor claim for
  `crate::world::terrain::satellite`, and `hud_suffix` for the Mission Creator's debug HUD.

## Boundaries

- Depends on:
  - `crate::world` (terrain, environment, meshes and the DEM, satellite, water, forest and label
    loaders), `crate::io` (the `TBDC` container and its rows), `crate::spatial` (the world spatial
    index, the line-of-sight occluder, BVH sidecars), `crate::overlay` (level-of-detail gates,
    glyph math, lane ids), `crate::frame` (the render engine handle) and
    `crate::diagnostics::platform::console`; nothing of `crate::data`, `crate::editing`,
    `crate::camera` or `crate::doll`;
  - `serde`, `serde_json`, `flate2`, `bytemuck`, `thiserror` and `futures`, and on wasm32
    `gloo-net`, `web-sys`, `js-sys`, `wasm-bindgen` and `wasm-bindgen-futures`;
  - the [API](/documentation_v2/glossary.md#api)'s `/map-assets` mount, which serves
    `assets_v2/terrains/` and `assets_v2/glyphs/` unless `MAP_ASSETS_DIR` or `GLYPH_ASSETS_DIR`
    names another folder (`apps/website/api_v2/src/core/http_router.rs`).
- Used by:
  - `crate::world` and `crate::spatial::los::world`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/` and the debug world
    line-of-sight bench in `apps/website/frontend/src/v2/apps/debug/world_los/`;
  - the developer tools in `tools_v2/developer-tools/src/` (the world export pipeline, the map
    raster pipeline, the map verifications) and their editor smoke tests, which read
    `window.__mapAssets`.
- Rules:
  - the tree reaches the render engine only through `crate::frame` and names no
    `website_graphics_engine` module itself (rules 3a and 3b of `cargo xtask verify engine-layers`),
    and nothing under `crate::data` may name it (rules 4 and 7);
  - the tree imports no UI crate: editor state enters only through the preference readers and the
    progress callback the page supplies, and browser I/O compiles only for wasm32;
  - the crate's tests need every feature (`map_engine_tests_require_all_features` in
    `apps/website/map-engine/src/tests/feature_gate_tripwire.rs`), and `cargo xtask mk wasm-ci`
    runs them with `--all-features`;
  - the Mission Creator's boot-progress tests read files of `host/` and `loaders/world_loader/` by
    path, as those folders' READMEs state.

## Related documentation

- [Terrain assets](/assets_v2/terrains/README.md) — the served terrain tree the loaders read.
- [Architecture gates](/tools_v2/xtask/src/verifications/architecture/README.md) — the
  `engine-layers` gate that scans this tree.
- [Map streaming](/documentation_v2/website/map-engine/map_streaming.md) — the boot sequence,
  viewport passes, residency, memory budget and loaders as one flow, with open work and decisions.
