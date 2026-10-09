# Road network source

The source of `road_network`: the segments, the class codec, the styling, the meshes, the strips,
the airfield, the error and the crate root that declares them.

## Contents

```text
crates/terrain/road_network/src/
├── airfield.rs              the box around the runways, its apron fill, and the airfield structures
├── cartographic_strip.rs    thin strips along a footprint's long axis: fences, piers and bridge rails
├── error.rs                 `Error` and `Result`: the archive's `BinaryError` behind one type
├── export_image_styling.rs  the road export images' class colours, widths, layer files, draw order and junctions
├── lib.rs                   the crate root: module header, `mod` lines and re-exports
├── mesh.rs                  `compose_roads_mesh`: casing and centreline buffers of the visible roads
├── network.rs               road segments from the JSON export or the archive, centred and measured
├── prelude.rs               the names most readers import
├── road_class.rs            the closed road-class table and its one-byte wire code
├── styling.rs               the road class table, the zoom gates, and the polyline-to-strip expansion
└── tests/                   unit tests for the network, the styling, the strips and the airfield
```

## How it works

`road_class` and `styling` are the base: `network` names archive classes through the codec,
`mesh` and `cartographic_strip` expand polylines with the styling, and `airfield` boxes the
network's runways.

## Boundaries

- Depends on: `world_file_formats`, `prefab_catalog`, `terrain_elevation`, `render_primitives`,
  `map_coordinates`, `rkyv`, `serde_json` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
