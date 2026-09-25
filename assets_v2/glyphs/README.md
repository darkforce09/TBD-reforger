# World glyphs

The icon set the map draws world objects with: the glyph manifest, the SVG sources and the packed
raster atlas built from them. The [API](/documentation_v2/glossary.md#api) serves the folder at
`/map-assets/glyphs`, beside the terrain mount rather than inside it, because every terrain shares
one glyph set.

## Contents

```text
assets_v2/glyphs/
├── atlas/         the packed WebP atlas and its cell index, built from the manifest and SVGs
├── manifest.json  the glyph registry: source SVG, base size, anchor, tint and default colour
└── svg/           the hand-authored SVG source of each glyph
```

## How it works

`manifest.json` declares a `schemaVersion`, a reference zoom (`refZoom`, 3), the paths of the atlas
image and its cell index, and one entry per glyph key under `glyphs`. Each entry names its SVG,
its base size in pixels (`baseSizePx`), its anchor as a unit offset inside its own box, whether it
accepts a tint (`tintable`), and, for a tintable glyph, the colour it keeps when drawn untinted
(`defaultColor`). An anchor of `[0.5, 1.0]` puts the origin at the bottom centre, so a tree or a
fence stands on its world position instead of floating centred over it; buildings, badges, rocks
and the unknown prop are centred at `[0.5, 0.5]`.

The 29 glyphs cover the kinds the world export classifies: buildings by function (residential,
military, industrial, agricultural, civic, commercial, hangar, tower, bunker, castle, lighthouse,
ruin, shed, tent, garage, container, bridge, generic), three badge overlays that mark a building
as military, bunker or tower, conifer, deciduous and palm trees, a bush, a rock, a fence, a
powerline and an unknown-prop fallback.

```text
prefab-classify.json ──render.iconKey──▶ terrain prefab catalogue ──▶ map engine: a key per object
                                                                              │ key → atlas cell
manifest.json + svg/ ──build-glyph-atlas──▶ atlas/ ──/map-assets/glyphs──▶ map engine ──▶ GPU pass
```

The world export gives every classified prefab a `render.iconKey` from
`contracts_v2/rules/prefab-classify.json`, and the key names a glyph here. The glyphs are raster,
not signed-distance fields, so they are rasterised once at 128 px per cell and sized on screen
around the reference zoom (`displayPx = baseSizePx * 2^(deckZoom - REF_ZOOM)`, in the map engine's
`overlay/lod.rs`). At world boot the map engine fetches the atlas, maps each key to its cell and
hands the pixels to the graphics engine's glyph-atlas texture for the instanced glyph pass.

## Format

- Encoding: `manifest.json` is UTF-8 JSON; `svg/` holds SVG files and `atlas/` a WebP image with a
  JSON index. A glyph key is `<kind>-<name>`, where the kind is a value of the `kind` enum in
  `contracts_v2/definitions/map-object-enums.schema.json`.
- Schema: no JSON Schema; `cargo xtask schema map-glyphs` checks the manifest (`baseSizePx` above
  0, both anchor components in 0 to 1, an existing SVG with a `viewBox`) and
  `cargo xtask schema map-object-enums` checks each key's kind prefix.
- Adding a file: add the SVG under `svg/` and its entry to `manifest.json`, rebuild `atlas/` with
  `cargo run -q -p developer-tools --bin map -- build-glyph-atlas`, then run both gates.

## Producers and consumers

- Producers: people write `manifest.json` and the SVGs; `build_glyph_atlas` in
  `tools_v2/developer-tools/src/map_raster_pipeline/glyphs.rs` writes `atlas/`.
- Consumers:
  - the API's `/map-assets/glyphs` mount in `apps/website/api_v2/src/core/http_router.rs`, whose
    folder comes from `GLYPH_ASSETS_DIR` (default `../../../assets_v2/glyphs`, from the API's
    working directory) and sits below the rate limiter;
  - the map engine's world boot in
    `apps/website/map-engine/src/streaming/loaders/world_loader/atlas.rs`, which reads `atlas/`;
  - `cargo xtask schema map-glyphs`, which also requires every `render.iconKey` in
    `contracts_v2/fixtures/map/map-object-prefabs-sample.json` and in the committed Everon
    catalogue `assets_v2/terrains/everon/objects/prefabs.json.gz` to have a manifest entry, and
    `cargo xtask schema map-object-enums`; both run in the `schema-validate` CI task;
  - the map engine's native tests in `apps/website/map-engine/src/streaming/loaders/tests/` and
    `apps/website/map-engine/src/streaming/scheduler/residency/t152_3_tests/`, which read the
    manifest's keys;
  - the headless-browser test server in `tools_v2/developer-tools/src/browser_testing/server.rs`,
    which serves `/map-assets/glyphs/` itself;
  - `cargo xtask deploy website`, whose rsync ships this folder (it excludes `assets_v2/terrains/`),
    and the staging compose file `apps/website/docker-compose.staging.yml`, which mounts it
    read-only into the API container.

## Boundaries

- Depends on: the `render.iconKey` values that `contracts_v2/rules/prefab-classify.json` assigns,
  and the `kind` enum of `contracts_v2/definitions/map-object-enums.schema.json`.
- Used by: the API, the map engine, the xtask schema gates, the developer tools' test server and
  the deploy and staging paths, as listed above.
- Rules: every icon key the classification can emit has a manifest entry, and a missing one is a
  classification defect, not a missing asset (`cargo xtask schema map-glyphs`); `atlas/` is rebuilt
  whenever the manifest or an SVG changes and holds exactly the manifest's keys; the set stays at
  or under 32 glyphs, the graphics engine's `ATLAS_GLYPH_COUNT`.
