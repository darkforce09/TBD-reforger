# Streaming crates

The engine category for the served world the map streams in chunk by chunk: which world chunks
stay resident for the viewport, and the draw buffers composed from the resident chunks. The two
crates are the CPU half of world streaming; the fetches, the browser host and the GPU uploads stay
in the map engine's loaders.

## Contents

```text
crates/streaming/
├── chunk_draw_buffers/  `chunk_draw_buffers`: the draw buffers, the layer toggles and the world residency that owns both halves
└── chunk_scheduler/     `chunk_scheduler`: the chunk residency, its pin, LRU eviction, ingest, object index and rebuild requests
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

The scheduler never calls the draw buffers: every residency change that leaves them stale
returns a `DrawRebuild`, and `WorldResidency` applies it before the call that produced it
returns. The draw buffers read the residency only through its public accessors.

## Boundaries

- Depends on: the world format crates (`world_chunks`, `prefab_catalog`, `world_file_formats`),
  the geometry crates (`spatial_indexes`, `map_coordinates`), the overlay crates
  (`map_draw_lanes`, `label_layout`), `road_network` (terrain), `vegetation` (world objects),
  `render_primitives` (graphics) and external crates (`serde_json`, `thiserror`).
- Used by: the map engine (`legacy/map_engine`, behind `streaming`), whose world and occluder
  loaders hold a `WorldResidency` and import both crates directly, and the frontend's debug world
  line-of-sight bench.
- Rules: a streaming crate declares `category = "crates/streaming"`, targets `any` and depends
  only on lower engine categories (`cargo xtask verify crate-tiers`).
