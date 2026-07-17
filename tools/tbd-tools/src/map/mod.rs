//! T-165.9 — the map-asset image pipeline (ports of the scripts/map-assets image lane:
//! stitch/blend/seam-metrics, unified satellite, tile pyramid, glyph atlas, landcover,
//! water composite/analyze, cartographic compose, location/height-label exporters).
//! Pure Rust: png/image (decode+Lanczos), image-webp (lossless), webp (the ONE lossy leg —
//! vendored libwebp C, N3), resvg (SVG raster + road strokes).

pub mod carto;
pub mod glyphs;
pub mod img;
pub mod labels;
pub mod sap;
pub mod unified;
pub mod water;
