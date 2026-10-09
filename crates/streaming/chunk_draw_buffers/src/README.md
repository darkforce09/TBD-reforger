# World draw buffers

The CPU-side composers that turn the resident world chunks into the packed buffers the render
engine draws: the draw set of chunks, the tree, prop and building-badge icon instances, the pier,
bridge-rail and fence strips, the building footprint fill and outline, the world-layer toggles
that gate them, and the accessors the world loader uploads from. `WorldResidency` owns the
scheduler's `ChunkResidency` and the `DrawBuffers` composed over it, and applies every rebuild
request the residency returns.

## Contents

```text
crates/streaming/chunk_draw_buffers/src/
├── chunk_residency_delegates.rs  `WorldResidency`'s residency calls that request no rebuild
├── draw_buffers.rs               `DrawBuffers`: glyph lookup, toggles, draw set, buffers, memo keys
├── error.rs                      `Error` and `Result`: the chunk residency's refusal, wrapped
├── footprint.rs                  building fill and outline instances, the footprint and fill colours
├── glyphs.rs                     the prefab-to-glyph lookup and the tree, prop and badge buffers
├── lib.rs                        the crate root: the module tree
├── packer.rs                     the draw set refresh, the glyph memo key, the fill-band test
├── prelude.rs                    the common names for `use chunk_draw_buffers::prelude::*;`
├── residency_statistics.rs       `stats_json`: the residency's counters as one JSON object
├── revision.rs                   the buffer, count and lane getters the loader uploads
├── strips.rs                     the pier, bridge-rail and fence strips and their memo key
├── tests/                        unit tests of the world residency, by what they drive
├── toggles.rs                    the world-layer toggles, the atlas key order, the airfield box
└── world_residency.rs            `WorldResidency`: both halves, and the rebuild requests applied
```

## How it works

```text
WorldResidency call ─> ChunkResidency method ─> DrawRebuild ─> apply_draw_rebuild
  AllBuffers (new pin, frame close, unpin) ─> rebuild_buffers
  ZoomUnderUnchangedPin ─> fill band crossed? ─> rebuild_buffers, else refresh_draw_set_and_glyphs
  DrawSetAndGlyphs ─> refresh_draw_set_and_glyphs
  GlyphLookup (prefab load) ─> rebuild_glyph_lookup_from_prefabs
rebuild_buffers (footprint.rs: building fill and outline, empty below building_visible)
  ├─ rebuild_strip_buffers        pinned chunks: piers and docks, 2 rails a bridge, fences
  └─ refresh_draw_set_and_glyphs  draw set = the residency's draw_chunk_ids(last viewport)
       ├─ glyph memo key changed ─> tree heatmap decision, rebuild_glyph_buffers
       ├─ strip memo key changed ─> rebuild_strip_buffers
       └─ either rebuilt ─> buffers_revision + 1 ─> the world loader uploads
trees, props or buildings toggle ─> rebuild_buffers; new atlas keys ─> glyph lookup, refresh
fences toggle ─> rebuild_strip_buffers; airfield toggle or box ─> rebuild_glyph_buffers
```

The draw buffers read the chunk residency only through its public accessors and never write it;
the residency never calls the buffers. A fill band is the building gate, the badge band (zoom 1)
or the lowest importance zoom: an unchanged pin whose zoom crosses one recomposes every buffer.

The draw set is the chunks under the strict viewport (no cull margin) that the chunk index lists
and the residency has pinned; the pin is wider by its preload margin. The glyph lookup gives each
prefab whose icon key is in the atlas a glyph index, a size in metres, a tint and a group (trees
and vegetation, props and large rocks, white building badges); below `glyph_size_floor_zoom` the
minimum pixel size governs and every zoom step rebuilds, above it the glyph memo key ignores zoom
until a class gate or an importance zoom is crossed.

Icon instances pack 20 bytes each. Above `INSTANCE_BUDGET` (150,000) visible trees the tree lane
hands over to the density heatmap and returns below 85 % of it; props and badges together stop at
`INSTANCE_BUDGET`. From the badge band (zoom 1) every building with a landmark glyph gets a badge,
below it only those whose importance zoom is reached, and airfield structures only inside a shown
airfield. Building fills are 10 floats an instance (`[x, y, hx, hy, cos, sin, r, g, b, a]`) and
outlines 6 floats a `LineList` vertex, in world coordinates. `forest_fill_effective` keeps the
forest mass on while its class shows, or while trees are on and the heatmap owns them or none
packed.

`stats_json` renders 18 counters as one JSON object: resident, pinned, applied and drawn chunks;
apply frames with the last and longest apply time and the frames over budget; pinned building
instances; the object index size; the in-flight count and whether the pin has settled; the exact
tree count and the heatmap state; the buffers revision and the glyph and fill recompose counts;
and the known-empty chunks. The world loader merges it into the page's asset statistics.

## Public surface

- `world_residency::WorldResidency`: the residency the world loader holds, the occluder loader
  mirrors and the debug world line-of-sight bench builds; every public method of the scheduler's
  `ChunkResidency` under the same name, plus the draw getters of `revision.rs`, the toggles and
  visibility of `toggles.rs` and `stats_json`; a chunk is named by `world_chunks::ChunkId` in
  every method that takes one, in the missing chunks `set_viewport` returns and in the id lists
  it returns (`draw_ids`, `draw_chunk_ids`, `resident_chunk_ids`, `eviction_log`).
- `Error` and `Result` at the crate root; the common names in `prelude`.

## Boundaries

- Depends on: `chunk_scheduler` (`ChunkResidency` and its accessors, `DrawRebuild`,
  `IngestOutcome`, `ResidencyEvent`); `map_draw_lanes::zoom_gates` and
  `label_layout::glyph_math` (gates, `INSTANCE_BUDGET`, glyph sizes, keys and packing,
  `building_visible`); `prefab_catalog` (class codes, oriented boxes),
  `world_chunks` (`ChunkId`, `WorldChunk`), `map_coordinates` (`Bbox`, `TerrainSizeM`),
  `render_primitives` (`norm`, icon packing, pixel-size floors), `vegetation::canopy` (the tree
  counts and heatmap rule) and `road_network` (airfield structures and box, strip composers).
- Used by: the map engine's world loader (uploads, toggles, loads, ingest, statistics) and
  occluder loader
  (`take_residency_events`, `chunk`, `prefab_rows`), the debug world line-of-sight bench in
  `crates/frontend/workspaces/debug_benches/src/world_los/`, and the tests in `tests/`. Inside the folder,
  `footprint.rs` asks `early_landmark_glyph_active` for the fill de-emphasis and calls the strip
  and glyph rebuilds after the building fill, and `strips.rs` reads its `fill_color`.
- Rules:
  - every rebuild request is applied before the call that produced it returns, in the order an
    inline rebuild ran; the draw set equals the strict-viewport reference inside the pinned set
    (`class_s_draw_set_equals_strict_reference`), and `draw_chunk_ids` asserts in debug builds
    that `DRAW_CULL_MARGIN_M` stays 0;
  - an unchanged memo key rebuilds nothing and leaves `buffers_revision` alone
    (`compose_memo_stable_then_bumps`, `a5_glyph_memo_hits_in_band_busts_on_cross`), the heatmap
    switch keeps its hysteresis (`class_r_heatmap_hysteresis`) and no zoom step leaves the forest
    blank (`property_never_blank_zoom_ladder`), all in
    `crates/streaming/chunk_draw_buffers/src/tests/residency_lifecycle/cases_1.rs`;
  - under budget every visible tree packs a glyph;
  - `strips_visible` follows the toggles and the zoom only, never the buffer contents, so an empty
    strip buffer mid-hydration uploads as visible instead of blanking the lane;
  - `stats_json` is written by hand with `format!`, so it must stay valid JSON and keep its key
    names, which `merge_residency_stats` in the map engine's `streaming::bridge::statistics` and the tests
    read: `class_r_chunks_draw_matches_draw_ids_len` parses it and reads `chunks_draw`, and
    `parsed_empty_chunk_is_known_empty_and_not_refetched` reads `known_empty_count`.
