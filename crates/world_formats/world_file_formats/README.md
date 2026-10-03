# World file formats

The `world_file_formats` crate: the on-disk formats of a terrain's served map data, each defined
once so the developer tools that write a file and the map engine that reads it in the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) compile against the same
layout. It holds the rkyv archives, the four fixed-header binary containers, the vegetation
density grid and the world object row, with the one writer and the one validating reader of each,
and the identifier types their records hold.

## Contents

```text
crates/world_formats/world_file_formats/
├── Cargo.toml  the package: `bytemuck`, `newtype_ids`, `rkyv` (`std`, `bytecheck`, `little_endian`), `thiserror`; layout tier 1
└── src/        the archives, containers, density, object row and identifier modules, the error and the prelude
```

## How it works

The fixed layouts (the container headers, the object row and the density header) are
`#[repr(C)]` `bytemuck::Pod` structs whose size and little-endian target are asserted at compile
time; a reader casts a payload in place when the buffer is aligned. The rkyv archives are
validated whole before use through `archives::codec::access_checked`. Every read failure is a
value, never a panic: `archives::codec::BinaryError` for the archives, containers and rows,
`density::tbdd::TbddError` for the density grids, and the crate's `Error`, which wraps either.
Every identifier field of a record is a type of its own (`ids`), declared with the `newtype_ids`
macros, whose bytes in rkyv, in the object row and in JSON are exactly its inner value's, so the
identifier types change no committed file.

## Getting started

Run from the repository root:

```bash
cargo test -p world_file_formats   # every format's unit tests; reads the Everon density tiles (Git LFS)
```

## Configuration

None: the crate reads no environment variable and declares no feature. The density tests read
the committed Everon tiles under `assets/terrains/everon/objects/density/`; fetch them with
`git lfs pull --include "assets/terrains/everon/objects/density/*"` in a fresh checkout.

## Public surface

- `archives`: the archive types (`RoadNetworkArchive`, `MapLabelsArchive`, `WaterVectorsArchive`,
  `PrefabCatalogArchive`, `TypeInventory`, `ForestRegionsArchive`, `BuildingBlueprintArchive`,
  `TbdSatIndexV2`), `version::ARCHIVE_SCHEMA_VERSION`, and `codec` (`BinaryError`,
  `access_checked`, `to_bytes`).
- `containers`: `header` (`ContainerHeader`, `HEADER_BYTES`, `CONTAINER_VERSION`,
  `peek_version`) and the four headers `TbdcHeader`, `TbdeHeader`, `TbdbHeader` (with
  `LevelSpan`) and `TbdsHeader`, with their magics.
- `density::tbdd`: `decode_tbdd`, `encode_tbdd`, `TbddHeader`, `TbddGrid` and `TbddError`.
- `pod::instance`: `ObjectInstancePod`, `POD_BYTES`, `POD_NAME` and the byte casts.
- `ids`: `PrefabId` and its 16-bit row form `InstancePrefabId`, `TerrainId`, `RoadSegmentId`,
  `ForestRegionId`, `WaterFeatureId`, `WallId`, `DoorId`, `WindowId`, `StairsId` and
  `FurnitureId`, each with its archived form's accessors.
- `Error` and `Result`: either error family behind one type.
- `prelude`: the names most readers and writers import.

## Boundaries

- Depends on: `newtype_ids` (the identifier macros), `bytemuck`, `rkyv` and `thiserror`.
- Used by: the streaming crates `chunk_scheduler` and `map_asset_loading`, the terrain, world
  object and line of sight crates, whose loaders and decoders read the files, and the developer
  tools (`tools/developer_tools`), whose export, raster and blueprint
  pipelines write and verify them.
- Rules:
  - a committed file under `assets/terrains/` or `contracts/fixtures/map/` keeps reading, so a
    layout change comes with a new version, a reader for it and regenerated files;
  - rkyv stays `little_endian` with `bytecheck`, and an archive is read only through
    `access_checked`;
  - world formats tier 1, so the crate depends on tier 0 crates only, here `newtype_ids`
    (`cargo xtask verify crate-tiers`);
  - no record field is a bare primitive identifier (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Terrain assets](/assets/terrains/README.md) — the served terrain tree these formats make up.
- [Terrain manifest schema](/contracts/definitions/terrain-manifest.schema.json) — the manifest
  block that names the container, its version and the row shape.
