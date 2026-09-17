# Aerial Orthophoto Pipeline (`map_raster_pipeline/aerial_orthophoto`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Stitches and color-blends high-resolution aerial imagery tiles from Enfusion supertextures.

Decomposed from the 1,070-line legacy `sap.rs` file into modular single-responsibility files (<350 LOC each).

---

## Submodules

- **`seam_metrics.rs`** (<300 LOC): Computes inter-cell gradient deltas across tile borders.
- **`stitcher.rs`** (<350 LOC): Decodes 50×50 EDDS grid into a continuous 12,800×12,800 pixel canvas.
- **`seam_blender.rs`** (<250 LOC): Smooths visible color steps across adjacent tile boundaries.
- **`verifier.rs`** (<200 LOC): Validates orientation angles and global standard deviations.
