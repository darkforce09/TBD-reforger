# Map label and elevation gates

The gate bodies behind the `labels` module: they check a terrain's height labels, locations, town
labels and road names against the map engine's own placement and declutter code, and its
elevation model against the surface heights sampled in the game engine.

## Contents

```text
tools/developer_tools/src/map_verification/labels/
├── read_json.rs          the height-label and location gates, and the JSON reader both share
├── terrain_alignment.rs  the elevation model against the surface-height anchors, per anchor
└── town_labels.rs        the town-label and road-name gates, with PNG and rounding helpers
```

## How it works

`tools/developer_tools/src/map_verification/labels.rs` declares the three files with `#[path]`,
holds the shared constants (`REQUIRED_EVERON_TOWNS`, `MAJOR_EVERON_ROADS`, the peak cap
`PEAK_LABEL_MAX`) and re-exports the five gate functions. Each takes the checkout root and a
terrain id, reads from `assets/terrains/<terrain>/`, prints one `PASS` or `FAIL` line per
check and returns the exit code: 0 when every check held, 1 otherwise.

| Function | File | Reads | Checks |
|---|---|---|---|
| `height_labels` | `read_json.rs` | `height-labels.json`, `locations.json`, the DEM the manifest names | G2 to G6 floor, dedupe, named merge, declutter and completeness; each label within 0.5 m of the DEM (skipped when the DEM is absent) |
| `locations` | `read_json.rs` | `locations.json`, `locations.schema.json` | G2 the schema, then G3 to G7: count, required towns, row quality, placeholder names, kind hygiene |
| `town_labels` | `town_labels.rs` | `locations.json` | G1 to G5 at a zoom: kinds drawn, required towns drawn, the declutter invariant, name provenance, fade and zoom band |
| `road_names` | `town_labels.rs` | `road-names.json`, `objects/roads.json.gz` | G3 to G7 at a zoom: major roads drawn, name length, placement within tolerance of the segment, declutter distance, on-screen cap |
| `terrain_alignment` | `terrain_alignment.rs` | `manifest.json`, `anchors/verification.json`, the DEM | the manifest and anchors schemas, the DEM size, and every anchor within the anchors file's `thresholdM` of the DEM |

`terrain_alignment` falls back to `anchors/verification.example.json` when the real anchors file
is absent, and passes on the schema alone when the manifest's DEM is a stub (zero width or
height). With `strict` set, both cases fail, as does a file with fewer than 10 anchors.

## Boundaries

- Depends on: elevation decoding and sampling (`terrain_elevation`), peak declutter
  (`place_names::peaks`), town-label importance (`label_layout::importance`), road-name
  placement (`place_names::route_placement`, `route_geometry`, `route_labels`) and the road
  network (`road_network::network`); the schemas in `contracts/definitions/`;
  the `repository_layout` crate for every path.
- Used by: `cargo xtask schema height-labels`, `locations`, `town-labels`, `road-names` and
  `terrain-alignment`, through `tools/commands/ci_task_catalog/src/map_asset_checks/mod.rs`; the CI tasks
  `schema-validate` (height labels), `verify-terrain` and `verify-terrain-strict` (alignment)
  in `tools/commands/ci_task_catalog/src/task_definitions.rs`.
- Rules: the gates run the map engine's placement and declutter functions rather than copies of
  them, so a gate and the map cannot disagree; `--strict` alignment refuses the example anchors
  and a stub DEM (`cargo xtask schema terrain-alignment --terrain everon --strict`).

## Related documentation

- [Everon anchors](/assets/terrains/everon/anchors/README.md) — the anchor files the alignment
  gate reads.
- [Everon elevation model](/assets/terrains/everon/dem/README.md) — the DEM these gates sample.
