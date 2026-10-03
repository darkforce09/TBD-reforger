# World chunk residency scheduler

Decides which world chunks, 512 m squares unless the manifest sets another size, the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map keeps in memory: the
chunk math that turns a viewport into chunk ids, `WorldResidency` with its pin, in-flight marks,
fetch-failure cap and LRU eviction, the loads and chunk ingest that feed it, the per-frame ingest
budget, and the object index, picking and lookups over the resident chunks.

## Contents

```text
legacy/map_engine/src/streaming/scheduler/
├── budget.rs              the per-frame ingest budget (`APPLY_BUDGET_MS`), frame accounting
├── chunk_ingest.rs        manifest, prefab and chunk-index loads, and chunk ingest
├── mod.rs                 the module tree
├── queries.rs             picks and lookups over resident chunks: nearest, rect, chunk
├── residency/             one path for the residency's types and constants, and its tests
├── state.rs               `WorldResidency`, `IngestOutcome` and `ResidencyEvent`
├── tests/                 unit tests: object index, chunk ingest
├── viewport.rs            pin, in-flight marks, failure cap, chunk insert, LRU eviction
└── world_object_index.rs  `WorldSpatialIndex`: class-filtered picks over resident objects
```

## How it works

```text
set_viewport(min_x, min_y, max_x, max_y, zoom)
  below the building gate and every importance zoom ─> unpin all, rebuild, request nothing
  ids = chunk_ids_for_viewport(viewport + preload margin, +1 ring if oversized) ∩ chunk index
  ids equal the pin ─> rebuild or refresh buffers; ask again for missing pinned chunks only
                       while nothing is in flight and the pin has not settled
  new ids ─> pin, clear failure counts, re-touch resident members, mark the missing in flight,
             evict, rebuild buffers ─> return the missing ids
loader ─> ingest_chunk_bin or ingest_chunk_gz ─> insert_chunk
            (spatial index, building count, Inserted event, LRU tick, content epoch)
fetch or parse failure ─> note_fetch_failure: retried later, an empty stub at the cap
end_ingest_frame_at ─> apply-frame statistics ─> evict ─> rebuild buffers
```

`chunk_ids_for_viewport` grows the viewport by its preload margin (5 % of its longer side, at
least one chunk), clamps the chunk rectangle to the terrain, adds one ring of chunks when a prefab
is oversized, and lists `<cx>_<cy>` ids row by row, `cy` outer and `cx` inner: the fetch, dedupe
and pin order. Once a chunk index is loaded, the residency keeps only the ids it lists, and the
joined ids form the pin key that tells an unchanged pin from a new one.

Eviction starts when the resident count passes `LRU_MIN_CHUNKS` (64) or three times the pinned
count, whichever is larger; it takes unpinned chunks that are not known-empty, least recently
used first and oldest insert on a tie, and each eviction leaves the spatial index, queues
`ResidencyEvent::Evicted`, joins `eviction_log` and bumps the content epoch. A failed fetch or a
malformed body releases the in-flight mark until `FETCH_FAILURE_CAP` (3) failures, then stores an
empty stub, so a chunk missing on the server is not asked for again while its stub stays
resident; a new pin clears the counts. The ingest budget is `APPLY_BUDGET_MS` (4 ms) a frame:
`end_apply_frame` records the last and longest apply time and the frames over budget, then evicts
and rebuilds once, and `ingest_budget_exhausted_at` answers whether an open frame has spent it.
Residency events for inserted and evicted chunks queue until the occluder loader takes them.

`chunk_ingest.rs` feeds the residency: `load_manifest_json` (the `objects` block and the terrain
size from `worldBounds`), `load_prefabs` and `load_prefabs_gz` (the prefab tables, the archive
refusing a catalogue built for another terrain), `load_chunk_index_json` (the cells that bound
every pin), and `ingest_chunk_gz` and `ingest_chunk_bin`, which answer `Applied`, `ParsedEmpty`
(known-empty from then on) or a shape mismatch that counts toward the fetch-failure cap.

`WorldSpatialIndex` (`world_object_index.rs`) keeps the resident objects per chunk:
`insert_chunk` replaces a chunk, drops rows whose class is `NO_CLASS` and names each kept row
`"{chunk_id}:{row}"`; the grid, of `INDEX_CELL_M` (256 m) cells, is rebuilt on the first query
after a change, with the chunks in id order. Each pick takes an optional class mask, bit `n` of a
`u32` admitting class `n`.

The residency's other methods live beside the code they serve: its buffer composers, the building
fill among them, in `crate::streaming::buffers`, its toggles in `crate::streaming::bridge` and its
statistics in `crate::streaming::memory`.

## Public surface

- `state::WorldResidency` with `IngestOutcome` and `ResidencyEvent`: the residency the world
  loader drives and the occluder loader mirrors, which the debug world line-of-sight bench builds
  on its own.
- The residency's `set_viewport`, in-flight and failure calls (`mark_inflight`, `clear_inflight`,
  `release_inflight`, `note_fetch_failure`, `note_undelivered`, `invalidate_chunk`,
  `pin_settled`, `inflight_count`), its ingest-frame calls and its queries (`pick_nearest`,
  `pick_rect`, `chunk`, `terrain`, `chunk_size_m`, `prefab_rows`).
- The loads and ingests of `chunk_ingest.rs`, for the world loader and the debug bench.
- The constants `LRU_MIN_CHUNKS`, `FETCH_FAILURE_CAP`, `DRAW_CULL_MARGIN_M` and `APPLY_BUDGET_MS`.

## Boundaries

- Depends on: `world_chunks` (`WorldChunk`, `ObjectsManifest`, `DEFAULT_CHUNK_SIZE_M`, the
  chunk and manifest parsers) and `prefab_catalog` (the prefab tables and rows, the payload
  reader, footprint lookups, class codes); `crate::streaming::buffers` (the rebuilds and
  `deinterleave`); `spatial_indexes::point_indexes` (`PointIndex`) for the object index;
  `map_draw_lanes::zoom_gates` (`building_visible`); `label_layout::glyph_math`;
  `vegetation::canopy`;
  `map_coordinates::chunk_math` (chunk ids, rectangles and terrain sizes).
- Used by:
  - the rest of `crate::streaming`: the loaders, the buffer composers, the bridge's toggles and
    the memory statistics;
  - the debug world line-of-sight bench in `apps/frontend/src/workspaces/debug/world_los/`.
- Rules:
  - the chunk math it relies on clamps to the terrain, keeps the preload margin, lists ids row-major and adds
    the oversized ring (`chunk_rect_pinned_cases`, `preload_margin_pinned_cases`,
    `viewport_ids_length_and_order`, `ids_for_rect_row_major` and `oversized_ring_expands_rect` in
    `crates/geometry/map_coordinates/src/tests/chunk_math.rs`);
  - pinned and known-empty chunks are never evicted, a chunk at the failure cap becomes an empty
    stub, and a new pin resets the failure counts; the tests in `residency/` hold the lifecycle;
  - re-inserting a chunk replaces it and `NO_CLASS` rows are never indexed
    (`remove_and_reinsert_are_idempotent`, `no_class_rows_are_skipped` in
    `tests/world_object_index_tests.rs`);
  - the binary and JSON lanes build the same residency
    (`everon_archive_lane_builds_the_same_residency_as_the_json_lane` in
    `tests/chunk_ingest_prefab_lane_tests.rs`, `ingest_chunk_bin_matches_ingest_chunk_gz` in
    `tests/chunk_ingest_chunk_bin_tests.rs`).
