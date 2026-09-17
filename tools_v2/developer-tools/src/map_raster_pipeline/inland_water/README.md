# Inland Water Pipeline (`map_raster_pipeline/inland_water`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Detects inland lakes, rivers, and ponds, applying water tinting and mask contours.

Decomposed from the 943-line legacy `water.rs` file into modular components (<480 LOC each).

---

## Submodules

- **`classifier.rs`** (<480 LOC): Connected component labeling and wet/dry pixel classification.
- **`compositor.rs`** (<450 LOC): Composites nautical water tints over base orthophoto layers.
