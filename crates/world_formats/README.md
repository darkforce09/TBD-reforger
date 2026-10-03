# World format crates

The engine category for the files a terrain's map data is stored in: the byte layouts the
developer tools write and the map engine reads, each defined once so the two sides cannot
disagree, and the headless store that reads a terrain's world data through them.

## Contents

```text
crates/world_formats/
├── prefab_catalog/      `prefab_catalog`: prefab rows, render classes, footprint lookups, prefab tables, payload decoding
├── world_chunks/        `world_chunks`: chunk JSON and `TBDC` container decoding, chunk ids, the terrain manifest
├── world_file_formats/  `world_file_formats`: rkyv archives, fixed-header containers, density grids, object rows
└── world_store/         `world_store`: the headless world store, a terrain's manifest, prefabs, roads, regions, chunks
```

## Boundaries

- Depends on: `newtype_ids` (foundation) for the identifier types, `map_coordinates` (geometry)
  for the chunk identifier's spelling, and external crates (`rkyv`, `bytemuck`, `serde_json`,
  `flate2`, `thiserror`); inside the category, `prefab_catalog` on `world_file_formats` and
  `world_chunks` on both; `world_store` also on `road_network` (terrain) and `vegetation` (world
  objects), the crates whose payloads it loads.
- Used by: the map engine, behind its `streaming` feature (`world_file_formats`,
  `prefab_catalog`, `world_chunks`, `world_store`), and the developer tools, which import the
  crates directly.
- Rules: a world formats crate declares `category = "crates/world_formats"` and depends only on
  lower engine categories (`cargo xtask verify crate-tiers`).
