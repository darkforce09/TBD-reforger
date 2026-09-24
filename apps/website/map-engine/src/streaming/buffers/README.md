# World draw buffers

The CPU-side composers that turn the resident world chunks into the packed buffers the render
engine draws: the draw set of chunks, the tree, prop and building-badge icon instances, the pier,
bridge-rail and fence strips, and the accessors the world loader uploads from. Each of its four
modules adds methods to `WorldResidency`, and each needs the `streaming` feature.

## Contents

```text
apps/website/map-engine/src/streaming/buffers/
├── glyphs.rs    the prefab-to-glyph lookup and the tree, prop and badge icon instance buffers
├── mod.rs       the module tree
├── packer.rs    the draw set, the glyph and strip memo keys, the tree heatmap switch
├── revision.rs  `BUILDING_MIN_ZOOM` and the buffer, count and lane accessors the loader uploads
└── strips.rs    the pier, bridge-rail and fence strips over the pinned chunks, and their memo key
```

## How it works

```text
pin or fill-band change, ingest frame, trees, props or buildings toggle
  └─ rebuild_buffers (building fill and outline, crate::world::environment::buildings::footprint)
       ├─ rebuild_strip_buffers        pinned chunks: piers and docks, 2 rails a bridge, fences
       └─ refresh_draw_set_and_glyphs  draw set = strict viewport ∩ chunk index ∩ pinned, sorted
            ├─ glyph memo key changed ─> tree heatmap decision, rebuild_glyph_buffers
            ├─ strip memo key changed ─> rebuild_strip_buffers
            └─ either rebuilt ─> buffers_revision + 1 ─> the world loader uploads
same pin and fill band, or new atlas keys ─> refresh_draw_set_and_glyphs alone
fences toggle ─> rebuild_strip_buffers; airfield toggle or box ─> rebuild_glyph_buffers
```

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

## Boundaries

- Depends on: `crate::streaming::scheduler` (`WorldResidency`, `chunk_math`,
  `DRAW_CULL_MARGIN_M`, `ResidencyEvent`); `crate::overlay::lod` and
  `crate::overlay::symbology::labels::glyph_math` (gates, `INSTANCE_BUDGET`, glyph sizes, keys and
  packing); `crate::world::environment` (class codes, `building_visible`, the tree counts and
  heatmap rule) and `crate::world::terrain::roads` (airfield structures, strip composers).
- Used by: the rest of `crate::streaming` (the scheduler's pin, the bridge's toggles, the prefab
  load, the world loader's uploads, the occluder loader's `take_residency_events`, the memory
  statistics) and `crate::world::environment::buildings::footprint`, which reads
  `BUILDING_MIN_ZOOM` and `norm`, asks `early_landmark_glyph_active` for the fill de-emphasis and
  calls the strip and glyph rebuilds after the building fill.
- Rules:
  - the draw set equals the strict-viewport reference inside the pinned set
    (`class_s_draw_set_equals_strict_reference`), and `draw_chunk_ids` asserts in debug builds
    that `DRAW_CULL_MARGIN_M` stays 0;
  - an unchanged memo key rebuilds nothing and leaves `buffers_revision` alone
    (`compose_memo_stable_then_bumps`, `a5_glyph_memo_hits_in_band_busts_on_cross`), the heatmap
    switch keeps its hysteresis (`class_r_heatmap_hysteresis`) and no zoom step leaves the forest
    blank (`property_never_blank_zoom_ladder`), all in
    `apps/website/map-engine/src/streaming/scheduler/residency/tests/cases_1.rs`;
  - under budget every visible tree packs a glyph (`tree_glyphs_pack_from_real_everon_data` in
    `apps/website/map-engine/src/streaming/scheduler/residency/t152_3_tests/cases_1.rs`).
