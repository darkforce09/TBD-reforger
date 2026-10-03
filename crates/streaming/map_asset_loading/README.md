# Map asset loading

The `map_asset_loading` crate: the browser loaders that get a terrain's served files under
`/map-assets` into the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map. It holds the world chunk loader and the line-of-sight occluder loader, the elevation, relief,
satellite and water loaders, the forest mass and label loaders, the CPU mesh composition they
share, the live memory budget the loads report to and the asset statistics published at
`window.__mapAssets`. Every loader writes the renderer only through the asset sink of
[`map_streaming_model`](/crates/streaming/map_streaming_model/README.md), held as a
`BrowserAssetSinkHandle`.

## Contents

```text
crates/streaming/map_asset_loading/
├── Cargo.toml  the package: the streaming, terrain, world-format and world-object crates; streaming category, tier 6, wasm32
└── src/        the loaders, the mesh composition, the live memory budget and the asset statistics
```

## How it works

```text
map_streaming_host ── owns and drives ──▶ world_loader::WorldHost ──▶ chunk_draw_buffers::WorldResidency
                                        occluder_loader::OccluderHost ──▶ world_line_of_sight::WorldOccluder
                                        terrain::{elevation, relief, satellite_quadtree, water}
                                        environment::{forest_mass_loader, location_labels}
every loader ── BrowserAssetSinkHandle ──▶ dyn MapAssetSink (the renderer's impl)
             ── live_memory_budget ──▶ window.__t9386     asset_statistics ──▶ window.__mapAssets
```

The loaders, their fetch rules and the files they read are described in
[`src/README.md`](/crates/streaming/map_asset_loading/src/README.md) and each module folder's
README. Everything that fetches or names a browser type compiles only for wasm32; the mesh
composition and the live budget compile natively too, and their tests run on the host.

## Getting started

Run from the repository root:

```bash
cargo test -p map_asset_loading   # the mesh composition and the native budget readers
```

## Configuration

No feature. The live budget reads `?memBudgetMb` and `window.__memBudgetMb` on the page.

## Public surface

- `world_loader::WorldHost` and `occluder_loader::OccluderHost`, for the map host and the debug
  world line-of-sight bench.
- `terrain::{elevation, relief, satellite_quadtree, water}` and
  `environment::{forest_mass_loader, location_labels}`: the terrain and environment loaders, for
  the map host.
- `browser_asset_sink::{BrowserAssetSink, BrowserAssetSinkHandle}`, for the map host, the loaders
  and the frontend's render context registration.
- `mesh_composition`: the land cover, contour and hairline meshes.
- `live_memory_budget`: the accounting calls, the satellite floor claim and `hud_suffix`.
- `asset_statistics`: `MapAssetsBridge`, `BridgeHandle`, `new_bridge`, `publish`,
  `publish_engine`.
- The common names in `prelude`.

## Boundaries

- Depends on: `map_streaming_model`, `chunk_scheduler`, `chunk_draw_buffers`; the terrain crates
  (`terrain_elevation`, `terrain_relief`, `satellite_imagery`, `water_bodies`); the world format
  crates (`world_chunks`, `world_file_formats`, `world_store`); `road_network`, `vegetation`,
  `place_names`, `map_draw_lanes`, `label_layout`, `world_line_of_sight`, `spatial_indexes`,
  `map_coordinates`, `render_primitives`, `browser_platform`; `serde_json`, `futures` and the
  browser bindings; the files under `/map-assets/<terrain>/`.
- Used by: `map_streaming_host`; the single-page app (`apps/frontend`, WebAssembly build only),
  which imports the loaders and `live_memory_budget` directly.
- Rules:
  - streaming category, tier 6, wasm32: no GPU crate and no rendering crate
    (`cargo xtask verify crate-tiers`);
  - the Mission Creator's boot-progress, satellite and scale tests and the map asset
    verification's occluder test read the loaders' sources by path.

## Related documentation

- [Map streaming](/documentation/crates/streaming/map_streaming.md) — the boot sequence,
  viewport passes, residency, memory budget and loaders as one flow.
