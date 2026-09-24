# Map labels: towns, roads and heights

The cartographic labels of a terrain: town and place names, road names placed along their roads,
and spot heights on the hilltops. The folder reads their sources, a binary labels archive or its
JSON files, decides which labels show at each zoom so that none crowd another, and in the browser
hands the chosen labels to the text lanes.

## Contents

```text
apps/website/map-engine/src/world/environment/locations/
├── loader.rs           `LabelHost`: loads the label sources in the browser and uploads the lanes
├── mod.rs              the module tree
├── peaks.rs            spot heights: peak finding on the elevation model, declutter, text specs
├── route_geometry.rs   polyline length, tangents, anchor fractions and distances for road names
├── route_labels.rs     the `road-names.json` model and the labels archive's `road_names` lane
├── route_placement.rs  road classes, and road name placement and declutter
├── routes/             the road name items under one path, and their unit tests
├── tests/              unit tests for spot heights and the town and archive readers
└── towns.rs            the label JSON parsers, the archive's town and height lanes, its reader
```

## How it works

```text
manifest.json "labels" block
  ├─ names the archive ─► locations/map_labels.rkyv ─ map_labels_from_bytes ─► towns,
  │                                                            placed road name candidates
  └─ names none ────────► locations.json, road-names.json ─ parse_* ─────────► towns,
                                                                     curated road list
DEM raster ─ find_peaks ─► spot heights

LabelHost::push(zoom, layer toggles)
  ├─ road names ─► placed and decluttered by route_placement or route_labels
  ├─ towns, spot heights ─► decluttered by the text packers of crate::overlay::symbology
  │                          (the heights through declutter_height_labels)
  └─► the town, road and height label lanes of the engine
```

The sources sit in the terrain's asset folder (`assets_v2/terrains/everon/` for Everon, served
under `/map-assets/`). `LabelHost::init` reads its `manifest.json`: when the `labels` block names
an archive of encoding `rkyv-map-labels-v1`, the towns and the baked road name candidates come
from that archive, and stay empty if it fails to load; when the manifest names none, they come
from `locations.json` and `road-names.json`. Spot heights come from the elevation model itself:
`find_peaks` keeps each land cell that no cell of its `PEAK_WINDOW_PX` (9 px) window exceeds,
that rises `PEAK_PROMINENCE_M` (15 m) above the window's lowest cell and that reaches
`PEAK_MIN_VALUE_M` (80 m), plus the terrain's highest point. `push` recomputes the lanes only
when the half-zoom band or a layer toggle changes.

Road names: `place_road_labels` anchors each curated name at the middle of each of its road
segments (at a quarter, a half and three quarters of a segment longer than
`ROAD_NAME_LONG_SEGMENT_M`, 3 km), `ROAD_NAME_OFFSET_M` (6 m) left of the centreline and turned
upright; `declutter_road_labels` keeps them in priority order (the road class, plus 80 for a
curated zoom floor) while each stays `ROAD_NAME_DECLUTTER_BASE_M · 2^−zoom` from every kept one,
at most `ROAD_NAME_MAX_ON_SCREEN` (24). Highways and runways show from zoom 0 and paved roads from
zoom 1; other classes never show unless a name sets its own floor. The archive stores the anchors
already placed and sorted, so the browser only filters and declutters them. Spot heights
declutter the same way, highest first, `HEIGHT_LABEL_SCREEN_PITCH_PX · 2^−zoom` metres apart (a
constant 150 px on screen), at most `PEAK_LABEL_MAX` (48), and only between zoom −2 and 3.

## Public surface

- `loader::LabelHost`: `init` and `push`, owned by the streaming host state.
- `towns`: the JSON parsers and the archive lane conversions, for the label archive tooling;
  `locations_to_label_specs`, for the text packer.
- `peaks`: `find_peaks`, `declutter_height_labels`, `height_labels_to_specs` and the height label
  constants, for the text packer and the height label export.
- `route_placement`, `route_labels`, `route_geometry`: the road class table and codes (also used
  by the road network), the placement and declutter pipeline, and the archive lane conversions.

## Boundaries

- Depends on: `crate::world::terrain::dem` (`DemManifest`) and `crate::world::terrain::roads`
  (`RoadSegment`); `crate::io::archives` (`MapLabelsArchive`, `access_checked`);
  `crate::overlay::symbology` (`LocationLabel`, `LabelSpec`, the text packers); and, for
  `loader.rs`, `crate::streaming` (fetch, manifest parsing, boot progress, layer preferences) and
  `crate::frame` (`EngineHandle`).
- Used by:
  - `crate::streaming::host`, whose state owns the `LabelHost`;
  - `crate::overlay::symbology`, whose text packer and text metrics pack the three label lanes;
  - `crate::world::terrain::roads`, whose road network names classes with `road_class_name`;
  - the map tooling: the labels archive and the height label export in
    `tools_v2/developer-tools/src/map_raster_pipeline/`, the road export
    (`tools_v2/developer-tools/src/world_export_pipeline/roads_emit.rs`) and the label checks in
    `tools_v2/developer-tools/src/map_verification/`.
- Rules:
  - the labels archive reads exactly the JSON it was baked from, including an empty lane, at any
    buffer offset, and a truncated file or another schema version is an error
    (`towns_round_trip_through_the_archive`, `a_truncated_file_is_an_error_not_a_wild_read`,
    `a_future_schema_version_is_refused` in `tests/towns_tests.rs`);
  - the spot-height declutter keeps its spacing and cap and a constant on-screen density
    (`declutter_respects_sep_and_cap`, `screen_space_density_holds_across_zoom` in
    `tests/peaks_tests.rs`);
  - the road name rules are held by the tests in `routes/`; `loader.rs` compiles only for wasm32
    with the `render` feature, and the road and town modules only with `streaming`.

## Related documentation

- [Locations schema](/contracts_v2/definitions/locations.schema.json) — the rows of
  `locations.json`.
- [Height labels schema](/contracts_v2/definitions/height-labels.schema.json) — the rows of
  `height-labels.json`, the source of the archive's height lane.
