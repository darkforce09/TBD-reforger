# 2D Map Raster Pipeline (`developer-tools/src/map_raster_pipeline`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Processes satellite imagery, elevation data, and vector labels into web-optimized multi-resolution raster tiles.

Eradicates cryptic abbreviations (`sap.rs`, `tbds_v2.rs`, `water.rs`, `labels.rs`) in compliance with **Law 4 (Zero Context Needed)**.

---

## Submodules

- **`aerial_orthophoto/`** (formerly `sap.rs`): Decodes 2,500 Enfusion supertexture EDDS tiles, stitches them into a 12,800² px aerial image, and blends visible seam lines.
- **`satellite_container/`** (formerly `tbds_v2.rs` & `unified.rs`): Emits `.tbd-sat` binary containers with zero-copy rkyv index headers and VP8L WebP tile payloads.
- **`inland_water/`** (formerly `water.rs`): Detects water pixels and blends tactical water tints onto orthophotos.
- **`water_vector_export/`** (formerly `water_emit.rs`): Generates shoreline contour polygons and bathymetry depth grids.
- **`cartographic_composer/`** (formerly `carto.rs`): Blends landcover color masks with SVG vector road strokes.
- **`map_labels/`** (formerly `labels.rs` & `labels_emit.rs`): Samples elevation contours and exports town/hill labels to `.rkyv`.
- **`image_codecs.rs`** (formerly `img.rs`): Fast lossless WebP, PNG, and Lanczos3 resamplers.
- **`tactical_glyph_atlas.rs`** (formerly `glyphs.rs`): Rasterizes NATO MIL-STD-2525 SVG icons into texture atlases.
