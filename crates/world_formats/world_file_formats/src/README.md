# World file formats source

The source of `world_file_formats`: the fixed-header containers, the object instance row, the rkyv
archives and the vegetation density tiles, the error that gathers their failures, and the crate
root that declares them. Every format is little-endian and validated on read.

## Contents

```text
crates/world_formats/world_file_formats/src/
├── archives/    the rkyv archives, their validating reader and writer, and the shared `BinaryError`
├── containers/  the 32-byte-header containers `TBDC`, `TBDE`, `TBDB` and `TBDS`
├── density/     the `TBDD` vegetation density tile codec
├── error.rs     `Error` and `Result`: a `BinaryError` or a `TbddError` behind one type
├── ids/         the identifier types the archive records and the object row hold
├── lib.rs       the crate root: module header, `mod` lines and re-exports
├── pod/         `ObjectInstancePod`, the 32-byte world object row inside `TBDC` chunks
└── prelude.rs   the names most readers and writers import
```

## How it works

```text
tools/developer_tools (world export, map raster pipeline, blueprint tooling)
   │ encode_tbdd, TbdcHeader + instances_to_bytes, Tbd*Header::new, to_bytes
   ▼
assets/terrains/<terrain>/   served under /map-assets
   │ objects/chunks/*.bin (TBDC)   objects/density/*.bin (TBDD)   *.rkyv   *.tbd-sat (TBDS)
   │ dem/elevation.dem (TBDE)      water/bathymetry.tbd-bath (TBDB)
   ▼
terrain, world object, world format, line of sight and streaming crates (the readers)
     ContainerHeader::read, TbdcHeader::instances, access_checked, decode_tbdd
```

Each format has one definition here that both sides compile against, so a writer and a reader
cannot disagree on a layout. The fixed layouts (the container headers, the instance row and the
density header) are `#[repr(C)]` `bytemuck::Pod` structs whose size, alignment and little-endian
target are asserted at compile time, and a reader casts a payload in place when its buffer is
aligned. The rkyv archives are validated whole before use. Every read failure is a value, never a
panic: `BinaryError` from `archives/` for the containers, the row and the archives, and
`TbddError` for the density tiles; `error::Error` wraps either for a caller that reads both. A
container reader checks the magic, then the version, and
refuses a version it does not implement; the density decoder checks the magic only.

No mission crate depends on this crate, so the mission build the API links carries neither the
crate nor `rkyv`.

## Public surface

- `archives::codec`: `BinaryError`, `access_checked` and `to_bytes`, and the archive types, for the
  map engine's loaders and the developer tools.
- `containers`: `ContainerHeader`, `HEADER_BYTES`, `CONTAINER_VERSION` and the four header types.
- `pod::instance`: `ObjectInstancePod`, `POD_BYTES`, `POD_NAME` and the byte casts.
- `density::tbdd`: `decode_tbdd`, `encode_tbdd`, `TbddGrid` and `TbddError`.
- `ids`: the prefab, terrain feature and building element identifier types.
- `Error` and `Result` (`error.rs`), and `prelude`.

## Boundaries

- Depends on: `newtype_ids`, `rkyv`, `bytemuck` and `thiserror`.
- Used by:
  - the map engine's `streaming::loaders` (chunks, the manifest's row check, the prefab
    catalogue), `world` (DEM, water, satellite, roads, labels, prefabs, forest regions, density
    tiles, building blueprints) and `spatial::los::world` (building descriptors);
  - the developer tools in `tools/developer_tools/src/`: the world export pipeline, the map
    raster pipeline, the blueprint archive writer and the map verifications, as writers and
    checkers.
- Rules:
  - a committed file under `assets/terrains/` or `contracts/fixtures/map/` must keep
    reading, so a layout change comes with a new version, a reader for it and regenerated files;
  - rkyv stays `little_endian` with `bytecheck` (the crate's `Cargo.toml`), and the `Pod`
    layouts refuse to compile on a big-endian target;
  - no mission crate may name the formats: a `crates/mission` crate depends on no
    `crates/world_formats` crate (`cargo xtask verify crate-tiers`).

## Related documentation

- [Terrain assets](/assets/terrains/README.md) — the served terrain tree these formats make up.
- [Terrain manifest schema](/contracts/definitions/terrain-manifest.schema.json) — the manifest
  block that names the container, its version and the row shape.
- [BVH sidecars](/crates/geometry/spatial_indexes/src/bounding_volume_hierarchy/README.md) — the `TBVH` building sidecar,
  a binary format that lives with the spatial index rather than here.
