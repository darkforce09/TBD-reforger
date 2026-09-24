# Map data archives

The rkyv archives a terrain's structured map data ships in (roads, map labels, water vectors,
the prefab catalogue and type inventory, forest regions, building blueprints and the satellite
index), the one validating reader and the one writer for all of them, and `BinaryError`, the error
every binary format in `crate::io` reports.

## Contents

```text
apps/website/map-engine/src/io/archives/
├── blueprints.rs  `BuildingBlueprintArchive`: occluder descriptors, the BLAS index, building levels
├── codec.rs       `BinaryError`, `access_checked` (the validating reader) and `to_bytes`
├── forest.rs      `ForestRegionsArchive`: land-cover polygons with tree counts and density
├── labels.rs      `MapLabelsArchive`: town, height and road name labels
├── mod.rs         the module tree
├── models/        a flat re-export of every archive type, and the archives' round-trip tests
├── prefabs.rs     `PrefabCatalogArchive`: prefab entries and the `TypeInventory` census
├── roads.rs       `RoadNetworkArchive`: road segments with class, width and centreline
├── satellite.rs   `TbdSatIndexV2`: the satellite tile pyramid's index inside a `TBDS` container
├── tests/         unit tests for how `BinaryError` renders
├── version.rs     `ARCHIVE_SCHEMA_VERSION`, the contract version the archives carry
└── water.rs       `WaterVectorsArchive`: lakes, rivers and ponds
```

## How it works

Each archive is a plain Rust struct deriving rkyv's `Archive`, `Serialize` and `Deserialize`; the
crate builds rkyv with `bytecheck` and `little_endian`, so the bytes are the same on every target.
A writer calls `to_bytes`, and a reader calls `access_checked`, which validates the whole buffer
before handing back the archived view and reports a failure as `BinaryError::Archive` with the type
name. Nothing reads an archive unchecked. The roads, labels, water, prefab catalogue, forest and
blueprint archives carry a `schema_version` field, written as `ARCHIVE_SCHEMA_VERSION` (1), and
their readers refuse any other value with `UnsupportedVersion`; `TypeInventory` and
`TbdSatIndexV2` carry none.

| Archive | File under a terrain's folder | Writer | Reader in the map engine |
|---|---|---|---|
| `RoadNetworkArchive` | `roads/road_network.rkyv` | `tools_v2/developer-tools/src/world_export_pipeline/roads_emit.rs` | `crate::world::terrain::roads` |
| `MapLabelsArchive` | `locations/map_labels.rkyv` | `tools_v2/developer-tools/src/map_raster_pipeline/map_label_archives.rs` | `crate::world::environment::locations` |
| `WaterVectorsArchive` | `water/water_vectors.rkyv` | `tools_v2/developer-tools/src/map_raster_pipeline/inland_water_archive.rs` | `crate::world::terrain::water` |
| `PrefabCatalogArchive` | `objects/prefabs.rkyv` | `tools_v2/developer-tools/src/world_export_pipeline/catalog_emit.rs` | `crate::world::environment::buildings`, `crate::streaming::loaders` |
| `TypeInventory` | `objects/type-inventory.rkyv` | `tools_v2/developer-tools/src/world_export_pipeline/catalog_emit.rs` | none; the same census is embedded in the prefab catalogue |
| `ForestRegionsArchive` | `objects/forest-regions.rkyv` | `tools_v2/developer-tools/src/world_export_pipeline/catalog_emit.rs` | `crate::world::environment::vegetation` |
| `BuildingBlueprintArchive` | `prefabs/building_blueprints.rkyv` | `tools_v2/developer-tools/src/blueprint/archive_emission/archive_writer.rs` | `crate::world::architecture`, `crate::spatial::los::world` |
| `TbdSatIndexV2` | inside `satellite/{terrain}-sat.tbd-sat` | `tools_v2/developer-tools/src/map_raster_pipeline/satellite_archive_container.rs` | `crate::world::terrain::satellite` |

`BinaryError` covers every way a buffer can be wrong: `Truncated`, `BadMagic`,
`UnsupportedVersion`, `Misaligned` (recoverable by copying into an aligned buffer),
`LengthMismatch` and `Archive`. It is `PartialEq` and holds no rkyv type, and it prints a printable
magic as a byte string (`b"TBDC"`).

## Public surface

- `codec`: `BinaryError`, `access_checked` and `to_bytes`, for every reader of the formats in
  `crate::io` and for the developer tools' writers.
- `version::ARCHIVE_SCHEMA_VERSION`, for the writers.
- The archive types in `roads`, `labels`, `water`, `prefabs`, `forest`, `blueprints` and
  `satellite`, for the readers and writers in the table.

## Boundaries

- Depends on: `rkyv` (with `bytecheck` and `little_endian`, set in
  `apps/website/map-engine/Cargo.toml`).
- Used by:
  - `crate::io::containers` and `crate::io::pod`, which report `BinaryError`;
  - the readers in the table, and `crate::streaming::loaders` and
    `crate::world::terrain::dem`, which report `BinaryError`;
  - the developer tools' writers in the table and their tests.
- Rules:
  - an archive is read only through `access_checked`, never unchecked;
  - a change to a field's meaning raises `ARCHIVE_SCHEMA_VERSION` rather than reusing the field
    (the doc comment in `version.rs`), and every archive committed under `assets_v2/terrains/` is
    then written again, since the readers accept only the current version;
  - each archive round-trips and refuses corrupted bytes and the bytes of another archive type
    (`models/tests/cases_1.rs`); every `BinaryError` variant renders
    (`every_variant_renders_and_is_an_error` in `tests/codec_tests.rs`).

## Related documentation

- [Terrain assets](/assets_v2/terrains/README.md) — the served terrain tree the archives sit in.
- [Satellite terrain](/apps/website/map-engine/src/world/terrain/satellite/README.md) — the
  satellite streamer that reads the index.
