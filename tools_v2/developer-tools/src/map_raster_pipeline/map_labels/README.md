# Map label exporters

The two JSON label sets a terrain commits: `locations.json`, the town, locality, airport and peak
names read from the [Workbench](/documentation_v2/glossary.md#workbench) world export, and
`height-labels.json`, the spot heights found in the terrain elevation model. These files are the
submodules `tools_v2/developer-tools/src/map_raster_pipeline/map_labels.rs` declares; it holds the
required Everon towns and the locality thresholds, and re-exports the entry points. The binary label
archive built from these files comes from the sibling `map_label_archives.rs`.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/map_labels/
├── export_height_labels.rs  `export-height-labels`: elevation-model peaks and named heights
└── importance_by_name.rs    `export-locations`: rows from the world export, importances, and gates G3–G7
```

## How it works

```text
assets_v2/scratch/<terrain>/export/raw-entities.jsonl (or --src)
  ─▶ export_locations_from_jsonl ─▶ verify_locations_gates ─▶ assets_v2/terrains/<terrain>/locations.json
assets_v2/terrains/<terrain>/manifest.json + dem/everon-dem-16bit.png + locations.json
  ─▶ export_height_labels ─▶ assets_v2/terrains/<terrain>/height-labels.json
```

- `export_locations` keeps the rows whose prefab sits under `World/Locations/`: each named location
  becomes a town, a locality (a sawmill, farm, quarry or mine inside a town name, at importance 0.4)
  or an airport, and hills and peaks keep their kind. It adds the four Everon places the export
  lacks (the `cfgworld_supplement`), gives each row an id, a kind and an importance from
  `importance_by_name`, and sorts by name. The gates then require at least ten rows and the seven
  required Everon towns, and reject short or placeholder names, sub-features tagged as towns and
  localities above importance 0.45; any failure exits 1 without writing. `--dry-run` prints the
  first five rows instead.
- `export_height_labels` decodes the elevation model with the manifest's scaling, finds peaks with
  `website_map_engine::world::environment::locations::peaks::find_peaks`, samples the named peaks
  and hills of `locations.json`, drops duplicates within 200 m, keeps what `declutter_height_labels`
  draws, and refuses to write an empty set.

## Boundaries

- Depends on: `website_map_engine::world::environment::locations::peaks` and
  `website_map_engine::world::terrain::dem` for the elevation model and peak rules;
  `crate::world_export_pipeline::json_number_formatting` for the number spelling;
  `crate::repository_layout` and `crate::browser_testing::server::repo_root` for the folders.
- Used by: `tools_v2/developer-tools/src/map_raster_pipeline/cli.rs` (`export-locations`,
  `export-height-labels`); `map labels-rkyv` reads both outputs; `cargo xtask schema locations` and
  `cargo xtask schema height-labels` check the committed files.
- Rules: an empty height-label set is never written (`height_labels_refuse_empty_contract`); the
  location gates G3–G7 hold before `locations.json` is overwritten; `export-height-labels` reads
  `dem/everon-dem-16bit.png` whatever the terrain, so only Everon exports height labels as the code
  stands.
