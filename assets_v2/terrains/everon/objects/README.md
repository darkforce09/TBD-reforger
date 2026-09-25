# Everon world objects

Everything placed on Everon, as the world export writes it: the object chunks and vegetation
density tiles in their child folders, and beside them the prefab catalogue, the land-cover regions,
the road network and the object census, most in a gzip JSON form and an rkyv archive form. The map
engine streams them into the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
map.

## Contents

```text
assets_v2/terrains/everon/objects/
├── chunks/                 the placed objects, a `TBDC` binary and a gzip JSON file per 512 m cell
├── density/                the tree and rock density at 8 m, one `TBDD` tile per 512 m cell
├── forest-regions.json.gz  land-cover region outlines with their tree counts and density
├── forest-regions.rkyv     the same regions as a `ForestRegionsArchive`
├── prefabs.json.gz         the prefab catalogue: one entry per prefab the chunks place, by index
├── prefabs.rkyv            the same catalogue as a `PrefabCatalogArchive`, with the census inside
├── roads.json.gz           the road segments with their class and edge point pairs
├── type-inventory.json     the census of prefab types and instances by kind and classification
└── type-inventory.rkyv     the same census as a standalone `TypeInventory`
```

## How it works

`world build-objects` writes the whole folder from one staged
[Workbench](/documentation_v2/glossary.md#workbench) export: it partitions the objects into
chunks, derives the density tiles and the forest regions from the trees and rocks (on the
vegetation phases), writes the catalogue and the census, and writes each archive by reading back
the JSON it has just written through the map engine's own parsers, so the two forms decode to the
same rows. `world build-roads` writes `roads.json.gz` and its archive, which sits in
`assets_v2/terrains/everon/roads/`. The terrain manifest's `objects` block names the JSON paths
(`prefabsPath`, `chunksPath`, `roadsPath`, `regionsPath`, `typeInventoryPath`, `densityPath`) and
its `objects.binary` block the archives.

At boot the map engine's world loader reads the manifest, loads the prefab catalogue, the roads
and the regions from their archives when `objects.binary` names them and from the gzip JSON
otherwise, telling the two apart by their first bytes, then reads the chunk index and streams the
chunks the viewport needs. The vegetation loader fetches all 625 density tiles. The census files
are read by tools and gates, not by the browser, and `prefabs.rkyv` carries its own copy of the
census.

| Data | JSON form | Archive form | Schema |
|---|---|---|---|
| Prefab catalogue | `prefabs.json.gz` | `prefabs.rkyv` | `map-object-catalog.schema.json`, `map-object-prefab.schema.json` |
| Land-cover regions | `forest-regions.json.gz` | `forest-regions.rkyv` | `map-object-region.schema.json` |
| Census | `type-inventory.json` | `type-inventory.rkyv` | `map-object-type-inventory.schema.json` |
| Roads | `roads.json.gz` | `../roads/road_network.rkyv` | `map-object-roads.schema.json` |

## Format

- Encoding: the `.json.gz` files are gzip-compressed UTF-8 JSON and `type-inventory.json` is plain
  JSON, all plain git blobs; the `.rkyv` files are little-endian rkyv archives validated whole on
  read, stored in Git LFS (`.gitattributes`: `assets_v2/terrains/**/*.rkyv`). The catalogue holds
  1,623 prefabs, each with its resource name, kind, class, classification, spatial box and
  gameplay traits; its entry `i` is the prefab that `prefabId` `i` in a chunk row names.
- Schema: the JSON files follow the schemas in `contracts_v2/definitions/` named in the table; the
  archive types are in `apps/website/map-engine/src/io/archives/` (`prefabs.rs`, `forest.rs`).
  The prefab and region archives carry archive schema version 1 and refuse another;
  `type-inventory.rkyv` carries none, and the census inside `prefabs.rkyv` is the checked copy.
- Adding a file: never by hand. `cargo xtask map export-terrain everon --phase <phase>` rewrites
  the folder; `cargo run -p developer-tools --bin world -- reclassify --terrain everon --write`
  rewrites the catalogue's classification from `contracts_v2/rules/prefab-classify.json` without
  a new export, and without `--write` reports drift; `cargo xtask schema type-inventory` and
  `cargo xtask schema map-object-golden` check the census and catalogue invariants.

## Producers and consumers

- Producers: the world export pipeline in `tools_v2/developer-tools/src/world_export_pipeline/`:
  `world build-objects` (`chunk_partitioner/`, `catalog_emit.rs`, `binary_emit.rs`,
  `forest_contours.rs`), `world build-roads` (`roads_emit.rs`), `world reclassify` and
  `world redensify`; `cargo xtask map export-terrain` runs the first two.
- Consumers:
  - the map engine's world loader, residency and store
    (`apps/website/map-engine/src/streaming/loaders/`), the prefab and region readers under
    `apps/website/map-engine/src/world/environment/`, and the road reader in
    `apps/website/map-engine/src/world/terrain/roads/`, over `/map-assets/everon/objects/…`;
  - the developer tools: the blueprint compiler reads the catalogue and chunks for the prefab
    library, `map labels-rkyv` reads `roads.json.gz`, and the export validation and mathematical
    verification read the whole folder;
  - `cargo xtask schema type-inventory`, and the map engine's loader, prefab and region tests and
    the world export's tests, which read the committed files.

## Boundaries

- Depends on: the classification rules in `contracts_v2/rules/prefab-classify.json`, which set
  each prefab's kind, class and glyph; the terrain manifest's `objects` block.
- Used by: the map engine, the developer tools, the xtask schema gates and the tests listed above.
- Rules: each JSON file and its archive change together and decode to the same rows
  (`everon_archive_lane_builds_the_same_residency_as_the_json_lane`); the catalogue's order is the
  `prefabId` numbering that every chunk row and descriptor file uses, so a re-export that reorders
  it rewrites all of them; a catalogue built for another terrain or with a repeated prefab id is
  refused.

## Related documentation

- [World asset loaders](/apps/website/map-engine/src/streaming/loaders/README.md) — how these
  files are fetched, parsed and made resident.
- [Map data archives](/apps/website/map-engine/src/io/archives/README.md) — the rkyv archives.
