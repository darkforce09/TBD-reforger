# Glyph atlas and text packing

The CPU side of drawing text and sprites: a built-in 16 by 32 pixel bitmap font, the ASCII atlas
baked from it, the character-to-cell map, the laying out of an already-chosen string into glyph
instances, and the bit-packing of the 20-byte sprite instance they share with the icon lanes.
Which strings exist and where they sit is the caller's decision.

## Contents

```text
apps/website/graphics-engine/src/text/
├── atlas.rs    `bake_ascii_atlas_rgba`: the haloed 16 by 6 cell atlas, its colours and UVs
├── font.rs     `FONT_16X32`: 96 bitmap glyphs, 16 pixels of ink in a 32-pixel cell
├── layout.rs   `GlyphSpec`, width declutter, glyph placement and packing of placed glyphs
├── metrics.rs  `TextGlyphInstance`, glyph size in metres, and the character-to-cell map
├── mod.rs      the module tree
├── pack.rs     RGBA and yaw packing, the 20-byte sprite writer, the `TextUniforms` block
├── scale.rs    `REF_ZOOM` and `size_with_min_px`: zoom-anchored glyph size, pixel floor
└── tests/      unit tests for the width declutter
```

## How it works

The atlas holds 96 cells in 16 columns and 6 rows (`TEXT_ATLAS_COLS`, `TEXT_ATLAS_ROWS`), 32
pixels each (`TEXT_CELL_PX`): cells 0 to 94 are printable ASCII 32 to 126 and cell 95
(`TOFU_GLYPH`) is the fallback. `bake_ascii_atlas_rgba` paints each `FONT_16X32` glyph in
`TEXT_INK_RGBA` over a 2-pixel `TEXT_HALO_RGBA` halo, top row first, and returns the RGBA bytes
with the width and height; `glyph_cell_uv` maps a cell and a unit quad corner to atlas UVs,
flipping V.

`glyph_index_for_char` maps printable ASCII straight to its cell, folds accented Latin letters,
curly quotes and dashes to ASCII, and sends everything else to the fallback cell, so no
character is dropped. `text_char_meters(zoom)` sizes a cell in world metres: 24 pixels at
`REF_ZOOM` (3.0), never below 20 pixels on screen, through `size_with_min_px`.

A caller hands `layout.rs` a list of `GlyphSpec`s (an id, an anchor in whole metres, the string and
its cell size). `declutter_specs_by_width` drops a spec whose laid-out width overlaps one already
kept, first come first kept. `glyphs_from_specs` centres each string on its anchor with an
advance of half a cell (`TEXT_GLYPH_ADVANCE_RATIO`), and `pack_text_icon_bytes` or
`pack_text_icon_bytes_tint` writes the glyphs as 20-byte sprite instances.

`pack.rs` writes that 20-byte layout for any sprite: position, size, yaw as `snorm16` of the angle
wrapped into -180 to 180 degrees, cell index and an RGBA8 tint packed into a `u32`.
`screen_yaw_for_heading_deg` turns a compass heading into the screen yaw the shader expects.
`text_uniform_bytes` writes the 16-byte `TextUniforms` block: a pixel-to-metre factor of 1 and the
atlas grid size.

## Boundaries

- Depends on: nothing outside the crate; `std` only.
- Used by: `crate::layout`, which re-exports `metrics`, `layout`, `REF_ZOOM` and `pack`;
  `crate::frame::atlas`, which writes `text_uniform_bytes`; and `website-map-engine`, in
  `apps/website/map-engine/src/overlay/`: `symbology/text_metrics.rs` re-exports `atlas`, `font`
  and the metrics items, `symbology/text_packing.rs` declutters, places and packs labels through
  `layout`, `symbology/instances/mod.rs` re-exports `pack` as `packing`, and `lod.rs` re-exports
  `REF_ZOOM`; the text-lane upload `apps/website/map-engine/src/frame/upload/text.rs` bakes the
  atlas through those re-exports.
- Rules: every character maps to a cell, the fallback included; the declutter keeps the first of
  overlapping specs (`width_declutter_drops_overlapping_long_names`); the `TextUniforms` block
  stays 16 bytes, matching `shader.wgsl` (tested in `crate::shaders`); nothing here owns a GPU
  resource, so the map engine may import this module directly (`cargo xtask verify engine-layers`
  restricts only `text::gpu`, which does not exist).
