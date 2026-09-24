# World asset loaders

The parsers and browser loaders for a terrain's served files: the manifest's object and binary
blocks, prefab catalogues, object chunks in gzip JSON or `TBDC` binary form, roads and regions,
the HTTP fetch helpers, and the two loaders that keep the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map fed: world objects and the
line-of-sight occluder. The parsers compile natively for the tools and tests; `fetch.rs`,
`occluder_loader.rs` and `world_loader/` compile only for wasm32 with the `render` feature.

## Contents

```text
apps/website/map-engine/src/streaming/loaders/
├── chunk.rs            `WorldChunk`, one chunk's instance columns, and the gzip JSON chunk parser
├── chunk_bin.rs        the `TBDC` binary chunk parser, its tile id check and the path template
├── fetch.rs            same-origin GETs: bytes, text, a streamed body with progress, Range requests
├── manifest.rs         the terrain manifest's `objects` block, binary blocks and chunk index cells
├── mod.rs              the module tree
├── occluder_loader.rs  `OccluderHost`: building descriptors and BVH sidecars for resident chunks
├── prefab.rs           prefab tables from gzip JSON or the rkyv catalogue, chosen by first bytes
├── residency.rs        the residency's manifest, prefab and chunk-index loads and its chunk ingest
├── store.rs            `WorldStore`, `WorldError` and the gzip-or-plain JSON reader
├── tests/              unit tests for the chunk, binary chunk, manifest, prefab and store parsers
└── world_loader/       `WorldHost`: fetch, ingest and upload of a terrain's world objects
```

## How it works

```text
manifest.json, chunks/manifest.json ──> manifest.rs: objects and binary blocks, index cells
prefabs.rkyv | prefabs.json.gz ──> prefab.rs ──> PrefabTables ──┐
chunks/<cx>_<cy>.bin ─────────> chunk_bin.rs ──┐                 ├─> residency.rs ─> WorldResidency
chunks/<cx>_<cy>.json.gz ─────> chunk.rs ──────┴─> WorldChunk ───┘
roads, regions (rkyv or gzip JSON) ──> store.rs: WorldStore
blas-manifest.json, descriptors, BVH sidecars ──> occluder_loader.rs ──> WorldOccluder
```

A `WorldChunk` holds one chunk's instances as columns (interleaved x and y, prefab id, yaw, z,
pitch, roll, scale, class code) plus the rows of each render class, and both chunk parsers build
the same columns. A `TBDC` chunk is a header and 32-byte `ObjectInstancePod` rows: a payload
whose length disagrees with the header's count is an error, a misaligned buffer is copied into
aligned words first, and `parse_chunk_bin_for` refuses a well-formed chunk whose header names
another tile. The manifest parser never fails on a binary block: a missing or malformed block is
absent, and `ObjectsBinaryBlock::matches_this_build` decides whether chunk binaries are read. The
`objects` block needs `prefabsPath` and `chunksPath`; `chunkSizeM` defaults to
`DEFAULT_CHUNK_SIZE_M` (512 m).

Prefab and road payloads are told apart by their first bytes: an empty buffer is
`WorldError::EmptyPayload`, `1f 8b` is gzip JSON, anything else goes to the validating rkyv
reader. A prefab catalogue records the terrain it was built for and is refused for another, and
one that repeats a prefab id is refused. `residency.rs` adds the loads to `WorldResidency`: the
manifest (and the terrain size from `worldBounds`), the prefab tables, the chunk index whose cells
bound every pin, and `ingest_chunk_gz` and `ingest_chunk_bin`, which answer `Applied`,
`ParsedEmpty` (known-empty from then on) or a shape mismatch that counts toward the fetch-failure
cap. `WorldStore` is the headless reader the tools use: manifest, prefab table, roads, regions and
one chunk at a time.

`fetch_bytes` and `fetch_text` answer `None` on a transport failure or a status outside 2xx;
`fetch_bytes_streamed` reports the `content-length` as its segment's byte budget and the body's
bytes as they arrive, every 512 KiB; `fetch_range_outcome` accepts only a 206 with a
`Content-Range` total, reports a 429 with its `Retry-After`, and refuses a 200, so a server that
ignores `Range` never sends a whole satellite bundle. `OccluderHost` reads the building blueprint
archive the manifest names and `/map-assets/<terrain>/prefabs/blas-manifest.json` with its hot
prefabs, then on each viewport mirrors the residency's inserted and evicted chunks into the
`WorldOccluder` and fetches what resident chunks still need, descriptors before sidecars: up to 96
wants a round, 12 requests at a time, 8 rounds, and a file that fails 3 times is given up for the
session.

## Public surface

- `chunk::{WorldChunk, parse_chunk}` and `chunk_bin::{parse_chunk_bin, parse_chunk_bin_for,
  chunk_bin_path, ChunkBinError}`: the chunk formats, for `crate::spatial::los::world`,
  `crate::world::environment::vegetation::canopy` and the developer tools' map verification.
- `manifest`: `parse_manifest_binary` with its blocks, `parse_objects_manifest`, `narrow_cells`
  and `DEFAULT_CHUNK_SIZE_M`, for the DEM, label and water loaders under `crate::world`, the
  streaming host and the developer tools.
- `store::{WorldStore, WorldError, bytes_to_json}`: for the developer tools' export, raster and
  verification pipelines and the tests under `crate::world`.
- `fetch`: `fetch_bytes`, `fetch_text`, `fetch_bytes_streamed` and `fetch_range_outcome` with
  `RangeBody` and `RangeOutcome`, for the streaming host, the satellite, water, label and
  vegetation loaders, and the debug world line-of-sight bench.
- `occluder_loader::OccluderHost` for the streaming host and the debug bench, and
  `world_loader::WorldHost` for the streaming host.
- The `WorldResidency` loads and ingests of `residency.rs`, for the world loader and the debug
  bench.

## Boundaries

- Depends on: `crate::streaming::scheduler` (`WorldResidency`, `IngestOutcome`, `ResidencyEvent`,
  `TerrainSizeM`) and `crate::streaming::bridge` (progress, statistics, preferences); `crate::io`
  (the `TBDC` container header, `ObjectInstancePod`, `BinaryError`); `crate::world` (prefab rows,
  catalogues and class codes, footprint lookups, road and region payloads, the airfield box, the
  road and landcover meshes); `crate::spatial::los::world` and `crate::spatial::bvh::sidecar` for
  the occluder; `crate::frame::EngineHandle` and `crate::overlay::lanes` for the uploads;
  `crate::diagnostics::platform::console`; `serde`, `serde_json`, `flate2`, `bytemuck`,
  `thiserror`, `futures`, `gloo-net` and the browser bindings; the files under
  `/map-assets/<terrain>/`, whose manifest follows
  `contracts_v2/definitions/terrain-manifest.schema.json`.
- Used by:
  - `crate::streaming::host`; `crate::streaming::scheduler` (`WorldChunk`, `ObjectsManifest`,
    `DEFAULT_CHUNK_SIZE_M`); `crate::spatial::los::world`; and the DEM, water, label, vegetation
    and satellite loaders and tests under `crate::world`;
  - the debug world line-of-sight bench in `apps/website/frontend/src/v2/apps/debug/world_los/`;
  - the developer tools in `tools_v2/developer-tools/src/`: the world export pipeline, the map
    raster pipeline and the map verifications.
- Rules:
  - the binary and JSON lanes build the same data: chunk columns and residencies
    (`everon_chunk_bin_columns_equal_the_gz_decode`, `ingest_chunk_bin_matches_ingest_chunk_gz`),
    prefab residencies (`everon_archive_lane_builds_the_same_residency_as_the_json_lane`) and roads
    (`load_roads_sniffs_gzip_versus_rkyv`);
  - a truncated, mis-served or empty payload is an error, never a short chunk or the other
    parser's input (`truncated_payload_is_err_not_a_short_chunk`,
    `id_mismatch_is_rejected_even_though_the_bytes_are_perfect`,
    `truncated_payloads_error_rather_than_reaching_the_wrong_parser`,
    `an_empty_payload_is_refused_before_the_sniff`);
  - a catalogue for another terrain or with a repeated prefab id is refused
    (`a_catalogue_for_another_terrain_is_refused_by_the_sniff`,
    `a_catalogue_with_a_duplicate_prefab_id_is_refused`);
  - the tests read the committed Everon export in `assets_v2/terrains/everon/` and the goldens in
    `contracts_v2/fixtures/map/`, and pin its census (`full_island_census_matches_pinned_inventory`,
    `everon_manifest_parses_unchanged`).
