//! The map-asset image pipeline (ports of the scripts/map-assets image lane:
//! stitch/blend/seam-metrics, unified satellite, tile pyramid, glyph atlas, landcover,
//! water composite/analyze, cartographic compose, location/height-label exporters).
//! Pure Rust: png/image (decode+Lanczos), image-webp (lossless), webp (the ONE lossy leg —
//! vendored libwebp C, N3), resvg (SVG raster + road strokes).

use anyhow::{Result, bail};

pub mod aerial_orthophoto;
pub mod cartographic_rendering;
pub mod glyphs;
pub mod image_operations;
pub mod inland_water;
/// `water/water_vectors.rkyv` + `water/bathymetry.tbd-bath` from the Workbench inland-
/// water staging export. Separate from `inland_water`, which is the *image* lane (inland-water
/// classifier + ortho tint) and shares nothing with it but the word: this module reads the staging
/// rasters and writes binaries. Same split as `map_labels` versus `map_label_archives`.
pub mod inland_water_archive;
/// `locations/map_labels.rkyv`, the binary twin of `locations.json` +
/// `height-labels.json` + `road-names.json`. Separate from `map_labels`, which *produces* two of
/// those three JSON files: this module only ever reads them.
pub mod map_label_archives;
pub mod map_labels;
pub mod satellite_archive;
/// The `TBDS` v2 satellite container (32-byte header + rkyv `TbdSatIndexV2`). Split
/// out of `satellite_archive`, which is already a SIZE-1 file and keeps only the call sites.
pub mod satellite_archive_container;

/// Refuse structurally empty / vacuous overwrites of committed map assets.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/module/refuse_empty_tests.rs"]
mod refuse_empty_tests;

pub mod cli;
