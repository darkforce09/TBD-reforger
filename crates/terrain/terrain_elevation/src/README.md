# Terrain elevation source

The source of `terrain_elevation`: the raster's placement, the two decoders, the sampling
functions, the two grids, the error the decoders share, and the crate root that declares them.

## Contents

```text
crates/terrain/terrain_elevation/src/
├── error.rs            `Error` and `Result`: a `PngError` or a raw grid `BinaryError` behind one type
├── full_resolution.rs  `FullResolutionDem`: the native `u16` raster, its handle, its 2 m lookup
├── grid.rs             `DemVectorGrid`: the box-averaged metres grid, its 2× reduction, its sampling
├── lib.rs              the crate root: module header, `mod` lines and re-exports
├── manifest.rs         `DemManifest`: the raster's world rectangle, size, axis flips and height range
├── png.rs              16-bit PNG decode into samples and into the `f32` metres cache
├── prelude.rs          the names most readers import
├── raw.rs              the `TBDE` raw grid: whole or streamed decode, and the emitter framing
├── sampling.rs         sample-to-metres conversion, world-to-pixel mapping and bilinear sampling
└── tests/              unit tests: full-resolution raster, vector grid, PNG decode, raw grid, sampling
```

## How it works

`manifest` and `sampling` are the base: every other module converts samples with
`uint16_to_meters` or interpolates with `bilinear_sample`. `png` and `raw` produce samples, `grid`
averages a metres cache into the vector grid, and `full_resolution` keeps the samples at their
native spacing behind a shared handle.

## Boundaries

- Depends on: `world_file_formats`, `map_coordinates`, `png`, `bytemuck` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
