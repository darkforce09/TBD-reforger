# Everon map labels archive

Everon's cartographic labels in one binary archive: the town and place names, the spot heights and
the road names already placed along their roads. The map engine reads it at boot to label the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map.

## Contents

```text
assets_v2/terrains/everon/locations/
└── map_labels.rkyv  the town, spot-height and placed road-name labels as one `MapLabelsArchive`
```

## Format

- Encoding: an rkyv archive, little-endian and validated whole on read, of
  `MapLabelsArchive` (`apps/website/map-engine/src/io/archives/labels.rs`) at archive schema
  version 1: a lane of towns, a lane of spot heights, and a lane of road-name anchors, each with
  its position, angle and road class baked in. Stored in Git LFS (`.gitattributes`:
  `assets_v2/terrains/**/*.rkyv`).
- Schema: no JSON Schema; the Rust type is the contract. The manifest's `labels` block names the
  file with the encoding `rkyv-map-labels-v1`
  (`contracts_v2/definitions/terrain-manifest.schema.json`).
- Adding a file: never by hand. `cargo run -p developer-tools --bin map -- labels-rkyv --terrain
  everon` rebuilds it from the three label JSON files in the parent folder and the road
  centrelines in `objects/roads.json.gz`.

## Producers and consumers

- Producers: `map labels-rkyv`
  (`tools_v2/developer-tools/src/map_raster_pipeline/map_label_archives.rs`), which places the
  road names with the same code the browser would, reads the archive back before writing it, and
  refuses to write one with every lane empty or a road-name list without its road file.
- Consumers:
  - the map engine's label host (`apps/website/map-engine/src/world/environment/locations/`),
    which fetches `/map-assets/everon/locations/map_labels.rkyv` when the manifest's `labels`
    block names it, and takes the towns and road names from it (spot heights it finds on the
    elevation model itself);
  - the map raster pipeline's label archive tests, which rebuild the archive from the committed
    sources.

## Boundaries

- Depends on: `assets_v2/terrains/everon/locations.json`,
  `assets_v2/terrains/everon/height-labels.json`, `assets_v2/terrains/everon/road-names.json` and
  `assets_v2/terrains/everon/objects/roads.json.gz`, the sources it is derived from.
- Used by: the map engine's label host and the tests listed above.
- Rules: the JSON files stay the source of truth, so an edit to one of them is followed by a
  rebuild of this archive in the same change; while the manifest names the archive, the browser
  never reads the JSON label files.

## Related documentation

- [Map labels: towns, roads and heights](/apps/website/map-engine/src/world/environment/locations/README.md)
  — how the labels are chosen and placed.
- [Map data archives](/apps/website/map-engine/src/io/archives/README.md) — the rkyv archives and
  their validating reader.
