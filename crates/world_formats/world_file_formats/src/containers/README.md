# Binary map containers

The four fixed-header binary containers a terrain's map data ships in: object chunks (`TBDC`), the
elevation grid (`TBDE`), the water depth pyramid (`TBDB`) and the satellite archive (`TBDS`). Each
starts with a 32-byte header that names its format and version and says how long the payload is.

## Contents

```text
crates/world_formats/world_file_formats/src/containers/
├── header.rs  `ContainerHeader`, `HEADER_BYTES`, `CONTAINER_VERSION` and `peek_version`
├── mod.rs     the module tree and the header test modules
├── tbdb.rs    `TbdbHeader` and `LevelSpan`: the water depth and mask pyramid
├── tbdc.rs    `TbdcHeader`: one world object chunk of `ObjectInstancePod` rows
├── tbde.rs    `TbdeHeader`: the quantised elevation grid
├── tbds.rs    `TbdsHeader`: the satellite archive, its rkyv index and its tiles
└── tests/     unit tests: sizes, wire layouts, round trips, level spans and every refusal path
```

## How it works

Every header is `#[repr(C)]`, `Pod`, little-endian and exactly `HEADER_BYTES` (32) long, and the
payload starts right after it. Its first six bytes are always the four-byte magic and a `u16`
version, which is what `peek_version` reads without choosing a header type. The
`ContainerHeader` trait gives each header its `MAGIC`, `VERSION` and `NAME` and three readers:

- `validate` checks the magic first, then the version, and returns `BadMagic` or
  `UnsupportedVersion` (both `crate::archives::codec::BinaryError`);
- `parse` borrows the header in place and fails with `Misaligned` on a buffer that is not
  aligned; `read` copies the header out and works on any buffer, and it is what every reader in
  the crate calls;
- `to_header_bytes` gives the 32 bytes a writer puts first.

| Magic | File under a terrain's folder | Version | Payload |
|---|---|---|---|
| `TBDC` | `objects/chunks/{cx}_{cy}.bin` | 1 | `count` rows of 32 bytes, nothing else; `cx` and `cy` repeat the file name |
| `TBDE` | `dem/elevation.dem` | 1 | `width × height` `u16` samples, row-major, row 0 at the north edge; metres = `offset_m + sample · scale_m` |
| `TBDB` | `water/bathymetry.tbd-bath` | 1 | `mip_count` levels, each `u16` depths then `u8` water mask, padded to 4 bytes; metres = `depth · depth_scale` |
| `TBDS` | `satellite/{terrain}-sat.tbd-sat` | 2 | `index_len` bytes of rkyv `TbdSatIndexV2`, then the tile bytes |

The size helpers (`payload_bytes`, `file_bytes`, `sample_count`, `level_span`, `level_dims`) use
checked arithmetic and return `None` rather than wrap on a 32-bit target. `TbdbHeader::level_span`
computes a level's offsets from the header alone, so a reader can fetch one level by HTTP Range;
its offsets count from the payload start, and `HEADER_BYTES` more gives the file offset. `TBDS`
tile offsets count from `tiles_offset()`, the end of the index.

## Public surface

- `header`: `ContainerHeader` (with `read`), `HEADER_BYTES` and `CONTAINER_VERSION`, for the readers
  in `map_engine::streaming::loaders` and `map_engine::world::terrain` and the writers in the developer tools.
- `tbdc::TbdcHeader` for `map_engine::streaming::loaders::chunk_bin`; `tbde::TbdeHeader` for
  `map_engine::world::terrain::dem`; `tbdb::TbdbHeader` for `map_engine::world::terrain::water`;
  `tbds::TbdsHeader` and `TBDS_VERSION_V2` for `map_engine::world::terrain::satellite`.

## Boundaries

- Depends on: `crate::archives::codec::BinaryError`, `crate::pod::instance` (the `TBDC`
  row) and `bytemuck`.
- Used by:
  - the map engine's readers: `map_engine::streaming::loaders::chunk_bin` (`TBDC`),
    `map_engine::world::terrain::dem::raw` (`TBDE`), `map_engine::world::terrain::water` (`TBDB`) and
    `map_engine::world::terrain::satellite::streamer` (`TBDS`); `map_engine::streaming::loaders::manifest`
    compares the manifest's `containerVersion` with `CONTAINER_VERSION`;
  - the developer tools' writers: `tools/map_assets/world_export_pipeline/src/binary_emit.rs`
    (`TBDC`), `tools/map_assets/world_export_pipeline/src/export_preparation/dem_elevation.rs`
    (`TBDE`), `tools/map_assets/map_raster_pipeline/src/inland_water_archive.rs` (`TBDB`)
    and `tools/map_assets/map_raster_pipeline/src/satellite_archive_container.rs`
    (`TBDS`), plus the map verifications in `tools/map_assets/map_asset_verification/src/`.
- Rules:
  - every header is 32 bytes (a compile-time assertion in each file, and
    `all_headers_are_thirty_two_bytes` in `tests/container_header_tests.rs`);
  - a reader refuses a version other than the header's `VERSION` instead of guessing, and a short
    or wrongly sized buffer is an error, never a panic (the refusal tests in the same file);
  - a new container gets its own four-byte magic and a header type that implements
    `ContainerHeader`.

## Related documentation

- [Terrain assets](/assets/terrains/README.md) — the served terrain tree the containers sit in.
- [Map loaders](/legacy/map_engine/src/streaming/loaders/README.md) — the chunk and manifest
  parsers that read `TBDC`.
