# Vegetation (map engine)

The map engine's half of the vegetation: the browser loader and the GPU lane of the forest mass.
The regions, the forest mass outline, the tree counts and the density grid are the `vegetation`
crate (`crates/world_objects/vegetation/`), which every caller imports directly.

## Contents

```text
legacy/map_engine/src/world/environment/vegetation/
├── buffers.rs  the engine's forest density texture lane and its fill and outline settings
├── loader.rs   `ForestMassHost`: fetches the density bins, uploads the forest fill and outline
└── mod.rs      the module tree: `buffers` and `loader`
```

## How it works

`ForestMassHost` fetches the 625 density bins `objects/density/{cx}_{cy}.bin` from the terrain's
asset folder (`assets/terrains/everon/` for Everon, served under `/map-assets/`), twelve at a time
with up to three attempts each, stitches them with `vegetation::density`, and once every bin has
arrived uploads the island grid as one texture through `forest_density_upload` and traces the
forest outline with `vegetation::mass` into hairlines. After that, each frame changes only the
fill's opacity (`forest_fill_alpha`) and whether the fill and the outline show at that zoom.

## Boundaries

- Depends on: `vegetation` (the grid, the marching squares), `world_file_formats::density` (the
  bin decoder), `crate::world::mesh` (hairline composition), `crate::world::scene`,
  `crate::world::terrain::satellite` (`TexLane`), `crate::frame`, `map_draw_lanes` (lanes and
  zoom gates), `crate::streaming` (boot progress, the statistics bridge), `browser_platform` for
  fetch, and `wgpu`.
- Used by: `crate::streaming::host`, which owns the `ForestMassHost`.
- Rules: `buffers.rs` and `loader.rs` compile only for wasm32 with the `render` feature.

## Related documentation

- [Vegetation crate](/crates/world_objects/vegetation/README.md) — the data these files load and
  draw.
