# World glyph atlas

The packed raster form of the world-object glyph set: one WebP image and the index of its cells.
The map engine fetches both from the [API](/documentation_v2/glossary.md#api) at world boot and
draws every world-object icon from them.

## Contents

```text
assets_v2/glyphs/atlas/
├── world-glyphs.json  the cell index: one pixel rectangle, anchor and mask flag per glyph key
└── world-glyphs.webp  the atlas image: every glyph rasterised into one 128 px cell
```

## Format

- Encoding: `world-glyphs.webp` is a lossless RGBA WebP with straight alpha, a power-of-two canvas
  of at most 4096 by 4096 (1024 by 512 for the current 29 glyphs). `world-glyphs.json` is UTF-8
  JSON: `meta` holds `schemaVersion`, `refZoom`, `width`, `height` and `cellPx` (128); `icons`
  maps each glyph key to `x`, `y`, `width`, `height`, `anchorX`, `anchorY` in pixels and `mask`,
  true when the glyph takes a tint.
- Schema: no JSON Schema; the builder writes the shape above, and `cargo xtask schema map-glyphs`
  checks it. The keys are the keys of `assets_v2/glyphs/manifest.json`, sorted, laid out row-major
  one cell each, each glyph fitted into its cell with its aspect kept and centred.
- Adding a file: none is added by hand. Both files are rebuilt together from the manifest and the
  SVG sources with `cargo run -q -p developer-tools --bin map -- build-glyph-atlas`.

## Producers and consumers

- Producers: `build_glyph_atlas` in `tools_v2/developer-tools/src/map_raster_pipeline/glyphs.rs`,
  run as the `map` binary's `build-glyph-atlas` command. It rasterises each SVG with resvg, checks
  the WebP header before writing, and refuses to overwrite the atlas with an empty image.
- Consumers:
  - the map engine's world boot, `load_glyph_atlas` in
    `apps/website/map-engine/src/streaming/loaders/world_loader/atlas.rs`, which fetches
    `/map-assets/glyphs/atlas/world-glyphs.json` and `world-glyphs.webp`, turns each rectangle into
    a UV quad in sorted key order and uploads the pixels through `RenderEngine::upload_glyph_atlas`;
  - `cargo xtask schema map-glyphs`
    (`tools_v2/xtask/src/verifications/schemas/checks/map_glyphs.rs`), which checks the canvas size,
    every rectangle and anchor, and the WebP header;
  - the map engine's native residency tests in
    `apps/website/map-engine/src/streaming/scheduler/residency/t152_3_tests/`, which require the
    index keys to equal the manifest keys.

## Boundaries

- Depends on: `assets_v2/glyphs/manifest.json` and the SVG sources in `assets_v2/glyphs/svg/`.
- Used by: the map engine through the API's `/map-assets/glyphs` mount, the xtask glyph gate and
  the tests above.
- Rules: both files are build output and change only together, by a rebuild; the index holds
  exactly the manifest's keys (`cargo xtask schema map-glyphs`); the atlas holds at most 32 cells,
  the graphics engine's `ATLAS_GLYPH_COUNT`, above which `upload_glyph_atlas` refuses the upload.
