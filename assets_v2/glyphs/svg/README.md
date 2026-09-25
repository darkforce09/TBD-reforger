# World glyph SVG sources

The hand-authored vector sources of the world-object glyph set, one SVG per glyph. The atlas
builder rasterises them into `assets_v2/glyphs/atlas/`; nothing reads them at runtime.

## Contents

```text
assets_v2/glyphs/svg/
└── *.svg  one glyph each, named `<kind>-<name>.svg` after its glyph key in the manifest
```

## How it works

`assets_v2/glyphs/manifest.json` names one of these files as each glyph's `svg`, and 29 glyphs name
29 files: 18 building functions (`building-residential.svg` to `building-generic.svg`), three
building badge overlays (`building-badge-military.svg`, `building-badge-bunker.svg`,
`building-badge-tower.svg`), three trees and a bush, a rock, a fence, a powerline, and the
`prop-unknown.svg` fallback. The part of the name before the first hyphen is a world-object kind:
`building`, `tree`, `vegetation`, `rock`, `prop` or `utility`.

## Format

- Encoding: UTF-8 SVG with a `viewBox` (every current file uses `0 0 24 24`) and no external
  references. The four glyphs that take a tint (`tintable` in the manifest: the three trees and
  the bush) have their main fill in the colour the manifest records as their `defaultColor`.
- Schema: the glyph gate requires an `<svg>` root element with a `viewBox`; the builder fits the
  drawing into a 128 px cell with its aspect kept.
- Adding a file: add `<kind>-<name>.svg` here, add its entry to `assets_v2/glyphs/manifest.json`,
  rebuild the atlas with `cargo run -q -p developer-tools --bin map -- build-glyph-atlas`, and run
  `cargo xtask schema map-glyphs` and `cargo xtask schema map-object-enums`.

## Producers and consumers

- Producers: people; no tool writes these files.
- Consumers: the atlas builder `build_glyph_atlas` in
  `tools_v2/developer-tools/src/map_raster_pipeline/glyphs.rs`, and `cargo xtask schema map-glyphs`
  (`tools_v2/xtask/src/verifications/schemas/checks/map_glyphs.rs`), which checks that every
  manifest entry's file exists and is an SVG with a `viewBox`.

## Boundaries

- Depends on: the glyph keys in `assets_v2/glyphs/manifest.json`, and the `kind` enum of
  `contracts_v2/definitions/map-object-enums.schema.json`, which the name prefix must belong to.
- Used by: the atlas builder and the glyph gate above.
- Rules: every manifest entry names an existing SVG with a `viewBox` (`cargo xtask schema
  map-glyphs`); the name prefix is a kind from the enum (`cargo xtask schema map-object-enums`); a
  changed SVG reaches the map only through an atlas rebuild.
