**Status:** live

# Map streaming

How the map engine gets a terrain's served map data into the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map and keeps only what the
view needs: the boot sequence, the viewport passes that pin, fetch, ingest and evict world chunks,
the memory budget the loads report to, and the loaders that parse the served files. It runs in the
browser, and its residency, parsers and budget also run natively for the offline tools and the
tests.

## Where it lives

- Code: [`apps/website/map-engine/src/streaming/`](/apps/website/map-engine/src/streaming/README.md)
  and its children: [`host/`](/apps/website/map-engine/src/streaming/host/README.md) (the boot
  sequence and settle passes), [`loaders/`](/apps/website/map-engine/src/streaming/loaders/README.md)
  with [`world_loader/`](/apps/website/map-engine/src/streaming/loaders/world_loader/README.md),
  [`scheduler/`](/apps/website/map-engine/src/streaming/scheduler/README.md) (the chunk
  residency), [`buffers/`](/apps/website/map-engine/src/streaming/buffers/README.md),
  [`memory/`](/apps/website/map-engine/src/streaming/memory/README.md) with
  [`budget/`](/apps/website/map-engine/src/streaming/memory/budget/README.md), and
  [`bridge/`](/apps/website/map-engine/src/streaming/bridge/README.md). The terrain, satellite,
  water, forest and label loaders the host drives live under
  [`world/`](/apps/website/map-engine/src/world/README.md).
- Entry: `streaming::host::bootstrap`, which the Mission Creator calls once its render engine
  exists (`apps/website/frontend/src/v2/apps/editor/bridge/world_assets.rs`); the settle and
  `flush_viewport` calls, which its pointer and wheel gestures and camera dock make after the
  camera moves.
- Related features: the [map engine overview](/documentation_v2/website/map-engine/map_engine_overview.md),
  the [terrain assets](/assets_v2/terrains/README.md) the loaders read, and the Mission Creator's
  [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  for the map features streaming backs.

## Behaviour

### Boot

1. The page mounts a render engine, registers the engine and its map host as the render context
   (`RENDER_CTX`), and calls `bootstrap` with the terrain id, its preference readers and a
   progress callback.
2. The host fetches `/map-assets/<terrain>/manifest.json` and declares the boot's files to the
   progress bar before it fetches them.
3. It holds forecasts for the elevation model and the hillshade (width × height × 4 bytes each) in
   the memory ledger, then loads two things at once: the elevation model (a raw `TBDE` body or a
   PNG streamed against its content length), turned into metres and a hillshade texture; and the
   satellite basemap, whose finest level the memory budget chooses.
4. It builds the elevation grid, then applies the grid, hillshade and basemap preferences.
5. It loads the world objects, the forest, the water, the airfield apron and the labels, then runs
   up to 12 viewport passes until nothing more arrives, and closes the world segment of the
   progress bar. Every segment closes on every path, a failed manifest fetch included.

### Viewport passes

After each camera change the host waits for the camera to settle (120 ms debounce, 250 ms at
most) and runs up to 6 passes; a pass stops the run once neither the world nor the forest did
work. Each pass of the world loader:

1. Asks the residency which chunks the viewport pins (`WorldResidency::set_viewport`). The
   viewport grows by a preload margin of 5 % of its longer side, at least one 512 m chunk, is
   clamped to the terrain, and grows one more ring of chunks when a prefab is oversized. Below the
   building zoom gate and every importance zoom it unpins everything and requests nothing.
2. When the pin is new, the residency marks the missing chunks in flight, clears their failure
   counts and returns them; when it is unchanged, it asks again only for pinned chunks still
   missing, once nothing is in flight.
3. The loader declares the batch to the progress bar, then fetches the missing chunks 12 at a
   time, as `TBDC` binaries when the manifest's `objects.binary` block matches this build, else as
   gzip JSON.
4. It ingests at most 24 fetched chunks a pass, inside one ingest frame. A parsed chunk enters the
   spatial index; an empty one is marked known-empty; a failed fetch or a malformed body counts
   toward a cap of 3, after which the chunk is stored as an empty stub.
5. The residency evicts once it holds more than 64 chunks or three times the pinned count,
   whichever is larger: unpinned, not known-empty chunks, least recently used first, oldest insert
   on a tie.
6. The loader uploads the rebuilt draw buffers to the render engine when the buffers revision,
   the pin's settled state or the in-flight emptiness changed. It skips an empty building fill
   while chunks are pending, and uploads an icon lane once it has instances, the pin has settled
   or the lane is off.
7. The occluder loader mirrors the inserted and evicted chunks into the line-of-sight occluder and
   fetches the building descriptors and BVH sidecars the resident chunks need: up to 96 wants a
   round, 12 requests at a time, 8 rounds, and a file that fails 3 times is given up for the
   session.

The residency's lifecycle, constants and events are in the
[scheduler README](/apps/website/map-engine/src/streaming/scheduler/README.md#how-it-works). The
ingest frame records its apply time against a 4 ms budget (`APPLY_BUDGET_MS`) for the statistics;
the 24-chunk count is what bounds a pass.

### Memory budget

1. One ledger per page holds seven rows (elevation model, hillshade, satellite, world, forest,
   labels, water) against a budget of 1,536 MiB, or the `memBudgetMb` query parameter, or a
   positive `window.__memBudgetMb`.
2. Loaders declare what they hold: `hold` records a forecast, `set_held` replaces it with the real
   size, `release` lowers it, and the host measures each load's heap growth beside the declared
   bytes. The ledger allocates and frees nothing.
3. The satellite floor is the one choice the budget makes: starting at the level the GPU limit
   allows, it moves one level coarser each time the cost of that level and every coarser one
   would not fit, loads the coarsest when none fits, and reserves the chosen cost.
4. Each change republishes the ledger as `window.__t9386`, and the Mission Creator's debug HUD
   shows `· mem <held>/<budget>MB · sat L<floor> (+<raised>)`.

The ledger's figures and `decide` answers are in the
[budget README](/apps/website/map-engine/src/streaming/memory/budget/README.md#how-it-works).

### Loaders

- Chunk bodies: a `TBDC` chunk is a header and 32-byte instance rows; a payload whose length
  disagrees with the header, or a chunk whose header names another tile, is refused. Both chunk
  parsers build the same column layout.
- Prefab, road and region payloads are told apart by their first bytes: gzip JSON (`1f 8b`) or a
  validating rkyv archive. A prefab catalogue built for another terrain, or one that repeats a
  prefab id, is refused.
- Satellite range fetches accept only a `206` with a `Content-Range` total, so a server that
  ignores `Range` never sends the whole bundle; a `429` is reported with its `Retry-After`.

### Known discrepancies

- The doc comment on `note_undelivered` says an undelivered chunk's empty stub is never requested
  again (`apps/website/map-engine/src/streaming/scheduler/viewport.rs:126`); `evict`
  (`viewport.rs:169-180`) spares only pinned and known-empty chunks, and only a parsed empty chunk
  becomes known-empty (`loaders/residency.rs:112-122`), so an evicted stub is fetched again when
  its chunk is pinned later.
- `apps/website/map-engine/src/streaming/loaders/store.rs:41` names a `super::binary::BinaryError`
  that does not exist.

## Data

Files read under `/map-assets/<terrain>/`, which the [API](/documentation_v2/glossary.md#api)
serves from `assets_v2/terrains/` (or `MAP_ASSETS_DIR`):

- `manifest.json`: the terrain manifest, with the `objects` block (`prefabsPath`, `chunksPath`,
  `chunkSizeM`, default 512 m) and the `objects.binary` block that decides whether chunk binaries
  are read;
- the chunk index (`chunks/manifest.json`), whose cells bound every pin, and the chunks,
  `{cx}_{cy}.bin` or `<chunksPath>/<id>.json.gz`;
- the prefab catalogue (`prefabs.rkyv` or `prefabs.json.gz`), roads and forest regions;
- `prefabs/blas-manifest.json`, the building blueprint archive, descriptors and BVH sidecars, for
  the occluder;
- the elevation model, hillshade inputs, satellite bundle, water, forest density and labels, read
  by the `world/` loaders;
- the glyph atlas under `/map-assets/glyphs/atlas/` (`assets_v2/glyphs/`, or `GLYPH_ASSETS_DIR`).

Page inputs and outputs: the `memBudgetMb`, `sat=preview` and `t9382=1` query parameters (the
map engine README's [Configuration](/apps/website/map-engine/README.md#configuration)); the
`window.__mapAssets` statistics and the `window.__t9386` ledger the page and the editor gates
read.

## Design

- Residency is decided by the viewport alone: a chunk is fetched because the view pins it, and
  eviction is least recently used above a floor of 64 chunks, so recently seen ground stays
  resident and panning back over it fetches nothing until the floor is passed.
- Every limit that bounds the work of one pass is a count (12 fetches at a time, 24 ingests, 96
  occluder wants); the 4 ms ingest budget is measured and reported, not enforced.
- Design target: the [map binary storage spec](/documentation_v2/tickets/specs/t935_map_binary_storage.md)
  and the [engine performance spec](/documentation_v2/tickets/specs/t938_engine_perf.md), a
  design-phase reference. Differences: the world objects still ship as one file per 512 m chunk
  with gzip JSON beside the binaries, where the target is one object container with a spatial
  index read by range requests, the shape the satellite bundle already has.
- The grid and the basemap extent are fixed at 12,800 m for every terrain
  (`apps/website/map-engine/src/streaming/host/queries.rs:9`), while Arland is 4,096 m.

## Open work

- [T-1057 — Fix occluder residency events piling up after prefab load failure](/.ai/tickets/T-1057.toml)
  (idea, no plan): when the prefab catalogue fails to load, the occluder never becomes ready and
  returns before draining the residency's events, so the queue grows for the session; the fix
  drains them.
- [T-1060 — Check map streaming stubs re-fetched and satellite budget never released](/.ai/tickets/T-1060.toml)
  (idea, no plan): an evicted stub stops being fetched again, and the satellite's reserved bytes
  are released or replaced.
- [T-1062 — Derive map grid, basemap, peaks and forest from terrain size](/.ai/tickets/T-1062.toml)
  (idea, no plan): the grid and basemap follow the loaded terrain's size.
- [T-1123 — Decide whether chunk index paths name the .bin or .json.gz](/.ai/tickets/T-1123.toml)
  (idea, no plan): Everon's chunk index and the object builder agree on one chunk form.
- [T-935.15 — Delete chunking: one container, spatial index, range fetch](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-935_15_plan.md)),
  [T-935.18 — Emit the object container and spatial index beside the chunks](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-935_18_plan.md)) and
  [T-935.20 — Delete residency chunk_bin and the chunk vocabulary](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-935_20_plan.md)): world objects move to one
  container with a spatial index and range fetches, and the chunk residency's chunk vocabulary
  goes.
- [T-935.16 — Finish the gz-JSON cutover T-935.13 left half done](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-935_16_plan.md)): the gzip JSON chunks,
  prefabs, roads and forest regions go, and with them `flate2`.
- [T-938 — Engine and wasm performance](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-938_plan.md)): measured chunk-crossing uploads
  and a wasm memory budget guard, among the render findings.
- [T-1042 — Rename ticket ids out of code names and UI strings](/.ai/tickets/T-1042.toml) (idea,
  no plan): the test folders `memory/budget/t938_6/`, `scheduler/residency/t151_11_3_tests/` and
  `t152_3_tests/`, and the `t9382` and `__t9386` names, get subject names.
- [T-1067 — Remove dead map engine code, facades and duplicated constants](/.ai/tickets/T-1067.toml)
  (idea, no plan): the test-only `ingest_budget_exhausted_at`, the callerless `invalidate_chunk`
  and `release_inflight`, and the duplicated `BUILDING_MIN_ZOOM` go.

## Decisions

- A chunk that fails 3 times becomes an empty stub: a chunk missing on the server is not asked
  for again while its stub stays resident, and one bad file cannot stall the pin.
- Chunk ids come only from the residency's pin and are marked in flight before they are fetched:
  overlapping viewports never request a chunk twice.
- An empty building fill is not uploaded while chunks are pending: a half-hydrated pin never
  wipes buildings already drawn.
- The memory ledger records what loaders declare and never allocates: the budget is a decision
  aid for the satellite floor and a readout, not an allocator.
- Satellite range fetches refuse a full `200` body: a server that ignores `Range` would otherwise
  send the whole bundle to a page that asked for one level.
- Streaming names no UI crate: editor state enters only through the preference readers and the
  progress callback the page supplies, so the residency, parsers and budget run under
  `cargo test` and in the offline tools.
