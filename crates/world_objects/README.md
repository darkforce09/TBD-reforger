# World object crates

The engine category for what stands on a terrain's ground as the map and the line of sight read
it: the vegetation, the interiors of buildings, and the place names written on the ground. Each crate is plain computation over a
terrain's served files; the browser loads, GPU uploads and line-of-sight queries that use them
stay in the map engine.

## Contents

```text
crates/world_objects/
├── building_interiors/  `building_interiors`: blueprints and sight-line attribution, compounds with doors, section cuts
├── place_names/         `place_names`: spot heights, town and road names, their declutter, the labels archive, glyph packing
└── vegetation/          `vegetation`: forest regions, canopy mass outline, tree counts, island density bins
```

## Boundaries

- Depends on: the geometry crates (`geometry_primitives`, `map_coordinates`, `spatial_indexes`),
  the world format crates (`world_file_formats`, `prefab_catalog`, `world_chunks`), the terrain
  crates (`terrain_elevation`, `road_network`), the overlay crates (`map_draw_lanes`,
  `label_layout`), `render_primitives`, `newtype_ids`, and external crates (`rkyv`, `serde`,
  `serde_json`, `thiserror`).
- Used by: `map_asset_loading`, whose forest and location label loaders read these crates, and
  `chunk_draw_buffers`, which packs their lanes; the world store and the line of sight
  crates; the Mission Creator's debug benches and the developer tools, which import the crates
  directly.
- Rules: a world object crate declares `category = "crates/world_objects"`, depends only on lower
  engine categories and on lower world object crates, and holds no browser or GPU code
  (`cargo xtask verify crate-tiers`).
