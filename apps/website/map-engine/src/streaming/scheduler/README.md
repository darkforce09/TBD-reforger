# World chunk residency scheduler

Decides which world chunks, 512 m squares unless the manifest sets another size, the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map keeps in memory: the
chunk math that turns a viewport into chunk ids, `WorldResidency` with its pin, in-flight marks,
fetch-failure cap and LRU eviction, the per-frame ingest budget, and the picking and lookups over
the resident chunks.

## Contents

```text
apps/website/map-engine/src/streaming/scheduler/
├── budget.rs      the per-frame ingest budget (`APPLY_BUDGET_MS`) and the apply-frame accounting
├── chunk_math.rs  viewport to chunk ids: preload margin, clamped chunk rects, row-major id order
├── mod.rs         the module tree
├── queries.rs     picking and lookups over the resident chunks: nearest, rectangle, chunk, sizes
├── residency/     one path for the residency's types and constants, and the residency tests
├── state.rs       `WorldResidency`, `IngestOutcome` and `ResidencyEvent`
├── tests/         unit tests for the chunk math
└── viewport.rs    the pin, in-flight marks, the fetch-failure cap, chunk insert and LRU eviction
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

The residency's other methods live beside the code they serve: its loads and ingest in
`crate::streaming::loaders`, its buffer composers in `crate::streaming::buffers`, its toggles in
`crate::streaming::bridge`, its statistics in `crate::streaming::memory`, and the building fill
in `crate::world::environment::buildings::footprint`.

## Public surface

- `state::WorldResidency` with `IngestOutcome` and `ResidencyEvent`: the residency the world
  loader drives and the occluder loader mirrors, which the debug world line-of-sight bench builds
  on its own.
- The residency's `set_viewport`, in-flight and failure calls (`mark_inflight`, `clear_inflight`,
  `release_inflight`, `note_fetch_failure`, `note_undelivered`, `invalidate_chunk`,
  `pin_settled`, `inflight_count`), its ingest-frame calls and its queries (`pick_nearest`,
  `pick_rect`, `chunk`, `terrain`, `chunk_size_m`, `prefab_rows`).
- `chunk_math`: `Bbox`, `TerrainSizeM`, `ChunkRect`, `chunk_id`, `expand_bbox` and the id and
  rectangle functions, for `crate::spatial::los::world`, `crate::world::terrain::roads::airfield`,
  `crate::world::environment::vegetation::canopy` and the developer tools' world line-of-sight
  verification.
- The constants `LRU_MIN_CHUNKS`, `FETCH_FAILURE_CAP`, `DRAW_CULL_MARGIN_M` and `APPLY_BUDGET_MS`.

## Boundaries

- Depends on: `crate::streaming::loaders` (`WorldChunk`, `ObjectsManifest`,
  `DEFAULT_CHUNK_SIZE_M`) and `crate::streaming::buffers` (the rebuilds and `deinterleave`);
  `crate::spatial::indexing::world::WorldSpatialIndex` for picking;
  `crate::world::environment` (prefab entries, footprint lookups, class codes,
  `building_visible`).
- Used by:
  - the rest of `crate::streaming`: the loaders, the buffer composers, the bridge's toggles and
    the memory statistics;
  - `crate::spatial::los::world`, `crate::world::terrain::roads::airfield`,
    `crate::world::environment::vegetation::canopy` and
    `crate::world::environment::buildings::footprint`;
  - the debug world line-of-sight bench in `apps/website/frontend/src/v2/apps/debug/world_los/`,
    and the world line-of-sight verification and its tests in
    `tools_v2/developer-tools/src/map_verification/`.
- Rules:
  - the chunk math clamps to the terrain, keeps the preload margin, lists ids row-major and adds
    the oversized ring (`chunk_rect_pinned_cases`, `preload_margin_pinned_cases`,
    `viewport_ids_length_and_order`, `ids_for_rect_row_major` and `oversized_ring_expands_rect` in
    `tests/chunk_math_tests.rs`);
  - pinned and known-empty chunks are never evicted, a chunk at the failure cap becomes an empty
    stub, and a new pin resets the failure counts; the tests in `residency/` hold the lifecycle.
