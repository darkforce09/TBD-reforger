# World store

The `world_store` crate: one terrain's served world data read into memory without a browser. The
terrain manifest's `objects` block, the prefab table, the road network, the land-cover regions
and the object chunks, one chunk at a time, each loaded from its gzip JSON or binary payload. The
developer tools read terrains through it, and the map engine's browser world loader keeps one for
the roads and regions it draws.

## Contents

```text
crates/world_formats/world_store/
├── Cargo.toml  the package: `world_chunks`, `prefab_catalog`, `road_network`, `vegetation`, tier 5
└── src/        `WorldStore`, its error and its tests
```

## How it works

```text
manifest.json ─ load_manifest_json ─► world_chunks::terrain_manifest ─► manifest
prefabs.json.gz ─ load_prefabs_gz ─► prefab_catalog::prefab_rows ─► prefab_by_id, has_oversized
objects/chunks/<id>.json.gz ─ parse_chunk_gz ─► world_chunks::world_chunk ─► last_chunk
roads (rkyv or gzip JSON) ─ load_roads ─► road_network::network ─► roads ─► airfield_bbox
forest-regions.json.gz ─ load_forest_regions_gz ─► vegetation::regions ─► regions
```

Payloads go through `prefab_catalog::world_payload::bytes_to_json`, which inflates a payload that
starts with the gzip magic and parses the JSON. `load_roads` tells the two road forms apart by
their first bytes: an empty buffer is refused, `1f 8b` is gzip JSON, anything else goes to the
validating archive reader, and a refused load leaves the roads as they were. Each load replaces
what the store held and returns the count it kept.

## Getting started

Run from the repository root:

```bash
cargo test -p world_store   # payload loads, the road sniff, and the Everon census
```

## Public surface

- `store::WorldStore`: `load_manifest_json`, `load_prefabs_gz`, `parse_chunk_gz`, `load_roads`,
  `load_roads_gz`, `load_forest_regions_gz`, `instance_count_total`, `runway_segments` and
  `airfield_bbox`, and the fields they fill.
- `Error` and `Result` (`error`), and `prelude`, which re-exports `WorldStore`, `Error` and
  `Result`.

## Boundaries

- Depends on: `world_chunks` (chunk rows, the terrain manifest, `ChunkId`); `prefab_catalog`
  (prefab rows, payload decoding, `WorldError`); `road_network` (segments, the archive reader,
  the airfield box); `vegetation` (land-cover regions); `world_file_formats` (`BinaryError`);
  `map_coordinates` (`Bbox`); `serde_json` and `thiserror`.
- Used by:
  - the map engine's browser world loader (`crates/streaming/map_asset_loading/src/world_loader/`);
  - the developer tools: the world export, map raster and map verification pipelines in
    `tools/developer_tools/src/`.
- Rules:
  - the binary and JSON lanes build the same roads (`load_roads_sniffs_gzip_versus_rkyv`);
  - a truncated payload is an error, never the other parser's input
    (`truncated_payloads_error_rather_than_reaching_the_wrong_parser`);
  - the tests read the committed Everon export in `assets/terrains/everon/` and pin its census
    (`full_island_census_matches_pinned_inventory`).
