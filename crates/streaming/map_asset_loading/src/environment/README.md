# Environment loaders

The browser loaders of what stands on the ground: the forest mass and the cartographic labels.
The vegetation data and the place names are the
[`vegetation`](/crates/world_objects/vegetation/README.md) and
[`place_names`](/crates/world_objects/place_names/README.md) crates, which every caller imports
directly.

## Contents

```text
crates/streaming/map_asset_loading/src/environment/
├── forest_mass_loader.rs  `ForestMassHost`: fetches the density bins, uploads the forest fill and outline
├── location_labels/       `LabelHost`: the town names, road names and spot heights
└── mod.rs                 the module tree
```

## How it works

`ForestMassHost` fetches the 625 density bins `objects/density/{cx}_{cy}.bin` from the terrain's
asset folder (`assets/terrains/everon/` for Everon, served under `/map-assets/`), twelve at a time
with up to three attempts each, stitches them with `vegetation::density`, and once every bin has
arrived uploads the island grid as one texture through the asset sink's `forest_density_upload`
and traces the forest outline with `vegetation::mass` into hairlines. After that, each settle
changes only the fill's opacity and whether the fill and the outline show at that zoom.
`LabelHost` loads the label sources and uploads the label lanes that `place_names` places and
declutters. Both loaders compile only for wasm32.

## Public surface

- `forest_mass_loader`: `ForestMassHost` and `planned_density_bins`.
- `location_labels::loader`: `LabelHost` and `WORLD_LABEL_FILES`.

## Boundaries

- Depends on: `vegetation`, `place_names`, `label_layout`, `road_network`, `terrain_elevation`
  and `world_chunks`; `world_file_formats::density` (the density bins); `map_draw_lanes` (lanes
  and zoom gates); `map_streaming_model` (boot progress, layer preferences, the asset sink);
  `crate::asset_statistics`, `crate::mesh_composition`; `browser_platform` for fetch.
- Used by: the map host of `map_streaming_host`, which owns both loaders; the Mission Creator's
  boot-progress tests, which read the forest loader by path.
- Rules: a density bin counts toward the boot progress only once it decoded.
