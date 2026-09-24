# Map binary formats

The on-disk formats of a terrain's served map data, shared by the developer tools that write them
and the map engine that reads them in the
[Mission Creator](/documentation_v2/glossary.md#mission-creator): the fixed-header containers, the
object instance row, the rkyv archives and the vegetation density tiles. Every format is
little-endian and validated on read.

## Contents

```text
apps/website/map-engine/src/io/
├── archives/    the rkyv archives, their validating reader and writer, and the shared `BinaryError`
├── containers/  the 32-byte-header containers `TBDC`, `TBDE`, `TBDB` and `TBDS`
├── density/     the `TBDD` vegetation density tile codec
├── mod.rs       the module tree
└── pod/         `ObjectInstancePod`, the 32-byte world object row inside `TBDC` chunks
```

## How it works

```text
tools_v2/developer-tools (world export, map raster pipeline, blueprint tooling)
   │ encode_tbdd, TbdcHeader + instances_to_bytes, Tbd*Header::new, to_bytes
   ▼
assets_v2/terrains/<terrain>/   served under /map-assets
   │ objects/chunks/*.bin (TBDC)   objects/density/*.bin (TBDD)   *.rkyv   *.tbd-sat (TBDS)
   │ dem/elevation.dem (TBDE)      water/bathymetry.tbd-bath (TBDB)
   ▼
crate::streaming::loaders, crate::world, crate::spatial::los::world
     ContainerHeader::read, TbdcHeader::instances, access_checked, decode_tbdd
```

Each format has one definition here that both sides compile against, so a writer and a reader
cannot disagree on a layout. The fixed layouts (the container headers, the instance row and the
density header) are `#[repr(C)]` `bytemuck::Pod` structs whose size, alignment and little-endian
target are asserted at compile time, and a reader casts a payload in place when its buffer is
aligned. The rkyv archives are validated whole before use. Every read failure is a value, never a
panic: `BinaryError` from `archives/` for the containers, the row and the archives, and
`TbddError` for the density tiles. A container reader checks the magic, then the version, and
refuses a version it does not implement; the density decoder checks the magic only.

The module compiles with the `io` feature (`apps/website/map-engine/src/lib.rs`), which brings in
`rkyv` with `bytecheck` and `little_endian`; the `scenario` build the API links does not include
it.

## Public surface

- `archives::codec`: `BinaryError`, `access_checked` and `to_bytes`, and the archive types, for the
  loaders in `crate::streaming`, `crate::world` and `crate::spatial` and the developer tools.
- `containers`: `ContainerHeader`, `HEADER_BYTES`, `CONTAINER_VERSION` and the four header types.
- `pod::instance`: `ObjectInstancePod`, `POD_BYTES`, `POD_NAME` and the byte casts.
- `density::tbdd`: `decode_tbdd`, `encode_tbdd`, `TbddGrid` and `TbddError`.

## Boundaries

- Depends on: `rkyv` and `bytemuck`; nothing else in the crate.
- Used by:
  - `crate::streaming::loaders` (chunks, the manifest's row check, the prefab catalogue),
    `crate::world` (DEM, water, satellite, roads, labels, prefabs, forest regions, density tiles,
    building blueprints) and `crate::spatial::los::world` (building descriptors);
  - the developer tools in `tools_v2/developer-tools/src/`: the world export pipeline, the map
    raster pipeline, the blueprint archive writer and the map verifications, as writers and
    checkers.
- Rules:
  - a committed file under `assets_v2/terrains/` or `contracts_v2/fixtures/map/` must keep
    reading, so a layout change comes with a new version, a reader for it and regenerated files;
  - rkyv stays `little_endian` with `bytecheck` (`apps/website/map-engine/Cargo.toml`), and the
    `Pod` layouts refuse to compile on a big-endian target;
  - nothing under `crate::data` may name the module (rules 4 and 7 of
    `cargo xtask verify engine-layers`).

## Related documentation

- [Terrain assets](/assets_v2/terrains/README.md) — the served terrain tree these formats make up.
- [Terrain manifest schema](/contracts_v2/definitions/terrain-manifest.schema.json) — the manifest
  block that names the container, its version and the row shape.
- [BVH sidecars](/apps/website/map-engine/src/spatial/bvh/README.md) — the `TBVH` building sidecar,
  a binary format that lives with the spatial index rather than here.
