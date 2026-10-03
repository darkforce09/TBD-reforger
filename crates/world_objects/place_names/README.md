# Place names

The `place_names` crate: the names the 2D map writes on a terrain. Spot heights on the hilltops,
town and place names, and road names placed along their roads; which of them show at each zoom so
that none crowd another; the labels archive that carries all three; and the glyph instances the
kept labels draw as. The browser loader that fetches the sources and uploads the label lanes stays
in the map engine.

## Contents

```text
crates/world_objects/place_names/
├── Cargo.toml  the package: `label_layout`, `terrain_elevation`, `road_network`, layout tier 4
└── src/        spot heights, town labels, road name placement, the archive lanes, glyph packing
```

## How it works

```text
manifest.json "labels" block
  ├─ names the archive ─► map_labels.rkyv ─ map_labels_from_bytes ─► towns, spot heights,
  │                                                    placed road name candidates
  └─ names none ────────► locations.json, road-names.json ─ parse_* ─► towns, curated road list
DEM raster ─ find_peaks ─► spot heights

per zoom band:
  road names ─► build_road_label_draw_set (JSON) or ..._from_archive ─► pack_road_label_bytes
  towns, spot heights ─► declutter ─► LabelSpec ─► pack_town_label_bytes, pack_height_label_glyphs
```

The sources sit in the terrain's asset folder (`assets/terrains/everon/` for Everon, served under
`/map-assets/`). `find_peaks` keeps each land cell that no cell of its `PEAK_WINDOW_PX` (9 px)
window exceeds, that rises `PEAK_PROMINENCE_M` (15 m) above the window's lowest cell and that
reaches `PEAK_MIN_VALUE_M` (80 m), plus the terrain's highest point. Spot heights declutter
highest first, `HEIGHT_LABEL_SCREEN_PITCH_PX · 2^−zoom` metres apart (a constant 150 px on
screen), at most `PEAK_LABEL_MAX` (48), and only between zoom −2 and 3.

Road names: `place_road_labels` anchors each curated name at the middle of each of its road
segments (at a quarter, a half and three quarters of a segment longer than
`ROAD_NAME_LONG_SEGMENT_M`, 3 km), `ROAD_NAME_OFFSET_M` (6 m) left of the centreline and turned
upright; `declutter_road_labels` keeps them in priority order (the road class, plus 80 for a
curated zoom floor) while each stays `ROAD_NAME_DECLUTTER_BASE_M · 2^−zoom` from every kept one,
at most `ROAD_NAME_MAX_ON_SCREEN` (24). Highways and runways show from zoom 0 and paved roads from
zoom 1; other classes never show unless a name sets its own floor. The archive stores the anchors
already placed and sorted, each name's floor carried as the code of a road class with that floor,
so drawing them only filters and declutters.

Each kind converts to `label_layout`'s `LabelSpec` here: a spot height's importance is its
elevation and its text the value (or `name - value m`); a town's importance is its 0–1 importance
scaled to 10 000 and its text the trimmed name.

## Getting started

Run from the repository root:

```bash
cargo test -p place_names   # peaks, towns, archive lanes and road name placement tests
```

The Everon spot-height case reads `assets/terrains/everon/dem/everon-dem-16bit.png`, a Git LFS
file; a checkout without it fails that case with the `git lfs pull` command that fetches it.

## Public surface

- `peaks`: `HeightLabel`, `find_peaks`, `declutter_height_labels`, `height_labels_to_specs` and the
  spot-height constants.
- `towns`: `parse_locations_json`, `parse_height_labels_json`, `locations_to_label_specs`, the
  town and height lane conversions, `MapLabels` and `map_labels_from_bytes`.
- `route_placement`: `RoadLabelPlacement`, `place_road_labels`, `declutter_road_labels`,
  `build_road_label_draw_set`, the class priorities and floors and the road name constants.
- `route_labels`: `RoadNamesFile`, `RoadNameEntry`, `parse_road_names_json`,
  `road_names_to_archive`, `road_names_from_archive`, `build_road_label_draw_set_from_archive`.
- `route_geometry`: the polyline length, tangent, upright angle and distance functions.
- `label_packing`: `pack_height_label_glyphs`, `pack_town_label_glyphs`, `pack_town_label_bytes`
  and `pack_road_label_bytes`.
- `RoadNameId` (`road_name_ids`), `Error` and `Result` (`error`), and `prelude`, which re-exports
  the main items above.

## Boundaries

- Depends on: `label_layout` (`LabelSpec`, `LocationLabel`, the town declutter and fade, the
  glyph packing); `terrain_elevation` (`DemManifest`, the PNG decode of the Everon case);
  `road_network` (`RoadSegment`, the road class codec); `world_file_formats` (the labels archive,
  `RoadSegmentId`, `TerrainId`); `render_primitives` (glyph metrics and instance packing);
  `newtype_ids`, `rkyv`, `serde`, `serde_json` and `thiserror`.
- Used by:
  - the map engine (`legacy/map_engine`), whose label loader (`world/environment/locations/`)
    fetches the sources and uploads the lanes;
  - the developer tools: the labels archive and the height label export in
    `tools/developer_tools/src/map_raster_pipeline/` and the label checks in
    `tools/developer_tools/src/map_verification/`.
- Rules:
  - the labels archive reads exactly the JSON it was baked from, including an empty lane, at any
    buffer offset, and a truncated file or another schema version is an error
    (`towns_round_trip_through_the_archive`, `a_truncated_file_is_an_error_not_a_wild_read`,
    `a_future_schema_version_is_refused` in `src/tests/towns_tests.rs`);
  - the spot-height declutter keeps its spacing, cap and constant on-screen density
    (`declutter_respects_sep_and_cap`, `screen_space_density_holds_across_zoom` in
    `src/tests/peaks_tests.rs`);
  - road names stay within `ROAD_NAME_PERP_TOL_M` (12 m) of their segment, at most 24 survive, the
    archive draws as the JSON does at every zoom, and a floor no road class carries is refused
    (`placement_within_perp_tol`, `cap_at_24`,
    `archive_road_labels_match_the_json_draw_set_at_every_zoom`,
    `an_unrepresentable_override_is_refused_rather_than_rounded` in
    `src/tests/route_placement_tests.rs`);
  - no module fetches, uploads or touches a browser API.

## Related documentation

- [Locations schema](/contracts/definitions/locations.schema.json) — the rows of
  `locations.json`.
- [Height labels schema](/contracts/definitions/height-labels.schema.json) — the rows of
  `height-labels.json`, the source of the archive's height lane.
