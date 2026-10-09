# Map asset loading sources

The browser loaders for a terrain's served files, the mesh composition they share, the live
memory budget and the asset statistics. The parsers and the store they feed are crates
every caller imports directly: [`world_chunks`](/crates/world_formats/world_chunks/README.md) (the
terrain manifest, object chunks in gzip JSON or `TBDC` binary form),
[`prefab_catalog`](/crates/world_formats/prefab_catalog/README.md) (prefab tables, payload
decoding) and [`world_store`](/crates/world_formats/world_store/README.md) (the headless world
store). The world and occluder loaders keep the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map fed with world objects
and the line-of-sight occluder; the terrain and environment loaders with the ground, its imagery,
the forest mass and the labels. The loaders fetch through the `fetch` module of the
[`browser_platform`](/crates/foundation/browser_platform/README.md) crate, and reach the renderer
only through the asset sink of
[`map_streaming_model`](/crates/streaming/map_streaming_model/README.md), held as a
`BrowserAssetSinkHandle`. `occluder_loader.rs`, `world_loader/`, `browser_asset_sink.rs`,
`asset_statistics.rs` and every loader under `terrain/` and `environment/` compile only for
wasm32.

## Contents

```text
crates/streaming/map_asset_loading/src/
├── asset_statistics.rs    `MapAssetsBridge`: asset and upload counters published at `window.__mapAssets`
├── browser_asset_sink.rs  `BrowserAssetSink`, `BrowserAssetSinkHandle`: the asset sink with an `ImageBitmap` image
├── environment/           the forest mass loader and the label loader
├── lib.rs                 the crate root: the module tree
├── live_memory_budget/    the page's live memory budget: the thread-local ledger, its calls, page readers and snapshot
├── mesh_composition.rs    CPU meshes of the static world: land cover, contour and forest outline lines
├── occluder_loader.rs     `OccluderHost`: building descriptors and BVH sidecars for resident chunks
├── prelude.rs             the common names for `use map_asset_loading::prelude::*;`
├── terrain/               the elevation, relief, satellite and water loaders
├── tests/                 tests of the mesh composition
└── world_loader/          `WorldHost`: fetch, ingest and upload of a terrain's world objects
```

## How it works

```text
manifest.json, chunks/manifest.json ──> world_chunks::terrain_manifest: blocks, index cells
prefabs.rkyv | prefabs.json.gz ──> prefab_catalog::prefab_tables ──> PrefabTables ──┐
chunks/<cx>_<cy>.bin ─────────> world_chunks::chunk_container ──┐                   ├─> scheduler::chunk_ingest
chunks/<cx>_<cy>.json.gz ─────> world_chunks::world_chunk ──────┴─> WorldChunk ─────┘   ─> WorldResidency
roads, regions (rkyv or gzip JSON) ──> world_store::store::WorldStore
blas-manifest.json, descriptors, BVH sidecars ──> occluder_loader.rs ──> WorldOccluder
```

The chunk, manifest, prefab and payload parsers and the world store are described in the three
crates' READMEs. The chunk scheduler's `chunk_ingest.rs` feeds the parsers' output into the
`ChunkResidency` that `WorldResidency` owns; `WorldHost` keeps a `WorldStore` for the roads and
regions it draws.

`browser_platform::fetch`'s `fetch_bytes` and `fetch_text` answer `None` on a transport failure or a status outside 2xx;
`fetch_bytes_streamed` reports `ByteProgress` (the bytes received and the `content-length`) as
the body arrives, which the map host turns into its segment's byte budget and the bytes
done every 512 KiB; `fetch_range_outcome` accepts only a 206 with a `Content-Range` total,
reports a 429 with its `Retry-After`, and refuses a 200, so a server that ignores `Range` never
sends a whole satellite bundle. `OccluderHost` reads the building blueprint
archive the manifest names and `/map-assets/<terrain>/prefabs/blas-manifest.json` with its hot
prefabs, then on each viewport mirrors the residency's inserted and evicted chunks into the
`WorldOccluder` and fetches what resident chunks still need, descriptors before sidecars: up to 96
wants a round, 12 requests at a time, 8 rounds, and a file that fails 3 times is given up for the
session.

## Public surface

- `occluder_loader::OccluderHost` for the map host and the debug world line-of-sight bench, and
  `world_loader::WorldHost` for the map host.
- `terrain` and `environment`: the elevation, relief, satellite, water, forest mass and label
  loaders, for the map host.
- `browser_asset_sink::{BrowserAssetSink, BrowserAssetSinkHandle}` for the map host, the loaders
  and the frontend's render context registration.
- `mesh_composition`, `live_memory_budget` and `asset_statistics`, for the loaders, the map host
  and the Mission Creator's debug HUD.

## Boundaries

- Depends on: `chunk_draw_buffers` (`WorldResidency`) and `chunk_scheduler` (`ResidencyEvent`)
  for the two world loaders, `map_streaming_model` (the asset sink, the boot progress, the
  world-layer preferences, the budget model); `world_chunks` and `world_store` (the manifest, the
  chunk paths and the store); `road_network` and `vegetation` (the road meshes and the forest
  regions); `world_line_of_sight` and `spatial_indexes` (the occluder library and the BVH sidecar)
  for the occluder; the terrain crates and `place_names` for the terrain and label loaders;
  `map_draw_lanes` for the uploads; `browser_platform` for `fetch` and the console macros;
  `serde_json`, `futures` and the browser bindings; the files under `/map-assets/<terrain>/`,
  whose manifest follows `contracts/definitions/terrain-manifest.schema.json`.
- Used by: `map_streaming_host`; the debug world line-of-sight bench in
  `crates/frontend/workspaces/debug_benches/src/world_los/`.
- Rules: the chunk, prefab and road lanes and the Everon census are pinned in the crates; the
  mesh composition's tests are in `tests/`.
