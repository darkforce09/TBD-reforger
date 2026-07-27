//! T-165.9 — the map-asset image pipeline (ports of the scripts/map-assets image lane:
//! stitch/blend/seam-metrics, unified satellite, tile pyramid, glyph atlas, landcover,
//! water composite/analyze, cartographic compose, location/height-label exporters).
//! Pure Rust: png/image (decode+Lanczos), image-webp (lossless), webp (the ONE lossy leg —
//! vendored libwebp C, N3), resvg (SVG raster + road strokes).

use anyhow::{Result, bail};

pub mod carto;
pub mod glyphs;
pub mod img;
pub mod labels;
pub mod sap;
pub mod unified;
pub mod water;

/// T-537 / T-383 — refuse structurally empty / vacuous overwrites of committed map assets.
pub(crate) fn refuse_empty_write(context: &str, empty: bool, detail: &str) -> Result<()> {
    if empty {
        bail!("refusing empty write ({context}): {detail}");
    }
    Ok(())
}

#[cfg(test)]
mod refuse_empty_tests {
    use super::refuse_empty_write;

    #[test]
    fn refuse_empty_write_reds_on_empty() {
        let err = refuse_empty_write("probe", true, "structurally empty").expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("refusing empty write (probe)"), "{msg}");
        assert!(msg.contains("structurally empty"), "{msg}");
    }

    #[test]
    fn refuse_empty_write_ok_when_nonempty() {
        refuse_empty_write("probe", false, "unused").expect("non-empty must pass");
    }
}
