# Map Labels & Contours (`map_raster_pipeline/map_labels`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Exports cartographic town, hill, and water location names, and generates elevation contour labels.

Decomposed from `labels.rs` (520 LOC) and `labels_emit.rs` (403 LOC).

---

## Submodules

- **`location_names.rs`** (<280 LOC): Parses settlement, landmark, and marine feature JSONL records.
- **`height_contours.rs`** (<240 LOC): Samples DEM elevation rasters to generate topographic contour labels.
- **`binary_archiver.rs`** (<400 LOC): Emits high-speed zero-copy rkyv label catalogs for client map rendering.
