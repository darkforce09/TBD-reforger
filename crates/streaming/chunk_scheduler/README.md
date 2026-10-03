# Chunk scheduler

The `chunk_scheduler` crate: which world chunks the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map keeps resident. It turns
a viewport into the pinned chunk set and the chunks to fetch, ingests chunks from gzip JSON or
`TBDC` binary, evicts the least recently used ones, indexes the resident world objects for picking,
and hands every change the draw buffers must see back as a rebuild request.

## Contents

```text
crates/streaming/chunk_scheduler/
├── Cargo.toml  the package: the world format, geometry and overlay dependencies, the dev-only `test_fixtures` feature, tier 4
└── src/        the chunk residency, its ingest, budget, queries, object index and rebuild requests
```

## How it works

```text
set_viewport ─> ViewportUpdate { missing: Vec<ChunkId>, rebuild: DrawRebuild }
ingest_chunk_gz / ingest_chunk_bin ─> IngestOutcome (Applied, ParsedEmpty, ShapeMismatch)
end_ingest_frame_at, end_apply_frame ─> evict once ─> DrawRebuild::AllBuffers
load_prefabs, load_prefabs_gz ─> new prefab tables keyed by PrefabId ─> DrawRebuild::GlyphLookup
```

`ChunkResidency` holds the terrain manifest, the prefab tables, the resident chunks with their
LRU clocks, the pin, the in-flight marks, the known-empty and fetch-failure sets, the object index
and the ingest-frame statistics; every field is private to the crate. The draw buffers read it
through the accessors of `draw_inputs.rs` and the queries of `queries.rs`. A chunk is named by
`world_chunks::ChunkId` wherever it crosses the crate's public surface; a chunk's numeric `pid`
joins the prefab tables through `prefab_catalog::prefab_rows::prefab_id_from_f64`, so a fractional
or negative `pid` joins no prefab. The folder README in `src/` details the pin, eviction and
ingest rules.

## Getting started

Run from the repository root:

```bash
cargo test -p chunk_scheduler   # object index, binary and JSON chunk lanes over the Everon export
```

The chunk lane tests read `assets/terrains/everon/manifest.json`, the Everon prefab catalogue and
`assets/terrains/everon/objects/chunks/*.json.gz`.

## Configuration

One feature, `test_fixtures`, off by default: it compiles the test-only writers of `test_hooks.rs`
(viewport, zoom, resident chunk rows, content epoch) for the draw buffer tests of
`chunk_draw_buffers` and is enabled only from their `[dev-dependencies]`. The crate reads no
environment variable.

## Public surface

- `state`: `ChunkResidency`, `IngestOutcome`, `ResidencyEvent`; `draw_rebuild`: `DrawRebuild`,
  `ViewportUpdate`.
- The residency's methods in `viewport`, `budget`, `chunk_ingest`, `queries` and `draw_inputs`;
  `world_object_index::WorldSpatialIndex`.
- The constants `LRU_MIN_CHUNKS`, `FETCH_FAILURE_CAP`, `DRAW_CULL_MARGIN_M` (`viewport`) and
  `APPLY_BUDGET_MS` (`budget`).
- `Error` and `Result` at the crate root; the common names in `prelude`.

## Boundaries

- Depends on: `world_chunks` (chunks, chunk ids, the manifest and chunk parsers),
  `prefab_catalog` (the prefab tables, rows, footprint lookups, class codes, payload decoding),
  `world_file_formats` (`PrefabId`), `spatial_indexes` (the point index), `map_draw_lanes`
  (`building_visible`), `map_coordinates` (the chunk math), `serde_json`, `thiserror`.
- Used by: `chunk_draw_buffers`, whose `WorldResidency` owns a `ChunkResidency`; the map engine
  (`legacy/map_engine`, behind `streaming`), directly, for the occluder loader's `ResidencyEvent`.
- Rules:
  - streaming category, tier 4 (`cargo xtask verify crate-tiers`);
  - nothing here names the draw buffers; every change they must see leaves as a `DrawRebuild`;
  - the binary and JSON chunk lanes build the same residency
    (`ingest_chunk_bin_matches_ingest_chunk_gz`);
  - re-inserting a chunk replaces it and `NO_CLASS` rows are never indexed
    (`remove_and_reinsert_are_idempotent`, `no_class_rows_are_skipped`).
