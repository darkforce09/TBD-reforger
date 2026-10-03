# Place names source

The source of `place_names`: the spot heights, the town labels and the archive reader, the road
name placement with its geometry and file model, the glyph packing, the error and id types, and
the crate root that declares them.

## Contents

```text
crates/world_objects/place_names/src/
├── error.rs            `Error` and `Result`: label JSON, road name baking and archive failures
├── label_packing.rs    spot-height, town and road-name labels packed into glyph instances and bytes
├── lib.rs              the crate root: module header, `mod` lines and re-exports
├── peaks.rs            spot heights: peak finding on the elevation model, declutter, label specs
├── prelude.rs          the names most readers import
├── road_name_ids.rs    `RoadNameId`, the id of one curated road name
├── route_geometry.rs   polyline length, tangents, anchor fractions and distances for road names
├── route_labels.rs     the `road-names.json` model and the labels archive's `road_names` lane
├── route_placement.rs  road name placement and declutter, and the class visibility floors
├── tests/              unit tests for the spot heights, the town and archive readers, road names
└── towns.rs            the label JSON parsers, the archive's town and height lanes, its reader
```

## How it works

`peaks` and `towns` hold the spot-height and town rows; `route_placement` places and declutters
road names over `route_geometry` and reads the curated list of `route_labels`; `towns` reads the
road lane through `route_labels`; `label_packing` declutters and packs all three.

## Boundaries

- Depends on: `label_layout`, `terrain_elevation`, `road_network`, `world_file_formats`,
  `render_primitives`, `newtype_ids`, `rkyv`, `serde`, `serde_json` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
