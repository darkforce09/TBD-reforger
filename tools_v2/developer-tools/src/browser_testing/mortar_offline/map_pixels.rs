//! Whether the map canvas in a screenshot shows drawn imagery.
//!
//! **Role:** measures the map's rectangle of a full-viewport screenshot: how many distinct
//! colours it holds and how much its brightness varies, and judges whether map imagery drew.
//! **Position:** the last step of `super::run`, over `Page.captureScreenshot` at a device scale
//! factor of 1, so CSS pixels are image pixels.
//! **Signals & state:** none; pure functions over PNG bytes.
//! **Invariants:** a flat or near-flat rectangle (an empty canvas, a loading card) never counts
//! as drawn; the rectangle is clipped to the image, and an empty clip is an error.

use anyhow::{Result, anyhow, bail};
use std::collections::HashSet;

/// A rectangle in CSS pixels: left, top, width, height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CssRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// What a rectangle of the screenshot holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionStats {
    /// Distinct RGB colours.
    pub distinct_colours: usize,
    /// Standard deviation of the Rec. 601 luma, 0–255.
    pub luma_std_dev: f64,
}

/// The fewest distinct colours drawn map imagery shows.
pub const MIN_DISTINCT_COLOURS: usize = 256;

/// The least luma standard deviation drawn map imagery shows.
pub const MIN_LUMA_STD_DEV: f64 = 4.0;

/// Measures `rect` of the PNG `png`.
///
/// # Errors
///
/// When the PNG does not decode or the rectangle lies outside it.
pub fn region_stats(png: &[u8], rect: CssRect) -> Result<RegionStats> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|e| anyhow!("decode screenshot: {e}"))?
        .to_rgb8();
    let (w, h) = image.dimensions();
    let clamp = |v: f64, max: u32| v.max(0.0).min(f64::from(max)) as u32;
    let (x0, y0) = (clamp(rect.x, w), clamp(rect.y, h));
    let (x1, y1) = (
        clamp(rect.x + rect.width, w),
        clamp(rect.y + rect.height, h),
    );
    if x1 <= x0 || y1 <= y0 {
        bail!("the map rectangle {rect:?} lies outside the {w}×{h} screenshot");
    }
    let mut colours = HashSet::new();
    let (mut sum, mut sum_sq, mut n) = (0.0_f64, 0.0_f64, 0.0_f64);
    for y in y0..y1 {
        for x in x0..x1 {
            let [r, g, b] = image.get_pixel(x, y).0;
            colours.insert((r, g, b));
            let luma = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b);
            sum += luma;
            sum_sq += luma * luma;
            n += 1.0;
        }
    }
    let mean = sum / n;
    Ok(RegionStats {
        distinct_colours: colours.len(),
        luma_std_dev: (sum_sq / n - mean * mean).max(0.0).sqrt(),
    })
}

/// Whether `stats` shows drawn map imagery.
#[must_use]
pub fn imagery_drew(stats: RegionStats) -> bool {
    stats.distinct_colours >= MIN_DISTINCT_COLOURS && stats.luma_std_dev >= MIN_LUMA_STD_DEV
}

#[cfg(test)]
#[path = "../tests/mortar_offline/map_pixels.rs"]
mod tests;
