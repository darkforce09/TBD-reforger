# Terrain elevation

The `terrain_elevation` crate: a terrain's elevation model. It places the height raster on the
world, decodes the 16-bit PNG or the raw `TBDE` grid into a metres cache, samples heights
bilinearly, and box-averages the cache into the vector grid that the contours, the sea band, the
airfield apron, the line of sight and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s height readout read.

## Contents

```text
crates/terrain/terrain_elevation/
├── Cargo.toml  the package: `world_file_formats`, `map_coordinates`, `png`, layout tier 2
└── src/        the manifest, sampling, PNG and raw decoders, vector grid, full-resolution raster
```

## How it works

A terrain manifest's `dem` block names the PNG and its encoding. Everon's
(`assets/terrains/everon/manifest.json`) names `dem/everon-dem-16bit.png`: 6400 × 6400 samples
at 2 m, `uint16-linear` from −204.78 m to 375.53 m, no axis flip. A manifest may also declare a
`raw` block (`dem/elevation.dem`, encoding `tbde-v1`). The map engine's terrain boot streams the
raw grid through `RawDemSink` when the block names an encoding this build reads, and otherwise
fetches the PNG and runs `decode_png_to_meters`. Either path yields a `DecodedDem`: one `f32`
height in metres per sample, row-major. The `TBDE` header is
`world_file_formats::containers::tbde::TbdeHeader`.

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

`FullResolutionDem` keeps the source `u16` samples at their native spacing with the linear
`SampleEncoding` of either source, over the manifest's `worldBounds` footprint, oriented like the
vector grid (sample `(0, 0)` at the footprint's south-west corner). `height_at` interpolates
bilinearly and answers `None` off the footprint. The terrain boot publishes it into a
`FullResolutionDemHandle` only when its scope keeps the raster (the terrain-and-imagery scope a
fire-planning map uses); the Mission Creator's full scope leaves the handle empty.

## Getting started

Run from the repository root:

```bash
cargo test -p terrain_elevation   # sampling, vector grid, PNG, raw grid and full-resolution tests
```

## Public surface

- `manifest`: `DemManifest` and `PixelCoord`.
- `sampling`: `uint16_to_meters`, `meters_cache`, `world_to_pixel`, `bilinear_sample`,
  `sample_elevation_meters`, `sample_elevation_from_meters_cache` and `in_coverage`.
- `png`: `decode_png_gray16`, `decode_png_to_meters`, `DecodedDem` and `PngError`.
- `raw`: `RawDem`, `RawDemSink` and `to_bytes`.
- `grid`: `DemVectorGrid`, `DEM_VECTOR_GRID_FACTOR`, `APRON_DEM_DOWNSAMPLE_FACTOR`,
  `dem_grid_dims`, `downsample_dem_grid`, `reduce_grid_2x` and `sample_grid_meters`.
- `full_resolution`: `FullResolutionDem`, `SampleEncoding`, `RasterFootprint`,
  `FullResolutionDemHandle`, `new_full_resolution_dem_handle` and `height_from_handle`.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `world_file_formats` (the `TBDE` header and the `BinaryError` the raw grid
  reports); `map_coordinates` (JavaScript rounding, which sizes the vector grid); `png`,
  `bytemuck` and `thiserror`.
- Used by:
  - `terrain_relief` (contours, sea band), which marches the vector grid;
  - the streaming crates (`map_streaming_host`'s terrain boot and `map_asset_loading`'s raw grid
    loader and airfield apron), `place_names` (the spot heights), `road_network`,
    `terrain_line_of_sight` and `map_editing_tools`, which take a `DemManifest`;
  - the Mission Creator's canvas and pointer handlers in `crates/frontend/workspaces/mission_creator_workspace/src/`,
    which read heights with `sample_grid_meters`;
  - the world export in `tools/map_assets/world_export_pipeline/src/`, which writes
    `dem/elevation.dem` with `raw::to_bytes`, and the label and alignment checks in
    `tools/map_assets/map_raster_pipeline/src/` and
    `tools/map_assets/map_asset_verification/src/`.
- Rules: the raw grid decodes the same in any chunk size and from a misaligned buffer
  (`streamed_in_any_chunk_size_matches_the_whole_buffer`,
  `misaligned_payload_decodes_identically_to_the_aligned_one` in `src/tests/raw_tests.rs`);
  dimensions the file cannot hold are refused before any allocation
  (`hostile_dimensions_are_rejected_before_allocating`); the raw grid and the PNG decode to the
  same samples, and over Everon's height range to metres within 0.1 mm
  (`dem_and_png_decode_to_the_same_grid_and_metres`,
  `everon_range_keeps_the_grid_exact_and_metres_within_f32_rounding`); the box average keeps a
  constant grid constant (`box_average_of_constant_is_constant` in `src/tests/grid_tests.rs`); a
  stored 0 reads exactly the minimum height and 65 535 the maximum (`zero_is_exact_min`,
  `full_scale_is_max_within_epsilon` in `src/tests/sampling_tests.rs`); the world rectangle's
  corners map to the first and last pixel, mirrored when the manifest flips an axis
  (`world_to_pixel_endpoints`, `world_to_pixel_axis_flip`); a point off the raster samples as
  `None` (`sample_elevation_out_of_bounds_is_none`); terrain tier 2 (`cargo xtask verify
  crate-tiers`).

## Related documentation

- [Everon dataset](/assets/terrains/everon/README.md) — the elevation files this crate decodes.
