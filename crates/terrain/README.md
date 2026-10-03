# Terrain crates

The engine category for the ground itself as the map reads it: the elevation model, the relief
drawn from it, the satellite container and the water data. Each crate is plain computation over a
terrain's served files; the browser loads and GPU uploads that use them stay in the map engine.

## Contents

```text
crates/terrain/
├── road_network/       `road_network`: road segments and class codec, styling, road meshes, strips, airfield
├── satellite_imagery/  `satellite_imagery`: the `.tbd-sat` container reader, index checks and level picks
├── terrain_elevation/  `terrain_elevation`: raster placement, PNG and raw grid decoding, sampling, vector grid
├── terrain_relief/     `terrain_relief`: hillshade image, contour rings with summit picks, sea band fills
└── water_bodies/       `water_bodies`: bathymetry water mask, level suffix plan, inland archive, sea fill mesh
```

## Boundaries

- Depends on: the world formats crate (`world_file_formats`), the geometry crate
  `map_coordinates`, the graphics crate `render_primitives`, and external crates (`png`, `rkyv`,
  `bytemuck`, `serde`, `thiserror`).
- Used by: the map engine (`legacy/map_engine`), whose terrain modules keep the browser loaders,
  the relief host and the texture layers over these crates; the single-page app and the
  developer tools, which import the crates directly.
- Rules: a terrain crate declares `category = "crates/terrain"`, depends only on lower engine
  categories and on lower terrain crates, and holds no browser or GPU code
  (`cargo xtask verify crate-tiers`).
