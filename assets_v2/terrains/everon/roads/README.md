# Everon road network archive

Everon's roads as one binary archive: every road, track, path and runway segment with its class,
width and centreline. The map engine draws the road layer of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map from it.

## Contents

```text
assets_v2/terrains/everon/roads/
└── road_network.rkyv  every road segment with its class, width and centreline, as one archive
```

## Format

- Encoding: an rkyv archive, little-endian and validated whole on read, of `RoadNetworkArchive`
  (`apps/website/map-engine/src/io/archives/roads.rs`) at archive schema version 1: the segments
  of `objects/roads.json.gz` with their centrelines already derived and each class as a code.
  About 435 KB, stored in Git LFS (`.gitattributes`: `assets_v2/terrains/**/*.rkyv`).
- Schema: no JSON Schema; the Rust type is the contract, and the gzip JSON twin follows
  `contracts_v2/definitions/map-object-roads.schema.json`. The manifest names the file in
  `objects.binary.roads`.
- Adding a file: never by hand. `cargo xtask map export-terrain everon` writes it with its JSON
  twin through `world build-roads`, which decodes the road topology from the game paks;
  `cargo run -p developer-tools --bin world -- roads-rkyv --terrain everon` re-emits it from the
  committed `objects/roads.json.gz` alone.

## Producers and consumers

- Producers: `world build-roads` and `world roads-rkyv`
  (`tools_v2/developer-tools/src/world_export_pipeline/roads_emit.rs`).
- Consumers:
  - the map engine's world loader, which fetches `/map-assets/everon/roads/road_network.rkyv`
    when the manifest's `objects.binary` block names it and the gzip JSON otherwise, and tells the
    two apart by their first bytes (`apps/website/map-engine/src/streaming/loaders/`); the reader
    is `road_network_from_bytes` in `apps/website/map-engine/src/world/terrain/roads/`;
  - `WorldStore`, the headless reader the developer tools use;
  - the world export's road emission tests, which rebuild the archive from the committed JSON.

## Boundaries

- Depends on: `assets_v2/terrains/everon/objects/roads.json.gz`, which holds the same segments;
  the road class names of the map engine's label placement, which the class codes index.
- Used by: the map engine and the developer tools listed above.
- Rules: the archive and `objects/roads.json.gz` change together and read as the same segments
  (`load_roads_sniffs_gzip_versus_rkyv`); a reader refuses another archive schema version or a
  class code it cannot name.

## Related documentation

- [Roads, runways and cartographic strips](/apps/website/map-engine/src/world/terrain/roads/README.md)
  — the road model, its classes and how it is drawn.
