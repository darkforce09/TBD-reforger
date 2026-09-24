# Road name labels under one path

`mod.rs` re-exports the road name label items of the three `route_*` files in the parent
folder (the road class table, the placement and declutter pipeline, the `road-names.json` model
with its archive lane, and the polyline geometry) and mounts the unit tests of those items.

## Contents

```text
apps/website/map-engine/src/world/environment/locations/routes/
├── mod.rs  the module tree; re-exports the road label items of the parent's `route_*` files
└── tests/  unit tests for road label placement, declutter, class codes and the archive lane
```

## Boundaries

- Depends on: `route_placement`, `route_labels` and `route_geometry` in
  `crate::world::environment::locations`; the tests also use `crate::world::terrain::roads`
  (`RoadSegment`) and `crate::io::archives` (the map labels archive).
- Used by: nothing outside the folder; callers import the same items from the `route_*` modules.
- Rules: the module compiles only with the `streaming` feature and defines no item of its own. Its
  tests hold the road label rules: at most `ROAD_NAME_MAX_ON_SCREEN` (24) labels survive the
  declutter (`cap_at_24`), each within `ROAD_NAME_PERP_TOL_M` (12 m) of its segment
  (`placement_within_perp_tol`); labels baked into the archive draw as the JSON path draws them,
  to f32 precision, at every zoom (`archive_road_labels_match_the_json_draw_set_at_every_zoom`); a
  zoom floor no road class can carry is refused rather than rounded
  (`an_unrepresentable_override_is_refused_rather_than_rounded`); an unknown class code is
  rejected (`class_codes_round_trip_and_reject_the_unknown`).
