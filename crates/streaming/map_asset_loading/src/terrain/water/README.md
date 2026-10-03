# Water loader

The browser side of a terrain's water: the browser load of the bathymetry and the inland
water archive when a terrain manifest declares them. The water data itself (the bathymetry mask, the
suffix plan, the inland archive, the sea fill mesh) is the
[`water_bodies`](/crates/terrain/water_bodies/README.md) crate, which every caller imports
directly.

## Contents

```text
crates/streaming/map_asset_loading/src/terrain/water/
├── loader.rs  `WaterHost`: loads the manifest's water files, the bathymetry as a coarse suffix
└── mod.rs     the module tree: `loader`
```

## How it works

A terrain manifest's `water` block names `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath`
and the encoding `tbdb-v1`; `WaterHost::init` skips a block with another encoding or an empty
path, and Everon's manifest (`assets/terrains/everon/manifest.json`) has none. The host fetches
the bathymetry header by one Range request and, by another, the tail `suffix_plan` picks within
`MAX_BATHYMETRY_BYTES` (16 MiB); the vectors archive is fetched whole; each file may fail alone.
The host keeps the `WaterMask` and the `WaterVectors` but draws neither.

## Boundaries

- Depends on: `water_bodies`, `world_file_formats` (the `TBDB` header),
  `world_chunks::terrain_manifest` (the manifest's water block), `browser_platform::fetch` (Range
  fetches) and `map_streaming_model` (boot progress).
- Used by: the map host of `map_streaming_host`, which owns the `WaterHost` and answers `is_water` and
  `is_known_dry_land` from its mask.
- Rules: `loader.rs` compiles only for wasm32.
