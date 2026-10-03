# Streaming crates

The engine category for the served world the map streams in chunk by chunk: which world chunks
stay resident for the viewport, and the draw buffers composed from the resident chunks. The two
crates are the CPU half of world streaming; the fetches are `map_asset_loading`'s browser loaders,
which the browser map host `map_streaming_host` drives, and the GPU uploads stay with the
renderer behind the asset sink. `map_streaming_model` holds what the loaders, the host and the
frontend share without a browser or a GPU: the preferences, the boot progress, the memory budget
model and the `MapAssetSink` contract the loaders write the renderer through.

## Contents

```text
crates/streaming/
├── chunk_draw_buffers/   `chunk_draw_buffers`: the draw buffers, the layer toggles and the world residency that owns both halves
├── chunk_scheduler/      `chunk_scheduler`: the chunk residency, its pin, LRU eviction, ingest, object index and rebuild requests
├── map_asset_loading/    `map_asset_loading`: the browser loaders, the mesh composition, the live memory budget and the asset statistics
├── map_streaming_host/   `map_streaming_host`: the browser map host: boot, camera settle, view preferences and queries
└── map_streaming_model/  `map_streaming_model`: the preferences, boot progress, memory budget model and `MapAssetSink` contract
```

## How it works

```text
viewport, chunk bytes ─> chunk_scheduler::ChunkResidency ─> DrawRebuild requests
                                      │                              │
                                      └── read accessors ──> chunk_draw_buffers::DrawBuffers
                         chunk_draw_buffers::WorldResidency owns both and applies each request
```

| Crate | Holds | Tier |
|---|---|---|
| `chunk_scheduler` | the pin, the in-flight marks, LRU eviction, the prefab tables, the object index | 4 |
| `chunk_draw_buffers` | the glyph lookup, the toggles, the draw set and the packed draw buffers | 5 |
| `map_streaming_model` | the world-layer and host preferences, the boot progress, the budget ledger, the asset sink contract | 1 |
| `map_asset_loading` | the world, occluder, terrain and environment loaders, the mesh composition, the live budget, the asset statistics | 6 |
| `map_streaming_host` | the boot sequence, the host state, the camera settle, the view preferences, the queries | 7 |

The scheduler never calls the draw buffers: every residency change that leaves them stale
returns a `DrawRebuild`, and `WorldResidency` applies it before the call that produced it
returns. The draw buffers read the residency only through its public accessors.

## Boundaries

- Depends on: the world format crates (`world_chunks`, `prefab_catalog`, `world_file_formats`),
  the geometry crates (`spatial_indexes`, `map_coordinates`), the overlay crates
  (`map_draw_lanes`, `label_layout`), `road_network` (terrain), `vegetation` (world objects),
  `render_primitives` (graphics), the terrain and line-of-sight crates and `browser_platform`
  (the loaders and the host), and external crates (`serde`, `serde_json`, `thiserror`, the
  browser bindings).
- Used by: `map_asset_loading`'s world and occluder loaders, which hold a `WorldResidency` and
  import both residency crates directly; `map_streaming_host`, which drives the loaders;
  `map_renderer`, whose render engine implements `map_streaming_model`'s asset sink; the
  single-page app (`apps/frontend`), whose map view and Mission Creator drive the host and the
  loaders and whose debug world line-of-sight bench holds the draw buffers.
- Rules: a streaming crate declares `category = "crates/streaming"`, targets `any` (or `wasm32`
  when it names a browser crate, as the loaders and the host do) and depends only on foundation,
  contract, engine and graphics crates, never on a rendering crate or wgpu
  (`cargo xtask verify crate-tiers`).
