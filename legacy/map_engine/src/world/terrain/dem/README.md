# Elevation model loader

The map engine's side of the terrain's elevation model: the browser fetch of a raw `TBDE` grid
that a terrain manifest declares. The model itself (placement, decoders, sampling, the vector grid,
the full-resolution raster) is the [`terrain_elevation`](/crates/terrain/terrain_elevation/README.md)
crate, which every caller imports directly.

## Contents

```text
legacy/map_engine/src/world/terrain/dem/
├── loader.rs  the browser fetch of a manifest-declared raw grid, streamed with progress
└── mod.rs     the module tree: `loader`
```

## How it works

At boot `crate::streaming::host` asks the loader for the raw grid first: `raw_block_is_readable`
accepts a `raw` block only when it names an encoding this build reads (`tbde-v1`), and the loader
then streams the file through `terrain_elevation::raw::RawDemSink`, reporting byte progress, into
the same `DecodedDem` the PNG path produces. Without a readable block the host fetches the PNG and
runs `terrain_elevation::png::decode_png_to_meters`.

## Boundaries

- Depends on: `terrain_elevation`, `world_chunks::terrain_manifest` (the manifest's raw block)
  and `browser_platform::fetch` (the streamed GET and its byte progress, which the streaming host
  turns into boot progress).
- Used by: `crate::streaming::host`, which boots the elevation model.
- Rules: `loader.rs` compiles only for wasm32 with the `render` feature.
