# World object loader

`WorldHost`, the browser loader behind the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s world objects: it fetches a
terrain's object manifest, prefabs, chunk index, glyph atlas, roads and regions from
`/map-assets`, feeds the chunks each viewport pins into the world residency, and uploads what they
compose to the render engine. It compiles only for wasm32 with `render`.

## Contents

```text
apps/website/map-engine/src/streaming/loaders/world_loader/
├── atlas.rs      the world glyph atlas: fetch, WebP decode to RGBA, UV table, one GPU upload
├── bootstrap.rs  `WorldHost::init`: manifest, prefabs, chunk index, atlas, roads, regions
├── ingest.rs     chunk fetches in batches of 12 and the drain of up to 24 chunks into the residency
├── metrics.rs    the warm-crossing allocation probe published at `window.__t9382`
├── mod.rs        the module tree; re-exports `WorldHost`
├── state.rs      `WorldHost` and its pending-chunk and atlas-upload records
├── terrain.rs    the road mesh per road-class signature and the landcover mesh, pushed to lanes
├── upload.rs     the airfield apron, and buildings, icon lanes and strips when the buffers change
└── viewport.rs   `run_viewport`: layer preferences, pin, fetch, drain, upload, occluder
```

## How it works

```text
init         manifest.json ─> residency and store manifests, objects.binary block
             prefabs (archive or gzip JSON) ─> residency tables ─> occluder init
             chunk index ─> residency cells; glyph atlas JSON + WebP; roads ─> airfield box; regions
run_viewport atlas upload (once) ─> layer preferences ─> road mesh for the zoom's signature
             residency.set_viewport(bounds, zoom) ─> missing ids ─> fetch_and_queue (12 at a time)
             drain (≤ 24 chunks: ingest_chunk_bin or ingest_chunk_gz) ─> push_to_engine
             occluder.run_viewport(residency) ─> true when anything changed
```

`init` reports one world file done per step, the seven `WORLD_INIT_FILES` the host declares.
Chunks come from the manifest's `{cx}_{cy}.bin` template as `TBDC` containers when its
`objects.binary` block matches this build, else from `<chunksPath>/<id>.json.gz`; a prefab, road
or region archive the block names replaces the JSON, and a missing or rejected one logs a console
warning and leaves that layer unloaded (roads and regions otherwise default to
`objects/roads.json.gz` and `objects/forest-regions.json.gz` under the asset base).

`fetch_and_queue` declares its batch (`Files(World, n)`) before the first request and marks the
ids in flight; `drain` applies at most 24 chunks inside one residency ingest frame, and a missing
or rejected body counts toward the fetch-failure cap. `push_to_engine` uploads only when the
buffers revision, the pin's settled state or the in-flight emptiness changes, skips an empty
building fill while chunks are pending, so a half-hydrated pin never wipes drawn buildings, and
uploads an icon lane once it has instances, the pin has settled or the lane is off. Road meshes
are cached per road-class signature, the non-forest landcover mesh shows while the residency's
forest fill is in effect, and `metrics.rs` keeps the last eight warm crossings, logged with
`?t9382=1` or `window.__t9382Log`.

## Boundaries

- Depends on: the residency in `crate::streaming::scheduler`, the sibling loaders, the streaming
  bridge (progress, statistics, world-layer preferences), `crate::frame::EngineHandle`,
  `crate::world` (road, landcover and apron meshes, the region archive), `crate::overlay::lanes`
  and `crate::spatial::los::world`; `futures`, `serde_json`, `web-sys`, `js-sys` and
  `wasm-bindgen-futures`; the files under `/map-assets/<terrain>/` and `/map-assets/glyphs/atlas/`.
- Used by: `crate::streaming::host`, whose map host owns one `WorldHost`: boot `init`, the boot
  and settle passes, the airfield apron, the road list for labels, the occluder for queries.
- Rules:
  - the Mission Creator's boot-progress tests read these nine files by path and require the chunk
    batch to be declared before its first fetch
    (`every_world_batch_declares_its_files_before_it_fetches_them` in
    `apps/website/frontend/src/v2/apps/editor/tests/t628_boot_progress.rs`);
  - a full `init` reports exactly `WORLD_INIT_FILES` world files; a step that fails early reports
    fewer, and the host's `Finish` closes the segment;
  - chunk ids come only from the residency's pin and are marked in flight before they are
    fetched, so overlapping viewports never request a chunk twice.
