# Water export image modules

The submodules of `water_export_images.rs`, the `map water-images` lane: they find a
[Workbench](/documentation/glossary/n_to_z.md#workbench) water export, rasterize its grids and
vectors into a water class mask and a depth grid, and write the five water PNG images.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/water_export_images/
├── dem_sampling.rs           `DemHeightfield`: the `--dem` 16-bit PNG and its nearest-sample terrain height
├── export_location.rs        the water folder search, the metadata, grid and vector file candidates, the image prefix and folder
├── json_field_reading.rs     the export JSON readers: given values with their fallbacks, JSON numbers, NaN coordinates
├── polygon_rasterization.rs  the even-odd scanline fill of lake and pond rings with their depth rule
├── raster_geometry.rs        the image size and sample spacing from the meta, `--res` and `--roi`, and the region suffix
├── river_rasterization.rs    the Catmull-Rom ribbon stamp of river centre lines with the parabolic channel depth
├── vector_rasterization.rs   reads the lake, pond and river files and the heightfield, and draws them in order
├── water_grid_decoding.rs    the streaming digit-run reader of the ASCII mask and depth grids
├── water_image_outputs.rs    the class and depth statistics and the five PNG writers
└── water_raster.rs           `WaterRaster`: the mask and depth grids, the keep-the-deeper merge, NaN-propagating min and max
```

## How it works

```text
--export-dir ─▶ export_location ─▶ water folder, meta, grid and vector paths, image folder
meta + --res + --roi ─▶ raster_geometry ─▶ SampleGrid (size, origin, spacing), region suffix
bathymetry_mask.txt  ─┐
bathymetry_depth.txt ─┴▶ water_grid_decoding ─▶ WaterRaster   (export grid only)
lakes.json, ponds.json ─▶ polygon_rasterization ─┐
rivers.json            ─▶ river_rasterization   ─┴▶ WaterRaster (vector_rasterization, unless --no-vector-enhance)
WaterRaster ─▶ water_image_outputs ─▶ -bathymetry, -bathymetry-dark, -depth-16bit, -mask, -preview .png
```

- The water folder is the first of the export folder, `<terrain>/terrain/water`,
  `<terrain>/water`, `terrain/water`, `water` and `$tbd_framework:worlds/water` that holds a
  metadata file or the mask grid. `water_meta.json` is preferred over `inland_water_meta.json` and
  the legacy `TBD_WaterExport_meta.json` / `TBD_InlandWaterExport_meta.json`; an inland metadata
  file names the images `<terrain>-inland-water-*`, every other `<terrain>-water-*`. Without
  `--out-dir` the images go to `images/` under the water folder.
- The image grid is the meta's own (`widthPx` × `heightPx`, default 12800) unless `--res` resizes
  the world or `--roi` cuts a region of `max(16, round(extent / resolution))` pixels, suffixed
  `-roi-<min x>_<min z>`. Samples span the extent end to end, `extent / (pixels − 1)` apart.
- The ASCII grids are read only on the export's own grid. Their reader stores every run of digits
  in order, wrapped to the sample type, and ignores everything else.
- Lakes, then ponds, fill as water class 2 by the even-odd rule; rivers stamp class 3 ribbons
  last. A depth merges into the raster only when it is deeper than the stored one. A lake or
  pond's depth is its exported average (or `0.6 × maxDepthM`, or 1.5 m); with `--dem` a sample
  whose terrain lies below the surface takes the water column above the terrain instead.
- Every image is north-up. Every rounding that can meet a negative value or a tie uses
  `grid_rasterization`'s ties-toward-+∞ rounding; `f64::round` appears only on values that are
  never negative.
- The export JSON is read by one truthiness rule: `null`, `false`, `0` and `""` count as absent
  and fall back; arrays and objects count even when empty; a value that is not a number never
  stands in for a size, width or depth.

## Boundaries

- Depends on: `grid_rasterization` (half-up rounding, the Catmull-Rom spline, the scanline sample
  grid), `water_bodies::bathymetry_palette` (the depth ramps, contours and dark land colour),
  `terrain_elevation`'s `decode_png_gray16`, `serde_json`, and the crate's
  `image_operations::png_writing` and `error` modules.
- Used by: `tools/map_assets/map_raster_pipeline/src/water_export_images.rs`, whose `run` the
  `map water-images` command calls.
- Rules: a synthetic export draws exactly the reference pixels in all five images
  (`synthetic_export_draws_the_reference_pixels` in
  `tools/map_assets/map_raster_pipeline/src/tests/water_export_images_tests.rs`); the grid reader's
  wrap, truncation and separator rules
  (`tools/map_assets/map_raster_pipeline/src/tests/water_export_images_grid_decoding_tests.rs`);
  the folder search order (`tools/map_assets/map_raster_pipeline/src/tests/water_export_images_location_tests.rs`);
  the geometry rules (`tools/map_assets/map_raster_pipeline/src/tests/water_export_images_geometry_tests.rs`).
