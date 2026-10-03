# Road network

The `road_network` crate: a terrain's road network and how the 2D map strokes it. Road segments
read from the JSON export or its binary archive, drawn as a dark casing under a centreline
coloured by class and gated by zoom; the airfield's box, apron fill and runway styling; and the
thin strips the map draws for fences, piers and bridge rails.

## Contents

```text
crates/terrain/road_network/
├── Cargo.toml  the package: `world_file_formats`, `prefab_catalog`, `terrain_elevation`, layout tier 3
└── src/        the network, the class codec, the styling, the meshes, the strips and the airfield
```

## How it works

`objects/roads.json.gz` holds `roadSegments`, each an `id`, a `roadClass` and `points`, pairs of
points across the road. `parse_roads_payload` keeps a segment of a known class whose points are
finite pairs; `extract_road_centerline` runs its centreline through the pairs' midpoints (skipping
one within `CENTERLINE_DEDUPE_M`, 0.05 m, of the last) and takes the median pair width, falling
back to the class's width outside 0.3–40 m. The archive `roads/road_network.rkyv` holds the same
segments already centred: `road_network_from_bytes` aligns, validates and version-checks it and
names each class code through `road_class_name` (`road_class.rs`, the closed class table whose
index plus one is the wire byte), refusing a code it cannot name. Everon
ships both files.

| Class | Fallback width | Centreline | Visible from zoom |
|---|---|---|---|
| `highway_paved` | 4 m | solid, `#c8c8c8` | −6 |
| `road_paved` | 2.5 m | solid, `#a0a0a0` | −6 |
| `road_dirt` | 2 m | dashed, `#8b6914` | −2 |
| `track` | 1.5 m | dashed, `#6b5010` | −2 |
| `path` | 1 m | dashed, `#5a4a3a` | 4 |
| `runway` | 20 m | solid, white | −6 |

`compose_road_segment` expands a centreline into a triangle strip with mitred joins and round caps:
a near-black casing `ROAD_CASING_FACTOR` (1.4) times the width, then the centreline in the class
colour, in 8 m dashes with 6 m gaps for a dashed class; with the airfield styling on, a runway
draws 20 m wide in grey over a grey-green casing. `compose_roads_mesh` packs every road visible at
the zoom as `[x, y, r, g, b, a]` in world metres, and `road_class_signature` sets one bit per zoom
gate so a caller rebuilds only when the visible classes change.

`compute_airfield_bbox` joins the runways' boxes and widens them by `AIRFIELD_BBOX_MARGIN_M`
(30 m). `build_airfield_apron_mesh` fills the vector-grid cells in that box whose 5 × 5
neighbourhood varies by less than `APRON_FLATNESS_SIGMA_M` (0.3 m) and whose height is within
`APRON_ELEV_TOLERANCE_M` (0.5 m) of those flat cells' mean; `hangar` and `tower` buildings show
only inside the box (`is_airfield_structure_class`). The cartographic strips run along a
footprint's long axis: fences at `FENCE_STRIP_WIDTH_M` (0.35 m), piers up to
`PIER_STRIP_MAX_WIDTH_M` (6 m) wide, bridge rails up to `BRIDGE_RAILING_RADIUS_M` (8 m) off the
axis, and none narrower than `STRIP_MIN_PX` (1.5) pixels.

## Getting started

Run from the repository root:

```bash
cargo test -p road_network   # network, styling, strip and airfield tests
```

## Public surface

- `network`: `RoadSegment` (its id a `RoadSegmentId`), `parse_roads_payload`,
  `extract_road_centerline`, `road_network_from_bytes` and `road_style_width`.
- `road_class`: `ROAD_CLASSES`, `road_class_code` and `road_class_name`.
- `styling`: `RoadStyle`, `road_style`, the zoom gates, `road_class_signature`, `StripVertex` and
  the polyline strip expansion.
- `mesh`: `RoadMeshGpu`, `RoadInput` and `compose_roads_mesh`.
- `cartographic_strip`: the fence, pier and bridge-rail strips.
- `airfield`: `compute_airfield_bbox`, `build_airfield_apron_mesh` and the structure gate.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `world_file_formats` (the road archive, its codec and `RoadSegmentId`);
  `prefab_catalog` (footprint corners); `terrain_elevation` (the vector grid the apron reads);
  `render_primitives` (the apron's triangulation and mesh); `map_coordinates` (the box type);
  `rkyv`, `serde_json` and `thiserror`.
- Used by:
  - the streaming crates `map_asset_loading` and `chunk_draw_buffers`: the loaders read the
    network and build the road mesh and apron, the strip and glyph packers draw the strips and
    gate airfield structures, and the toggles recompute the box;
  - `place_names`, whose road labels follow `RoadSegment`s and code their gate class with
    `road_class_code`;
  - the debug benches in `apps/frontend/src/workspaces/debug/`, which stroke lines with
    `expand_polyline_strip`;
  - `tools/developer_tools/src/`, whose world export writes the archive from the JSON (its class
    bytes through `road_class_code`) and whose label pipeline and checks read the network.
- Rules: the archive reader refuses a class code it cannot name, another schema version and
  corrupt bytes (`unnameable_class_code_is_an_error_not_a_vanished_road`,
  `wrong_schema_version_is_refused_even_though_the_bytes_validate`, `corrupt_buffers_are_refused`
  in `src/tests/network_tests.rs`); the width is the median across the pairs
  (`width_is_median_across_cross_edges`); the visibility signature changes exactly at a class gate
  (`road_signature_matches_visibility_and_boundaries` in `src/tests/styling_tests.rs`); terrain
  tier 3 (`cargo xtask verify crate-tiers`).
