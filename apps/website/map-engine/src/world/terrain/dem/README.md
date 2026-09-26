# Elevation model

The terrain's elevation model: the manifest that places the height raster on the world, the
decoders that turn the 16-bit PNG or the raw `TBDE` grid into a metres cache, bilinear sampling,
and the downsampled grid that contours, the sea band, the airfield apron and the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s height readout read.

## Contents

```text
apps/website/map-engine/src/world/terrain/dem/
├── grid.rs      `DemVectorGrid`: the box-averaged metres grid, its 2× reduction and its sampling
├── loader.rs    the browser fetch of a manifest-declared raw grid, streamed with progress reports
├── manifest.rs  `DemManifest`: the raster's world rectangle, size, axis flips and height range
├── mod.rs       the module tree
├── png.rs       16-bit PNG decode into samples and into the `f32` metres cache
├── raw.rs       the `TBDE` raw grid: whole or streamed decode, and the framing an emitter writes
├── sample/      the sampling and terrain line-of-sight items under one path, and their tests
├── sampling.rs  sample-to-metres conversion, world-to-pixel mapping and bilinear sampling
└── tests/       unit tests for the vector grid, the PNG decode and the raw grid
```

## How it works

A terrain manifest's `dem` block names the PNG and its encoding. Everon's
(`assets_v2/terrains/everon/manifest.json`) names `dem/everon-dem-16bit.png`: 6400 × 6400 samples
at 2 m, `uint16-linear` from −204.78 m to 375.53 m, no axis flip. A manifest may also declare a
`raw` block (`dem/elevation.dem`, encoding `tbde-v1`). At boot `crate::streaming::host` asks
`load_declared_raw` for the raw grid first, which streams it only when the block names an encoding
this build reads; otherwise it fetches the PNG and runs `decode_png_to_meters`. Either path yields
a `DecodedDem`: one `f32` height in metres per sample, row-major. The `TBDE` header is
`crate::io::containers::tbde::TbdeHeader`.

| Source | Layout | Sample to metres |
|---|---|---|
| PNG | 16-bit, channel 0 read big-endian | `min + v / 65535 · (max − min)` (`uint16_to_meters`) |
| `TBDE` | a `TbdeHeader`, then `width × height` little-endian `u16` | `offset_m + v · scale_m` |

`RawDemSink` decodes the raw grid chunk by chunk as the response arrives: it checks the length the
header declares against the whole file's length before it allocates, fills one sample vector that
never moves, carries a sample split across two chunks, and `finish` refuses a payload that is short
or long.

`world_to_pixel` maps a world `(x, z)` onto continuous pixel coordinates across the manifest's
rectangle, the far corner landing on the last pixel, mirrored on a flipped axis;
`sample_elevation_meters` (a `u16` raster) and `sample_elevation_from_meters_cache` (the `f32`
cache) interpolate bilinearly and answer `None` off the raster. `downsample_dem_grid` box-averages
the cache by `DEM_VECTOR_GRID_FACTOR` (4), Everon's 6400² samples at 2 m becoming a 1600² grid of
8 m cells, and records its highest value, which bounds the contour levels; `reduce_grid_2x` halves
a grid for the coarse contour intervals, and `sample_grid_meters` reads a height from it.

## Public surface

- `manifest`: `DemManifest` and `PixelCoord`.
- `sampling`: `uint16_to_meters`, `meters_cache`, `world_to_pixel`, `bilinear_sample`,
  `sample_elevation_meters`, `sample_elevation_from_meters_cache` and `in_coverage`.
- `png`: `decode_png_gray16`, `decode_png_to_meters`, `DecodedDem` and `PngError`.
- `raw`: `RawDem`, `RawDemSink` and `to_bytes`; `loader::load_declared_raw`.
- `grid`: `DemVectorGrid`, `DEM_VECTOR_GRID_FACTOR`, `downsample_dem_grid`, `reduce_grid_2x` and
  `sample_grid_meters`.

## Boundaries

- Depends on: `crate::io::containers` and `crate::io::archives::codec` (the `TBDE` header and its
  errors); `crate::camera::math::shaping` (rounding); the `png` crate; and, for the loader,
  `crate::streaming` (the manifest's raw block, boot progress) and the browser fetch.
- Used by:
  - `crate::streaming::host`, which boots the elevation model and keeps the vector grid;
  - `crate::world::terrain::relief` (contours, sea band), `crate::world::terrain::roads` (the
    airfield apron) and `crate::world::environment::locations` (spot heights);
  - `crate::spatial::los::terrain` and `crate::editing::tools::line_of_sight`, which take a
    `DemManifest`;
  - the Mission Creator's canvas and pointer handlers in
    `apps/website/frontend/src/v2/apps/editor/`, which read heights with `sample_grid_meters`;
  - the world export in `tools_v2/developer-tools/src/world_export_pipeline/`, which writes
    `dem/elevation.dem` with `raw::to_bytes`, and the label and alignment checks in
    `tools_v2/developer-tools/src/map_raster_pipeline/` and
    `tools_v2/developer-tools/src/map_verification/`.
- Rules: `png.rs` compiles with the `world` feature, `raw.rs` with `io` and `loader.rs` only for
  wasm32 with `render`; the raw grid decodes the same in any chunk size and from a misaligned
  buffer (`streamed_in_any_chunk_size_matches_the_whole_buffer`,
  `misaligned_payload_decodes_identically_to_the_aligned_one` in `tests/raw_tests.rs`); dimensions
  the file cannot hold are refused before any allocation
  (`hostile_dimensions_are_rejected_before_allocating`); the raw grid and the PNG decode to the same
  samples, and over Everon's height range to metres within 0.1 mm
  (`dem_and_png_decode_to_the_same_grid_and_metres`,
  `everon_range_keeps_the_grid_exact_and_metres_within_f32_rounding`); the box average keeps a
  constant grid constant (`box_average_of_constant_is_constant` in `tests/grid_tests.rs`).
