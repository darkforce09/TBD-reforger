# Map streaming

Everything that gets a terrain's served map data into the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map and keeps it there:
fetching the files under `/map-assets`, deciding which world chunks stay resident, composing their
draw buffers, accounting for the memory they hold, and reporting progress and statistics to the
page. It hands finished data to the render engine and depends on no UI crate.

## Contents

```text
legacy/map_engine/src/streaming/
├── bridge/     what crosses to the page: preference readers, boot progress, statistics
├── host/       the browser entry point: the boot sequence, settle refreshes and map queries
├── loaders/    the world and occluder loaders
├── memory/     the memory budget ledger
└── mod.rs      the module tree
```

## How it works

```text
Mission Creator
   │ bootstrap, settles, queries; preference readers and a progress callback
   ▼
host/        boot sequence, settle passes, camera, place and occluder queries
   │ also drives the DEM, satellite, water, forest and label loaders of crate::world
   ▼
loaders/     fetch /map-assets/<terrain>/ and parse it with the world format crates
   │ missing chunk ids out, parsed chunks in
   ▼
chunk_scheduler      ChunkResidency: pin, in-flight marks, failure cap, LRU eviction, ingest budget
   │ rebuild requests (DrawRebuild)
   ▼
chunk_draw_buffers   WorldResidency: draw set; icon, strip and building buffers ─> render engine uploads

bridge/      the preference and progress types; statistics back to the page (window.__mapAssets)
memory/      the budget ledger the loads report to
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

`WorldResidency`, defined in the `chunk_draw_buffers` crate (`crates/streaming/chunk_draw_buffers`),
owns the `chunk_scheduler` crate's `ChunkResidency` (pin, LRU, ingest, object index) and the draw
buffers composed over it, with the layer toggles and the statistics; the scheduler names nothing
of the draw buffers and returns a rebuild request from each change the buffers must see, which the
world residency applies at once; the loaders and the debug bench import both crates directly.
The module compiles with the `streaming` feature;
`host/`, the two browser loaders and `bridge/statistics.rs` need wasm32 with `render`. So the
budget also runs natively, as the crate's tests use it.

## Public surface

- `host`: `bootstrap`, the host and DEM grid handles, `RENDER_CTX`, the settle and viewport calls,
  the hillshade, grid, basemap and world-layer calls, and the camera, place-name, water and
  occluder queries, for the Mission Creator.
- `bridge`: the preference types (`HostPreferences`, `RenderPreferences`, `WorldLayerPrefs`) and
  the progress types (`BootEvent`, `BootSeg`, `ProgressFn`) for the Mission Creator; progress,
  the Range helpers and statistics for the loaders in `crate::world`.
- `loaders`: `WorldHost` and `OccluderHost`, for the host and the debug world line-of-sight
  bench.
- `memory::budget`: the accounting calls and the satellite floor claim for
  `crate::world::terrain::satellite`, and `hud_suffix` for the Mission Creator's debug HUD.

## Boundaries

- Depends on:
  - `crate::world` (terrain, environment, meshes and the DEM, satellite, water, forest and label
    loaders), `crate::overlay` (the lane preferences), `crate::frame` (the render engine handle);
    the streaming crates `chunk_scheduler` and `chunk_draw_buffers`; the world format crates
    `world_chunks`, `prefab_catalog` and `world_store`;
    `spatial_indexes` and `world_line_of_sight` (the world object index, the line-of-sight
    occluder, BVH sidecars); `map_draw_lanes`, `label_layout`, `road_network`, `vegetation`,
    `terrain_elevation`, `terrain_relief` and `water_bodies`; `map_coordinates` (the chunk math) and
    `browser_platform` (the fetch helpers and console macros); nothing of `crate::camera` or
    `crate::doll`, and no mission crate;
  - `serde`, `serde_json`, `flate2`, `bytemuck`, `thiserror` and `futures`, and on wasm32
    `gloo-net`, `web-sys`, `js-sys`, `wasm-bindgen` and `wasm-bindgen-futures`;
  - the [API](/documentation/glossary/a_to_f.md#api)'s `/map-assets` mount, which serves
    `assets/terrains/` and `assets/glyphs/` unless `MAP_ASSETS_DIR` or `GLYPH_ASSETS_DIR`
    names another folder (`apps/api/src/router.rs`).
- Used by:
  - the DEM, satellite, water, forest and label loaders of `crate::world`;
  - the Mission Creator in `apps/frontend/src/workspaces/editor/`, the shared map mount in
    `apps/frontend/src/foundation/map_view/` and the debug world line-of-sight bench in
    `apps/frontend/src/workspaces/debug/world_los/`;
  - the editor smoke tests in `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/`,
    which read `window.__mapAssets`.
- Rules:
  - the tree reaches the render engine only through `crate::frame` and names no
    `graphics_engine` module itself (rules 3a and 3b of `cargo xtask verify engine-layers`),
    and no mission crate may name it (a `crates/mission` crate depends on no map engine module,
    `cargo xtask verify crate-tiers`);
  - the tree imports no UI crate: editor state enters only through the preference readers and the
    progress callback the page supplies, and browser I/O compiles only for wasm32;
  - the crate's tests need every feature (`map_engine_tests_require_all_features` in
    `legacy/map_engine/src/tests/feature_gate_tripwire.rs`), and `cargo xtask mk wasm-ci`
    runs them with `--all-features`;
  - the Mission Creator's boot-progress tests read files of `host/` and `loaders/world_loader/` by
    path, as those folders' READMEs state.

## Related documentation

- [Terrain assets](/assets/terrains/README.md) — the served terrain tree the loaders read.
- [Architecture gates](/tools/checks/repository_checks/src/architecture/README.md) — the
  `engine-layers` gate that scans this tree.
- [Map streaming](/documentation/legacy/map_engine/map_streaming.md) — the boot sequence,
  viewport passes, residency, memory budget and loaders as one flow, with open work and decisions.
