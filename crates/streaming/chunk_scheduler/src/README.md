# World chunk residency scheduler

Decides which world chunks, 512 m squares unless the manifest sets another size, the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map keeps in memory: the
chunk math that turns a viewport into chunk ids, `ChunkResidency` with its pin, in-flight marks,
fetch-failure cap and LRU eviction, the loads and chunk ingest that feed it, the per-frame ingest
budget, the object index, picking and lookups over the resident chunks, and the rebuild requests
it hands to the draw buffers composed over it.

## Contents

```text
crates/streaming/chunk_scheduler/src/
├── budget.rs              the per-frame ingest budget (`APPLY_BUDGET_MS`), frame accounting
├── chunk_ingest.rs        manifest, prefab and chunk-index loads, and chunk ingest
├── draw_inputs.rs         read accessors of the state the draw buffers and the statistics read
├── draw_rebuild.rs        `DrawRebuild` and `ViewportUpdate`: what a change leaves stale
├── error.rs               `Error` and `Result`: a refused payload or chunk container
├── lib.rs                 the crate root: the module tree
├── prelude.rs             the common names for `use chunk_scheduler::prelude::*;`
├── queries.rs             picks, lookups, counts, events and the strict draw set
├── state.rs               `ChunkResidency`, `IngestOutcome` and `ResidencyEvent`
├── test_hooks.rs          test-only writers of the viewport, zoom and resident chunks (`test_fixtures`)
├── tests/                 unit tests: object index, chunk ingest binary lane
├── viewport.rs            pin, in-flight marks, failure cap, chunk insert, LRU eviction
└── world_object_index.rs  `WorldSpatialIndex`: class-filtered picks over resident objects
```

## How it works

```text
set_viewport(min_x, min_y, max_x, max_y, zoom) ─> ViewportUpdate { missing, rebuild }
  below the building gate and every importance zoom ─> unpin all (AllBuffers) or
                                                       DrawSetAndGlyphs; request nothing
  ids = chunk_ids_for_viewport(viewport + preload margin, +1 ring if oversized) ∩ chunk index
  ids equal the pin ─> ZoomUnderUnchangedPin; ask again for missing pinned chunks only
                       while nothing is in flight and the pin has not settled
  new ids ─> pin, clear failure counts, re-touch resident members, mark the missing in flight,
             evict ─> AllBuffers, and the missing ids
loader ─> ingest_chunk_bin or ingest_chunk_gz ─> insert_chunk
            (spatial index, building count, Inserted event, LRU tick, content epoch)
fetch or parse failure ─> note_fetch_failure: retried later, an empty stub at the cap
end_ingest_frame_at ─> apply-frame statistics ─> evict ─> AllBuffers
load_prefabs, load_prefabs_gz ─> new prefab tables ─> GlyphLookup
```

The scheduler never calls the draw buffers. Each method that changes something they read returns
a `DrawRebuild` (`#[must_use]`): `Nothing`, `GlyphLookup`, `AllBuffers`, `DrawSetAndGlyphs`, or
`ZoomUnderUnchangedPin { previous_zoom, zoom }`, for which the buffers decide whether the zoom move
crosses a building fill band. The world residency in `chunk_draw_buffers::world_residency`
applies each request before the call that returned it returns, so the buffers it composes equal
what an inline rebuild at the same point produces. The draw buffers read the residency only
through the accessors of `draw_inputs.rs` and `queries.rs`; the residency's fields stay private to
this crate. A chunk crosses the public surface as a `world_chunks::ChunkId` (`set_viewport`'s
missing chunks, `ResidencyEvent`, every method that takes a chunk, and the views the draw buffers
and `vegetation` read: `pinned_ids`, `cell_ids`, `resident_chunks`, `resident_chunk_ids`,
`draw_chunk_ids` and `eviction_log`), and the internal maps key on it too; the chunk math's
`cx_cy` strings become `ChunkId`s where the pin and the draw set are computed. The prefab tables are keyed by `PrefabId`, and a chunk row's numeric
`pid` joins them through `prefab_catalog::prefab_rows::prefab_id_from_f64`.

`chunk_ids_for_viewport` grows the viewport by its preload margin (5 % of its longer side, at
least one chunk), clamps the chunk rectangle to the terrain, adds one ring of chunks when a prefab
is oversized, and lists `<cx>_<cy>` ids row by row, `cy` outer and `cx` inner: the fetch, dedupe
and pin order. Once a chunk index is loaded, the residency keeps only the ids it lists, and the
joined ids form the pin key that tells an unchanged pin from a new one. `draw_chunk_ids` gives the
strict draw set: the chunks under the viewport with no cull margin that the chunk index lists and
the pin holds, sorted.

Eviction starts when the resident count passes `LRU_MIN_CHUNKS` (64) or three times the pinned
count, whichever is larger; it takes unpinned chunks that are not known-empty, least recently
used first and oldest insert on a tie, and each eviction leaves the spatial index, queues
`ResidencyEvent::Evicted`, joins `eviction_log` and bumps the content epoch. A failed fetch or a
malformed body releases the in-flight mark until `FETCH_FAILURE_CAP` (3) failures, then stores an
empty stub, so a chunk missing on the server is not asked for again while its stub stays
resident; a new pin clears the counts. The ingest budget is `APPLY_BUDGET_MS` (4 ms) a frame:
`end_apply_frame` records the last and longest apply time and the frames over budget, then evicts
once and asks for one full rebuild, and `ingest_budget_exhausted_at` answers whether an open frame
has spent it. Residency events for inserted and evicted chunks queue until the occluder loader
takes them.

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

`test_hooks.rs` compiles only in test builds and behind the `test_fixtures` feature: it sets the
viewport and zoom and rewrites resident chunks for the draw buffer tests of `chunk_draw_buffers`,
bypassing the pin and the rebuild requests.

## Public surface

- `state::ChunkResidency` with `IngestOutcome` and `ResidencyEvent`, and `draw_rebuild::DrawRebuild`
  with `ViewportUpdate`: the residency the world residency of `chunk_draw_buffers` owns.
- The residency's `set_viewport`, in-flight and failure calls (`mark_inflight`, `clear_inflight`,
  `release_inflight`, `note_fetch_failure`, `note_undelivered`, `invalidate_chunk`,
  `pin_settled`, `inflight_count`), its ingest-frame calls, its queries (`pick_nearest`,
  `pick_rect`, `chunk`, `terrain`, `chunk_size_m`, `prefab_rows`, `draw_chunk_ids`,
  `take_residency_events`, `resident_chunk_ids`, `pinned_building_count`, `chunks_resident`,
  `resident_instance_count`) and the read accessors of `draw_inputs.rs`.
- The loads and ingests of `chunk_ingest.rs`.
- The constants `LRU_MIN_CHUNKS`, `FETCH_FAILURE_CAP`, `DRAW_CULL_MARGIN_M` and `APPLY_BUDGET_MS`.
- `Error` and `Result` at the crate root; the common names in `prelude`.

## Boundaries

- Depends on: `world_chunks` (`WorldChunk`, `ObjectsManifest`, `DEFAULT_CHUNK_SIZE_M`, the
  chunk and manifest parsers) and `prefab_catalog` (the prefab tables and rows, the payload
  reader, footprint lookups, class codes); `world_file_formats` (`PrefabId`);
  `spatial_indexes::point_indexes` (`PointIndex`) for
  the object index; `map_draw_lanes::zoom_gates` (`building_visible`); `map_coordinates::chunk_math`
  (chunk ids, rectangles and terrain sizes); `serde_json`; `thiserror`. Nothing of
  `chunk_draw_buffers` or the map engine.
- Used by:
  - `chunk_draw_buffers`, whose world residency owns a `ChunkResidency` and applies its
    rebuild requests, and the map engine's loaders through that world residency;
  - the map engine's occluder loader, for `ResidencyEvent`.
- Rules:
  - the chunk math it relies on clamps to the terrain, keeps the preload margin, lists ids row-major and adds
    the oversized ring (`chunk_rect_pinned_cases`, `preload_margin_pinned_cases`,
    `viewport_ids_length_and_order`, `ids_for_rect_row_major` and `oversized_ring_expands_rect` in
    `crates/geometry/map_coordinates/src/tests/chunk_math.rs`);
  - pinned and known-empty chunks are never evicted, a chunk at the failure cap becomes an empty
    stub, and a new pin resets the failure counts; the tests in
    `crates/streaming/chunk_draw_buffers/src/tests/residency_lifecycle/` hold the lifecycle;
  - re-inserting a chunk replaces it and `NO_CLASS` rows are never indexed
    (`remove_and_reinsert_are_idempotent`, `no_class_rows_are_skipped` in
    `tests/world_object_index_tests.rs`);
  - the binary and JSON chunk lanes build the same residency
    (`ingest_chunk_bin_matches_ingest_chunk_gz` in `tests/chunk_ingest_chunk_bin_tests.rs`), and
    the archive and JSON prefab lanes the same draw buffers
    (`everon_archive_lane_builds_the_same_residency_as_the_json_lane` in
    `crates/streaming/chunk_draw_buffers/src/tests/prefab_lane_parity.rs`).
