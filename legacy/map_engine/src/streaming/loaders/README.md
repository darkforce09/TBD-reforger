# World asset loaders

The browser loaders for a terrain's served files. The parsers and the store they feed are crates
every caller imports directly: [`world_chunks`](/crates/world_formats/world_chunks/README.md) (the
terrain manifest, object chunks in gzip JSON or `TBDC` binary form),
[`prefab_catalog`](/crates/world_formats/prefab_catalog/README.md) (prefab tables, payload
decoding) and [`world_store`](/crates/world_formats/world_store/README.md) (the headless world
store). The two loaders keep the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map fed: world objects and the
line-of-sight occluder. The loaders fetch through the `fetch` module of the
[`browser_platform`](/crates/foundation/browser_platform/README.md) crate. `occluder_loader.rs`
and `world_loader/` compile only for wasm32 with the `render` feature.

## Contents

```text
legacy/map_engine/src/streaming/loaders/
├── mod.rs              the module tree: `world_loader` and `occluder_loader`
├── occluder_loader.rs  `OccluderHost`: building descriptors and BVH sidecars for resident chunks
└── world_loader/       `WorldHost`: fetch, ingest and upload of a terrain's world objects
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
the body arrives, which the streaming host turns into its segment's byte budget and the bytes
done every 512 KiB; `fetch_range_outcome` accepts only a 206 with a `Content-Range` total,
reports a 429 with its `Retry-After`, and refuses a 200, so a server that ignores `Range` never
sends a whole satellite bundle. `OccluderHost` reads the building blueprint
archive the manifest names and `/map-assets/<terrain>/prefabs/blas-manifest.json` with its hot
prefabs, then on each viewport mirrors the residency's inserted and evicted chunks into the
`WorldOccluder` and fetches what resident chunks still need, descriptors before sidecars: up to 96
wants a round, 12 requests at a time, 8 rounds, and a file that fails 3 times is given up for the
session.

## Public surface

- `occluder_loader::OccluderHost` for the streaming host and the debug world line-of-sight
  bench, and `world_loader::WorldHost` for the streaming host.

## Boundaries

- Depends on: `crate::streaming::buffers` (`WorldResidency`) and `crate::streaming::scheduler`
  (`ResidencyEvent`) for the two browser loaders, and `crate::streaming::bridge` (progress, statistics, preferences);
  `world_chunks` and `world_store` (the manifest, the chunk paths and the store);
  `road_network` and `vegetation` (the road meshes and the forest regions); `crate::world::mesh`
  (the landcover mesh); `world_line_of_sight` and `spatial_indexes` (the occluder library and the
  BVH sidecar) for the occluder; `crate::frame::EngineHandle` and `map_draw_lanes` for the
  uploads;
  `browser_platform` for `fetch` and the console macros; `serde_json`, `futures` and the
  browser bindings; the files under
  `/map-assets/<terrain>/`, whose manifest follows
  `contracts/definitions/terrain-manifest.schema.json`.
- Used by: `crate::streaming::host`; the debug world line-of-sight bench in
  `apps/frontend/src/workspaces/debug/world_los/`.
- Rules: the chunk, prefab and road lanes and the Everon census are pinned in the crates; this
  folder holds no test of its own.
