# Map data archives

The rkyv archives a terrain's structured map data ships in (roads, map labels, water vectors,
the prefab catalogue and type inventory, forest regions, building blueprints and the satellite
index), the one validating reader and the one writer for all of them, and `BinaryError`, the error
every binary format in this crate reports.

## Contents

```text
crates/world_formats/world_file_formats/src/archives/
├── blueprints.rs  `BuildingBlueprintArchive`: occluder descriptors, the BLAS index, building levels
├── codec.rs       `BinaryError`, `access_checked` (the validating reader) and `to_bytes`
├── forest.rs      `ForestRegionsArchive`: land-cover polygons with tree counts and density
├── labels.rs      `MapLabelsArchive`: town, height and road name labels
├── mod.rs         the module tree and the round-trip and wire identity test modules
├── prefabs.rs     `PrefabCatalogArchive`: prefab entries and the `TypeInventory` census
├── roads.rs       `RoadNetworkArchive`: road segments with class, width and centreline
├── satellite.rs   `TbdSatIndexV2`: the satellite tile pyramid's index inside a `TBDS` container
├── tests/         unit tests: how `BinaryError` renders, every archive's round trip and refusals, identifier byte identity
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
`TbdSatIndexV2` carry none. Every identifier field of a record is an identifier type from
`crate::ids` (`PrefabId`, `TerrainId`, `RoadSegmentId`, `ForestRegionId`, `WaterFeatureId`, and
the building element ids), which archives exactly as the bare `u32` or `String` it wraps; a reader
reads an archived identifier through `get`, `as_str` or `to_native`.

| Archive | File under a terrain's folder | Writer | Reader |
|---|---|---|---|
| `RoadNetworkArchive` | `roads/road_network.rkyv` | `tools/map_assets/world_export_pipeline/src/roads_emit.rs` | `road_network::network` |
| `MapLabelsArchive` | `locations/map_labels.rkyv` | `tools/map_assets/map_raster_pipeline/src/map_label_archives.rs` | `place_names::towns`, `place_names::route_labels` |
| `WaterVectorsArchive` | `water/water_vectors.rkyv` | `tools/map_assets/map_raster_pipeline/src/inland_water_archive.rs` | `water_bodies::vectors` |
| `PrefabCatalogArchive` | `objects/prefabs.rkyv` | `tools/map_assets/world_export_pipeline/src/catalog_emit.rs` | `prefab_catalog::prefab_rows` |
| `TypeInventory` | `objects/type-inventory.rkyv` | `tools/map_assets/world_export_pipeline/src/catalog_emit.rs` | none; the same census is embedded in the prefab catalogue |
| `ForestRegionsArchive` | `objects/forest-regions.rkyv` | `tools/map_assets/world_export_pipeline/src/catalog_emit.rs` | `vegetation::regions` |
| `BuildingBlueprintArchive` | `prefabs/building_blueprints.rkyv` | `tools/map_assets/blueprint_compiler/src/archive_emission/archive_writer.rs` | `world_line_of_sight::occluder_library::building_archive` |
| `TbdSatIndexV2` | inside `satellite/{terrain}-sat.tbd-sat` | `tools/map_assets/map_raster_pipeline/src/satellite_archive_container.rs` | `satellite_imagery::header`, `satellite_imagery::archive` |

`BinaryError` covers every way a buffer can be wrong: `Truncated`, `BadMagic`,
`UnsupportedVersion`, `Misaligned` (recoverable by copying into an aligned buffer),
`LengthMismatch` and `Archive`. It is `PartialEq` and holds no rkyv type, and it prints a printable
magic as a byte string (`b"TBDC"`).

## Public surface

- `codec`: `BinaryError`, `access_checked` and `to_bytes`, for every reader of the formats in
  this crate and for the developer tools' writers.
- `version::ARCHIVE_SCHEMA_VERSION`, for the writers.
- The archive types in `roads`, `labels`, `water`, `prefabs`, `forest`, `blueprints` and
  `satellite`, for the readers and writers in the table.

## Boundaries

- Depends on: `rkyv` (with `bytecheck` and `little_endian`, set in the crate's `Cargo.toml`), and
  `crate::ids` for the identifier fields.
- Used by:
  - `crate::containers` and `crate::pod`, which report `BinaryError`;
  - the readers in the table, and `world_chunks`, `world_store` and `terrain_elevation::raw`,
    which report `BinaryError`;
  - the developer tools' writers in the table and their tests.
- Rules:
  - an archive is read only through `access_checked`, never unchecked;
  - a change to a field's meaning raises `ARCHIVE_SCHEMA_VERSION` rather than reusing the field
    (the doc comment in `version.rs`), and every archive committed under `assets/terrains/` is
    then written again, since the readers accept only the current version;
  - each archive round-trips and refuses corrupted bytes and the bytes of another archive type
    (`tests/archive_round_trip_tests.rs`);
  - an identifier type leaves the archive bytes those of its bare primitive
    (`tests/archive_wire_identity_tests.rs`).

## Related documentation

- [Terrain assets](/assets/terrains/README.md) — the served terrain tree the archives sit in.
- [Satellite basemap loader](/crates/streaming/map_asset_loading/src/terrain/satellite_quadtree/README.md)
  — the satellite loader that reads the index.
