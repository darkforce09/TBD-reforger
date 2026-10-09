# Image operation modules

The submodules of `image_operations.rs`: the streaming PNG writer the export image lanes write
every image through.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/image_operations/
└── png_writing.rs  `write_png_rows` and `PngPixelLayout`: a PNG written row by row through one reusable row buffer
```

## How it works

`write_png_rows` opens the file, writes the header of the layout's colour type and bit depth
(8-bit grey, 16-bit grey, 8-bit RGB or 8-bit RGBA, never interlaced), then asks the caller to fill
one row buffer per row, top to bottom, and streams each row through `png`'s stream writer at the
default zlib level with no row filter. A 16-bit grey sample is filled most significant byte
first. No image is held whole, so a 12800 × 12800 export costs one row of memory.

## Boundaries

- Depends on: the `png` crate and the crate's `Error` (`crate::error`).
- Used by: the water and road export image lanes (`water_export_images.rs`,
  `road_export_images.rs`).
- Rules: every image round-trips byte for byte through the decoder in each layout
  (`rgba8_rows_round_trip` and its siblings in
  `tools/map_assets/map_raster_pipeline/src/tests/png_writing_tests.rs`).
